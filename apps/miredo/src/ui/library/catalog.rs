//! Catalog browsing, filtering and song rows.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_library(&mut self, ui: &mut egui::Ui) {
        self.draw_filters(ui, false);
        let ids = self.visible_ids(false, None);
        self.draw_result_count(ui, ids.len());
        self.draw_song_list(ui, ids, None);
    }

    pub(in crate::ui) fn draw_favorites(&mut self, ui: &mut egui::Ui) {
        let ids = self.visible_ids(true, None);
        if ids.is_empty() {
            ui.add_space(20.0);
            ui.label(self.tr("favorites.empty"));
            return;
        }
        self.draw_filters(ui, true);
        self.draw_result_count(ui, ids.len());
        self.draw_song_list(ui, ids, None);
    }

    fn draw_filters(&mut self, ui: &mut egui::Ui, favorites_page: bool) {
        let all_collections_label = self.tr("search.all");
        let pdf_filter_label = self.tr("search.pdf_only");
        let collection_labels: Vec<(Collection, String)> = Collection::ALL
            .into_iter()
            .map(|collection| (collection, self.collection_label(collection)))
            .collect();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(self.tr("library.filter_collection"))
                    .color(self.palette().get("text_secondary")),
            );
            let selected = self
                .collection_filter
                .map(|collection| self.collection_label(collection))
                .unwrap_or_else(|| self.tr("search.all"));
            egui::ComboBox::from_id_salt("collection-filter")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut self.collection_filter,
                            None,
                            all_collections_label.clone(),
                        )
                        .clicked()
                    {
                        self.page = if favorites_page {
                            Page::Favorites
                        } else {
                            Page::Library
                        };
                    }
                    for (collection, label) in &collection_labels {
                        if ui
                            .selectable_value(&mut self.collection_filter, Some(*collection), label)
                            .clicked()
                        {
                            self.page = if favorites_page {
                                Page::Favorites
                            } else {
                                Page::Library
                            };
                        }
                    }
                });
            ui.checkbox(&mut self.pdf_filter, pdf_filter_label);
        });
        ui.add_space(10.0);
    }

    fn draw_result_count(&self, ui: &mut egui::Ui, count: usize) {
        ui.label(
            RichText::new(format!("{count} {}", self.tr("search.results")))
                .size(12.0)
                .color(self.palette().get("text_muted")),
        );
        ui.add_space(8.0);
    }

    pub(in crate::ui) fn draw_song_list(
        &mut self,
        ui: &mut egui::Ui,
        ids: Vec<String>,
        remove_from_playlist: Option<String>,
    ) {
        if ids.is_empty() {
            ui.add_space(20.0);
            ui.label(if self.query.is_empty() {
                self.tr("search.empty")
            } else {
                self.tr("search.no_results")
            });
            return;
        }

        let navigation_ids = ids.clone();
        let mut open_id = None;
        let mut favorite_id = None;
        let mut remove_id = None;
        ScrollArea::vertical()
            .id_salt("song-results")
            .wheel_scroll_multiplier(Vec2::new(1.0, 1.5))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for song_id in &ids {
                    let Some(song) = self.song(song_id) else {
                        continue;
                    };
                    let title = self.title_for_song(song);
                    let number = song.display_number();
                    let collection = self.collection_label(song.collection);
                    let preview = song.preview().to_owned();
                    let author = if song.authors.is_empty() {
                        self.tr("common.no_author")
                    } else {
                        song.authors.join(", ")
                    };
                    let is_favorite = self.favorites.contains(song_id);
                    let has_pdf = song.has_pdf();
                    let selected = self.selected_song_id.as_deref() == Some(song_id);
                    let subtitle = if preview.is_empty() {
                        author
                    } else {
                        format!("{author} · {collection} · {preview}")
                    };

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let action_width = 44.0
                            + if remove_from_playlist.is_some() {
                                44.0
                            } else {
                                0.0
                            };
                        let content_width =
                            (ui.available_width() - action_width - ui.spacing().item_spacing.x)
                                .max(1.0);
                        let (row_rect, _) = ui.allocate_exact_size(
                            Vec2::new(content_width, 48.0),
                            egui::Sense::hover(),
                        );
                        let row_response = ui.interact(
                            row_rect,
                            Id::new(("song-row", song_id)),
                            egui::Sense::click(),
                        );
                        let row_bg = if selected {
                            self.palette().get("surface_selected")
                        } else if row_response.hovered() {
                            self.palette().get("surface_hover")
                        } else {
                            self.palette().get("surface_elevated")
                        };
                        ui.painter().rect_filled(row_rect.shrink(1.0), 8.0, row_bg);
                        if selected {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    row_rect.left_top() + Vec2::new(1.0, 12.0),
                                    Vec2::new(3.0, 24.0),
                                ),
                                2.0,
                                self.palette().get("accent"),
                            );
                        }
                        ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.add_sized(
                                    [48.0, 38.0],
                                    egui::Label::new(
                                        RichText::new(number)
                                            .monospace()
                                            .size(12.5)
                                            .color(self.palette().get("text_muted")),
                                    ),
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(title)
                                            .size(14.5)
                                            .strong()
                                            .color(self.palette().get("text_primary")),
                                    );
                                    ui.add_space(2.0);
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(subtitle)
                                                    .size(12.0)
                                                    .color(self.palette().get("text_secondary")),
                                            )
                                            .truncate(),
                                        );
                                        if has_pdf {
                                            ui.label(
                                                RichText::new("PDF")
                                                    .size(10.5)
                                                    .color(self.palette().get("accent_secondary")),
                                            );
                                        }
                                    });
                                });
                            });
                        });
                        if row_response.clicked() {
                            open_id = Some(song_id.clone());
                        }
                        row_response.on_hover_cursor(egui::CursorIcon::PointingHand);

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let favorite = icon_button(
                                ui,
                                AppIcon::Favorite,
                                if is_favorite {
                                    self.palette().get("accent")
                                } else {
                                    self.palette().get("text_muted")
                                },
                                if is_favorite {
                                    self.tr("status.favorite_removed")
                                } else {
                                    self.tr("status.favorite_added")
                                },
                            );
                            if favorite.clicked() {
                                favorite_id = Some(song_id.clone());
                            }
                            if remove_from_playlist.is_some() {
                                let remove = ui
                                    .button("x")
                                    .on_hover_text(self.tr("playlists.remove_song"));
                                if remove.clicked() {
                                    remove_id = Some(song_id.clone());
                                }
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
            });

        if let Some(song_id) = favorite_id {
            self.toggle_favorite(&song_id);
        }
        if let Some(song_id) = remove_id
            && let Some(playlist_id) = remove_from_playlist
        {
            match self
                .storage
                .remove_song_from_playlist(&playlist_id, &song_id)
            {
                Ok(()) => {
                    self.refresh_user_data();
                    self.set_status(self.tr("status.song_removed"));
                }
                Err(error) => self.show_error(error),
            }
        }
        if let Some(song_id) = open_id {
            let mode = if self.song(&song_id).is_some_and(|song| song.has_pdf()) {
                ViewerMode::Pdf
            } else {
                ViewerMode::Text
            };
            self.set_viewer_mode(mode);
            self.open_song(song_id, navigation_ids);
        }
    }
}
