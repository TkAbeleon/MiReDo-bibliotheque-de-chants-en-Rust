use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Collection {
    Ffpm,
    Fanampiny,
    Antema,
    Tsanta,
}

impl Collection {
    pub const ALL: [Self; 4] = [Self::Ffpm, Self::Fanampiny, Self::Antema, Self::Tsanta];

    pub fn key(self) -> &'static str {
        match self {
            Self::Ffpm => "ff",
            Self::Fanampiny => "fanampiny",
            Self::Antema => "antema",
            Self::Tsanta => "tsanta",
        }
    }

    pub fn translation_key(self) -> &'static str {
        match self {
            Self::Ffpm => "collection.ffpm",
            Self::Fanampiny => "collection.fanampiny",
            Self::Antema => "collection.antema",
            Self::Tsanta => "collection.tsanta",
        }
    }

    pub fn from_source_file(file_name: &str) -> Option<Self> {
        match file_name {
            "01_fihirana_ffpm.json" => Some(Self::Ffpm),
            "02_fihirana_fanampiny.json" => Some(Self::Fanampiny),
            "03_antema.json" => Some(Self::Antema),
            "04_tsanta.json" => Some(Self::Tsanta),
            _ => None,
        }
    }

    pub fn pdf_prefixes(self) -> &'static [&'static str] {
        match self {
            Self::Ffpm => &["FFPM"],
            Self::Fanampiny => &["FF"],
            Self::Antema => &["A"],
            Self::Tsanta => &["T"],
        }
    }

    pub fn sort_order(self) -> u8 {
        match self {
            Self::Ffpm => 0,
            Self::Fanampiny => 1,
            Self::Antema => 2,
            Self::Tsanta => 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Verse {
    pub order: u32,
    pub text: String,
    pub is_refrain: bool,
}

#[derive(Clone, Debug)]
pub struct Song {
    pub id: String,
    pub number: String,
    pub collection: Collection,
    pub title: String,
    pub authors: Vec<String>,
    pub verses: Vec<Verse>,
    pub pdf_path: Option<PathBuf>,
}

impl Song {
    pub fn display_number(&self) -> String {
        self.number
            .parse::<u32>()
            .map(|number| format!("{number:03}"))
            .unwrap_or_else(|_| self.number.clone())
    }

    pub fn display_title(&self, untitled_label: &str) -> String {
        let title = self.title.trim();
        if title.is_empty() {
            format!("{untitled_label} {}", self.display_number())
        } else {
            title.to_owned()
        }
    }

    pub fn preview(&self) -> &str {
        self.verses
            .iter()
            .flat_map(|verse| verse.text.lines())
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("")
    }

    pub fn has_pdf(&self) -> bool {
        self.pdf_path.is_some()
    }

    pub fn sort_number(&self) -> (u8, u32, String) {
        (
            self.collection.sort_order(),
            self.number.parse().unwrap_or(u32::MAX),
            self.number.clone(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub song_ids: Vec<String>,
}
