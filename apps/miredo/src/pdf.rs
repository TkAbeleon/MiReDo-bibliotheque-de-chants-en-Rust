use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc::{Receiver, Sender, SyncSender, channel, sync_channel}, atomic::{AtomicU64, Ordering}};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::thread;

use anyhow::{bail, Context, Result};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{render, PixmapSettings, RenderCache, RenderSettings};

const CACHE_VERSION: &str = "hayro-v1";
const DEFAULT_RENDER_DPI: f32 = 96.0;
const MAX_RENDER_PIXELS: i32 = 4096;
const MAX_CACHE_BYTES: u64 = 512 * 1024 * 1024;
const CACHE_WRITE_QUEUE: usize = 2;

static CACHE_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct RenderedPage {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

#[derive(Debug)]
pub struct PdfRenderResponse {
    pub song_id: String,
    pub key: String,
    pub page_count: Option<usize>,
    pub result: Result<RenderedPage, String>,
}

struct PdfRenderRequest {
    sender: Sender<PdfRenderResponse>,
    song_id: String,
    page: usize,
    zoom: f32,
    pdf_path: PathBuf,
    cache_dir: PathBuf,
    key: String,
}

struct CacheWriteRequest {
    path: PathBuf,
    size: [usize; 2],
    rgba: Vec<u8>,
}

#[derive(Clone)]
pub struct PdfRenderer {
    sender: Sender<PdfRenderRequest>,
}

impl PdfRenderer {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let (cache_sender, cache_receiver) = sync_channel(CACHE_WRITE_QUEUE);
        thread::spawn(move || cache_writer_loop(cache_receiver));
        thread::spawn(move || worker_loop(receiver, cache_sender));
        Self { sender }
    }

    pub fn request_page(
        &self,
        sender: Sender<PdfRenderResponse>,
        song_id: String,
        page: usize,
        zoom: f32,
        pdf_path: PathBuf,
        cache_dir: PathBuf,
    ) {
        let key = page_key(&song_id, page, zoom);
        let request = PdfRenderRequest {
            sender,
            song_id,
            page,
            zoom,
            pdf_path,
            cache_dir,
            key,
        };
        let _ = self.sender.send(request);
    }
}

impl Default for PdfRenderer {
    fn default() -> Self {
        Self::new()
    }
}

struct PdfWorker {
    document_path: Option<PathBuf>,
    document: Option<Pdf>,
    cache_sender: SyncSender<CacheWriteRequest>,
}

impl PdfWorker {
    fn new(cache_sender: SyncSender<CacheWriteRequest>) -> Self {
        Self {
            document_path: None,
            document: None,
            cache_sender,
        }
    }

    fn open_document(&mut self, path: &Path) -> Result<()> {
        if self.document_path.as_deref() == Some(path) && self.document.is_some() {
            return Ok(());
        }

        let bytes = fs::read(path)
            .with_context(|| format!("Impossible de lire le PDF {}", path.display()))?;
        self.document = Some(
            Pdf::new(bytes)
                .map_err(|error| anyhow::anyhow!("Impossible d’ouvrir le PDF {}: {error:?}", path.display()))?,
        );
        self.document_path = Some(path.to_path_buf());
        Ok(())
    }

    fn render_page(
        &mut self,
        page_number: usize,
        zoom: f32,
        cache_dir: &Path,
    ) -> Result<(usize, RenderedPage)> {
        if page_number == 0 {
            bail!("Le numéro de page doit commencer à 1");
        }

        let document_path = self
            .document_path
            .as_deref()
            .context("Aucun document PDF n’est chargé")?;
        let document = self.document.as_ref().context("Aucun document PDF n’est chargé")?;
        let page_count = document.pages().len();

        if page_number > page_count {
            bail!(
                "La page {page_number} n’existe pas dans ce document ({page_count} page(s))"
            );
        }

        fs::create_dir_all(cache_dir)
            .with_context(|| format!("Impossible de créer le cache {}", cache_dir.display()))?;

        let cache_path = cache_file(document_path, page_number, zoom, cache_dir);
        if cache_path.is_file() {
            if let Ok(rendered) = read_cached_page(&cache_path) {
                return Ok((page_count, rendered));
            }
            let _ = fs::remove_file(&cache_path);
        }

        let page = document
            .pages()
            .get(page_number - 1)
            .with_context(|| format!("Impossible de charger la page {page_number}"))?;
        let (page_width, page_height) = page.render_dimensions();
        let page_width = page_width.max(1.0);
        let page_height = page_height.max(1.0);

        let requested_scale = (DEFAULT_RENDER_DPI / 72.0) * zoom.max(0.1);
        let pixel_limit = MAX_RENDER_PIXELS as f32;
        let cap = (pixel_limit / (page_width * requested_scale))
            .min(pixel_limit / (page_height * requested_scale))
            .min(1.0);
        let effective_scale = requested_scale * cap;
        let target_width = (page_width * effective_scale).round().max(1.0) as u32;
        let target_height = (page_height * effective_scale).round().max(1.0) as u32;

        let x_scale = target_width as f32 / page_width.max(1.0);
        let y_scale = target_height as f32 / page_height.max(1.0);
        let cache = RenderCache::new();
        let pixmap = render(
            page,
            &cache,
            &InterpreterSettings::default(),
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale,
                y_scale,
                bg_color: WHITE,
            },
        );

        let rendered = RenderedPage {
            size: [pixmap.width() as usize, pixmap.height() as usize],
            rgba: pixmap.data_as_u8_slice().to_vec(),
        };

        let _ = self.cache_sender.try_send(CacheWriteRequest {
            path: cache_path,
            size: rendered.size,
            rgba: rendered.rgba.clone(),
        });

        Ok((page_count, rendered))
    }
}

fn cache_writer_loop(receiver: Receiver<CacheWriteRequest>) {
    while let Ok(request) = receiver.recv() {
        if save_cached_page(&request.path, request.size, &request.rgba).is_ok()
            && let Some(cache_dir) = request.path.parent()
        {
            let _ = prune_cache(cache_dir, MAX_CACHE_BYTES);
        }
    }
}

fn worker_loop(receiver: Receiver<PdfRenderRequest>, cache_sender: SyncSender<CacheWriteRequest>) {
    let mut worker = PdfWorker::new(cache_sender);

    while let Ok(request) = receiver.recv() {
        let result = worker
            .open_document(&request.pdf_path)
            .and_then(|_| worker.render_page(request.page, request.zoom, &request.cache_dir));
        let page_count = result.as_ref().ok().map(|(count, _)| *count);
        let rendered = result.map(|(_, page)| page).map_err(|error| format!("{error:#}"));

        let _ = request.sender.send(PdfRenderResponse {
            song_id: request.song_id,
            key: request.key,
            page_count,
            result: rendered,
        });
    }
}

pub fn page_key(song_id: &str, page: usize, zoom: f32) -> String {
    format!("{song_id}:{page}:{:.0}", zoom * 100.0)
}

fn cache_file(path: &Path, page: usize, zoom: f32, cache_dir: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("partition");
    let mut identity = DefaultHasher::new();
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .hash(&mut identity);
    if let Ok(metadata) = fs::metadata(path) {
        metadata.len().hash(&mut identity);
        metadata.modified().ok().hash(&mut identity);
    }
    cache_dir.join(format!(
        "{CACHE_VERSION}-{stem}-{:016x}-page-{page}-zoom-{:.0}.png",
        identity.finish(),
        zoom * 100.0
    ))
}

fn read_cached_page(path: &Path) -> Result<RenderedPage> {
    let bytes = fs::read(path)
        .with_context(|| format!("Impossible de lire le cache {}", path.display()))?;
    let image = image::load_from_memory(&bytes).context("Cache PDFium invalide")?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(RenderedPage {
        size: [width as usize, height as usize],
        rgba: rgba.into_raw(),
    })
}

fn save_cached_page(path: &Path, size: [usize; 2], rgba: &[u8]) -> Result<()> {
    let temp_path = path.with_extension(format!(
        "tmp-{}-{}.png",
        std::process::id(),
        CACHE_TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(error) = image::save_buffer_with_format(
        &temp_path,
        rgba,
        size[0] as u32,
        size[1] as u32,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    ) {
        let _ = fs::remove_file(&temp_path);
        return Err(error).with_context(|| format!("Impossible d’écrire le cache {}", path.display()));
    }
    if let Err(error) = fs::rename(&temp_path, path) {
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("Impossible de remplacer le cache {}", path.display()))?;
            fs::rename(&temp_path, path)
                .with_context(|| format!("Impossible de publier le cache {}", path.display()))?;
        } else {
            let _ = fs::remove_file(&temp_path);
            return Err(error).with_context(|| format!("Impossible de publier le cache {}", path.display()));
        }
    }
    Ok(())
}

fn prune_cache(cache_dir: &Path, max_bytes: u64) -> Result<()> {
    let mut entries = Vec::new();
    let mut total_bytes = 0_u64;
    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with(CACHE_VERSION) || path.extension().is_none_or(|ext| ext != "png") {
            continue;
        }
        if name.contains(".tmp-") {
            let _ = fs::remove_file(path);
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        total_bytes = total_bytes.saturating_add(metadata.len());
        entries.push((
            metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
            metadata.len(),
            path,
        ));
    }
    entries.sort_by_key(|(modified, _, _)| *modified);
    for (_, size, path) in entries {
        if total_bytes <= max_bytes {
            break;
        }
        if fs::remove_file(path).is_ok() {
            total_bytes = total_bytes.saturating_sub(size);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_page_key_changes_with_page_or_zoom() {
        assert_ne!(page_key("ffpm:001", 1, 1.0), page_key("ffpm:001", 2, 1.0));
        assert_ne!(page_key("ffpm:001", 1, 1.0), page_key("ffpm:001", 1, 1.2));
    }

    #[test]
    fn hayro_can_open_and_render_a_real_project_pdf() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("A1.pdf");
        if !path.is_file() {
            return;
        }

        let bytes = fs::read(&path).unwrap();
        let pdf = Pdf::new(bytes).unwrap();
        assert!(!pdf.pages().is_empty());

        let page = &pdf.pages()[0];
        let cache = RenderCache::new();
        let rendered = render(
            page,
            &cache,
            &InterpreterSettings::default(),
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale: 1.0,
                y_scale: 1.0,
                bg_color: WHITE,
            },
        );

        assert!(rendered.width() > 0 && rendered.height() > 0);
        assert_eq!(
            rendered.data().len(),
            usize::from(rendered.width()) * usize::from(rendered.height())
        );
    }

    #[test]
    fn cache_file_distinguishes_same_named_documents_in_different_directories() {
        let cache_dir = Path::new("cache");
        let first = cache_file(Path::new("/collection-a/score.pdf"), 1, 1.0, cache_dir);
        let second = cache_file(Path::new("/collection-b/score.pdf"), 1, 1.0, cache_dir);

        assert_ne!(first, second);
    }

    #[test]
    fn render_scale_preserves_requested_zoom() {
        let at_100 = (DEFAULT_RENDER_DPI / 72.0) * 1.0;
        let at_200 = (DEFAULT_RENDER_DPI / 72.0) * 2.0;
        assert!(at_200 > at_100);
        assert_eq!(at_200 / at_100, 2.0);
    }

    #[test]
    fn render_size_is_capped_without_changing_aspect_ratio() {
        let width = 595.0_f32;
        let height = 842.0_f32;
        let requested = 4.0_f32;
        let limit = MAX_RENDER_PIXELS as f32;
        let cap = (limit / (width * requested))
            .min(limit / (height * requested))
            .min(1.0);
        let scale = requested * cap;
        assert!(width * scale <= limit + 1.0);
        assert!(height * scale <= limit + 1.0);
        assert!(((width * scale) / (height * scale)) - (width / height) < 0.001);
    }

    #[test]
    fn cache_round_trip_and_pruning_keep_the_disk_budget() {
        let cache_dir = std::env::temp_dir().join(format!(
            "miredo-cache-test-{}-{}",
            std::process::id(),
            CACHE_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&cache_dir).unwrap();
        let cache_path = cache_dir.join(format!("{CACHE_VERSION}-page.png"));
        let rgba = vec![127; 8 * 8 * 4];

        save_cached_page(&cache_path, [8, 8], &rgba).unwrap();
        let cached = read_cached_page(&cache_path).unwrap();
        assert_eq!(cached.size, [8, 8]);
        assert_eq!(cached.rgba, rgba);

        prune_cache(&cache_dir, 1).unwrap();
        assert!(!cache_path.exists());
        let _ = fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn known_source_partition_loads_and_renders_one_page() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("A1.pdf");
        if !path.is_file() {
            return;
        }

        let cache_dir = std::env::temp_dir()
            .join(format!("miredo-pdf-test-{}", std::process::id()));
        let (cache_sender, _cache_receiver) = sync_channel(0);
        let mut worker = PdfWorker::new(cache_sender);
        worker.open_document(&path).unwrap();
        let (page_count, page) = worker.render_page(1, 1.0, &cache_dir).unwrap();
        assert_eq!(page_count, 1);
        assert!(page.size[0] > 0 && page.size[1] > 0);
        assert_eq!(page.rgba.len(), page.size[0] * page.size[1] * 4);
        let _ = fs::remove_dir_all(cache_dir);
    }
}
