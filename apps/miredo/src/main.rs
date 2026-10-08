mod data;
mod domain;
mod pdf;
mod resources;
mod search;
mod storage;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1320.0, 880.0])
            .with_min_inner_size([820.0, 600.0])
            .with_title("MiReDo"),
        ..Default::default()
    };

    eframe::run_native(
        "MiReDo",
        options,
        Box::new(
            |creation_context| match ui::MiReDoApp::new(creation_context) {
                Ok(app) => Ok(Box::new(app)),
                Err(error) => Ok(Box::new(StartupFailureApp {
                    message: format!("{error:#}"),
                })),
            },
        ),
    )
}

struct StartupFailureApp {
    message: String,
}

impl eframe::App for StartupFailureApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(context, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(90.0);
                ui.heading("MiReDo n'a pas pu démarrer");
                ui.add_space(12.0);
                ui.label(&self.message);
                ui.add_space(18.0);
                ui.label("Vérifiez les ressources locales et redémarrez l'application.");
            });
        });
    }
}
