use std::collections::{HashMap, HashSet, VecDeque};
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
use crate::pdf::{self, PdfRenderResponse, PdfRenderer};
use crate::resources::{Palette, Translator};
use crate::search::{self, SearchFilters};
use crate::storage::UserStorage;

mod icons;
mod library;
mod navigation;
mod pages;
mod reader;
mod state;
mod theme;

use self::icons::{icon_button, paint_app_icon, pdf_toolbar_icon_button};

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

/// UI and screen composition for MiReDo.
/// The app state and the view orchestration remain here, while reusable helpers are split
/// into dedicated modules for easier maintenance and testing.

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
}

impl eframe::App for MiReDoApp {
    fn update(&mut self, context: &EguiContext, _frame: &mut eframe::Frame) {
        self.poll_pdf(context);
        self.handle_shortcuts(context);
        theme::apply_theme(self, context);

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
