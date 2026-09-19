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

const DESC_TEXT_SIZE: f32 = 17.0;

fn desc_label(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.label(egui::RichText::new(text.into()).size(DESC_TEXT_SIZE))
}

fn big_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized([120.0, 44.0], egui::Button::new(egui::RichText::new(text).size(18.0)))
}

fn draw_header(ui: &mut egui::Ui, logo: &egui::TextureHandle, large: bool) {
    let (logo_size, title) = if large {
        (96.0, egui::RichText::new("DreamOS Machine Score").size(40.0).strong())
    } else {
        (40.0, egui::RichText::new("DreamOS Machine Score").size(18.0).strong())
    };
    ui.horizontal(|ui| {
        ui.image((logo.id(), egui::vec2(logo_size, logo_size)));
        ui.label(title);
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

        // Bottom action bar, right-aligned, on every screen - added before
        // CentralPanel so it reserves its space and the button(s) stay
        // pinned to the window bottom regardless of content length above.
        //
        // exact_height is required: without it, right_to_left(Align::Center)
        // has no fixed height to center within, so the panel's intrinsic
        // height calculation feeds back into itself frame over frame -
        // visibly a runaway-growing bottom bar that eventually covers the
        // whole window, worst during continuous repaint (Scoring screen).
        egui::TopBottomPanel::bottom("actions").exact_height(64.0).show(ctx, |ui| {
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match self.screen {
                    Screen::Welcome => {
                        if big_button(ui, "Start").clicked() {
                            self.screen = Screen::Scoring;
                        }
                    }
                    Screen::Scoring => {
                        let running = self.scoring_rx.is_some();
                        ui.add_enabled_ui(!running, |ui| {
                            if big_button(ui, "Run Scoring").clicked() {
                                self.start_scoring();
                            }
                        });
                    }
                    Screen::Results => {
                        // right_to_left: first added ends up rightmost, so
                        // add Close first to keep "Restart  Close" reading
                        // order left-to-right.
                        if big_button(ui, "Close").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if big_button(ui, "Restart").clicked() {
                            self.result = None;
                            self.screen = Screen::Welcome;
                        }
                    }
                }
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            draw_header(ui, &logo, self.screen == Screen::Welcome);

            // Scrollable: Results in particular can be taller than the
            // window (score + all component/readiness lines), and without
            // this the last lines were silently clipped at the window edge
            // instead of being reachable.
            egui::ScrollArea::vertical().show(ui, |ui| {
            match self.screen {
                Screen::Welcome => {
                    desc_label(
                        ui,
                        "DreamOS Machine Score evaluates whether this PC is ready for \
                         gaming and for game development with Godot or Unreal Engine 5. \
                         It detects your CPU, GPU, RAM, and storage, scores them, and \
                         shows a readiness breakdown for each target.",
                    );
                }
                Screen::Scoring => {
                    if self.scoring_rx.is_some() {
                        desc_label(ui, self.progress_label.clone());
                        ui.add(egui::widgets::Spinner::new());
                    }

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
                    } else if self.scoring_rx.is_some() {
                        ctx.request_repaint();
                    }
                }
                Screen::Results => {
                    if let Some(result) = self.result.clone() {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("{:.0}", result.composite)).size(64.0).strong());
                        desc_label(ui, "Overall Score (0-100)");
                        ui.add_space(16.0);

                        desc_label(ui, format!("CPU: {}", result.cpu_model));
                        desc_label(ui, format!("GPU: {}", result.gpu_name));
                        desc_label(ui, format!("RAM: {:.0} GiB", result.ram_total_gb));
                        ui.add_space(8.0);

                        for (label, component) in [
                            ("CPU", result.scores.cpu),
                            ("GPU", result.scores.gpu),
                            ("RAM", result.scores.ram),
                            ("Storage", result.scores.storage),
                        ] {
                            let note = if component.estimated { " (estimated - model not in database)" } else { "" };
                            desc_label(ui, format!("{label} score: {:.0}{note}", component.score));
                        }

                        ui.add_space(16.0);
                        ui.heading("Readiness");
                        desc_label(ui, format!("Gaming: {}", tier_label(result.readiness.gaming)));
                        desc_label(ui, format!("Godot: {}", tier_label(result.readiness.godot)));
                        desc_label(ui, format!("Unreal Engine 5: {}", tier_label(result.readiness.unreal_engine_5)));
                    } else {
                        desc_label(ui, "No result yet.");
                    }
                }
            }
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 650.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DreamOS Machine Score",
        options,
        Box::new(|_cc| Ok(Box::new(MachineScoreApp::new()))),
    )
}
