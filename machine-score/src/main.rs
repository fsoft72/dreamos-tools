use eframe::egui;

const LOGO_BYTES: &[u8] = include_bytes!("../assets/ball.png");

fn load_logo_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let img = image::load_from_memory(LOGO_BYTES)
        .expect("embedded assets/ball.png is a valid PNG")
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture("logo", color_image, egui::TextureOptions::default())
}

fn tier_label(tier: machine_score::scoring::Tier) -> &'static str {
    use machine_score::scoring::Tier;
    match tier {
        Tier::Low => "Low",
        Tier::Medium => "Medium",
        Tier::High => "High",
        Tier::Ultra => "Ultra",
    }
}

fn draw_header(ui: &mut egui::Ui, logo: &egui::TextureHandle) {
    ui.horizontal(|ui| {
        ui.image((logo.id(), egui::vec2(48.0, 48.0)));
        ui.heading("DreamOS Machine Score");
    });
    ui.separator();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Scoring,
    Results,
}

enum ScoringMsg {
    DetectingCpu,
    DetectingGpu,
    DetectingRam,
    DetectingStorage,
    Done(ResultData),
}

#[derive(Clone)]
struct ResultData {
    cpu_model: String,
    gpu_name: String,
    ram_total_gb: f64,
    scores: machine_score::scoring::ComponentScores,
    composite: f64,
    readiness: machine_score::scoring::TargetReadiness,
}

struct MachineScoreApp {
    logo: Option<egui::TextureHandle>,
    screen: Screen,
    scoring_rx: Option<std::sync::mpsc::Receiver<ScoringMsg>>,
    progress_label: String,
    result: Option<ResultData>,
}

impl MachineScoreApp {
    fn new() -> Self {
        Self {
            logo: None,
            screen: Screen::Welcome,
            scoring_rx: None,
            progress_label: String::new(),
            result: None,
        }
    }

    fn start_scoring(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.scoring_rx = Some(rx);
        self.progress_label = "Starting...".into();

        std::thread::spawn(move || {
            use machine_score::{hardware, scoring};

            let _ = tx.send(ScoringMsg::DetectingCpu);
            let cpu_ram_storage = hardware::detect_cpu_ram_storage();

            let _ = tx.send(ScoringMsg::DetectingGpu);
            let gpu = hardware::detect_gpu();

            let _ = tx.send(ScoringMsg::DetectingRam);
            let ram_score = scoring::score_ram(cpu_ram_storage.ram_total_gb);

            let _ = tx.send(ScoringMsg::DetectingStorage);
            let storage_score = scoring::score_storage(cpu_ram_storage.storage_kind);

            let cpu_db = scoring::load_cpu_database();
            let gpu_db = scoring::load_gpu_database();
            let cpu_score = scoring::score_cpu(
                &cpu_ram_storage.cpu_model,
                cpu_ram_storage.cpu_cores,
                cpu_ram_storage.cpu_base_clock_mhz,
                &cpu_db,
            );
            let gpu_score = scoring::score_gpu(&gpu.name, gpu.is_discrete, &gpu_db);

            let scores = scoring::ComponentScores {
                cpu: cpu_score,
                gpu: gpu_score,
                ram: ram_score,
                storage: storage_score,
            };
            let composite = scoring::composite_score(&scores);
            let readiness = scoring::target_readiness(composite);

            let _ = tx.send(ScoringMsg::Done(ResultData {
                cpu_model: cpu_ram_storage.cpu_model,
                gpu_name: gpu.name,
                ram_total_gb: cpu_ram_storage.ram_total_gb,
                scores,
                composite,
                readiness,
            }));
        });
    }
}

impl eframe::App for MachineScoreApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let logo = self.logo.get_or_insert_with(|| load_logo_texture(ctx)).clone();

        egui::CentralPanel::default().show(ctx, |ui| {
            draw_header(ui, &logo);

            match self.screen {
                Screen::Welcome => {
                    ui.label(
                        "DreamOS Machine Score evaluates whether this PC is ready for \
                         gaming and for game development with Godot or Unreal Engine 5. \
                         It detects your CPU, GPU, RAM, and storage, scores them, and \
                         shows a readiness breakdown for each target.",
                    );
                    ui.add_space(12.0);
                    if ui.button("Start").clicked() {
                        self.screen = Screen::Scoring;
                    }
                }
                Screen::Scoring => {
                    if self.scoring_rx.is_none() {
                        if ui.button("Run Scoring").clicked() {
                            self.start_scoring();
                        }
                    } else {
                        ui.label(&self.progress_label);
                        ui.add(egui::widgets::Spinner::new());

                        let mut finished = false;
                        if let Some(rx) = &self.scoring_rx {
                            for msg in rx.try_iter() {
                                match msg {
                                    ScoringMsg::DetectingCpu => self.progress_label = "Detecting CPU...".into(),
                                    ScoringMsg::DetectingGpu => self.progress_label = "Detecting GPU...".into(),
                                    ScoringMsg::DetectingRam => self.progress_label = "Detecting RAM...".into(),
                                    ScoringMsg::DetectingStorage => self.progress_label = "Detecting storage...".into(),
                                    ScoringMsg::Done(data) => {
                                        self.result = Some(data);
                                        finished = true;
                                    }
                                }
                            }
                        }
                        if finished {
                            self.scoring_rx = None;
                            self.screen = Screen::Results;
                        } else {
                            ctx.request_repaint();
                        }
                    }
                }
                Screen::Results => {
                    if let Some(result) = self.result.clone() {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("{:.0}", result.composite)).size(64.0).strong());
                        ui.label("Overall Score (0-100)");
                        ui.add_space(16.0);

                        ui.label(format!("CPU: {}", result.cpu_model));
                        ui.label(format!("GPU: {}", result.gpu_name));
                        ui.label(format!("RAM: {:.0} GiB", result.ram_total_gb));
                        ui.add_space(8.0);

                        for (label, component) in [
                            ("CPU", result.scores.cpu),
                            ("GPU", result.scores.gpu),
                            ("RAM", result.scores.ram),
                            ("Storage", result.scores.storage),
                        ] {
                            let note = if component.estimated { " (estimated - model not in database)" } else { "" };
                            ui.label(format!("{label} score: {:.0}{note}", component.score));
                        }

                        ui.add_space(16.0);
                        ui.heading("Readiness");
                        ui.label(format!("Gaming: {}", tier_label(result.readiness.gaming)));
                        ui.label(format!("Godot: {}", tier_label(result.readiness.godot)));
                        ui.label(format!("Unreal Engine 5: {}", tier_label(result.readiness.unreal_engine_5)));

                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.button("Restart").clicked() {
                                self.result = None;
                                self.screen = Screen::Welcome;
                            }
                            if ui.button("Close").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    } else {
                        ui.label("No result yet.");
                    }
                }
            }
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DreamOS Machine Score",
        options,
        Box::new(|_cc| Ok(Box::new(MachineScoreApp::new()))),
    )
}
