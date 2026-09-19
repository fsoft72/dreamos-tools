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

struct MachineScoreApp {
    logo: Option<egui::TextureHandle>,
    screen: Screen,
}

impl MachineScoreApp {
    fn new() -> Self {
        Self { logo: None, screen: Screen::Welcome }
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
                    ui.label("Scoring screen - Task 9");
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
