use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;

use anyhow::{Context, Result, bail};

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
        let page_count = pdf_page_count(&pdf_path);
        let result =
            render_page(&pdf_path, page, zoom, &cache_dir).map_err(|error| format!("{error:#}"));
        let _ = sender.send(PdfRenderResponse {
            song_id,
            key,
            page_count,
            result,
        });
    });
}

pub fn page_key(song_id: &str, page: usize, zoom: f32) -> String {
    format!("{}:{page}:{:.0}", song_id, zoom * 100.0)
}

fn pdf_page_count(path: &Path) -> Option<usize> {
    let output = Command::new("pdfinfo").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "Pages")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    })
}

fn render_page(path: &Path, page: usize, zoom: f32, cache_dir: &Path) -> Result<RenderedPage> {
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("Impossible de créer le cache {}", cache_dir.display()))?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("partition");
    let output_prefix = cache_dir.join(format!("{stem}-page-{page}-zoom-{:.0}", zoom * 100.0));
    let png_path = output_prefix.with_extension("png");

    if !png_path.is_file() {
        let dpi = (96.0 * zoom).round().clamp(72.0, 300.0) as u32;
        let output = Command::new("pdftoppm")
            .arg("-f")
            .arg(page.to_string())
            .arg("-l")
            .arg(page.to_string())
            .arg("-singlefile")
            .arg("-png")
            .arg("-r")
            .arg(dpi.to_string())
            .arg(path)
            .arg(&output_prefix)
            .output()
            .context("pdftoppm n'est pas disponible pour afficher les partitions")?;
        if !output.status.success() {
            let details = String::from_utf8_lossy(&output.stderr);
            bail!("Le rendu de la page {page} a échoué : {}", details.trim());
        }
    }

    let bytes = fs::read(&png_path)
        .with_context(|| format!("Le rendu PDF n'a pas produit {}", png_path.display()))?;
    let image = image::load_from_memory(&bytes).context("Image de partition invalide")?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(RenderedPage {
        size: [width as usize, height as usize],
        rgba: rgba.into_raw(),
    })
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
    fn known_source_partition_reports_one_page() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("FFPM1.pdf");
        if path.is_file() {
            assert_eq!(pdf_page_count(&path), Some(1));
        }
    }
}
