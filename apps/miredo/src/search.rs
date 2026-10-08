use std::collections::HashSet;

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::domain::{Collection, Song};

#[derive(Clone, Debug, Default)]
pub struct SearchFilters {
    pub collection: Option<Collection>,
    pub favorites_only: bool,
    pub pdf_only: bool,
    pub song_ids: Option<HashSet<String>>,
}

pub fn search_songs<'a>(
    songs: &'a [Song],
    query: &str,
    filters: &SearchFilters,
    favorites: &HashSet<String>,
) -> Vec<&'a Song> {
    let query_normalized = normalize_for_search(query);
    let query_number = query.trim().trim_start_matches('0');
    let mut matches: Vec<(u8, &'a Song)> = songs
        .iter()
        .filter(|song| {
            filters
                .collection
                .is_none_or(|collection| song.collection == collection)
                && (!filters.favorites_only || favorites.contains(&song.id))
                && (!filters.pdf_only || song.has_pdf())
                && filters
                    .song_ids
                    .as_ref()
                    .is_none_or(|song_ids| song_ids.contains(&song.id))
        })
        .filter_map(|song| {
            let rank = if query_normalized.is_empty() {
                Some(100)
            } else {
                relevance(song, &query_normalized, query_number)
            };
            rank.map(|rank| (rank, song))
        })
        .collect();

    matches.sort_by(|(left_rank, left_song), (right_rank, right_song)| {
        left_rank
            .cmp(right_rank)
            .then_with(|| left_song.sort_number().cmp(&right_song.sort_number()))
    });
    matches.into_iter().map(|(_, song)| song).collect()
}

fn relevance(song: &Song, query: &str, query_number: &str) -> Option<u8> {
    let number = song.number.trim().trim_start_matches('0');
    if !query_number.is_empty() && number == query_number {
        return Some(0);
    }

    let title = normalize_for_search(&song.title);
    if !title.is_empty() && title == query {
        return Some(1);
    }
    if !title.is_empty() && title.starts_with(query) {
        return Some(2);
    }
    if song
        .authors
        .iter()
        .any(|author| normalize_for_search(author) == query)
    {
        return Some(3);
    }
    if !query_number.is_empty() && number.starts_with(query_number) {
        return Some(4);
    }
    if !title.is_empty() && title.contains(query) {
        return Some(5);
    }
    if song
        .authors
        .iter()
        .any(|author| normalize_for_search(author).contains(query))
    {
        return Some(6);
    }
    if song
        .verses
        .iter()
        .any(|verse| normalize_for_search(&verse.text).contains(query))
    {
        return Some(7);
    }
    None
}

pub fn normalize_for_search(value: &str) -> String {
    let decomposed = value
        .nfkd()
        .filter(|character| !is_combining_mark(*character));
    let mut normalized = String::new();
    let mut previous_space = true;
    for character in decomposed.flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            normalized.push(character);
            previous_space = false;
        } else if !previous_space {
            normalized.push(' ');
            previous_space = true;
        }
    }
    normalized.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Verse;

    fn song(id: &str, number: &str, title: &str, lyric: &str) -> Song {
        Song {
            id: id.into(),
            number: number.into(),
            collection: Collection::Ffpm,
            title: title.into(),
            authors: Vec::new(),
            verses: vec![Verse {
                order: 1,
                text: lyric.into(),
                is_refrain: false,
            }],
            pdf_path: None,
        }
    }

    #[test]
    fn search_ignores_accents_and_punctuation() {
        assert_eq!(
            normalize_for_search("Mitsangàna, ry mino!"),
            "mitsangana ry mino"
        );
    }

    #[test]
    fn exact_number_and_title_precede_lyrics() {
        let songs = vec![
            song("ff:002", "2", "Hevitra", "Hira Faneva"),
            song("ff:001", "1", "Mitsangàna", "Hira Faneva"),
        ];
        let filters = SearchFilters::default();
        let favorites = HashSet::new();
        let by_number = search_songs(&songs, "1", &filters, &favorites);
        assert_eq!(by_number[0].id, "ff:001");
        let by_lyrics = search_songs(&songs, "faneva", &filters, &favorites);
        assert_eq!(by_lyrics.len(), 2);
    }
}
