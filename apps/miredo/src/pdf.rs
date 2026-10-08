use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc::{Receiver, Sender, channel}, Arc, OnceLock};
use std::thread;

use anyhow::{anyhow, bail, Context, Result};
use pdfium_render::prelude::*;

const CACHE_VERSION: &str = "pdfium-v2";
const DEFAULT_RENDER_DPI: f32 = 96.0;
const MAX_RENDER_PIXELS: i32 = 4096;

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

#[derive(Clone)]
pub struct PdfRenderer {
    sender: Sender<PdfRenderRequest>,
}

impl PdfRenderer {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        thread::spawn(move || worker_loop(receiver));
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

/// PDFium is loaded once. The worker keeps the currently opened document alive
/// so navigating between pages does not reopen and parse the PDF every time.
static PDFIUM: OnceLock<Result<Arc<Pdfium>, String>> = OnceLock::new();

fn pdfium_instance() -> Result<Arc<Pdfium>> {
    PDFIUM
        .get_or_init(|| {
            let mut errors = Vec::new();

            if let Some(path) = std::env::var_os("MIREDO_PDFIUM_PATH") {
                let configured = PathBuf::from(path);
                let library_path = if configured.is_dir() {
                    Pdfium::pdfium_platform_library_name_at_path(&configured)
                } else {
                    configured
                };

                match Pdfium::bind_to_library(&library_path) {
                    Ok(bindings) => return Ok(Arc::new(Pdfium::new(bindings))),
                    Err(error) => errors.push(format!(
                        "MIREDO_PDFIUM_PATH ({}): {error}",
                        library_path.display()
                    )),
                }
            }

            if let Ok(executable) = std::env::current_exe() {
                if let Some(parent) = executable.parent() {
                    let library_path = Pdfium::pdfium_platform_library_name_at_path(parent);
                    match Pdfium::bind_to_library(&library_path) {
                        Ok(bindings) => return Ok(Arc::new(Pdfium::new(bindings))),
                        Err(error) => errors.push(format!(
                            "répertoire de l’exécutable ({}): {error}",
                            library_path.display()
                        )),
                    }
                }
            }

            match Pdfium::bind_to_system_library() {
                Ok(bindings) => Ok(Arc::new(Pdfium::new(bindings))),
                Err(error) => {
                    errors.push(format!("bibliothèque système: {error}"));
                    Err(errors.join("\n"))
                }
            }
        })
        .clone()
        .map_err(|error| anyhow!("Impossible de charger PDFium : {error}"))
}

struct PdfWorker<'a> {
    pdfium: &'a Pdfium,
    document_path: Option<PathBuf>,
    document: Option<PdfDocument<'a>>,
    bitmap: Option<PdfBitmap<'a>>,
}

impl<'a> PdfWorker<'a> {
    fn new(pdfium: &'a Pdfium) -> Self {
        Self {
            pdfium,
            document_path: None,
            document: None,
            bitmap: None,
        }
    }

    fn open_document(&mut self, path: &Path) -> Result<()> {
        if self.document_path.as_deref() == Some(path) && self.document.is_some() {
            return Ok(());
        }

        // Release the old page handles and bitmap before switching documents.
        self.document = None;
        self.bitmap = None;
        self.document = Some(
            self.pdfium
                .load_pdf_from_file(path, None)
                .with_context(|| format!("Impossible d’ouvrir le PDF {}", path.display()))?,
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
        let page_count = self
            .document
            .as_ref()
            .context("Aucun document PDF n’est chargé")?
            .pages()
            .len() as usize;

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

        let page_width;
        let page_height;
        {
            let document = self.document.as_ref().expect("document checked above");
            let page_index = (page_number - 1) as PdfPageIndex;
            let page = document
                .pages()
                .get(page_index)
                .with_context(|| format!("Impossible de charger la page {page_number}"))?;
            page_width = page.width().value.max(1.0);
            page_height = page.height().value.max(1.0);
        }

        let requested_scale = (DEFAULT_RENDER_DPI / 72.0) * zoom.max(0.1);
        let pixel_limit = MAX_RENDER_PIXELS as f32;
        let cap = (pixel_limit / (page_width * requested_scale))
            .min(pixel_limit / (page_height * requested_scale))
            .min(1.0);
        let effective_scale = requested_scale * cap;
        let target_width = (page_width * effective_scale).round().max(1.0) as Pixels;
        let target_height = (page_height * effective_scale).round().max(1.0) as Pixels;

        let mut bitmap = self.bitmap.take();
        let bitmap_matches = bitmap
            .as_ref()
            .is_some_and(|value| value.width() == target_width && value.height() == target_height);
        if !bitmap_matches {
            bitmap = Some(PdfBitmap::empty(
                target_width,
                target_height,
                PdfBitmapFormat::BGRA,
            )?);
        }
        let mut bitmap = bitmap.expect("bitmap must exist after allocation");

        let render_config = PdfRenderConfig::new()
            .set_target_width(target_width)
            .set_maximum_height(target_height)
            .set_reverse_byte_order(true);

        {
            let document = self.document.as_ref().expect("document checked above");
            let page_index = (page_number - 1) as PdfPageIndex;
            let page = document
                .pages()
                .get(page_index)
                .with_context(|| format!("Impossible de charger la page {page_number}"))?;
            page.render_into_bitmap_with_config(&mut bitmap, &render_config)
                .context("Le rendu PDFium a échoué")?;
        }

        let rgba = bitmap.as_rgba_bytes();
        let rendered = RenderedPage {
            size: [target_width as usize, target_height as usize],
            rgba,
        };
        self.bitmap = Some(bitmap);

        // The first frame must not wait for PNG compression / disk I/O.
        // The small copy is deliberate: it lets the UI receive the rendered page immediately.
        let cache_path_for_write = cache_path.clone();
        let cache_page = rendered.rgba.clone();
        let cache_size = rendered.size;
        thread::spawn(move || {
            let _ = save_cached_page(&cache_path_for_write, cache_size, &cache_page);
        });

        Ok((page_count, rendered))
    }
}

fn worker_loop(receiver: Receiver<PdfRenderRequest>) {
    let pdfium = match pdfium_instance() {
        Ok(pdfium) => pdfium,
        Err(error) => {
            let message = format!("{error:#}");
            while let Ok(request) = receiver.recv() {
                let _ = request.sender.send(PdfRenderResponse {
                    song_id: request.song_id,
                    key: request.key,
                    page_count: None,
                    result: Err(message.clone()),
                });
            }
            return;
        }
    };

    let mut worker = PdfWorker::new(&pdfium);

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
    cache_dir.join(format!(
        "{CACHE_VERSION}-{stem}-page-{page}-zoom-{:.0}.png",
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
    image::save_buffer_with_format(
        path,
        rgba,
        size[0] as u32,
        size[1] as u32,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .with_context(|| format!("Impossible d’écrire le cache {}", path.display()))?;
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
    fn known_source_partition_reports_one_page() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("FFPM1.pdf");
        if !path.is_file() {
            return;
        }

        match pdfium_instance() {
            Ok(pdfium) => {
                let document = pdfium.load_pdf_from_file(&path, None).unwrap();
                assert_eq!(document.pages().len(), 1);
            }
            Err(error) => eprintln!(
                "PDFium non disponible dans cet environnement de test; test PDF ignoré: {error:#}"
            ),
        }
    }
}
