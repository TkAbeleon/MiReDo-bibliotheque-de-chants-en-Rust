//! App state transitions, persistence updates and data lookup helpers.

use super::*;

impl MiReDoApp {
    pub(super) fn tr(&self, key: &str) -> String {
        self.translator.text(&self.locale, key)
    }

    pub(super) fn palette(&self) -> &Palette {
        match self.theme_mode {
            ThemeMode::Light => &self.light_palette,
            ThemeMode::Dark => &self.dark_palette,
            ThemeMode::System if self.system_dark => &self.dark_palette,
            ThemeMode::System => &self.light_palette,
        }
    }

    pub(super) fn handle_shortcuts(&mut self, context: &EguiContext) {
        let escape = context.input(|input| input.key_pressed(egui::Key::Escape));
        if escape && self.nav_menu_open {
            self.nav_menu_open = false;
        } else if escape && self.pdf_search_open {
            self.pdf_search_open = false;
        } else if escape && self.fullscreen {
            self.fullscreen = false;
            context.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        }

        if context.wants_keyboard_input() {
            return;
        }

        let command_k =
            context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::K));
        if command_k {
            if self.page == Page::Reader && self.viewer_mode == ViewerMode::Pdf {
                self.pdf_search_open = true;
                let id = Id::new("miredo-pdf-search");
                self.search_id = Some(id);
                context.memory_mut(|memory| memory.request_focus(id));
            } else {
                self.page = Page::Library;
                if let Some(search_id) = self.search_id {
                    context.memory_mut(|memory| memory.request_focus(search_id));
                }
            }
        }

        let next = context.input(|input| {
            input.key_pressed(egui::Key::N) || input.key_pressed(egui::Key::PageDown)
        });
        let previous = context
            .input(|input| input.key_pressed(egui::Key::P) || input.key_pressed(egui::Key::PageUp));
        let toggle_favorite = context.input(|input| input.key_pressed(egui::Key::F));
        let toggle_viewer = context.input(|input| input.key_pressed(egui::Key::V));
        let fullscreen = context.input(|input| {
            input.key_pressed(egui::Key::F11)
                || (input.modifiers.command
                    && input.modifiers.ctrl
                    && input.key_pressed(egui::Key::F))
        });
        let zoom_in = context.input(|input| input.key_pressed(egui::Key::Plus));
        let zoom_out = context.input(|input| input.key_pressed(egui::Key::Minus));

        if next {
            self.move_song(1);
        }
        if previous {
            self.move_song(-1);
        }
        if toggle_favorite && let Some(song_id) = self.selected_song_id.clone() {
            self.toggle_favorite(&song_id);
        }
        if toggle_viewer {
            self.toggle_viewer();
        }
        if fullscreen {
            self.fullscreen = !self.fullscreen;
            context.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
        }

        if zoom_in {
            self.change_zoom(0.1);
        }
        if zoom_out {
            self.change_zoom(-0.1);
        }
    }

    pub(super) fn poll_pdf(&mut self, context: &EguiContext) {
        while let Ok(response) = self.pdf_receiver.try_recv() {
            self.pdf_pending.remove(&response.key);
            if let Some(page_count) = response.page_count {
                self.pdf_page_counts
                    .insert(response.song_id.clone(), page_count.max(1));
            }
            match response.result {
                Ok(page) => {
                    if self.pdf_page_size.is_none() {
                        self.pdf_page_size =
                            Some(Vec2::new(page.size[0] as f32, page.size[1] as f32));
                    }
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(page.size, &page.rgba);
                    let texture = context.load_texture(
                        format!("miredo-{}", response.key),
                        color_image,
                        TextureOptions::LINEAR,
                    );
                    self.pdf_texture_order.retain(|key| key != &response.key);
                    self.pdf_texture_order.push_back(response.key.clone());
                    self.pdf_textures.insert(response.key.clone(), texture);
                    while self.pdf_texture_order.len() > MAX_PDF_TEXTURES {
                        if let Some(oldest_key) = self.pdf_texture_order.pop_front() {
                            self.pdf_textures.remove(&oldest_key);
                        }
                    }
                    self.pdf_errors.remove(&response.key);
                }
                Err(error) => {
                    self.pdf_errors.insert(response.key, error);
                }
            }
        }
    }

    pub(super) fn request_pdf_page(&mut self, song: &Song, page: usize, available: Vec2) {
        let render_zoom = self.pdf_fit_zoom(available);
        let key = pdf::page_key(&song.id, page, render_zoom);
        if self.pdf_textures.contains_key(&key) || !self.pdf_pending.insert(key.clone()) {
            return;
        }
        let Some(path) = song.pdf_path.clone() else {
            self.pdf_pending.remove(&key);
            return;
        };
        self.pdf_renderer.request_page(
            self.pdf_response_sender.clone(),
            song.id.clone(),
            page,
            render_zoom,
            path,
            self.pdf_cache_dir.clone(),
        );
    }

    pub(super) fn visible_ids(
        &self,
        favorites_only: bool,
        song_ids: Option<HashSet<String>>,
    ) -> Vec<String> {
        let filters = SearchFilters {
            collection: self.collection_filter,
            favorites_only,
            pdf_only: self.pdf_filter,
            song_ids,
        };
        search::search_songs(&self.songs, &self.query, &filters, &self.favorites)
            .into_iter()
            .map(|song| song.id.clone())
            .collect()
    }

    pub(super) fn song(&self, id: &str) -> Option<&Song> {
        self.songs.iter().find(|song| song.id == id)
    }

    pub(super) fn current_song(&self) -> Option<&Song> {
        self.selected_song_id
            .as_deref()
            .and_then(|song_id| self.song(song_id))
    }

    pub(super) fn title_for_song(&self, song: &Song) -> String {
        song.display_title(&self.tr("library.untitled"))
    }

    pub(super) fn open_song(&mut self, song_id: String, navigation_ids: Vec<String>) {
        self.selected_song_id = Some(song_id.clone());
        self.last_song_id = Some(song_id.clone());
        self.navigation_song_ids = navigation_ids;
        self.page = Page::Reader;
        self.pdf_page = self
            .storage
            .reading_position(&song_id, "pdf")
            .ok()
            .flatten()
            .map(|page| page.round().max(1.0) as usize)
            .unwrap_or(1);
        if let Err(error) = self.storage.set_preference("last_song", &song_id) {
            self.show_error(error);
        }
    }

    pub(super) fn toggle_favorite(&mut self, song_id: &str) {
        match self.storage.toggle_favorite(song_id) {
            Ok(true) => {
                self.favorites.insert(song_id.to_owned());
                self.set_status(self.tr("status.favorite_added"));
            }
            Ok(false) => {
                self.favorites.remove(song_id);
                self.set_status(self.tr("status.favorite_removed"));
            }
            Err(error) => self.show_error(error),
        }
    }

    pub(super) fn set_status(&mut self, message: String) {
        self.status = Some((message, Instant::now() + Duration::from_secs(4)));
    }

    pub(super) fn show_error(&mut self, error: impl std::fmt::Display) {
        self.set_status(format!("{}: {error}", self.tr("common.error")));
    }

    pub(super) fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_owned();
        if let Err(error) = self.storage.set_preference("locale", locale) {
            self.show_error(error);
        }
    }

    pub(super) fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
        if let Err(error) = self.storage.set_preference("theme", mode.preference()) {
            self.show_error(error);
        }
    }

    pub(super) fn refresh_user_data(&mut self) {
        match self.storage.favorite_ids() {
            Ok(favorites) => self.favorites = favorites,
            Err(error) => self.show_error(error),
        }
        match self.storage.playlists() {
            Ok(playlists) => self.playlists = playlists,
            Err(error) => self.show_error(error),
        }
        self.pdf_textures.clear();
        self.pdf_texture_order.clear();
        self.pdf_pending.clear();
        self.pdf_errors.clear();
        self.pdf_page_counts.clear();
        self.pdf_page_size = None;
    }

    pub(super) fn set_viewer_mode(&mut self, mode: ViewerMode) {
        self.viewer_mode = mode;
        if let Err(error) = self
            .storage
            .set_preference("last_viewer", mode.preference())
        {
            self.show_error(error);
        }
    }

    pub(super) fn set_pdf_fit_mode(&mut self, mode: PdfFitMode) {
        self.pdf_fit_mode = mode;
        if let Err(error) = self
            .storage
            .set_preference("pdf_fit_mode", mode.preference())
        {
            self.show_error(error);
        }
        self.pdf_textures.clear();
        self.pdf_texture_order.clear();
        self.pdf_pending.clear();
        self.pdf_errors.clear();
    }

    pub(super) fn change_zoom(&mut self, delta: f32) {
        if self.pdf_fit_mode != PdfFitMode::Manual {
            self.pdf_fit_mode = PdfFitMode::Manual;
            let _ = self
                .storage
                .set_preference("pdf_fit_mode", PdfFitMode::Manual.preference());
        }
        self.zoom = (self.zoom + delta).clamp(0.5, 3.0);
        if let Err(error) = self
            .storage
            .set_preference("pdf_zoom", &self.zoom.to_string())
        {
            self.show_error(error);
        }
        self.pdf_textures.clear();
        self.pdf_texture_order.clear();
        self.pdf_pending.clear();
        self.pdf_errors.clear();
    }

    pub(super) fn set_pdf_layout(&mut self, layout: PdfLayout) {
        self.pdf_layout = layout;
        if layout == PdfLayout::Double && self.pdf_page > 1 && self.pdf_page.is_multiple_of(2) {
            self.pdf_page -= 1;
        }
        self.pdf_textures.clear();
        self.pdf_texture_order.clear();
        self.pdf_pending.clear();
        self.pdf_errors.clear();
        let value = if layout == PdfLayout::Double {
            "double"
        } else {
            "single"
        };
        if let Err(error) = self.storage.set_preference("pdf_layout", value) {
            self.show_error(error);
        }
    }

    pub(super) fn pdf_fit_zoom(&self, available: Vec2) -> f32 {
        let Some(page_size) = self.pdf_page_size else {
            return self.zoom.max(0.5);
        };
        match self.pdf_fit_mode {
            PdfFitMode::Manual => self.zoom.max(0.5),
            PdfFitMode::FitWidth | PdfFitMode::FitHeight => pdf_fit_scale(page_size, available),
        }
    }

    pub(super) fn move_song(&mut self, direction: isize) {
        if self.navigation_song_ids.is_empty() {
            self.navigation_song_ids = self.visible_ids(false, None);
        }
        let Some(current_id) = self.selected_song_id.as_ref() else {
            return;
        };
        let Some(index) = self
            .navigation_song_ids
            .iter()
            .position(|song_id| song_id == current_id)
        else {
            return;
        };
        let next = index as isize + direction;
        if next < 0 || next >= self.navigation_song_ids.len() as isize {
            return;
        }
        let song_id = self.navigation_song_ids[next as usize].clone();
        self.open_song(song_id, self.navigation_song_ids.clone());
    }

    pub(super) fn toggle_viewer(&mut self) {
        self.set_viewer_mode(match self.viewer_mode {
            ViewerMode::Text => ViewerMode::Pdf,
            ViewerMode::Pdf => ViewerMode::Text,
        });
    }

    pub(super) fn collection_label(&self, collection: Collection) -> String {
        self.tr(collection.translation_key())
    }

    pub(super) fn page_heading(&self) -> String {
        match self.page {
            Page::Home => self.tr("nav.home"),
            Page::Library => self.tr("library.heading"),
            Page::Favorites => self.tr("favorites.heading"),
            Page::Playlists => self.tr("playlists.heading"),
            Page::Settings => self.tr("settings.heading"),
            Page::Help => self.tr("help.heading"),
            Page::About => self.tr("about.heading"),
            Page::Reader => self
                .current_song()
                .map(|song| self.title_for_song(song))
                .unwrap_or_else(|| self.tr("reader.text")),
        }
    }

    pub(super) fn page_subtitle(&self) -> String {
        match self.page {
            Page::Home => String::new(),
            Page::Library => self.tr("library.subtitle"),
            Page::Favorites => self.tr("favorites.subtitle"),
            Page::Playlists => self.tr("playlists.subtitle"),
            Page::Settings => self.tr("settings.subtitle"),
            Page::Help => self.tr("help.subtitle"),
            Page::About => self.tr("about.description"),
            Page::Reader => self
                .current_song()
                .map(|song| self.reader_subtitle(song))
                .unwrap_or_default(),
        }
    }

    pub(super) fn reader_subtitle(&self, song: &Song) -> String {
        let author = if song.authors.is_empty() {
            self.tr("common.no_author")
        } else {
            song.authors.join(", ")
        };
        format!("{} · {}", author, self.collection_label(song.collection))
    }
}
