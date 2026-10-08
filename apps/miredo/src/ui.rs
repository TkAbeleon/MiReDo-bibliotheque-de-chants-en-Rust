use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use eframe::egui::{
    self, Align, Context as EguiContext, Id, Layout, RichText, ScrollArea, Stroke, TextureHandle,
    TextureOptions, Vec2, ViewportCommand,
};

use crate::data::{self, DataLoadReport};
use crate::domain::{Collection, Playlist, Song};
use crate::pdf::{self, PdfRenderResponse, PdfRenderer};
use crate::resources::{Palette, Translator};
use crate::search::{self, SearchFilters};
use crate::storage::UserStorage;

const APP_DIR: &str = env!("CARGO_MANIFEST_DIR");
const MAX_PDF_TEXTURES: usize = 6;
const PDF_VIEWPORT_PADDING: f32 = 12.0;

fn pdf_safe_available(available: Vec2) -> Vec2 {
    (available - Vec2::splat(PDF_VIEWPORT_PADDING * 2.0)).max(Vec2::splat(1.0))
}

fn pdf_fit_scale(page_size: Vec2, available: Vec2) -> f32 {
    let safe_available = pdf_safe_available(available);
    let width_ratio = safe_available.x / page_size.x.max(1.0);
    let height_ratio = safe_available.y / page_size.y.max(1.0);
    width_ratio.min(height_ratio).clamp(0.1, 6.0)
}

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

fn pdf_visible_pages(layout: PdfLayout, page: usize, total_pages: Option<usize>) -> Vec<usize> {
    let page = page.max(1);
    if layout == PdfLayout::Single {
        return vec![page];
    }

    let first_page = if page > 1 && page.is_multiple_of(2) {
        page - 1
    } else {
        page
    };
    if total_pages == Some(first_page) {
        vec![first_page]
    } else {
        vec![first_page, first_page + 1]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PdfFitMode {
    FitWidth,
    FitHeight,
    Manual,
}

impl PdfFitMode {
    fn from_preference(value: Option<String>) -> Self {
        match value.as_deref() {
            Some("height") => Self::FitHeight,
            Some("manual") => Self::Manual,
            _ => Self::FitWidth,
        }
    }

    fn preference(self) -> &'static str {
        match self {
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
    AddSong,
}

#[derive(Clone, Copy)]
enum AppIcon {
    Home,
    Library,
    Browse,
    Favorite,
    Playlist,
    Add,
    Edit,
    Delete,
    Help,
    About,
    Settings,
    Menu,
    Search,
    Previous,
    Next,
    Close,
    ZoomIn,
    ZoomOut,
    Fullscreen,
}

fn paint_app_icon(ui: &mut egui::Ui, rect: egui::Rect, icon: AppIcon, color: egui::Color32) {
    let lucide = match icon {
        AppIcon::Home => egui_lucide::Lucide::House,
        AppIcon::Library => egui_lucide::Lucide::Library,
        AppIcon::Browse => egui_lucide::Lucide::BookOpenText,
        AppIcon::Favorite => egui_lucide::Lucide::Heart,
        AppIcon::Playlist => egui_lucide::Lucide::ListMusic,
        AppIcon::Add => egui_lucide::Lucide::Plus,
        AppIcon::Edit => egui_lucide::Lucide::Pencil,
        AppIcon::Delete => egui_lucide::Lucide::Trash2,
        AppIcon::Help => egui_lucide::Lucide::MessageCircleQuestionMark,
        AppIcon::About => egui_lucide::Lucide::Info,
        AppIcon::Settings => egui_lucide::Lucide::Settings2,
        AppIcon::Menu => egui_lucide::Lucide::Menu,
        AppIcon::Search => egui_lucide::Lucide::Search,
        AppIcon::Previous => egui_lucide::Lucide::ChevronLeft,
        AppIcon::Next => egui_lucide::Lucide::ChevronRight,
        AppIcon::Close => egui_lucide::Lucide::X,
        AppIcon::ZoomIn => egui_lucide::Lucide::ZoomIn,
        AppIcon::ZoomOut => egui_lucide::Lucide::ZoomOut,
        AppIcon::Fullscreen => egui_lucide::Lucide::Maximize,
    };
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"1.75\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg>",
        color.r(),
        color.g(),
        color.b(),
        lucide.inner(),
    );
    let uri = format!(
        "bytes://miredo-lucide/{}-{:02x}{:02x}{:02x}.svg",
        lucide.name(),
        color.r(),
        color.g(),
        color.b(),
    );
    let source = egui::ImageSource::Bytes {
        uri: uri.into(),
        bytes: egui::load::Bytes::Shared(Arc::from(svg.into_bytes())),
    };
    ui.put(
        rect,
        egui::Image::new(source)
            .fit_to_exact_size(rect.size())
            .show_loading_spinner(false)
            .alt_text(lucide.name()),
    );
}

fn icon_button(ui: &mut egui::Ui, icon: AppIcon, color: egui::Color32, tooltip: String) -> egui::Response {
    icon_button_enabled(ui, icon, color, tooltip, true)
}

fn icon_button_enabled(
    ui: &mut egui::Ui,
    icon: AppIcon,
    color: egui::Color32,
    tooltip: String,
    enabled: bool,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            ui.add_sized(
                Vec2::splat(36.0),
                egui::Button::new("")
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            )
        })
        .inner;
    paint_app_icon(ui, response.rect, icon, color);
    response.on_hover_text(tooltip)
}

fn pdf_toolbar_icon_button(
    ui: &mut egui::Ui,
    icon: AppIcon,
    color: egui::Color32,
    tooltip: String,
    enabled: bool,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            ui.add_sized(
                Vec2::splat(28.0),
                egui::Button::new("")
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            )
        })
        .inner;
    let icon_rect = response.rect.shrink(6.0);
    paint_app_icon(ui, icon_rect, icon, color);
    response.on_hover_text(tooltip)
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
    playlist_add_query: String,
    dialog_error: Option<String>,
    status: Option<(String, Instant)>,
    fullscreen: bool,
    nav_menu_open: bool,
    pdf_page: usize,
    pdf_page_size: Option<Vec2>,
    pdf_renderer: PdfRenderer,
    pdf_response_sender: Sender<PdfRenderResponse>,
    pdf_receiver: Receiver<PdfRenderResponse>,
    pdf_textures: HashMap<String, TextureHandle>,
    pdf_texture_order: VecDeque<String>,
    pdf_errors: HashMap<String, String>,
    pdf_pending: HashSet<String>,
    pdf_page_counts: HashMap<String, usize>,
    pdf_cache_dir: PathBuf,
}

impl MiReDoApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Result<Self> {
        egui_extras::install_image_loaders(&_creation_context.egui_ctx);
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
        let (pdf_receiver_sender, pdf_receiver) = channel();
        let pdf_renderer = PdfRenderer::new();
        let pdf_cache_dir = directories::ProjectDirs::from("org", "MiReDo", "MiReDo")
            .context("Impossible de trouver le dossier de cache")?
            .cache_dir()
            .join("pdf-pages-pdfium-v1");

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
            playlist_add_query: String::new(),
            dialog_error: None,
            status: None,
            fullscreen: false,
            nav_menu_open: false,
            pdf_page: 1,
            pdf_page_size: None,
            pdf_renderer,
            pdf_response_sender: pdf_receiver_sender,
            pdf_receiver,
            pdf_textures: HashMap::new(),
            pdf_texture_order: VecDeque::new(),
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

    fn poll_pdf(&mut self, context: &EguiContext) {
        while let Ok(response) = self.pdf_receiver.try_recv() {
            self.pdf_pending.remove(&response.key);
            if let Some(page_count) = response.page_count {
                self.pdf_page_counts
                    .insert(response.song_id.clone(), page_count.max(1));
            }
            match response.result {
                Ok(page) => {
                    if self.pdf_page_size.is_none() {
                        self.pdf_page_size = Some(Vec2::new(
                            page.size[0] as f32,
                            page.size[1] as f32,
                        ));
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

    fn request_pdf_page(&mut self, song: &Song, page: usize, available: Vec2) {
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

    fn refresh_user_data(&mut self) {
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
        self.pdf_texture_order.clear();
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
        self.pdf_texture_order.clear();
        self.pdf_pending.clear();
        self.pdf_errors.clear();
    }

    fn set_pdf_layout(&mut self, layout: PdfLayout) {
        self.pdf_layout = layout;
        if layout == PdfLayout::Double && self.pdf_page > 1 && self.pdf_page.is_multiple_of(2) {
            self.pdf_page -= 1;
        }
        self.pdf_textures.clear();
        self.pdf_texture_order.clear();
        self.pdf_pending.clear();
        self.pdf_errors.clear();
        let value = if layout == PdfLayout::Double { "double" } else { "single" };
        if let Err(error) = self.storage.set_preference("pdf_layout", value) {
            self.show_error(error);
        }
    }

    fn pdf_fit_zoom(&self, available: Vec2) -> f32 {
        let Some(page_size) = self.pdf_page_size else {
            return self.zoom.max(0.5);
        };
        match self.pdf_fit_mode {
            PdfFitMode::Manual => self.zoom.max(0.5),
            PdfFitMode::FitWidth | PdfFitMode::FitHeight => pdf_fit_scale(page_size, available),
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

    fn reader_subtitle(&self, song: &Song) -> String {
        let author = if song.authors.is_empty() {
            self.tr("common.no_author")
        } else {
            song.authors.join(", ")
        };
        format!("{} · {}", author, self.collection_label(song.collection))
    }

    fn draw_sidebar(&mut self, context: &EguiContext) {
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
                                RichText::new(format!("{}  {language}", self.tr("settings.language")))
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

    fn draw_nav_drawer(&mut self, context: &EguiContext) {
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

    fn draw_topbar(&mut self, context: &EguiContext, ui: &mut egui::Ui) {
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
            paint_app_icon(ui, search_rect, AppIcon::Search, self.palette().get("text_muted"));
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
        let tile_width = ((available_width - gap * (column_count - 1) as f32) / column_count as f32)
            .max(1.0);
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
        ui.add_space(8.0);
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

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let action_width = 44.0
                            + if remove_from_playlist.is_some() { 44.0 } else { 0.0 };
                        let content_width = (ui.available_width()
                            - action_width
                            - ui.spacing().item_spacing.x)
                            .max(1.0);
                        let row = ui.allocate_ui_with_layout(
                            Vec2::new(content_width, 44.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                let row_bg = if selected {
                                    self.palette().get("surface_selected")
                                } else {
                                    self.palette().get("surface")
                                };
                                ui.painter().rect_filled(ui.max_rect().shrink(2.0), 8.0, row_bg);
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
                                                    .color(self.palette().get("accent")),
                                            );
                                        }
                                    });
                                });
                            },
                        );
                        let row_response = ui.interact(
                            row.response.rect,
                            Id::new(("song-row", song_id)),
                            egui::Sense::click(),
                        );
                        if row_response.clicked() {
                            open_id = Some(song_id.clone());
                        }
                        if row_response.hovered() && !selected {
                            ui.painter().rect_filled(
                                row.response.rect.shrink(2.0),
                                8.0,
                                self.palette().get("surface_hover"),
                            );
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
                    ui.add_space(4.0);
                    ui.separator();
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

    fn draw_playlists(&mut self, ui: &mut egui::Ui) {
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
                            let row = ui.allocate_ui_with_layout(
                                Vec2::new(title_width, 38.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    let response = ui.add_sized(
                                        [ui.available_width(), 32.0],
                                        egui::Button::new(
                                            RichText::new(format!("{}  ·  {count} {count_label}", playlist.name))
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
                                    dialog_action = Some((PlaylistDialog::Delete, playlist.id.clone()));
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
                        self.draw_song_list(ui, playlist.song_ids.clone(), Some(playlist.id.clone()));
                    });
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
                let current_start = if is_book && self.pdf_page > 1 && self.pdf_page.is_multiple_of(2) {
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
                                format!("{} {}–{} / {}", self.tr("reader.page"), current_start, (current_start + 1).min(total), total)
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
                let (search_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                paint_app_icon(ui, search_rect, AppIcon::Search, self.palette().get("text_muted"));
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
        for adjacent in [
            first_page.saturating_sub(1),
            first_page + 1,
            first_page + 2,
        ] {
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
                                        Vec2::new((available.x / 2.0).max(1.0), available.y.max(1.0)),
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

    fn draw_pdf_page(
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
        self.settings_card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        self.settings_section(ui, "settings.appearance");
        ui.horizontal_wrapped(|ui| {
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
        ui.horizontal_wrapped(|ui| {
            ui.label(self.tr("settings.language"));
            for (locale, label) in [("fr", "FR"), ("mg", "MG"), ("en", "EN")] {
                if ui.selectable_label(self.locale == locale, label).clicked() {
                    self.set_locale(locale);
                }
            }
        });
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        self.settings_section(ui, "settings.reader");
        ui.horizontal_wrapped(|ui| {
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
        ui.horizontal_wrapped(|ui| {
            ui.label(self.tr("settings.initial_zoom"));
            if icon_button(
                ui,
                AppIcon::ZoomOut,
                self.palette().get("text_primary"),
                self.tr("reader.zoom_out"),
            )
            .clicked()
            {
                self.change_zoom(-0.1);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if icon_button(
                ui,
                AppIcon::ZoomIn,
                self.palette().get("text_primary"),
                self.tr("reader.zoom_in"),
            )
            .clicked()
            {
                self.change_zoom(0.1);
            }
        });
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        self.settings_section(ui, "settings.pdf_fit");
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(
                    self.pdf_fit_mode == PdfFitMode::FitWidth,
                    self.tr("reader.fit_width"),
                )
                .clicked()
            {
                self.set_pdf_fit_mode(PdfFitMode::FitWidth);
            }
            if ui
                .selectable_label(
                    self.pdf_fit_mode == PdfFitMode::FitHeight,
                    self.tr("reader.fit_height"),
                )
                .clicked()
            {
                self.set_pdf_fit_mode(PdfFitMode::FitHeight);
            }
            if ui
                .selectable_label(
                    self.pdf_fit_mode == PdfFitMode::Manual,
                    self.tr("reader.manual_zoom"),
                )
                .clicked()
            {
                self.set_pdf_fit_mode(PdfFitMode::Manual);
            }
        });
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        self.settings_section(ui, "settings.data");
        let data_path = PathBuf::from(APP_DIR).join("data").display().to_string();
        ui.add(
            egui::Label::new(format!("{}: {data_path}", self.tr("settings.data_path")))
                .truncate(),
        )
        .on_hover_text(&data_path);
        if let Some(path) = UserStorage::database_path() {
            let path = path.display().to_string();
            ui.add(
                egui::Label::new(format!("{}: {path}", self.tr("settings.database_path")))
                    .truncate(),
            )
            .on_hover_text(&path);
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
                    self.pdf_texture_order.clear();
                    self.pdf_page_counts.clear();
                    self.set_status(self.tr("settings.reloaded"));
                }
                Err(error) => {
                    self.set_status(format!("{}: {error}", self.tr("settings.reload_failed")));
                }
            }
        }
        });

        ui.add_space(12.0);
        self.settings_card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        self.settings_section(ui, "settings.keyboard");
        for (key, shortcut) in [
            ("settings.shortcut_search", "Ctrl / ⌘  K"),
            ("settings.shortcut_next", "N / PageDown"),
            ("settings.shortcut_previous", "P / PageUp"),
            ("settings.shortcut_favorite", "F"),
            ("settings.shortcut_viewer", "V"),
            ("settings.shortcut_escape", "Esc"),
        ] {
            ui.horizontal_wrapped(|ui| {
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

    fn settings_card_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.palette().get("surface"))
            .stroke(Stroke::new(1.0_f32, self.palette().get("border_subtle")))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(16))
    }

    fn settings_section(&self, ui: &mut egui::Ui, key: &str) {
        ui.label(
            RichText::new(self.tr(key))
                .size(16.0)
                .strong()
                .color(self.palette().get("accent")),
        );
            ui.add_space(10.0);
    }

    fn draw_help(&self, ui: &mut egui::Ui) {
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
                                .map(|id| !self.playlists.iter().any(|playlist| &playlist.id == id && playlist.song_ids.contains(&song.id)))
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
        let narrow_window = context.screen_rect().width() < 760.0;
        if !immersive_pdf && !narrow_window {
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
                    self.draw_topbar(context, ui);
                    ScrollArea::vertical()
                        .id_salt("main-page-content")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Frame::new()
                                .fill(self.palette().get("background"))
                                .inner_margin(egui::Margin::same(16))
                                .show(ui, |ui| self.draw_page(context, ui));
                        });
                }
            });

        self.draw_nav_drawer(context);
        self.draw_playlist_dialog(context);
        if !self.pdf_pending.is_empty() {
            context.request_repaint_after(Duration::from_millis(80));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_layout_centers_a_single_last_page() {
        assert_eq!(pdf_visible_pages(PdfLayout::Double, 5, Some(5)), vec![5]);
        assert_eq!(pdf_visible_pages(PdfLayout::Double, 3, Some(5)), vec![3, 4]);
        assert_eq!(pdf_visible_pages(PdfLayout::Double, 4, Some(5)), vec![3, 4]);
    }

    #[test]
    fn fit_page_scale_keeps_page_inside_safe_area() {
        let page_size = Vec2::new(1000.0, 1500.0);
        let available = Vec2::new(900.0, 700.0);

        let scale = pdf_fit_scale(page_size, available);

        assert!(page_size.x * scale <= available.x - PDF_VIEWPORT_PADDING * 2.0 + 0.0001);
        assert!(page_size.y * scale <= available.y - PDF_VIEWPORT_PADDING * 2.0 + 0.0001);
    }

    #[test]
    fn fit_page_scale_uses_smallest_ratio() {
        let page_size = Vec2::new(2100.0, 2970.0);
        let available = Vec2::new(1200.0, 900.0);

        let scale = pdf_fit_scale(page_size, available);

        assert!(scale < 1.0);
        assert!(page_size.x * scale <= available.x - PDF_VIEWPORT_PADDING * 2.0 + 0.0001);
        assert!(page_size.y * scale <= available.y - PDF_VIEWPORT_PADDING * 2.0 + 0.0001);
    }

    #[test]
    fn safe_viewport_accounts_for_toolbar_padding() {
        let available = Vec2::new(1400.0, 760.0);
        let safe = pdf_safe_available(available);

        assert!(safe.x <= available.x - PDF_VIEWPORT_PADDING * 2.0 + 0.0001);
        assert!(safe.y <= available.y - PDF_VIEWPORT_PADDING * 2.0 + 0.0001);
        assert!(safe.x > 0.0);
        assert!(safe.y > 0.0);
    }
}