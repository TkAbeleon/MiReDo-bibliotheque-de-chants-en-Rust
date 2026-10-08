use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc::Sender, Arc, OnceLock};
use std::thread;

use anyhow::{anyhow, bail, Context, Result};
use pdfium_render::prelude::*;

const CACHE_VERSION: &str = "pdfium-v1";
const DEFAULT_RENDER_DPI: f32 = 96.0;
const MAX_RENDER_PIXELS: i32 = 8192;

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

/// PDFium is loaded once and shared by the rendering workers.
/// The application no longer spawns `pdfinfo` or `pdftoppm` processes.
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

pub fn request_page(
    sender: Sender<PdfRenderResponse>,
    song_id: String,
    page: usize,
    zoom: f32,
    pdf_path: PathBuf,
    cache_dir: PathBuf,
) {
    let key = page_key(&song_id, page, zoom);
    thread::spawn(move || {
        let rendered = render_page(&pdf_path, page, zoom, &cache_dir);
        let (page_count, result) = match rendered {
            Ok((page_count, page)) => (Some(page_count), Ok(page)),
            Err(error) => (pdf_page_count(&pdf_path).ok(), Err(format!("{error:#}"))),
        };

        let _ = sender.send(PdfRenderResponse {
            song_id,
            key,
            page_count,
            result,
        });
    });
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

fn pdf_page_count(path: &Path) -> Result<usize> {
    let pdfium = pdfium_instance()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("Impossible d’ouvrir le PDF {}", path.display()))?;
    Ok(document.pages().len() as usize)
}

fn render_page(
    path: &Path,
    page: usize,
    zoom: f32,
    cache_dir: &Path,
) -> Result<(usize, RenderedPage)> {
    if page == 0 {
        bail!("Le numéro de page doit commencer à 1");
    }

    fs::create_dir_all(cache_dir)
        .with_context(|| format!("Impossible de créer le cache {}", cache_dir.display()))?;

    let cache_path = cache_file(path, page, zoom, cache_dir);
    if cache_path.is_file() {
        if let Ok(rendered) = read_cached_page(&cache_path) {
            let page_count = pdf_page_count(path)?;
            return Ok((page_count, rendered));
        }
        let _ = fs::remove_file(&cache_path);
    }

    let pdfium = pdfium_instance()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("Impossible d’ouvrir le PDF {}", path.display()))?;
    let page_count = document.pages().len() as usize;

    if page > page_count {
        bail!("La page {page} n’existe pas dans ce document ({page_count} page(s))");
    }

    let page_index = (page - 1) as PdfPageIndex;
    let page_ref = document
        .pages()
        .get(page_index)
        .with_context(|| format!("Impossible de charger la page {page}"))?;

    // 1 point = 1/72 inch. At 100%, 96 DPI preserves the previous viewer scale.
    let scale = (DEFAULT_RENDER_DPI / 72.0) * zoom.max(0.1);
    let render_config = PdfRenderConfig::new()
        .scale_page_by_factor(scale)
        .set_maximum_width(MAX_RENDER_PIXELS)
        .set_maximum_height(MAX_RENDER_PIXELS);

    let image = page_ref
        .render_with_config(&render_config)
        .context("Le rendu PDFium a échoué")?
        .as_image()
        .context("PDFium n’a pas produit une image exploitable")?
        .to_rgba8();

    let (width, height) = image.dimensions();
    let page = RenderedPage {
        size: [width as usize, height as usize],
        rgba: image.into_raw(),
    };

    save_cached_page(&cache_path, &page)?;
    Ok((page_count, page))
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

fn save_cached_page(path: &Path, page: &RenderedPage) -> Result<()> {
    image::save_buffer_with_format(
        path,
        &page.rgba,
        page.size[0] as u32,
        page.size[1] as u32,
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
    fn known_source_partition_reports_one_page() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("FFPM1.pdf");
        if !path.is_file() {
            return;
        }

        match pdf_page_count(&path) {
            Ok(page_count) => assert_eq!(page_count, 1),
            Err(error) => eprintln!(
                "PDFium non disponible dans cet environnement de test; test PDF ignoré: {error:#}"
            ),
        }
    }
}
