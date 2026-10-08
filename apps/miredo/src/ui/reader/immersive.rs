//! PDF reader rendering and controls.

use super::super::*;

impl MiReDoApp {
    pub(in crate::ui) fn draw_pdf_reader_immersive(
        &mut self,
        context: &EguiContext,
        ui: &mut egui::Ui,
    ) {
        let Some(song) = self.current_song().cloned() else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(self.tr("reader.pdf_missing"));
            });
            return;
        };

        let navigation_position = self
            .navigation_song_ids
            .iter()
            .position(|id| id == &song.id);
        let previous_enabled = navigation_position.is_some_and(|index| index > 0);
        let next_enabled =
            navigation_position.is_some_and(|index| index + 1 < self.navigation_song_ids.len());

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 3.0);

                if pdf_toolbar_icon_button(
                    ui,
                    AppIcon::Menu,
                    self.palette().get("text_primary"),
                    self.tr("nav.library"),
                    true,
                )
                .clicked()
                {
                    self.nav_menu_open = !self.nav_menu_open;
                }

                if pdf_toolbar_icon_button(
                    ui,
                    AppIcon::Close,
                    self.palette().get("text_primary"),
                    self.tr("common.close"),
                    true,
                )
                .clicked()
                {
                    self.page = Page::Library;
                }

                if pdf_toolbar_icon_button(
                    ui,
                    AppIcon::Previous,
                    self.palette().get("text_primary"),
                    self.tr("common.previous"),
                    previous_enabled,
                )
                .clicked()
                {
                    self.move_song(-1);
                }

                let title = self.title_for_song(&song);
                let heading = format!("{}  ·  {}", song.display_number(), title);
                ui.add(
                    egui::Label::new(
                        RichText::new(heading)
                            .strong()
                            .color(self.palette().get("text_primary")),
                    )
                    .truncate(),
                );

                if pdf_toolbar_icon_button(
                    ui,
                    AppIcon::Next,
                    self.palette().get("text_primary"),
                    self.tr("common.next"),
                    next_enabled,
                )
                .clicked()
                {
                    self.move_song(1);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let is_favorite = self.favorites.contains(&song.id);
                    if pdf_toolbar_icon_button(
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
                        true,
                    )
                    .clicked()
                    {
                        self.toggle_favorite(&song.id);
                    }

                    if ui
                        .selectable_label(false, self.tr("reader.text"))
                        .on_hover_text(self.tr("reader.text"))
                        .clicked()
                    {
                        self.set_viewer_mode(ViewerMode::Text);
                    }

                    if pdf_toolbar_icon_button(
                        ui,
                        AppIcon::Search,
                        self.palette().get("text_primary"),
                        self.tr("search.placeholder"),
                        true,
                    )
                    .clicked()
                    {
                        self.pdf_search_open = true;
                        self.search_id = Some(Id::new("miredo-pdf-search"));
                    }
                });
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 3.0);

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

                let total_pages = self.pdf_page_counts.get(&song.id).copied();
                let is_book = self.pdf_layout == PdfLayout::Double;
                let step = if is_book { 2 } else { 1 };
                let current_start =
                    if is_book && self.pdf_page > 1 && self.pdf_page.is_multiple_of(2) {
                        self.pdf_page - 1
                    } else {
                        self.pdf_page
                    };
                let next_start = current_start.saturating_add(step);

                if pdf_toolbar_icon_button(
                    ui,
                    AppIcon::Previous,
                    self.palette().get("text_primary"),
                    self.tr("common.previous"),
                    current_start > 1,
                )
                .clicked()
                {
                    self.set_pdf_page(&song.id, current_start.saturating_sub(step).max(1));
                }

                ui.label(
                    total_pages
                        .map(|total| {
                            if is_book && current_start < total {
                                format!(
                                    "{} {}–{} / {}",
                                    self.tr("reader.page"),
                                    current_start,
                                    (current_start + 1).min(total),
                                    total
                                )
                            } else {
                                format!("{} {} / {}", self.tr("reader.page"), current_start, total)
                            }
                        })
                        .unwrap_or_else(|| format!("{} {}", self.tr("reader.page"), current_start)),
                );

                if pdf_toolbar_icon_button(
                    ui,
                    AppIcon::Next,
                    self.palette().get("text_primary"),
                    self.tr("common.next"),
                    total_pages.is_none_or(|total| next_start <= total),
                )
                .clicked()
                {
                    self.set_pdf_page(&song.id, next_start);
                }

                ui.separator();

                let fit_label = match self.pdf_fit_mode {
                    PdfFitMode::FitWidth => self.tr("reader.fit_width"),
                    PdfFitMode::FitHeight => self.tr("reader.fit_height"),
                    PdfFitMode::Manual => self.tr("reader.manual_zoom"),
                };
                egui::ComboBox::from_id_salt("pdf-fit-mode")
                    .selected_text(fit_label)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                self.pdf_fit_mode == PdfFitMode::FitWidth,
                                self.tr("reader.fit_width"),
                            )
                            .clicked()
                        {
                            self.set_pdf_fit_mode(PdfFitMode::FitWidth);
                            ui.close();
                        }
                        if ui
                            .selectable_label(
                                self.pdf_fit_mode == PdfFitMode::FitHeight,
                                self.tr("reader.fit_height"),
                            )
                            .clicked()
                        {
                            self.set_pdf_fit_mode(PdfFitMode::FitHeight);
                            ui.close();
                        }
                        if ui
                            .selectable_label(
                                self.pdf_fit_mode == PdfFitMode::Manual,
                                self.tr("reader.manual_zoom"),
                            )
                            .clicked()
                        {
                            self.set_pdf_fit_mode(PdfFitMode::Manual);
                            ui.close();
                        }
                    });

                if self.pdf_fit_mode == PdfFitMode::Manual {
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

                let mut list_to_add = None;
                ui.menu_button("⋯", |ui| {
                    ui.label(RichText::new(self.tr("reader.add_to_list")).strong());

                    let mut selected_list_id = self.selected_playlist_id.clone();
                    egui::ComboBox::from_id_salt("immersive-reader-list")
                        .selected_text(
                            selected_list_id
                                .as_ref()
                                .and_then(|id| {
                                    self.playlists.iter().find(|playlist| &playlist.id == id)
                                })
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

                    if ui.button(self.tr("reader.add_to_list")).clicked() {
                        list_to_add = selected_list_id;
                        ui.close();
                    }

                    if ui.button(self.tr("reader.fullscreen")).clicked() {
                        self.fullscreen = !self.fullscreen;
                        context.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
                        ui.close();
                    }
                });

                if let Some(playlist_id) = list_to_add {
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
        });

        if self.pdf_search_open {
            ui.add_space(2.0);

            // Préparer les valeurs avant d'emprunter self.query mutuellement
            // dans TextEdit. Cela évite le conflit E0502 du borrow-checker.
            let search_placeholder = self.tr("search.placeholder");
            let results_label = self.tr("search.results");
            let result_count = self.visible_ids(false, None).len();

            ui.horizontal(|ui| {
                let (search_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                paint_app_icon(
                    ui,
                    search_rect,
                    AppIcon::Search,
                    self.palette().get("text_muted"),
                );
                let response = ui.add_sized(
                    [320.0, 30.0],
                    egui::TextEdit::singleline(&mut self.query)
                        .id_salt("miredo-pdf-search")
                        .hint_text(search_placeholder),
                );
                self.search_id = Some(response.id);
                if ui.button("x").clicked() {
                    self.query.clear();
                    self.pdf_search_open = false;
                }
                ui.label(
                    RichText::new(format!("{result_count} {results_label}"))
                        .size(11.0)
                        .color(self.palette().get("text_muted")),
                );
            });

            if !self.query.is_empty() {
                let result_ids = self.visible_ids(false, None);
                let navigation_ids = result_ids.clone();
                let mut open_id = None;
                ScrollArea::horizontal()
                    .id_salt("miredo-pdf-search-results")
                    .auto_shrink([false, false])
                    .max_height(38.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for song_id in result_ids.iter().take(8) {
                                if let Some(result_song) = self.song(song_id) {
                                    let label = format!(
                                        "{} · {}",
                                        result_song.display_number(),
                                        self.title_for_song(result_song)
                                    );
                                    if ui.selectable_label(false, label).clicked() {
                                        open_id = Some(song_id.clone());
                                    }
                                }
                            }
                        });
                    });
                if let Some(song_id) = open_id {
                    self.open_song(song_id, navigation_ids);
                    self.pdf_search_open = false;
                }
            }
        }

        ui.add_space(2.0);
        self.draw_pdf_canvas_immersive(ui, &song);
    }

    fn draw_pdf_canvas_immersive(&mut self, ui: &mut egui::Ui, song: &Song) {
        let total_pages = self.pdf_page_counts.get(&song.id).copied();
        let first_page = if self.pdf_layout == PdfLayout::Double
            && self.pdf_page > 1
            && self.pdf_page.is_multiple_of(2)
        {
            self.pdf_page - 1
        } else {
            self.pdf_page
        };

        let pages = pdf_visible_pages(self.pdf_layout, first_page, total_pages);

        let available = ui.available_size().max(Vec2::splat(1.0));

        for page in &pages {
            if total_pages.is_none_or(|total| *page <= total) {
                self.request_pdf_page(song, *page, available);
            }
        }

        // Préchargement discret des pages voisines pour rendre précédent/suivant immédiat.
        for adjacent in [first_page.saturating_sub(1), first_page + 1, first_page + 2] {
            if adjacent >= 1 && total_pages.is_none_or(|total| adjacent <= total) {
                self.request_pdf_page(song, adjacent, available);
            }
        }

        ScrollArea::both()
            .id_salt("miredo-pdf-immersive")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if pages.len() == 2 {
                    let page_width = (available.x / 2.0).max(1.0);
                    let page_available = Vec2::new(page_width, available.y);
                    ui.columns(2, |columns| {
                        for (index, page) in pages.iter().enumerate() {
                            self.draw_pdf_page(
                                &mut columns[index],
                                &song.id,
                                *page,
                                page_available,
                            );
                        }
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        self.draw_pdf_page(
                            ui,
                            &song.id,
                            self.pdf_page,
                            Vec2::new(available.x.max(1.0), available.y.max(1.0)),
                        );
                    });
                }
            });
    }
}
