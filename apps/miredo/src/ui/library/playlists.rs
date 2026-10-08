//! Playlist selection and management screen.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_playlists(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if icon_button(
                ui,
                AppIcon::Add,
                self.palette().get("accent"),
                self.tr("playlists.create"),
            )
            .clicked()
            {
                self.playlist_dialog = Some(PlaylistDialog::Create);
                self.playlist_dialog_name.clear();
                self.dialog_error = None;
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new(self.tr("playlists.heading"))
                    .size(15.0)
                    .strong()
                    .color(self.palette().get("text_primary")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.selected_playlist_id.is_some()
                    && icon_button(
                        ui,
                        AppIcon::Add,
                        self.palette().get("text_primary"),
                        self.tr("playlists.add_song"),
                    )
                    .clicked()
                {
                    self.playlist_dialog = Some(PlaylistDialog::AddSong);
                    self.playlist_add_query.clear();
                    self.dialog_error = None;
                }
            });
        });
        ui.add_space(12.0);
        if self.playlists.is_empty() {
            ui.label(self.tr("playlists.empty"));
            return;
        }
        if self
            .selected_playlist_id
            .as_ref()
            .is_none_or(|id| !self.playlists.iter().any(|playlist| &playlist.id == id))
        {
            self.selected_playlist_id = self.playlists.first().map(|playlist| playlist.id.clone());
        }

        let mut dialog_action = None;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(284.0, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    for playlist in self.playlists.clone() {
                        let selected = self.selected_playlist_id.as_deref() == Some(&playlist.id);
                        let count = playlist.song_ids.len();
                        let count_label = if count == 1 {
                            self.tr("playlists.count_one")
                        } else {
                            self.tr("playlists.count_many")
                        };

                        ui.horizontal(|ui| {
                            let available_width = ui.available_width().max(140.0);
                            let action_width = 96.0;
                            let title_width = (available_width - action_width).max(120.0);
                            let row =
                                ui.allocate_ui_with_layout(
                                    Vec2::new(title_width, 38.0),
                                    Layout::left_to_right(Align::Center),
                                    |ui| {
                                        let response =
                                            ui.add_sized(
                                                [ui.available_width(), 32.0],
                                                egui::Button::new(
                                                    RichText::new(format!(
                                                        "{}  ·  {count} {count_label}",
                                                        playlist.name
                                                    ))
                                                    .color(if selected {
                                                        self.palette().get("accent")
                                                    } else {
                                                        self.palette().get("text_primary")
                                                    }),
                                                )
                                                .fill(if selected {
                                                    self.palette().get("surface_selected")
                                                } else {
                                                    self.palette().get("surface")
                                                })
                                                .stroke(Stroke::new(
                                                    if selected { 1.0_f32 } else { 0.0_f32 },
                                                    self.palette().get("border_subtle"),
                                                )),
                                            );
                                        if response.clicked() {
                                            self.selected_playlist_id = Some(playlist.id.clone());
                                        }
                                    },
                                );
                            if row.response.clicked() {
                                self.selected_playlist_id = Some(playlist.id.clone());
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if icon_button(
                                    ui,
                                    AppIcon::Add,
                                    self.palette().get("text_primary"),
                                    self.tr("playlists.add_song"),
                                )
                                .clicked()
                                {
                                    self.selected_playlist_id = Some(playlist.id.clone());
                                    self.playlist_dialog = Some(PlaylistDialog::AddSong);
                                    self.playlist_add_query.clear();
                                    self.dialog_error = None;
                                }
                                if icon_button(
                                    ui,
                                    AppIcon::Edit,
                                    self.palette().get("text_primary"),
                                    self.tr("common.rename"),
                                )
                                .clicked()
                                {
                                    self.selected_playlist_id = Some(playlist.id.clone());
                                    self.playlist_dialog_name = playlist.name.clone();
                                    self.playlist_dialog = Some(PlaylistDialog::Rename);
                                    self.dialog_error = None;
                                }
                                if icon_button(
                                    ui,
                                    AppIcon::Delete,
                                    self.palette().get("danger"),
                                    self.tr("common.delete"),
                                )
                                .clicked()
                                {
                                    dialog_action =
                                        Some((PlaylistDialog::Delete, playlist.id.clone()));
                                }
                            });
                        });
                        ui.add_space(8.0);
                    }
                },
            );
            ui.separator();
            ui.vertical(|ui| {
                if let Some(playlist) = self
                    .selected_playlist_id
                    .as_ref()
                    .and_then(|id| self.playlists.iter().find(|playlist| &playlist.id == id))
                    .cloned()
                {
                    self.settings_card_frame().show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.heading(&playlist.name);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        playlist.song_ids.len(),
                                        if playlist.song_ids.len() == 1 {
                                            self.tr("playlists.count_one")
                                        } else {
                                            self.tr("playlists.count_many")
                                        }
                                    ))
                                    .size(12.0)
                                    .color(self.palette().get("text_muted")),
                                );
                            });
                        });
                        ui.add_space(10.0);
                        self.draw_song_list(
                            ui,
                            playlist.song_ids.clone(),
                            Some(playlist.id.clone()),
                        );
                    });
                }
            });
        });
        if let Some(action) = dialog_action {
            self.playlist_dialog = Some(action.0);
            self.selected_playlist_id = Some(action.1);
        }
    }
}
