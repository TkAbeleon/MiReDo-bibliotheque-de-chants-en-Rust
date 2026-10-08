use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::Playlist;

pub struct UserStorage {
    connection: Connection,
}

impl UserStorage {
    pub fn open() -> Result<Self> {
        let project_dirs = ProjectDirs::from("org", "MiReDo", "MiReDo")
            .context("Impossible de déterminer le dossier utilisateur MiReDo")?;
        let data_dir = project_dirs.data_local_dir();
        fs::create_dir_all(data_dir)
            .with_context(|| format!("Impossible de créer {}", data_dir.display()))?;
        let db_path = data_dir.join("miredo.sqlite3");
        let connection = Connection::open(&db_path)
            .with_context(|| format!("Impossible d'ouvrir {}", db_path.display()))?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS favorites (
                 song_id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS playlists (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS playlist_songs (
                 playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                 song_id TEXT NOT NULL,
                 position INTEGER NOT NULL,
                 PRIMARY KEY (playlist_id, song_id)
             );
             CREATE TABLE IF NOT EXISTS preferences (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS reading_positions (
                 song_id TEXT NOT NULL,
                 viewer TEXT NOT NULL,
                 position REAL NOT NULL,
                 PRIMARY KEY (song_id, viewer)
             );",
        )?;
        Ok(Self { connection })
    }

    pub fn favorite_ids(&self) -> Result<HashSet<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT song_id FROM favorites ORDER BY created_at DESC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<HashSet<_>>>()
            .context("Impossible de charger les favoris")
    }

    pub fn toggle_favorite(&self, song_id: &str) -> Result<bool> {
        if self.is_favorite(song_id)? {
            self.connection
                .execute("DELETE FROM favorites WHERE song_id = ?1", [song_id])?;
            Ok(false)
        } else {
            self.connection.execute(
                "INSERT INTO favorites(song_id, created_at) VALUES (?1, ?2)",
                params![song_id, now_string()],
            )?;
            Ok(true)
        }
    }

    pub fn is_favorite(&self, song_id: &str) -> Result<bool> {
        let exists = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE song_id = ?1)",
            [song_id],
            |row| row.get::<_, bool>(0),
        )?;
        Ok(exists)
    }

    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, name FROM playlists ORDER BY name COLLATE NOCASE")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut playlists = Vec::new();
        for row in rows {
            let (id, name) = row?;
            playlists.push(Playlist {
                song_ids: self.playlist_song_ids(&id)?,
                id,
                name,
            });
        }
        Ok(playlists)
    }

    pub fn create_playlist(&self, name: &str) -> Result<String> {
        let id = format!("playlist-{}", now_nanos());
        let now = now_string();
        self.connection.execute(
            "INSERT INTO playlists(id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![id, name.trim(), now],
        )?;
        Ok(id)
    }

    pub fn rename_playlist(&self, id: &str, name: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE playlists SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name.trim(), now_string()],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM playlists WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn add_song_to_playlist(&self, playlist_id: &str, song_id: &str) -> Result<()> {
        let position = self.connection.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_songs WHERE playlist_id = ?1",
            [playlist_id],
            |row| row.get::<_, i64>(0),
        )?;
        self.connection.execute(
            "INSERT OR IGNORE INTO playlist_songs(playlist_id, song_id, position)
             VALUES (?1, ?2, ?3)",
            params![playlist_id, song_id, position],
        )?;
        self.connection.execute(
            "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
            params![playlist_id, now_string()],
        )?;
        Ok(())
    }

    pub fn remove_song_from_playlist(&self, playlist_id: &str, song_id: &str) -> Result<()> {
        self.connection.execute(
            "DELETE FROM playlist_songs WHERE playlist_id = ?1 AND song_id = ?2",
            params![playlist_id, song_id],
        )?;
        Ok(())
    }

    pub fn playlist_song_ids(&self, playlist_id: &str) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT song_id FROM playlist_songs WHERE playlist_id = ?1 ORDER BY position, song_id",
        )?;
        let rows = statement.query_map([playlist_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("Impossible de charger les chants de la liste")
    }

    pub fn preference(&self, key: &str) -> Result<Option<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT value FROM preferences WHERE key = ?1")?;
        let value = statement
            .query_row([key], |row| row.get::<_, String>(0))
            .optional()?;
        Ok(value)
    }

    pub fn set_preference(&self, key: &str, value: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO preferences(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn save_reading_position(&self, song_id: &str, viewer: &str, position: f64) -> Result<()> {
        self.connection.execute(
            "INSERT INTO reading_positions(song_id, viewer, position) VALUES (?1, ?2, ?3)
             ON CONFLICT(song_id, viewer) DO UPDATE SET position = excluded.position",
            params![song_id, viewer, position],
        )?;
        Ok(())
    }

    pub fn reading_position(&self, song_id: &str, viewer: &str) -> Result<Option<f64>> {
        let mut statement = self
            .connection
            .prepare("SELECT position FROM reading_positions WHERE song_id = ?1 AND viewer = ?2")?;
        let value = statement
            .query_row(params![song_id, viewer], |row| row.get::<_, f64>(0))
            .optional()?;
        Ok(value)
    }

    pub fn database_path() -> Option<PathBuf> {
        ProjectDirs::from("org", "MiReDo", "MiReDo")
            .map(|dirs| dirs.data_local_dir().join("miredo.sqlite3"))
    }
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn now_string() -> String {
    now_nanos().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorites_and_playlists_persist_in_sqlite() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE favorites(song_id TEXT PRIMARY KEY, created_at TEXT NOT NULL);
                 CREATE TABLE playlists(id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
                 CREATE TABLE playlist_songs(playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE, song_id TEXT NOT NULL, position INTEGER NOT NULL, PRIMARY KEY(playlist_id, song_id));
                 CREATE TABLE preferences(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE reading_positions(song_id TEXT NOT NULL, viewer TEXT NOT NULL, position REAL NOT NULL, PRIMARY KEY(song_id, viewer));",
            )
            .unwrap();
        let storage = UserStorage { connection };
        storage.toggle_favorite("ffpm:001").unwrap();
        assert!(storage.favorite_ids().unwrap().contains("ffpm:001"));
        let playlist_id = storage.create_playlist("Répétition").unwrap();
        storage
            .add_song_to_playlist(&playlist_id, "ffpm:001")
            .unwrap();
        assert_eq!(
            storage.playlist_song_ids(&playlist_id).unwrap(),
            vec!["ffpm:001"]
        );
    }
}
