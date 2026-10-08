//! Create, rename, delete and add-song dialogs.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_playlist_dialog(&mut self, context: &EguiContext) {
        let Some(kind) = self.playlist_dialog else {
            return;
        };
        let title = match kind {
            PlaylistDialog::Create => self.tr("playlists.create_title"),
            PlaylistDialog::Rename => self.tr("playlists.rename_title"),
            PlaylistDialog::Delete => self.tr("playlists.delete_title"),
            PlaylistDialog::AddSong => self.tr("playlists.add_song"),
        };
        let name_label = self.tr("playlists.name");
        let confirm_label = match kind {
            PlaylistDialog::Delete => self.tr("common.delete"),
            PlaylistDialog::Rename => self.tr("common.save"),
            PlaylistDialog::Create => self.tr("common.create"),
            PlaylistDialog::AddSong => self.tr("common.add"),
        };
        let cancel_label = self.tr("common.cancel");
        let delete_body = self.tr("playlists.delete_confirm");
        let add_song_hint = self.tr("playlists.add_song_hint");
        let mut action = None;
        let mut song_to_add = None;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(context, |ui| {
                if kind == PlaylistDialog::Delete {
                    ui.label(delete_body);
                } else if kind == PlaylistDialog::AddSong {
                    ui.label(&add_song_hint);
                    ui.add_space(8.0);
                    ui.add_sized(
                        [320.0, 30.0],
                        egui::TextEdit::singleline(&mut self.playlist_add_query)
                            .hint_text(add_song_hint.clone()),
                    );
                    ui.add_space(8.0);
                    let playlist_id = self.selected_playlist_id.clone();
                    let query = self.playlist_add_query.trim().to_lowercase();
                    let matching_songs = self
                        .songs
                        .iter()
                        .filter(|song| {
                            playlist_id
                                .as_ref()
                                .map(|id| {
                                    !self.playlists.iter().any(|playlist| {
                                        &playlist.id == id && playlist.song_ids.contains(&song.id)
                                    })
                                })
                                .unwrap_or(true)
                                && (query.is_empty()
                                    || song.title.to_lowercase().contains(&query)
                                    || song.display_number().to_lowercase().contains(&query)
                                    || song.authors.join(" ").to_lowercase().contains(&query))
                        })
                        .take(10)
                        .collect::<Vec<_>>();
                    if matching_songs.is_empty() {
                        ui.label(self.tr("playlists.no_song_match"));
                    } else {
                        ui.vertical(|ui| {
                            for song in matching_songs {
                                let button_label = format!(
                                    "{} · {}",
                                    song.display_number(),
                                    self.title_for_song(song)
                                );
                                if ui.button(button_label).clicked() {
                                    song_to_add = Some(song.id.clone());
                                }
                                ui.add_space(4.0);
                            }
                        });
                    }
                } else {
                    ui.label(name_label);
                    ui.text_edit_singleline(&mut self.playlist_dialog_name);
                    if let Some(error) = &self.dialog_error {
                        ui.label(RichText::new(error).color(self.palette().get("danger")));
                    }
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(cancel_label).clicked() {
                        action = Some(false);
                    }
                    if kind != PlaylistDialog::AddSong && ui.button(confirm_label).clicked() {
                        action = Some(true);
                    }
                });
            });
        match action {
            Some(false) => {
                self.playlist_dialog = None;
                self.dialog_error = None;
            }
            Some(true) => self.submit_playlist_dialog(kind),
            None => {}
        }
        if let Some(song_id) = song_to_add
            && let Some(playlist_id) = self.selected_playlist_id.clone()
        {
            match self.storage.add_song_to_playlist(&playlist_id, &song_id) {
                Ok(()) => {
                    self.playlist_dialog = None;
                    self.playlist_add_query.clear();
                    self.refresh_user_data();
                    self.set_status(self.tr("reader.added"));
                }
                Err(error) => self.show_error(error),
            }
        }
    }

    fn submit_playlist_dialog(&mut self, kind: PlaylistDialog) {
        match kind {
            PlaylistDialog::Delete => {
                let Some(playlist_id) = self.selected_playlist_id.clone() else {
                    self.playlist_dialog = None;
                    return;
                };
                match self.storage.delete_playlist(&playlist_id) {
                    Ok(()) => {
                        self.selected_playlist_id = None;
                        self.playlist_dialog = None;
                        self.refresh_user_data();
                        self.set_status(self.tr("status.list_deleted"));
                    }
                    Err(error) => self.show_error(error),
                }
            }
            PlaylistDialog::AddSong => {
                self.playlist_dialog = None;
                self.playlist_add_query.clear();
                self.dialog_error = None;
            }
            PlaylistDialog::Create | PlaylistDialog::Rename => {
                let name = self.playlist_dialog_name.trim().to_owned();
                if name.is_empty() {
                    self.dialog_error = Some(self.tr("playlists.name_required"));
                    return;
                }
                let current_id = (kind == PlaylistDialog::Rename)
                    .then_some(self.selected_playlist_id.as_deref())
                    .flatten();
                if self.playlists.iter().any(|playlist| {
                    Some(playlist.id.as_str()) != current_id
                        && playlist.name.eq_ignore_ascii_case(&name)
                }) {
                    self.dialog_error = Some(self.tr("playlists.duplicate_name"));
                    return;
                }
                let result = if kind == PlaylistDialog::Create {
                    self.storage.create_playlist(&name).map(|id| {
                        self.selected_playlist_id = Some(id);
                    })
                } else {
                    self.selected_playlist_id
                        .clone()
                        .context("Aucune liste sélectionnée")
                        .and_then(|id| self.storage.rename_playlist(&id, &name))
                };
                match result {
                    Ok(()) => {
                        self.playlist_dialog = None;
                        self.dialog_error = None;
                        self.refresh_user_data();
                        self.set_status(if kind == PlaylistDialog::Create {
                            self.tr("status.list_created")
                        } else {
                            self.tr("status.list_updated")
                        });
                    }
                    Err(error) => self.show_error(error),
                }
            }
        }
    }
}
