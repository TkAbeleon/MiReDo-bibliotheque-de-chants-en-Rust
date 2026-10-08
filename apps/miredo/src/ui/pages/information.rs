//! Help and application information screens.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_help(&self, ui: &mut egui::Ui) {
        for (title, body) in [
            ("help.search_title", "help.search_body"),
            ("help.reader_title", "help.reader_body"),
            ("help.organize_title", "help.organize_body"),
            ("help.offline_title", "help.offline_body"),
        ] {
            ui.label(RichText::new(self.tr(title)).size(16.0).strong());
            ui.add_space(4.0);
            ui.label(RichText::new(self.tr(body)).color(self.palette().get("text_secondary")));
            ui.add_space(20.0);
        }
    }

    pub(in crate::ui) fn draw_about(&self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            RichText::new("MiReDo")
                .size(30.0)
                .strong()
                .color(self.palette().get("accent")),
        );
        ui.label(self.tr("about.description"));
        ui.add_space(18.0);
        ui.label(format!(
            "{}  {}",
            self.tr("about.version"),
            env!("CARGO_PKG_VERSION")
        ));
        ui.add_space(15.0);
        ui.label(RichText::new(self.tr("about.data_source")).strong());
        ui.hyperlink_to(
            "TkAbeleon/Fihirana-FFPM",
            "https://github.com/TkAbeleon/Fihirana-FFPM",
        );
        ui.add_space(5.0);
        ui.label(
            RichText::new(self.tr("about.source_notice"))
                .size(12.0)
                .color(self.palette().get("warning")),
        );
        ui.add_space(15.0);
        ui.label(RichText::new(self.tr("about.docs")).strong());
        ui.label("docs/miredo/");
    }
}
