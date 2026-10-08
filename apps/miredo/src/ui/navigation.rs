//! Navigation sidebar, responsive top bar, global search and navigation drawer.

use super::*;

impl MiReDoApp {
    pub(super) fn draw_sidebar(&mut self, context: &EguiContext) {
        egui::SidePanel::left("miredo-sidebar")
            .resizable(false)
            .exact_width(236.0)
            .frame(
                egui::Frame::new()
                    .fill(self.palette().get("surface"))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(context, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(5.0, 3.0);
                        ui.set_min_height(30.0);

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
                    });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("MiReDo")
                            .size(22.0)
                            .strong()
                            .color(self.palette().get("accent")),
                    );
                    ui.label(
                        RichText::new(self.tr("app.tagline"))
                            .size(11.0)
                            .color(self.palette().get("text_muted")),
                    );
                    ui.add_space(22.0);

                    self.nav_item(ui, Page::Home, "nav.home", AppIcon::Home);
                    self.nav_item(ui, Page::Library, "nav.library", AppIcon::Library);
                    self.nav_item(ui, Page::Favorites, "nav.favorites", AppIcon::Favorite);
                    self.nav_item(ui, Page::Playlists, "nav.playlists", AppIcon::Playlist);

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    self.nav_item(ui, Page::Help, "nav.help", AppIcon::Help);
                    self.nav_item(ui, Page::About, "nav.about", AppIcon::About);
                    self.nav_item(ui, Page::Settings, "nav.settings", AppIcon::Settings);

                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        ui.add_space(8.0);
                        let language = match self.locale.as_str() {
                            "mg" => "MG",
                            "en" => "EN",
                            _ => "FR",
                        };
                        let response = ui.add_sized(
                            [ui.available_width(), 34.0],
                            egui::Button::new(
                                RichText::new(format!(
                                    "{}  {language}",
                                    self.tr("settings.language")
                                ))
                                .color(self.palette().get("text_secondary")),
                            )
                            .fill(self.palette().get("surface_hover"))
                            .stroke(Stroke::NONE),
                        );
                        if response.clicked() {
                            self.page = Page::Settings;
                        }
                    });
                });
            });
    }

    fn nav_item(&mut self, ui: &mut egui::Ui, page: Page, key: &str, icon: AppIcon) -> bool {
        let selected = self.page == page;
        let response = ui.add_sized(
            [ui.available_width(), 38.0],
            egui::Button::new("")
                .fill(if selected {
                    self.palette().get("surface_selected")
                } else {
                    self.palette().get("surface")
                })
                .stroke(Stroke::NONE),
        );
        let color = if selected {
            self.palette().get("accent")
        } else {
            self.palette().get("text_secondary")
        };
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(response.rect.left() + 20.0, response.rect.center().y),
            Vec2::splat(22.0),
        );
        paint_app_icon(ui, icon_rect, icon, color);
        ui.painter().text(
            egui::pos2(response.rect.left() + 42.0, response.rect.center().y),
            egui::Align2::LEFT_CENTER,
            self.tr(key),
            egui::FontId::proportional(14.0),
            color,
        );
        if response.clicked() {
            self.page = page;
        }
        response.clicked()
    }

    pub(super) fn draw_nav_drawer(&mut self, context: &EguiContext) {
        if !self.nav_menu_open {
            return;
        }
        let mut selection_changed = false;
        egui::Area::new(Id::new("miredo-nav-drawer"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(8.0, 48.0))
            .show(context, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(236.0);
                    ui.label(
                        RichText::new("MiReDo")
                            .size(18.0)
                            .strong()
                            .color(self.palette().get("accent")),
                    );
                    ui.add_space(8.0);
                    for (page, key, icon) in [
                        (Page::Home, "nav.home", AppIcon::Home),
                        (Page::Library, "nav.library", AppIcon::Library),
                        (Page::Favorites, "nav.favorites", AppIcon::Favorite),
                        (Page::Playlists, "nav.playlists", AppIcon::Playlist),
                        (Page::Help, "nav.help", AppIcon::Help),
                        (Page::About, "nav.about", AppIcon::About),
                        (Page::Settings, "nav.settings", AppIcon::Settings),
                    ] {
                        selection_changed |= self.nav_item(ui, page, key, icon);
                    }
                });
            });
        if selection_changed {
            self.nav_menu_open = false;
        }
    }

    pub(super) fn draw_topbar(&mut self, context: &EguiContext, ui: &mut egui::Ui) {
        let search_placeholder = self.tr("search.placeholder");
        ui.horizontal(|ui| {
            if context.screen_rect().width() < 760.0
                && icon_button(
                    ui,
                    AppIcon::Menu,
                    self.palette().get("text_primary"),
                    self.tr("nav.library"),
                )
                .clicked()
            {
                self.nav_menu_open = !self.nav_menu_open;
            }
            ui.label(
                RichText::new(self.page_heading())
                    .size(22.0)
                    .strong()
                    .color(self.palette().get("text_primary")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some((message, expiry)) = self.status.clone() {
                    if Instant::now() < expiry {
                        ui.label(
                            RichText::new(message)
                                .size(12.0)
                                .color(self.palette().get("success")),
                        );
                    } else {
                        self.status = None;
                    }
                }
                if self.page == Page::Reader
                    && self.current_song().is_some()
                    && ui
                        .button(self.tr("common.close"))
                        .on_hover_text(self.tr("common.close"))
                        .clicked()
                {
                    self.page = Page::Library;
                }
            });
        });

        let subtitle = self.page_subtitle();
        if !subtitle.is_empty() {
            ui.add_space(3.0);
            ui.label(
                RichText::new(subtitle)
                    .size(13.0)
                    .color(self.palette().get("text_secondary")),
            );
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let (search_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
            paint_app_icon(
                ui,
                search_rect,
                AppIcon::Search,
                self.palette().get("text_muted"),
            );
            let available = ui.available_width().min(660.0);
            let response = ui.add_sized(
                [available, 36.0],
                egui::TextEdit::singleline(&mut self.query)
                    .id_salt("miredo-global-search")
                    .hint_text(search_placeholder.as_str())
                    .vertical_align(Align::Center),
            );
            self.search_id = Some(response.id);
            if response.changed() && self.page != Page::Library {
                self.page = Page::Library;
            }
            if !self.query.is_empty() && ui.button("x").clicked() {
                self.query.clear();
            }
            if self.page == Page::Reader {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(song) = self.current_song() {
                        ui.label(
                            RichText::new(self.collection_label(song.collection))
                                .size(12.0)
                                .color(self.palette().get("text_muted")),
                        );
                    }
                });
            }
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(12.0);
    }
}
