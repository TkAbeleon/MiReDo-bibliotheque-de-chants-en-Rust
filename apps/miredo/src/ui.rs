use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use eframe::egui::{
    self, Align, Context as EguiContext, Id, Layout, RichText, ScrollArea, Stroke, TextureHandle,
    TextureOptions, Vec2, ViewportCommand,
};

use crate::data::{self, DataLoadReport};
use crate::domain::{Collection, Playlist, Song};
use crate::pdf::{self, PdfRenderResponse};
use crate::resources::{Palette, Translator};
use crate::search::{self, SearchFilters};
use crate::storage::UserStorage;

const APP_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Home,
    Library,
    Favorites,
    Playlists,
    Settings,
    Help,
    About,
    Reader,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    fn from_preference(value: Option<String>) -> Self {
        match value.as_deref() {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }

    fn preference(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewerMode {
    Text,
    Pdf,
}

impl ViewerMode {
    fn from_preference(value: Option<String>) -> Self {
        if value.as_deref() == Some("pdf") {
            Self::Pdf
        } else {
            Self::Text
        }
    }

    fn preference(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Pdf => "pdf",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PdfLayout {
    Single,
    Double,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PdfFitMode {
    FitScreen,
    FitWidth,
    FitHeight,
    Manual,
}

impl PdfFitMode {
    fn from_preference(value: Option<String>) -> Self {
        match value.as_deref() {
            Some("width") => Self::FitWidth,
            Some("height") => Self::FitHeight,
            Some("manual") => Self::Manual,
            _ => Self::FitScreen,
        }
    }

    fn preference(self) -> &'static str {
        match self {
            Self::FitScreen => "screen",
            Self::FitWidth => "width",
            Self::FitHeight => "height",
            Self::Manual => "manual",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlaylistDialog {
    Create,
    Rename,
    Delete,
}

pub struct MiReDoApp {
    translator: Translator,
    light_palette: Palette,
    dark_palette: Palette,
    storage: UserStorage,
    songs: Vec<Song>,
    report: DataLoadReport,
    favorites: HashSet<String>,
    playlists: Vec<Playlist>,
    locale: String,
    theme_mode: ThemeMode,
    system_dark: bool,
    page: Page,
    viewer_mode: ViewerMode,
    pdf_layout: PdfLayout,
    pdf_fit_mode: PdfFitMode,
    zoom: f32,
    query: String,
    search_id: Option<Id>,
    pdf_search_open: bool,
    collection_filter: Option<Collection>,
    pdf_filter: bool,
    selected_song_id: Option<String>,
    last_song_id: Option<String>,
    navigation_song_ids: Vec<String>,
    selected_playlist_id: Option<String>,
    playlist_dialog: Option<PlaylistDialog>,
    playlist_dialog_name: String,
    dialog_error: Option<String>,
    status: Option<(String, Instant)>,
    fullscreen: bool,
    pdf_page: usize,
    pdf_sender: Sender<PdfRenderResponse>,
    pdf_receiver: Receiver<PdfRenderResponse>,
    pdf_textures: HashMap<String, TextureHandle>,
    pdf_errors: HashMap<String, String>,
    pdf_pending: HashSet<String>,
    pdf_page_counts: HashMap<String, usize>,
    pdf_cache_dir: PathBuf,
}

impl MiReDoApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Result<Self> {
        let resources_dir = PathBuf::from(APP_DIR).join("resources");
        let translator = Translator::load(&resources_dir.join("i18n.json"))?;
        let light_palette = Palette::load(&resources_dir.join("palette.json"), "light")?;
        let dark_palette = Palette::load(&resources_dir.join("palette.json"), "dark")?;
        let data_dir = PathBuf::from(APP_DIR).join("data");
        let (songs, report) = data::load_library(&data_dir)?;
        let storage = UserStorage::open()?;
        let locale = storage
            .preference("locale")?
            .filter(|locale| ["fr", "mg", "en"].contains(&locale.as_str()))
            .unwrap_or_else(|| "fr".to_owned());
        let theme_mode = ThemeMode::from_preference(storage.preference("theme")?);
        let viewer_mode = ViewerMode::from_preference(storage.preference("last_viewer")?);
        let pdf_layout = if storage.preference("pdf_layout")?.as_deref() == Some("double") {
            PdfLayout::Double
        } else {
            PdfLayout::Single
        };
        let pdf_fit_mode = PdfFitMode::from_preference(storage.preference("pdf_fit_mode")?);
        let zoom = storage
            .preference("pdf_zoom")?
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(1.0)
            .clamp(0.5, 3.0);
        let favorites = storage.favorite_ids()?;
        let playlists = storage.playlists()?;
        let last_song_id = storage.preference("last_song")?;
        let (pdf_sender, pdf_receiver) = channel();
        let pdf_cache_dir = directories::ProjectDirs::from("org", "MiReDo", "MiReDo")
            .context("Impossible de trouver le dossier de cache")?
            .cache_dir()
            .join("pdf-pages");

        Ok(Self {
            translator,
            light_palette,
            dark_palette,
            storage,
            songs,
            report,
            favorites,
            playlists,
            locale,
            theme_mode,
            system_dark: std::env::var("GTK_THEME")
                .map(|value| value.to_lowercase().contains("dark"))
                .unwrap_or(false),
            page: Page::Home,
            viewer_mode,
            pdf_layout,
            pdf_fit_mode,
            zoom,
            query: String::new(),
            search_id: None,
            pdf_search_open: false,
            collection_filter: None,
            pdf_filter: false,
            selected_song_id: last_song_id.clone(),
            last_song_id,
            navigation_song_ids: Vec::new(),
            selected_playlist_id: None,
            playlist_dialog: None,
            playlist_dialog_name: String::new(),
            dialog_error: None,
            status: None,
            fullscreen: false,
            pdf_page: 1,
            pdf_sender,
            pdf_receiver,
            pdf_textures: HashMap::new(),
            pdf_errors: HashMap::new(),
            pdf_pending: HashSet::new(),
            pdf_page_counts: HashMap::new(),
            pdf_cache_dir,
        })
    }

    fn tr(&self, key: &str) -> String {
        self.translator.text(&self.locale, key)
    }

    fn palette(&self) -> &Palette {
        match self.theme_mode {
            ThemeMode::Light => &self.light_palette,
            ThemeMode::Dark => &self.dark_palette,
            ThemeMode::System if self.system_dark => &self.dark_palette,
            ThemeMode::System => &self.light_palette,
        }
    }

    fn apply_theme(&self, context: &EguiContext) {
        let palette = self.palette();
        let dark = matches!(self.theme_mode, ThemeMode::Dark)
            || (self.theme_mode == ThemeMode::System && self.system_dark);
        let mut visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = palette.get("background");
        visuals.window_fill = palette.get("surface_elevated");
        visuals.extreme_bg_color = palette.get("surface");
        visuals.faint_bg_color = palette.get("surface_hover");
        visuals.selection.bg_fill = palette.get("surface_selected");
        visuals.selection.stroke = Stroke::new(1.0_f32, palette.get("focus"));
        visuals.widgets.noninteractive.bg_fill = palette.get("surface");
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.get("text_primary"));
        visuals.widgets.inactive.bg_fill = palette.get("surface");
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.get("text_secondary"));
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.get("border_subtle"));
        visuals.widgets.hovered.bg_fill = palette.get("surface_hover");
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.get("text_primary"));
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.get("border_strong"));
        visuals.widgets.active.bg_fill = palette.get("surface_selected");
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, palette.get("text_primary"));
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, palette.get("focus"));
        visuals.hyperlink_color = palette.get("accent");
        visuals.override_text_color = Some(palette.get("text_primary"));
        visuals.window_stroke = Stroke::new(1.0_f32, palette.get("border_subtle"));
        visuals.widgets.noninteractive.corner_radius = 6.into();
        visuals.widgets.inactive.corner_radius = 6.into();
        visuals.widgets.hovered.corner_radius = 6.into();
        visuals.widgets.active.corner_radius = 6.into();
        context.set_visuals(visuals);
    }

    fn handle_shortcuts(&mut self, context: &EguiContext) {
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

        let escape = context.input(|input| input.key_pressed(egui::Key::Escape));
        if escape && self.pdf_search_open {
            self.pdf_search_open = false;
        } else if escape && self.fullscreen {
            self.fullscreen = false;
            context.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        }

        if context.wants_keyboard_input() {
            return;
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
        if toggle_favorite {            if let Some(song_id) = self.selected_song_id.clone() {
                self.toggle_favorite(&song_id);
            }
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

    fn poll_pdf(&mut self, context: &EguiContext) {
        while let Ok(response) = self.pdf_receiver.try_recv() {
            self.pdf_pending.remove(&response.key);
            if let Some(page_count) = response.page_count {
                self.pdf_page_counts
                    .insert(response.song_id.clone(), page_count.max(1));
            }
            match response.result {
                Ok(page) => {
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(page.size, &page.rgba);
                    let texture = context.load_texture(
                        format!("miredo-{}", response.key),
                        color_image,
                        TextureOptions::LINEAR,
                    );
                    self.pdf_textures.insert(response.key.clone(), texture);
                    self.pdf_errors.remove(&response.key);
                }
                Err(error) => {
                    self.pdf_errors.insert(response.key, error);
                }
            }
        }
    }

    fn request_pdf_page(&mut self, song: &Song, page: usize) {
        let render_zoom = self.pdf_render_zoom();
        let key = pdf::page_key(&song.id, page, render_zoom);
        if self.pdf_textures.contains_key(&key) || !self.pdf_pending.insert(key.clone()) {
            return;
        }
        let Some(path) = song.pdf_path.clone() else {
            self.pdf_pending.remove(&key);
            return;
        };
        pdf::request_page(
            self.pdf_sender.clone(),
            song.id.clone(),
            page,
            render_zoom,
            path,
            self.pdf_cache_dir.clone(),
        );
    }

    fn visible_ids(&self, favorites_only: bool, song_ids: Option<HashSet<String>>) -> Vec<String> {
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

    fn song(&self, id: &str) -> Option<&Song> {
        self.songs.iter().find(|song| song.id == id)
    }

    fn current_song(&self) -> Option<&Song> {
        self.selected_song_id
            .as_deref()
            .and_then(|song_id| self.song(song_id))
    }

    fn title_for_song(&self, song: &Song) -> String {
        song.display_title(&self.tr("library.untitled"))
    }

    fn open_song(&mut self, song_id: String, navigation_ids: Vec<String>) {
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

    fn toggle_favorite(&mut self, song_id: &str) {
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

    fn refresh_user_data(&mut self) {
        match self.storage.favorite_ids() {
            Ok(favorites) => self.favorites = favorites,
            Err(error) => self.show_error(error),
        }
        match self.storage.playlists() {
            Ok(playlists) => self.playlists = playlists,
            Err(error) => self.show_error(error),
        }
    }

    fn set_status(&mut self, message: String) {
        self.status = Some((message, Instant::now() + Duration::from_secs(4)));
    }

    fn show_error(&mut self, error: impl std::fmt::Display) {
        self.set_status(format!("{}: {error}", self.tr("common.error")));
    }

    fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_owned();
        if let Err(error) = self.storage.set_preference("locale", locale) {
            self.show_error(error);
        }
    }

    fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
        if let Err(error) = self.storage.set_preference("theme", mode.preference()) {
            self.show_error(error);
        }
    }

    fn set_viewer_mode(&mut self, mode: ViewerMode) {
        self.viewer_mode = mode;
        if let Err(error) = self
            .storage
            .set_preference("last_viewer", mode.preference())
        {
            self.show_error(error);
        }
    }

    fn set_pdf_fit_mode(&mut self, mode: PdfFitMode) {
        self.pdf_fit_mode = mode;
        if let Err(error) = self
            .storage
            .set_preference("pdf_fit_mode", mode.preference())
        {
            self.show_error(error);
        }
        self.pdf_textures.clear();
        self.pdf_pending.clear();
        self.pdf_errors.clear();
    }

    fn change_zoom(&mut self, delta: f32) {
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
        self.pdf_pending.clear();
        self.pdf_errors.clear();
    }

    fn set_pdf_layout(&mut self, layout: PdfLayout) {
        self.pdf_layout = layout;
        if layout == PdfLayout::Double && self.pdf_page > 1 && self.pdf_page % 2 == 0 {
            self.pdf_page -= 1;
        }
        let value = if layout == PdfLayout::Double { "double" } else { "single" };
        if let Err(error) = self.storage.set_preference("pdf_layout", value) {
            self.show_error(error);
        }
    }

    fn pdf_render_zoom(&self) -> f32 {
        match self.pdf_fit_mode {
            PdfFitMode::Manual => self.zoom.max(0.5),
            PdfFitMode::FitScreen | PdfFitMode::FitWidth | PdfFitMode::FitHeight => 2.0,
        }
    }

    fn move_song(&mut self, direction: isize) {
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

    fn toggle_viewer(&mut self) {
        self.set_viewer_mode(match self.viewer_mode {
            ViewerMode::Text => ViewerMode::Pdf,
            ViewerMode::Pdf => ViewerMode::Text,
        });
    }

    fn collection_label(&self, collection: Collection) -> String {
        self.tr(collection.translation_key())
    }

    fn page_heading(&self) -> String {
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

    fn page_subtitle(&self) -> String {
        match self.page {
            Page::Home => self.tr("home.subtitle"),
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

    fn reader_subtitle(&self, song: &Song) -> String {
        let author = if song.authors.is_empty() {
            self.tr("common.no_author")
        } else {
            song.authors.join(", ")
        };
        format!("{} · {}", author, self.collection_label(song.collection))
    }

    fn draw_sidebar(&mut self, context: &EguiContext) {
        let compact = context.screen_rect().width() < 980.0;
        let width = if compact { 72.0 } else { 236.0 };
        egui::SidePanel::left("miredo-sidebar")
            .resizable(false)
            .exact_width(width)
            .frame(
                egui::Frame::new()
                    .fill(self.palette().get("surface"))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if compact { "M" } else { "MiReDo" })
                            .size(22.0)
                            .strong()
                            .color(self.palette().get("accent")),
                    );
                });
                if !compact {
                    ui.label(
                        RichText::new(self.tr("app.tagline"))
                            .size(11.0)
                            .color(self.palette().get("text_muted")),
                    );
                }
                ui.add_space(22.0);

                self.nav_item(ui, Page::Home, "nav.home", "⌂", compact);
                self.nav_item(ui, Page::Library, "nav.library", "≡", compact);
                self.nav_item(ui, Page::Favorites, "nav.favorites", "☆", compact);
                self.nav_item(ui, Page::Playlists, "nav.playlists", "▤", compact);

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                self.nav_item(ui, Page::Help, "nav.help", "?", compact);
                self.nav_item(ui, Page::About, "nav.about", "i", compact);
                self.nav_item(ui, Page::Settings, "nav.settings", "⚙", compact);

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.add_space(8.0);
                    let language = match self.locale.as_str() {
                        "mg" => "MG",
                        "en" => "EN",
                        _ => "FR",
                    };
                    let response = ui.add_sized(
                        [ui.available_width(), 34.0],
                        egui::Button::new(                            RichText::new(if compact {
                                language.to_owned()
                            } else {
                                format!("{}  {language}", self.tr("settings.language"))
                            })
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
    }

    fn nav_item(&mut self, ui: &mut egui::Ui, page: Page, key: &str, glyph: &str, compact: bool) {
        let label = if compact {
            glyph.to_owned()
        } else {
            format!("{glyph}   {}", self.tr(key))
        };
        let selected = self.page == page;
        let button = egui::Button::new(RichText::new(label).color(if selected {
            self.palette().get("accent")
        } else {
            self.palette().get("text_secondary")
        }))
        .fill(if selected {
            self.palette().get("surface_selected")
        } else {
            self.palette().get("surface")
        })
        .stroke(Stroke::NONE)
        .min_size(Vec2::new(ui.available_width(), 38.0));
        let response = ui.add(button);
        if compact {
            response.clone().on_hover_text(self.tr(key));
        }
        if response.clicked() {
            self.page = page;
        }
    }

    fn draw_topbar(&mut self, ui: &mut egui::Ui) {
        let search_placeholder = self.tr("search.placeholder");
        ui.horizontal(|ui| {
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
                if self.page == Page::Reader && self.current_song().is_some() {
                    if ui
                        .button(self.tr("common.close"))
                        .on_hover_text(self.tr("common.close"))
                        .clicked()
                    {
                        self.page = Page::Library;
                    }
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
            ui.label(
                RichText::new("⌕")
                    .size(18.0)
                    .color(self.palette().get("text_muted")),
            );
            let available = ui.available_width().min(660.0);
            let response = ui.add_sized(
                [available, 36.0],
                egui::TextEdit::singleline(&mut self.query)
                    .id_salt("miredo-global-search")
                    .hint_text(search_placeholder),
            );
            self.search_id = Some(response.id);
            if response.changed() && self.page != Page::Library {
                self.page = Page::Library;
            }
            if !self.query.is_empty() && ui.button("×").clicked() {
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

    fn draw_page(&mut self, context: &EguiContext, ui: &mut egui::Ui) {
        match self.page {
            Page::Home => self.draw_home(ui),
            Page::Library => self.draw_library(ui),
            Page::Favorites => self.draw_favorites(ui),
            Page::Playlists => self.draw_playlists(ui),
            Page::Settings => self.draw_settings(ui),
            Page::Help => self.draw_help(ui),
            Page::About => self.draw_about(ui),
            Page::Reader => self.draw_reader(context, ui),
        }
    }

    fn draw_home(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(
            RichText::new(self.tr("home.greeting"))
                .size(27.0)
                .strong()
                .color(self.palette().get("text_primary")),
        );
        ui.label(
            RichText::new(self.tr("home.subtitle"))
                .size(14.0)
                .color(self.palette().get("text_secondary")),
        );
        ui.add_space(22.0);

        let action_label = self.tr("home.library_link");
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("{}   →", action_label))
                        .strong()
                        .color(self.palette().get("accent_text")),
                )
                .fill(self.palette().get("accent"))
                .min_size(Vec2::new(230.0, 44.0)),
            )
            .clicked()
        {
            self.page = Page::Library;
        }

        ui.add_space(28.0);
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
                                if ui.button(self.tr("common.open")).clicked() {
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
                ui.label(self.tr("home.continue_empty"));
            }
        } else {
            ui.label(self.tr("home.continue_empty"));
        }

        ui.add_space(26.0);
        ui.label(
            RichText::new(self.tr("home.collections"))
                .size(17.0)
                .strong(),
        );
        ui.add_space(8.0);
        let counts = data::collection_counts(&self.songs);
        ui.horizontal_wrapped(|ui| {
            for collection in Collection::ALL {
                let count = counts.get(&collection).copied().unwrap_or_default();
                egui::Frame::new()
                    .fill(self.palette().get("surface"))
                    .stroke(Stroke::new(1.0_f32, self.palette().get("border_subtle")))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_min_width(190.0);
                        ui.label(
                            RichText::new(self.collection_label(collection))
                                .size(14.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(count.to_string())
                                .size(20.0)
                                .color(self.palette().get("accent")),
                        );
                    });
            }
        });
    }

    fn draw_library(&mut self, ui: &mut egui::Ui) {
        self.draw_filters(ui, false);
        let ids = self.visible_ids(false, None);
        self.draw_result_count(ui, ids.len());
        self.draw_song_list(ui, ids, None);
    }

    fn draw_favorites(&mut self, ui: &mut egui::Ui) {
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
                    for (collection, label) in &collection_labels {                        if ui
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
        ui.add_space(5.0);
    }

    fn draw_song_list(
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

                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [44.0, 30.0],
                            egui::Label::new(
                                RichText::new(number)
                                    .monospace()
                                    .color(self.palette().get("text_muted")),
                            ),
                        );
                        ui.vertical(|ui| {
                            let response = ui.add(
                                egui::Button::new(
                                    RichText::new(title)
                                        .strong()
                                        .color(self.palette().get("text_primary")),
                                )
                                .selected(selected)
                                .frame(false),
                            );
                            if response.clicked() {
                                open_id = Some(song_id.clone());
                            }
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(subtitle)
                                        .size(11.5)
                                        .color(self.palette().get("text_secondary")),
                                );
                                if has_pdf {
                                    ui.label(
                                        RichText::new("PDF")
                                            .size(10.0)
                                            .color(self.palette().get("accent")),
                                    );
                                }
                            });
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let star = if is_favorite { "★" } else { "☆" };
                            let favorite = ui
                                .button(
                                    RichText::new(star)
                                        .size(18.0)
                                        .color(self.palette().get("accent")),
                                )
                                .on_hover_text(if is_favorite {
                                    self.tr("status.favorite_removed")
                                } else {
                                    self.tr("status.favorite_added")
                                });
                            if favorite.clicked() {
                                favorite_id = Some(song_id.clone());
                            }
                            if remove_from_playlist.is_some() {
                                let remove = ui
                                    .button("×")
                                    .on_hover_text(self.tr("playlists.remove_song"));
                                if remove.clicked() {
                                    remove_id = Some(song_id.clone());
                                }
                            }
                        });
                    });
                    ui.separator();
                }
            });

        if let Some(song_id) = favorite_id {
            self.toggle_favorite(&song_id);
        }
        if let Some(song_id) = remove_id {
            if let Some(playlist_id) = remove_from_playlist {
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
        }
        if let Some(song_id) = open_id {
            self.open_song(song_id, navigation_ids);
        }
    }

    fn draw_playlists(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let create_label = self.tr("playlists.create");
            if ui.button(format!("＋  {create_label}")).clicked() {
                self.playlist_dialog = Some(PlaylistDialog::Create);
                self.playlist_dialog_name.clear();
                self.dialog_error = None;
            }
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
                Vec2::new(244.0, ui.available_height()),
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
                        let label = format!("{}  ·  {count} {count_label}", playlist.name);
                        if ui.selectable_label(selected, label).clicked() {
                            self.selected_playlist_id = Some(playlist.id.clone());
                        }
                        if selected {
                            ui.horizontal(|ui| {
                                if ui.button(self.tr("common.rename")).clicked() {
                                    self.playlist_dialog_name = playlist.name.clone();
                                    self.playlist_dialog = Some(PlaylistDialog::Rename);
                                    self.dialog_error = None;
                                }
                                if ui.button(self.tr("common.delete")).clicked() {
                                    dialog_action =
                                        Some((PlaylistDialog::Delete, playlist.id.clone()));
                                }
                            });
                        }
                        ui.add_space(7.0);
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
                    ui.heading(&playlist.name);
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
                    ui.add_space(10.0);
                    self.draw_song_list(ui, playlist.song_ids.clone(), Some(playlist.id.clone()));
                }
            });
        });
        if let Some(action) = dialog_action {
            self.playlist_dialog = Some(action.0);
            self.selected_playlist_id = Some(action.1);
        }
    }

    fn draw_pdf_reader_immersive(&mut self, context: &EguiContext, ui: &mut egui::Ui) {
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
        let next_enabled = navigation_position
            .is_some_and(|index| index + 1 < self.navigation_song_ids.len());

        ui.horizontal(|ui| {
            ui.set_min_height(34.0);

            if ui
                .button("‹")
                .on_hover_text(self.tr("common.close"))
                .clicked()
            {
                self.page = Page::Library;
            }

            if ui
                .add_enabled(
                    previous_enabled,
                    egui::Button::new("←"),
                )
                .on_hover_text(self.tr("common.previous"))
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

            if ui
                .add_enabled(next_enabled, egui::Button::new("→"))
                .on_hover_text(self.tr("common.next"))
                .clicked()
            {
                self.move_song(1);
            }

            ui.separator();

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

            if ui
                .selectable_label(false, self.tr("reader.text"))
                .on_hover_text(self.tr("reader.text"))
                .clicked()
            {
                self.set_viewer_mode(ViewerMode::Text);
            }

            if ui
                .button("⌕")
                .on_hover_text(self.tr("search.placeholder"))
                .clicked()
            {
                self.pdf_search_open = true;
                self.search_id = Some(Id::new("miredo-pdf-search"));
            }

            ui.separator();

            if ui
                .button("−")
                .on_hover_text(self.tr("reader.zoom_out"))
                .clicked()
            {
                self.change_zoom(-0.1);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui
                .button("+")
                .on_hover_text(self.tr("reader.zoom_in"))
                .clicked()
            {
                self.change_zoom(0.1);
            }

            ui.separator();

            let total_pages = self.pdf_page_counts.get(&song.id).copied();
            let is_book = self.pdf_layout == PdfLayout::Double;
            let step = if is_book { 2 } else { 1 };
            let current_start = if is_book && self.pdf_page > 1 && self.pdf_page % 2 == 0 {
                self.pdf_page - 1
            } else {
                self.pdf_page
            };
            let next_start = current_start.saturating_add(step);

            if ui
                .add_enabled(current_start > 1, egui::Button::new("‹"))
                .on_hover_text(self.tr("common.previous"))
                .clicked()
            {
                self.set_pdf_page(&song.id, current_start.saturating_sub(step).max(1));
            }

            ui.label(
                total_pages
                    .map(|total| {
                        if is_book && current_start < total {
                            format!("{} {}–{} / {}", self.tr("reader.page"), current_start, (current_start + 1).min(total), total)
                        } else {
                            format!("{} {} / {}", self.tr("reader.page"), current_start, total)
                        }
                    })
                    .unwrap_or_else(|| format!("{} {}", self.tr("reader.page"), current_start)),
            );

            if ui
                .add_enabled(
                    total_pages.is_none_or(|total| next_start <= total),
                    egui::Button::new("›"),
                )
                .on_hover_text(self.tr("common.next"))
                .clicked()
            {
                self.set_pdf_page(&song.id, next_start);
            }

            ui.separator();

            ui.menu_button("Aa", |ui| {
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::FitScreen, self.tr("reader.fit_screen"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitScreen);
                    ui.close();
                }
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::FitWidth, self.tr("reader.fit_width"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitWidth);
                    ui.close();
                }
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::FitHeight, self.tr("reader.fit_height"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitHeight);
                    ui.close();
                }
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::Manual, self.tr("reader.manual_zoom"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::Manual);
                    ui.close();
                }
            });

            ui.label(match self.pdf_fit_mode {
                PdfFitMode::FitScreen => self.tr("reader.fit_screen_short"),
                PdfFitMode::FitWidth => self.tr("reader.fit_width_short"),
                PdfFitMode::FitHeight => self.tr("reader.fit_height_short"),
                PdfFitMode::Manual => format!("{:.0}%", self.zoom * 100.0),
            });

            if ui
                .button("−")
                .on_hover_text(self.tr("reader.zoom_out"))
                .clicked()
            {
                self.change_zoom(-0.1);
            }
            if ui
                .button("+")
                .on_hover_text(self.tr("reader.zoom_in"))
                .clicked()
            {
                self.change_zoom(0.1);
            }

            ui.separator();

            if ui
                .selectable_label(self.pdf_layout == PdfLayout::Single, self.tr("reader.single_page"))
                .clicked()
            {
                self.set_pdf_layout(PdfLayout::Single);
            }
            if ui
                .selectable_label(self.pdf_layout == PdfLayout::Double, self.tr("reader.book_mode"))
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
                            .and_then(|id| self.playlists.iter().find(|p| &p.id == id))
                            .map(|p| p.name.clone())
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

            if ui
                .button("⛶")
                .on_hover_text(self.tr("reader.fullscreen"))
                .clicked()
            {
                self.fullscreen = !self.fullscreen;
                context.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
        });

        if self.pdf_search_open {
            ui.add_space(2.0);

            // Préparer les valeurs avant d'emprunter self.query mutuellement
            // dans TextEdit. Cela évite le conflit E0502 du borrow-checker.
            let search_placeholder = self.tr("search.placeholder");
            let results_label = self.tr("search.results");
            let result_count = self.visible_ids(false, None).len();

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("⌕")
                        .color(self.palette().get("text_muted")),
                );
                let response = ui.add_sized(
                    [320.0, 30.0],
                    egui::TextEdit::singleline(&mut self.query)
                        .id_salt("miredo-pdf-search")
                        .hint_text(search_placeholder),
                );
                self.search_id = Some(response.id);
                if ui.button("×").clicked() {
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
                                    if ui
                                        .selectable_label(false, label)
                                        .clicked()
                                    {
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

    fn draw_pdf_canvas_immersive(
        &mut self,
        ui: &mut egui::Ui,
        song: &Song,
    ) {
        let total_pages = self.pdf_page_counts.get(&song.id).copied();
        let first_page = if self.pdf_layout == PdfLayout::Double
            && self.pdf_page > 1
            && self.pdf_page % 2 == 0
        {
            self.pdf_page - 1
        } else {
            self.pdf_page
        };

        let pages = if self.pdf_layout == PdfLayout::Double {
            vec![first_page, first_page + 1]
        } else {
            vec![first_page]
        };

        for page in &pages {
            if total_pages.is_none_or(|total| *page <= total) {
                self.request_pdf_page(song, *page);
            }
        }

        // Préchargement discret de la page voisine pour rendre précédent/suivant immédiat.
        for adjacent in [
            self.pdf_page.saturating_sub(1),
            self.pdf_page + 1,
            self.pdf_page + 2,
        ] {
            if adjacent >= 1 && total_pages.is_none_or(|total| adjacent <= total) {
                self.request_pdf_page(song, adjacent);
            }
        }

        ScrollArea::both()
            .id_salt("miredo-pdf-immersive")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let available = ui.available_size();

                if pages.len() == 2 {
                    let gap = 8.0_f32;
                    let page_width = ((available.x - gap) / 2.0).max(1.0);
                    ui.horizontal_centered(|ui| {
                        let page_available = Vec2::new(page_width, available.y.max(1.0));
                        self.draw_pdf_page(ui, &song.id, pages[0], page_available);
                        ui.add_space(gap);
                        self.draw_pdf_page(ui, &song.id, pages[1], page_available);
                    });
                } else {
                    self.draw_pdf_page(
                        ui,
                        &song.id,
                        pages[0],
                        Vec2::new(available.x.max(1.0), available.y.max(1.0)),
                    );
                }
            });
    }

    fn draw_reader(&mut self, context: &EguiContext, ui: &mut egui::Ui) {
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
                    .on_hover_text(if is_favorite {                        self.tr("status.favorite_removed")
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

    fn draw_pdf_viewer(&mut self, context: &EguiContext, ui: &mut egui::Ui, song: &Song) {
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
            let previous_enabled = self.pdf_page > 1;
            let next_enabled = total_pages.is_none_or(|total| self.pdf_page < total);
            if ui
                .add_enabled(previous_enabled, egui::Button::new("‹"))
                .on_hover_text(self.tr("common.previous"))
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
            if ui
                .add_enabled(next_enabled, egui::Button::new("›"))
                .on_hover_text(self.tr("common.next"))
                .clicked()
            {
                self.set_pdf_page(&song.id, self.pdf_page + 1);
            }
            ui.separator();
            if ui
                .button("−")
                .on_hover_text(self.tr("reader.zoom_out"))
                .clicked()
            {
                self.change_zoom(-0.1);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui
                .button("+")
                .on_hover_text(self.tr("reader.zoom_in"))
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
            if ui
                .button("⛶")
                .on_hover_text(self.tr("reader.fullscreen"))
                .clicked()
            {
                self.fullscreen = !self.fullscreen;
                context.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
        });

        let pages = if self.pdf_layout == PdfLayout::Double {
            vec![self.pdf_page, self.pdf_page + 1]
        } else {
            vec![self.pdf_page]
        };
        for page in &pages {
            if total_pages.is_none_or(|total| *page <= total) {
                self.request_pdf_page(song, *page);
            }
        }

        let available = ui.available_size();
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
                                        available.x / 2.0,
                                        available.y,
                                    );
                                }
                            });
                        } else {
                            self.draw_pdf_page(
                                ui,
                                &song.id,
                                self.pdf_page,
                                available.x,
                                available.y,
                            );
                        }
                    });
            });
        let _ = path;
    }

    fn draw_pdf_page(
        &self,
        ui: &mut egui::Ui,
        song_id: &str,
        page: usize,
        available: Vec2,
    ) {
        let render_zoom = self.pdf_render_zoom();
        let key = pdf::page_key(song_id, page, render_zoom);
        if let Some(texture) = self.pdf_textures.get(&key) {
            let size = texture.size_vec2();
            let logical_size = size / render_zoom.max(0.01);

            let scale = match self.pdf_fit_mode {
                PdfFitMode::FitScreen => {
                    (available.x / logical_size.x)
                        .min(available.y / logical_size.y)
                        .clamp(0.1, 6.0)
                }
                PdfFitMode::FitWidth => (available.x / logical_size.x).clamp(0.1, 6.0),
                PdfFitMode::FitHeight => (available.y / logical_size.y).clamp(0.1, 6.0),
                PdfFitMode::Manual => self.zoom.clamp(0.1, 6.0),
            };

            let display_size = logical_size * scale;
            ui.vertical(|ui| {
                ui.set_min_width(display_size.x);
                ui.image((texture.id(), display_size));
            });
        } else if self.pdf_pending.contains(&key) {
            ui.centered_and_justified(|ui| {
                ui.label(self.tr("common.loading"));
            });        } else if self.pdf_errors.contains_key(&key) {
            ui.centered_and_justified(|ui| {
                ui.label(self.tr("reader.pdf_error"));
            });
        }
    }

    fn set_pdf_page(&mut self, song_id: &str, page: usize) {
        self.pdf_page = page.max(1);
        if let Err(error) = self
            .storage
            .save_reading_position(song_id, "pdf", self.pdf_page as f64)
        {
            self.show_error(error);
        }
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            self.settings_section(ui, "settings.appearance");
            ui.horizontal(|ui| {
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
            ui.horizontal(|ui| {
                ui.label(self.tr("settings.language"));
                for (locale, label) in [("fr", "FR"), ("mg", "MG"), ("en", "EN")] {
                    if ui.selectable_label(self.locale == locale, label).clicked() {
                        self.set_locale(locale);
                    }
                }
            });

            ui.add_space(18.0);
            self.settings_section(ui, "settings.reader");
            ui.horizontal(|ui| {
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
            ui.horizontal(|ui| {
                ui.label(self.tr("settings.initial_zoom"));
                if ui.button("−").clicked() {
                    self.change_zoom(-0.1);
                }
                ui.label(format!("{:.0}%", self.zoom * 100.0));
                if ui.button("+").clicked() {
                    self.change_zoom(0.1);
                }
            });

            ui.add_space(18.0);
            self.settings_section(ui, "settings.pdf_fit");
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::FitScreen, self.tr("reader.fit_screen"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitScreen);
                }
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::FitWidth, self.tr("reader.fit_width"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitWidth);
                }
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::FitHeight, self.tr("reader.fit_height"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::FitHeight);
                }
                if ui
                    .selectable_label(self.pdf_fit_mode == PdfFitMode::Manual, self.tr("reader.manual_zoom"))
                    .clicked()
                {
                    self.set_pdf_fit_mode(PdfFitMode::Manual);
                }
            });

            ui.add_space(18.0);
            self.settings_section(ui, "settings.data");
            ui.label(format!(
                "{}: {}",
                self.tr("settings.data_path"),
                PathBuf::from(APP_DIR).join("data").display()
            ));
            if let Some(path) = UserStorage::database_path() {
                ui.label(format!(
                    "{}: {}",
                    self.tr("settings.database_path"),
                    path.display()
                ));
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
                        self.pdf_page_counts.clear();
                        self.set_status(self.tr("settings.reloaded"));
                    }
                    Err(error) => {
                        self.set_status(format!("{}: {error}", self.tr("settings.reload_failed")));
                    }
                }
            }

            ui.add_space(18.0);
            self.settings_section(ui, "settings.keyboard");
            for (key, shortcut) in [
                ("settings.shortcut_search", "Ctrl / ⌘  K"),
                ("settings.shortcut_next", "N / PageDown"),
                ("settings.shortcut_previous", "P / PageUp"),
                ("settings.shortcut_favorite", "F"),
                ("settings.shortcut_viewer", "V"),
                ("settings.shortcut_escape", "Esc"),
            ] {
                ui.horizontal(|ui| {
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

    fn settings_section(&self, ui: &mut egui::Ui, key: &str) {
        ui.label(
            RichText::new(self.tr(key))
                .size(16.0)
                .strong()
                .color(self.palette().get("accent")),
        );
        ui.separator();
        ui.add_space(8.0);
    }

    fn draw_help(&self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            for (title, body) in [
                ("help.search_title", "help.search_body"),
                ("help.reader_title", "help.reader_body"),
                ("help.organize_title", "help.organize_body"),
                ("help.offline_title", "help.offline_body"),
            ] {
                ui.label(RichText::new(self.tr(title)).size(16.0).strong());
                ui.add_space(4.0);
                ui.label(RichText::new(self.tr(body)).color(self.palette().get("text_secondary")));
                ui.add_space(20.0);
            }
        });
    }

    fn draw_about(&self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            RichText::new("MiReDo")
                .size(30.0)
                .strong()
                .color(self.palette().get("accent")),
        );
        ui.label(self.tr("about.description"));
        ui.add_space(18.0);
        ui.label(format!(
            "{}  {}",
            self.tr("about.version"),
            env!("CARGO_PKG_VERSION")
        ));
        ui.add_space(15.0);
        ui.label(RichText::new(self.tr("about.data_source")).strong());
        ui.hyperlink_to(
            "TkAbeleon/Fihirana-FFPM",
            "https://github.com/TkAbeleon/Fihirana-FFPM",
        );
        ui.add_space(5.0);
        ui.label(
            RichText::new(self.tr("about.source_notice"))
                .size(12.0)
                .color(self.palette().get("warning")),
        );
        ui.add_space(15.0);
        ui.label(RichText::new(self.tr("about.docs")).strong());
        ui.label("docs/miredo/");
    }

    fn draw_playlist_dialog(&mut self, context: &EguiContext) {
        let Some(kind) = self.playlist_dialog else {
            return;
        };
        let title = match kind {
            PlaylistDialog::Create => self.tr("playlists.create_title"),
            PlaylistDialog::Rename => self.tr("playlists.rename_title"),
            PlaylistDialog::Delete => self.tr("playlists.delete_title"),
        };
        let name_label = self.tr("playlists.name");
        let confirm_label = match kind {
            PlaylistDialog::Delete => self.tr("common.delete"),
            PlaylistDialog::Rename => self.tr("common.save"),
            PlaylistDialog::Create => self.tr("common.create"),
        };
        let cancel_label = self.tr("common.cancel");
        let delete_body = self.tr("playlists.delete_confirm");
        let mut action = None;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(context, |ui| {
                if kind == PlaylistDialog::Delete {
                    ui.label(delete_body);
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
                    if ui.button(confirm_label).clicked() {
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
                };                match result {
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

impl eframe::App for MiReDoApp {
    fn update(&mut self, context: &EguiContext, _frame: &mut eframe::Frame) {
        self.poll_pdf(context);
        self.handle_shortcuts(context);
        self.apply_theme(context);

        let immersive_pdf = self.page == Page::Reader && self.viewer_mode == ViewerMode::Pdf;
        if !immersive_pdf {
            self.draw_sidebar(context);
        }

        let frame = if immersive_pdf {
            egui::Frame::new()
                .fill(self.palette().get("viewer_background"))
                .inner_margin(egui::Margin::ZERO)
        } else {
            egui::Frame::new()
                .fill(self.palette().get("background"))
                .inner_margin(egui::Margin::same(22))
        };

        egui::CentralPanel::default()
            .frame(frame)
            .show(context, |ui| {
                if immersive_pdf {
                    self.draw_pdf_reader_immersive(context, ui);
                } else {
                    self.draw_topbar(ui);
                    self.draw_page(context, ui);
                }
            });

        self.draw_playlist_dialog(context);
        if !self.pdf_pending.is_empty() {
            context.request_repaint_after(Duration::from_millis(80));
        }
    }
}