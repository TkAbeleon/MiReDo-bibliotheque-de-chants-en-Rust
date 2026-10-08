//! Appearance, language, reader and data settings.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_settings(&mut self, ui: &mut egui::Ui) {
        self.settings_card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            self.settings_section(ui, "settings.appearance");
            ui.horizontal_wrapped(|ui| {
                ui.label(self.tr("settings.theme"));
                for (mode, key) in [
                    (ThemeMode::System, "settings.theme_system"),
                    (ThemeMode::Light, "settings.theme_light"),
                    (ThemeMode::Dark, "settings.theme_dark"),
                ] {
                    if ui
                        .selectable_label(self.theme_mode == mode, self.tr(key))
                        .clicked()
                    {
                        self.set_theme_mode(mode);
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(self.tr("settings.language"));
                for (locale, label) in [("fr", "FR"), ("mg", "MG"), ("en", "EN")] {
                    if ui.selectable_label(self.locale == locale, label).clicked() {
                        self.set_locale(locale);
                    }
                }
            });
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            self.settings_section(ui, "settings.reader");
            ui.horizontal_wrapped(|ui| {
                ui.label(self.tr("settings.pdf_layout"));
                if ui
                    .selectable_label(
                        self.pdf_layout == PdfLayout::Single,
                        self.tr("reader.single_page"),
                    )
                    .clicked()
                {
                    self.set_pdf_layout(PdfLayout::Single);
                }
                if ui
                    .selectable_label(
                        self.pdf_layout == PdfLayout::Double,
                        self.tr("reader.book_mode"),
                    )
                    .clicked()
                {
                    self.set_pdf_layout(PdfLayout::Double);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(self.tr("settings.initial_zoom"));
                if icon_button(
                    ui,
                    AppIcon::ZoomOut,
                    self.palette().get("text_primary"),
                    self.tr("reader.zoom_out"),
                )
                .clicked()
                {
                    self.change_zoom(-0.1);
                }
                ui.label(format!("{:.0}%", self.zoom * 100.0));
                if icon_button(
                    ui,
                    AppIcon::ZoomIn,
                    self.palette().get("text_primary"),
                    self.tr("reader.zoom_in"),
                )
                .clicked()
                {
                    self.change_zoom(0.1);
                }
            });
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            self.settings_section(ui, "settings.pdf_fit");
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(
                        self.pdf_fit_mode == PdfFitMode::FitWidth,
                        self.tr("reader.fit_width"),
                    )
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitWidth);
                }
                if ui
                    .selectable_label(
                        self.pdf_fit_mode == PdfFitMode::FitHeight,
                        self.tr("reader.fit_height"),
                    )
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitHeight);
                }
                if ui
                    .selectable_label(
                        self.pdf_fit_mode == PdfFitMode::Manual,
                        self.tr("reader.manual_zoom"),
                    )
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::Manual);
                }
            });
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            self.settings_section(ui, "settings.data");
            let data_path = PathBuf::from(APP_DIR).join("data").display().to_string();
            ui.add(
                egui::Label::new(format!("{}: {data_path}", self.tr("settings.data_path")))
                    .truncate(),
            )
            .on_hover_text(&data_path);
            if let Some(path) = UserStorage::database_path() {
                let path = path.display().to_string();
                ui.add(
                    egui::Label::new(format!("{}: {path}", self.tr("settings.database_path")))
                        .truncate(),
                )
                .on_hover_text(&path);
            }
            ui.horizontal_wrapped(|ui| {
                for (key, value) in [
                    ("settings.loaded", self.report.loaded),
                    ("settings.with_lyrics", self.report.with_lyrics),
                    ("settings.with_authors", self.report.with_authors),
                    ("settings.with_pdf", self.report.with_pdf),
                    ("settings.invalid", self.report.invalid_records),
                ] {
                    ui.label(format!("{}: {value}", self.tr(key)));
                }
            });
            if ui.button(self.tr("settings.reload")).clicked() {
                let data_dir = PathBuf::from(APP_DIR).join("data");
                match data::load_library(&data_dir) {
                    Ok((songs, report)) => {
                        self.songs = songs;
                        self.report = report;
                        self.pdf_textures.clear();
                        self.pdf_texture_order.clear();
                        self.pdf_page_counts.clear();
                        self.set_status(self.tr("settings.reloaded"));
                    }
                    Err(error) => {
                        self.set_status(format!("{}: {error}", self.tr("settings.reload_failed")));
                    }
                }
            }
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            self.settings_section(ui, "settings.keyboard");
            for (key, shortcut) in [
                ("settings.shortcut_search", "Ctrl / ⌘  K"),
                ("settings.shortcut_next", "N / PageDown"),
                ("settings.shortcut_previous", "P / PageUp"),
                ("settings.shortcut_favorite", "F"),
                ("settings.shortcut_viewer", "V"),
                ("settings.shortcut_escape", "Esc"),
            ] {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.tr(key));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(shortcut)
                                .monospace()
                                .color(self.palette().get("text_muted")),
                        );
                    });
                });
            }
        });
    }

    pub(in crate::ui) fn settings_card_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.palette().get("surface_elevated"))
            .stroke(Stroke::new(1.0_f32, self.palette().get("border_subtle")))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(20))
    }

    fn settings_section(&self, ui: &mut egui::Ui, key: &str) {
        ui.label(
            RichText::new(self.tr(key))
                .size(16.0)
                .strong()
                .color(self.palette().get("accent")),
        );
        ui.add_space(10.0);
    }
}
