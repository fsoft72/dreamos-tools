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
                    ui.label("Results screen - Task 10");
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
