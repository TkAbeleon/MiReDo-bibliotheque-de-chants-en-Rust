use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::domain::{Collection, Song, Verse};
use crate::search::normalize_for_search;

const SOURCES: [&str; 4] = [
    "01_fihirana_ffpm.json",
    "02_fihirana_fanampiny.json",
    "03_antema.json",
    "04_tsanta.json",
];

#[derive(Clone, Debug, Default)]
pub struct DataLoadReport {
    pub loaded: usize,
    pub with_lyrics: usize,
    pub with_authors: usize,
    pub with_pdf: usize,
    pub invalid_records: usize,
    pub duplicate_ids: usize,
}

pub fn load_library(data_dir: &Path) -> Result<(Vec<Song>, DataLoadReport)> {
    let mut songs = Vec::new();
    let mut report = DataLoadReport::default();
    let mut seen_ids = HashSet::new();

    for file_name in SOURCES {
        let collection = Collection::from_source_file(file_name)
            .with_context(|| format!("Source de données non reconnue : {file_name}"))?;
        let path = data_dir.join(file_name);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Impossible de lire la collection {}", path.display()))?;
        let records: Value = serde_json::from_str(&content)
            .with_context(|| format!("JSON invalide : {}", path.display()))?;
        let records = records
            .as_object()
            .with_context(|| format!("La racine de {} doit être un objet", path.display()))?;

        for (source_key, value) in records {
            let Some(song) = parse_song(collection, source_key, value, data_dir) else {
                report.invalid_records += 1;
                continue;
            };
            if !seen_ids.insert(song.id.clone()) {
                report.duplicate_ids += 1;
                report.invalid_records += 1;
                continue;
            }
            if !song.verses.is_empty() {
                report.with_lyrics += 1;
            }
            if !song.authors.is_empty() {
                report.with_authors += 1;
            }
            if song.has_pdf() {
                report.with_pdf += 1;
            }
            songs.push(song);
        }
    }

    songs.sort_by_key(Song::sort_number);
    report.loaded = songs.len();
    if songs.is_empty() {
        bail!(
            "Aucun chant exploitable n'a été chargé depuis {}",
            data_dir.display()
        );
    }
    Ok((songs, report))
}

fn parse_song(
    collection: Collection,
    _source_key: &str,
    value: &Value,
    data_dir: &Path,
) -> Option<Song> {
    let object = value.as_object()?;
    let number = string_field(object.get("laharana")?)?.trim().to_owned();
    if number.is_empty() {
        return None;
    }

    let id_number = number
        .parse::<u32>()
        .map(|number| format!("{number:03}"))
        .unwrap_or_else(|_| number.clone());
    let id = format!("{}:{id_number}", collection.key());
    let title = object
        .get("lohateny")
        .and_then(string_field)
        .unwrap_or_default()
        .trim()
        .to_owned();

    let mut authors = Vec::new();
    let mut seen_authors = HashSet::new();
    if let Some(author_values) = object.get("mpanoratra").and_then(Value::as_array) {
        for value in author_values {
            let Some(name) = string_field(value).map(|s| s.trim().to_owned()) else {
                continue;
            };
            if !name.is_empty() && seen_authors.insert(normalize_for_search(&name)) {
                authors.push(name);
            }
        }
    }

    let mut verses = Vec::new();
    if let Some(verse_values) = object.get("hira").and_then(Value::as_array) {
        for (index, value) in verse_values.iter().enumerate() {
            let Some(verse) = value.as_object() else {
                continue;
            };
            let text = verse
                .get("tononkira")
                .and_then(string_field)
                .unwrap_or_default()
                .trim()
                .to_owned();
            if text.is_empty() {
                continue;
            }
            let order = verse
                .get("andininy")
                .and_then(number_field)
                .unwrap_or(index as u32 + 1);
            let is_refrain = verse
                .get("fiverenany")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            verses.push(Verse {
                order,
                text,
                is_refrain,
            });
        }
    }

    Some(Song {
        id,
        number,
        collection,
        title,
        authors,
        verses,
        pdf_path: resolve_pdf(
            data_dir,
            collection,
            &string_field(object.get("laharana")?)?,
        ),
    })
}

fn string_field(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn number_field(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .or_else(|| value.as_str()?.parse::<u32>().ok())
}

fn resolve_pdf(data_dir: &Path, collection: Collection, number: &str) -> Option<PathBuf> {
    let number = number.trim();
    let numeric = number.parse::<u32>().ok();
    let mut candidates = Vec::new();

    for prefix in collection.pdf_prefixes() {
        candidates.push(format!("{prefix}{number}.pdf"));
        if let Some(value) = numeric {
            candidates.push(format!("{prefix}{value}.pdf"));
        }
    }

    candidates
        .into_iter()
        .map(|name| data_dir.join(name))
        .find(|path| path.is_file())
}

pub fn collection_counts(songs: &[Song]) -> HashMap<Collection, usize> {
    let mut counts = HashMap::new();
    for song in songs {
        *counts.entry(song.collection).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_pdf_names_follow_the_source_collection() {
        let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        assert_eq!(
            resolve_pdf(&data_dir, Collection::Ffpm, "1")
                .map(|path| path.file_name().unwrap().to_owned()),
            Some("FFPM1.pdf".into())
        );
        assert_eq!(
            resolve_pdf(&data_dir, Collection::Antema, "1")
                .map(|path| path.file_name().unwrap().to_owned()),
            Some("A1.pdf".into())
        );
        assert_eq!(
            resolve_pdf(&data_dir, Collection::Fanampiny, "1")
                .map(|path| path.file_name().unwrap().to_owned()),
            Some("FF1.pdf".into())
        );
    }

    #[test]
    fn full_local_catalog_loads_and_uses_stable_ids() {
        let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        let (songs, report) = load_library(&data_dir).unwrap();
        assert!(songs.len() > 800);
        assert_eq!(report.loaded, songs.len());
        assert!(songs.iter().any(|song| song.id == "ff:001"));
        assert!(
            songs
                .iter()
                .any(|song| song.collection == Collection::Tsanta)
        );
    }
}
