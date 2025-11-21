use anyhow::{Result, anyhow};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use crate::audio::Track;

/// BangTunes database integration for loading track metadata
#[derive(Clone)]
pub struct BangTunesDatabase {
    db_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DatabaseTrack {
    pub id: i64,
    pub youtube_id: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub file_path: Option<String>,
    pub added_on: Option<String>,
}

impl BangTunesDatabase {
    /// Create a new database connection to the BangTunes library
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    /// Try to find the BangTunes database in common locations
    pub fn find_database() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
        
        let candidates = vec![
            home.join("BangTunes").join("library.db"),
            home.join("Builds").join("BangTunes").join("library.db"),
            home.join("Downloads").join("BangTunes").join("library.db"),
            home.join("Music").join("BangTunes").join("library.db"),
            // Termux paths
            home.join("storage").join("shared").join("BangTunes").join("library.db"),
            PathBuf::from("/data/data/com.termux/files/home/BangTunes/library.db"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                return Ok(Self::new(candidate));
            }
        }

        Err(anyhow!("Could not find BangTunes database in any common location"))
    }

    /// Get a database connection
    fn get_connection(&self) -> Result<Connection> {
        if !self.db_path.exists() {
            return Err(anyhow!("BangTunes database not found at: {}", self.db_path.display()));
        }

        let conn = Connection::open(&self.db_path)?;
        Ok(conn)
    }

    /// Load all tracks from the BangTunes database
    pub fn load_all_tracks(&self) -> Result<Vec<DatabaseTrack>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(
            "SELECT id, youtube_id, title, artist, album, file_path, added_on 
             FROM tracks 
             ORDER BY artist, album, title"
        )?;

        let track_iter = stmt.query_map([], |row| {
            Ok(DatabaseTrack {
                id: row.get(0)?,
                youtube_id: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                file_path: row.get(5)?,
                added_on: row.get(6)?,
            })
        })?;

        let mut tracks = Vec::new();
        for track in track_iter {
            tracks.push(track?);
        }

        Ok(tracks)
    }

    /// Find track metadata by file path
    pub fn find_track_by_path<P: AsRef<Path>>(&self, file_path: P) -> Result<Option<DatabaseTrack>> {
        let conn = self.get_connection()?;
        let path_str = file_path.as_ref().to_string_lossy();
        
        let mut stmt = conn.prepare(
            "SELECT id, youtube_id, title, artist, album, file_path, added_on 
             FROM tracks 
             WHERE file_path = ?1"
        )?;

        let mut track_iter = stmt.query_map([&*path_str], |row| {
            Ok(DatabaseTrack {
                id: row.get(0)?,
                youtube_id: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                file_path: row.get(5)?,
                added_on: row.get(6)?,
            })
        })?;

        if let Some(track) = track_iter.next() {
            Ok(Some(track?))
        } else {
            Ok(None)
        }
    }

    /// Update track metadata in the database
    pub fn update_track_metadata(&self, track_id: i64, title: &str, artist: &str, album: &str) -> Result<()> {
        let conn = self.get_connection()?;
        
        conn.execute(
            "UPDATE tracks SET title = ?1, artist = ?2, album = ?3 WHERE id = ?4",
            [title, artist, album, &track_id.to_string()],
        )?;

        Ok(())
    }
}

/// Smart artist parsing logic (Rust version of Python implementation)
pub fn parse_artist_title(full_title: &str) -> (String, String) {
    if full_title.is_empty() {
        return ("Unknown Artist".to_string(), "Unknown Title".to_string());
    }

    // Common separators for "Artist - Title" format
    let separators = [" - ", " – ", " — ", " | ", ": "];
    
    for sep in &separators {
        if let Some(pos) = full_title.find(sep) {
            let artist = full_title[..pos].trim();
            let title = full_title[pos + sep.len()..].trim();
            
            if !artist.is_empty() && !title.is_empty() {
                return (artist.to_string(), title.to_string());
            }
        }
    }
    
    // If no separator found, assume the whole thing is the title
    ("Unknown Artist".to_string(), full_title.trim().to_string())
}

/// Enhance track metadata with database information and smart parsing
pub fn enhance_track_metadata(track: &mut Track, db_track: Option<&DatabaseTrack>) {
    if let Some(db_track) = db_track {
        // Use database metadata if available
        if let Some(title) = &db_track.title {
            track.metadata.title = Some(title.clone());
        }
        
        if let Some(artist) = &db_track.artist {
            if !artist.trim().is_empty() {
                track.metadata.artist = Some(artist.clone());
            }
        }
        
        if let Some(album) = &db_track.album {
            track.metadata.album = Some(album.clone());
        }
    }
    
    // If artist is still missing, try to parse from title
    if track.metadata.artist.is_none() || track.metadata.artist.as_ref().map_or(true, |a| a.trim().is_empty()) {
        if let Some(title) = &track.metadata.title {
            let (parsed_artist, parsed_title) = parse_artist_title(title);
            if parsed_artist != "Unknown Artist" {
                track.metadata.artist = Some(parsed_artist);
                // Optionally update the title if it was parsed
                if parsed_title != *title {
                    track.metadata.title = Some(parsed_title);
                }
            }
        }
    }
}
