//! Home screen and collection shortcuts.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_home(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            RichText::new(self.tr("home.greeting"))
                .size(26.0)
                .strong()
                .color(self.palette().get("text_primary")),
        );
        ui.label(
            RichText::new(self.tr("home.subtitle"))
                .size(14.0)
                .color(self.palette().get("text_secondary")),
        );
        ui.add_space(18.0);

        let action_label = self.tr("home.library_link");
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - 232.0) * 0.5).max(0.0));
            let browse_button = ui.add_sized(
                [232.0, 42.0],
                egui::Button::new("")
                    .fill(self.palette().get("accent"))
                    .stroke(Stroke::NONE),
            );
            let action_color = self.palette().get("accent_text");
            let action_font = egui::FontId::proportional(14.0);
            let action_galley = ui.painter().layout_no_wrap(
                action_label.clone(),
                action_font.clone(),
                action_color,
            );
            let content_width = 24.0 + 8.0 + action_galley.size().x;
            let content_left = browse_button.rect.center().x - content_width * 0.5;
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(content_left + 10.0, browse_button.rect.center().y),
                Vec2::splat(20.0),
            );
            let button_painter = ui.painter().with_clip_rect(browse_button.rect.shrink(8.0));
            paint_app_icon(ui, icon_rect, AppIcon::Browse, action_color);
            button_painter.text(
                egui::pos2(content_left + 24.0 + 8.0, browse_button.rect.center().y),
                egui::Align2::LEFT_CENTER,
                action_label,
                action_font,
                action_color,
            );
            if browse_button.clicked() {
                self.page = Page::Library;
            }
        });

        ui.add_space(24.0);
        ui.label(RichText::new(self.tr("home.continue")).size(17.0).strong());
        ui.add_space(8.0);
        if let Some(last_song_id) = self.last_song_id.clone() {
            if let Some(song) = self.song(&last_song_id).cloned() {
                let title = self.title_for_song(&song);
                let subtitle = self.reader_subtitle(&song);
                egui::Frame::new()
                    .fill(self.palette().get("surface"))
                    .stroke(Stroke::new(1.0_f32, self.palette().get("border_subtle")))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(format!("{}  ·  {title}", song.display_number()))
                                        .size(16.0)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(subtitle)
                                        .size(12.0)
                                        .color(self.palette().get("text_secondary")),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(self.tr("common.open"))
                                                .strong()
                                                .color(self.palette().get("accent_text")),
                                        )
                                        .fill(self.palette().get("accent"))
                                        .min_size(Vec2::new(92.0, 36.0)),
                                    )
                                    .clicked()
                                {
                                    self.open_song(
                                        song.id.clone(),
                                        self.songs
                                            .iter()
                                            .map(|candidate| candidate.id.clone())
                                            .collect(),
                                    );
                                }
                            });
                        });
                    });
            } else {
                egui::Frame::new()
                    .fill(self.palette().get("surface"))
                    .stroke(Stroke::new(1.0_f32, self.palette().get("border_subtle")))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(self.tr("home.continue_empty"));
                    });
            }
        } else {
            egui::Frame::new()
                .fill(self.palette().get("surface"))
                .stroke(Stroke::new(1.0_f32, self.palette().get("border_subtle")))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(self.tr("home.continue_empty"));
                });
        }

        ui.add_space(24.0);
        ui.label(
            RichText::new(self.tr("home.collections"))
                .size(17.0)
                .strong(),
        );
        ui.add_space(10.0);
        let counts = data::collection_counts(&self.songs);
        let available_width = ui.available_width();
        let column_count = if available_width >= 960.0 {
            4
        } else if available_width >= 560.0 {
            2
        } else {
            1
        };
        let gap = ui.spacing().item_spacing.x;
        let tile_width =
            ((available_width - gap * (column_count - 1) as f32) / column_count as f32).max(1.0);
        for row in Collection::ALL.chunks(column_count) {
            ui.horizontal(|ui| {
                for collection in row {
                    let count = counts.get(collection).copied().unwrap_or_default();
                    let response = ui.add_sized(
                        [tile_width, 72.0],
                        egui::Button::new("")
                            .fill(self.palette().get("surface"))
                            .stroke(Stroke::new(1.0_f32, self.palette().get("border_subtle"))),
                    );
                    let tile_painter = ui.painter().with_clip_rect(response.rect.shrink(10.0));
                    tile_painter.text(
                        egui::pos2(response.rect.left() + 14.0, response.rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        self.collection_label(*collection),
                        egui::FontId::proportional(14.0),
                        self.palette().get("text_primary"),
                    );
                    tile_painter.text(
                        egui::pos2(response.rect.right() - 14.0, response.rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        count.to_string(),
                        egui::FontId::proportional(16.0),
                        self.palette().get("accent"),
                    );
                    if response.clicked() {
                        self.collection_filter = Some(*collection);
                        self.page = Page::Library;
                    }
                }
            });
            ui.add_space(gap);
        }
    }
}
