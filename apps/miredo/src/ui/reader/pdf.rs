//! PDF reader rendering and controls.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_pdf_viewer(
        &mut self,
        context: &EguiContext,
        ui: &mut egui::Ui,
        song: &Song,
    ) {
        let Some(path) = song.pdf_path.as_ref() else {
            ui.vertical_centered(|ui| {
                ui.add_space(45.0);
                ui.label(
                    RichText::new(self.tr("reader.pdf_missing"))
                        .size(16.0)
                        .color(self.palette().get("text_secondary")),
                );
            });
            return;
        };

        let total_pages = self.pdf_page_counts.get(&song.id).copied();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 3.0);
            let previous_enabled = self.pdf_page > 1;
            let next_enabled = total_pages.is_none_or(|total| self.pdf_page < total);
            if pdf_toolbar_icon_button(
                ui,
                AppIcon::Previous,
                self.palette().get("text_primary"),
                self.tr("common.previous"),
                previous_enabled,
            )
            .clicked()
            {
                self.set_pdf_page(&song.id, self.pdf_page.saturating_sub(1));
            }
            let count = total_pages
                .map(|total| {
                    format!(
                        "{} {} {} {total}",
                        self.tr("reader.page"),
                        self.pdf_page,
                        self.tr("reader.of")
                    )
                })
                .unwrap_or_else(|| format!("{} {}", self.tr("reader.page"), self.pdf_page));
            ui.label(count);
            if pdf_toolbar_icon_button(
                ui,
                AppIcon::Next,
                self.palette().get("text_primary"),
                self.tr("common.next"),
                next_enabled,
            )
            .clicked()
            {
                self.set_pdf_page(&song.id, self.pdf_page + 1);
            }
            ui.separator();
            if pdf_toolbar_icon_button(
                ui,
                AppIcon::ZoomOut,
                self.palette().get("text_primary"),
                self.tr("reader.zoom_out"),
                true,
            )
            .clicked()
            {
                self.change_zoom(-0.1);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if pdf_toolbar_icon_button(
                ui,
                AppIcon::ZoomIn,
                self.palette().get("text_primary"),
                self.tr("reader.zoom_in"),
                true,
            )
            .clicked()
            {
                self.change_zoom(0.1);
            }
            ui.separator();
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
            if pdf_toolbar_icon_button(
                ui,
                AppIcon::Fullscreen,
                self.palette().get("text_primary"),
                self.tr("reader.fullscreen"),
                true,
            )
            .clicked()
            {
                self.fullscreen = !self.fullscreen;
                context.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
        });

        let pages = pdf_visible_pages(self.pdf_layout, self.pdf_page, total_pages);
        let available = ui.available_size();
        for page in &pages {
            if total_pages.is_none_or(|total| *page <= total) {
                self.request_pdf_page(song, *page, available);
            }
        }
        egui::Frame::new()
            .fill(self.palette().get("viewer_background"))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ScrollArea::both()
                    .id_salt("pdf-reading-surface")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if pages.len() == 2 {
                            ui.columns(2, |columns| {
                                for (index, page) in pages.iter().enumerate() {
                                    self.draw_pdf_page(
                                        &mut columns[index],
                                        &song.id,
                                        *page,
                                        Vec2::new(
                                            (available.x / 2.0).max(1.0),
                                            available.y.max(1.0),
                                        ),
                                    );
                                }
                            });
                        } else {
                            self.draw_pdf_page(
                                ui,
                                &song.id,
                                self.pdf_page,
                                Vec2::new(available.x.max(1.0), available.y.max(1.0)),
                            );
                        }
                    });
            });
        let _ = path;
    }

    pub(in crate::ui) fn draw_pdf_page(
        &self,
        ui: &mut egui::Ui,
        song_id: &str,
        page: usize,
        available: Vec2,
    ) {
        let render_zoom = self.pdf_fit_zoom(available);
        let key = pdf::page_key(song_id, page, render_zoom);
        if let Some(texture) = self.pdf_textures.get(&key) {
            let size = texture.size_vec2();
            let logical_size = size / render_zoom.max(0.01);
            let safe_available = pdf_safe_available(available);

            let scale = match self.pdf_fit_mode {
                PdfFitMode::FitWidth | PdfFitMode::FitHeight => {
                    let width_scale = (safe_available.x / logical_size.x.max(1.0)).clamp(0.1, 6.0);
                    let height_scale = (safe_available.y / logical_size.y.max(1.0)).clamp(0.1, 6.0);
                    width_scale.min(height_scale)
                }
                PdfFitMode::Manual => self.zoom.clamp(0.1, 6.0),
            };

            let display_size = (logical_size * scale).min(safe_available);
            ui.centered_and_justified(|ui| {
                ui.set_min_height(safe_available.y);
                ui.set_min_width(display_size.x);
                ui.image((texture.id(), display_size));
            });
        } else if self.pdf_pending.contains(&key) {
            ui.centered_and_justified(|ui| {
                ui.label(self.tr("common.loading"));
            });
        } else if self.pdf_errors.contains_key(&key) {
            ui.centered_and_justified(|ui| {
                ui.label(self.tr("reader.pdf_error"));
            });
        }
    }

    pub(in crate::ui) fn set_pdf_page(&mut self, song_id: &str, page: usize) {
        self.pdf_page = page.max(1);
        if let Err(error) = self
            .storage
            .save_reading_position(song_id, "pdf", self.pdf_page as f64)
        {
            self.show_error(error);
        }
    }
}
