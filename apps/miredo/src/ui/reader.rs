//! Text and PDF reading views, including page navigation and rendering controls.

use super::*;

mod immersive;
mod pdf;

impl MiReDoApp {
    pub(super) fn draw_reader(&mut self, context: &EguiContext, ui: &mut egui::Ui) {
        let Some(song) = self.current_song().cloned() else {
            ui.label(self.tr("search.empty"));
            if ui.button(self.tr("nav.library")).clicked() {
                self.page = Page::Library;
            }
            return;
        };
        let mut mode_changed = false;
        ui.horizontal(|ui| {
            let previous_enabled = self
                .navigation_song_ids
                .iter()
                .position(|song_id| song_id == &song.id)
                .is_some_and(|index| index > 0);
            let next_enabled = self
                .navigation_song_ids
                .iter()
                .position(|song_id| song_id == &song.id)
                .is_some_and(|index| index + 1 < self.navigation_song_ids.len());
            if ui
                .add_enabled(
                    previous_enabled,
                    egui::Button::new(self.tr("common.previous")),
                )
                .clicked()
            {
                self.move_song(-1);
            }
            if ui
                .add_enabled(next_enabled, egui::Button::new(self.tr("common.next")))
                .clicked()
            {
                self.move_song(1);
            }
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "{}  ·  {}",
                    song.display_number(),
                    self.collection_label(song.collection)
                ))
                .color(self.palette().get("text_secondary")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let is_favorite = self.favorites.contains(&song.id);
                if ui
                    .button(if is_favorite { "★" } else { "☆" })
                    .on_hover_text(if is_favorite {
                        self.tr("status.favorite_removed")
                    } else {
                        self.tr("status.favorite_added")
                    })
                    .clicked()
                {
                    self.toggle_favorite(&song.id);
                }
                let pdf_label = self.tr("reader.pdf");
                if ui
                    .selectable_label(self.viewer_mode == ViewerMode::Pdf, pdf_label)
                    .clicked()
                    && song.has_pdf()
                {
                    mode_changed = true;
                }
                let text_label = self.tr("reader.text");
                if ui
                    .selectable_label(self.viewer_mode == ViewerMode::Text, text_label)
                    .clicked()
                {
                    self.set_viewer_mode(ViewerMode::Text);
                }
            });
        });
        if mode_changed {
            self.set_viewer_mode(ViewerMode::Pdf);
        }
        ui.add_space(8.0);
        self.draw_reader_actions(ui, &song);
        ui.separator();
        ui.add_space(8.0);
        match self.viewer_mode {
            ViewerMode::Text => self.draw_text_viewer(ui, &song),
            ViewerMode::Pdf => self.draw_pdf_viewer(context, ui, &song),
        }
    }

    fn draw_reader_actions(&mut self, ui: &mut egui::Ui, song: &Song) {
        ui.horizontal(|ui| {
            let mut selected_list_id = self.selected_playlist_id.clone();
            egui::ComboBox::from_id_salt("reader-add-playlist")
                .selected_text(
                    selected_list_id
                        .as_ref()
                        .and_then(|id| self.playlists.iter().find(|playlist| &playlist.id == id))
                        .map(|playlist| playlist.name.clone())
                        .unwrap_or_else(|| self.tr("reader.select_list")),
                )
                .show_ui(ui, |ui| {
                    for playlist in &self.playlists {
                        ui.selectable_value(
                            &mut selected_list_id,
                            Some(playlist.id.clone()),
                            &playlist.name,
                        );
                    }
                });
            self.selected_playlist_id = selected_list_id;
            if ui.button(self.tr("reader.add_to_list")).clicked() {
                if let Some(playlist_id) = self.selected_playlist_id.clone() {
                    let already_added = self
                        .playlists
                        .iter()
                        .find(|playlist| playlist.id == playlist_id)
                        .is_some_and(|playlist| playlist.song_ids.contains(&song.id));
                    if already_added {
                        self.set_status(self.tr("reader.already_added"));
                    } else {
                        match self.storage.add_song_to_playlist(&playlist_id, &song.id) {
                            Ok(()) => {
                                self.refresh_user_data();
                                self.set_status(self.tr("reader.added"));
                            }
                            Err(error) => self.show_error(error),
                        }
                    }
                } else {
                    self.page = Page::Playlists;
                    self.playlist_dialog = Some(PlaylistDialog::Create);
                    self.playlist_dialog_name.clear();
                }
            }
        });
    }

    fn draw_text_viewer(&mut self, ui: &mut egui::Ui, song: &Song) {
        if song.verses.is_empty() {
            ui.label(self.tr("library.no_lyrics"));
            return;
        }
        ScrollArea::vertical()
            .id_salt(format!("lyrics-{}", song.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.set_max_width(720.0);
                    ui.label(
                        RichText::new(self.title_for_song(song))
                            .size(25.0)
                            .strong()
                            .color(self.palette().get("text_primary")),
                    );
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(self.reader_subtitle(song))
                            .size(12.0)
                            .color(self.palette().get("text_muted")),
                    );
                    ui.add_space(28.0);
                });
                for verse in &song.verses {
                    if verse.is_refrain {
                        ui.label(
                            RichText::new(self.tr("common.refrain"))
                                .size(11.0)
                                .strong()
                                .color(self.palette().get("accent")),
                        );
                    } else if verse.order > 0 {
                        ui.label(
                            RichText::new(verse.order.to_string())
                                .size(12.0)
                                .strong()
                                .color(self.palette().get("text_muted")),
                        );
                    }
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(&verse.text)
                            .size(16.0)
                            .color(self.palette().get("text_primary")),
                    );
                    ui.add_space(20.0);
                }
            });
    }
}
