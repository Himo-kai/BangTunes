use super::AudioFormat;
use anyhow::Result;
use id3::TagLike;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;
use xxhash_rust::xxh64::xxh64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: Uuid,
    pub file_path: PathBuf,
    pub metadata: TrackMetadata,
    pub format: AudioFormat,
    pub file_size: u64,
    pub duration: Option<Duration>,
    pub content_hash: Option<u64>, // xxhash64 for deduplication and move detection
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub duration_ms: Option<u64>,
}

impl Track {
    pub fn new(file_path: PathBuf) -> Self {
        let format = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(AudioFormat::from_extension)
            .unwrap_or(AudioFormat::Unknown);

        Self {
            id: Uuid::new_v4(),
            file_path,
            metadata: TrackMetadata::default(),
            format,
            file_size: 0,
            duration: None,
            content_hash: None,
        }
    }

    pub fn with_metadata(mut self, metadata: TrackMetadata) -> Self {
        self.metadata = metadata;
        if let Some(duration_ms) = self.metadata.duration_ms {
            self.duration = Some(Duration::from_millis(duration_ms));
        }
        self
    }

    /// Compute xxhash64 of file content for deduplication and move detection
    pub fn compute_content_hash(&mut self) -> Result<u64> {
        if let Some(hash) = self.content_hash {
            return Ok(hash);
        }

        let file = fs::File::open(&self.file_path)?;
        let mut buffer = Vec::new();
        
        // Read first 64KB for hash computation (balance between accuracy and performance)
        let mut limited_reader = file.take(65536);
        limited_reader.read_to_end(&mut buffer)?;
        
        let hash = xxh64(&buffer, 0);
        self.content_hash = Some(hash);
        Ok(hash)
    }

    /// Update duration based on actual playback time (duration learning)
    pub fn learn_duration(&mut self, actual_duration: Duration) {
        // Only update if we don't have duration data or if the learned duration is significantly different
        match self.duration {
            None => {
                self.duration = Some(actual_duration);
                // Also update metadata for consistency
                self.metadata.duration_ms = Some(actual_duration.as_millis() as u64);
            }
            Some(existing) => {
                // Update if the difference is more than 2 seconds (accounts for fade-outs, etc.)
                let diff = if actual_duration > existing {
                    actual_duration - existing
                } else {
                    existing - actual_duration
                };
                
                if diff > Duration::from_secs(2) {
                    self.duration = Some(actual_duration);
                    self.metadata.duration_ms = Some(actual_duration.as_millis() as u64);
                }
            }
        }
    }

    /// Check if this track is likely the same file as another (based on content hash)
    pub fn is_same_content(&self, other: &Track) -> bool {
        match (self.content_hash, other.content_hash) {
            (Some(hash1), Some(hash2)) => hash1 == hash2,
            _ => false,
        }
    }

    /// Check if this track has been moved (same content hash, different path)
    pub fn is_moved_version(&self, other: &Track) -> bool {
        self.is_same_content(other) && self.file_path != other.file_path
    }

    pub fn display_title(&self) -> String {
        self.metadata
            .title
            .clone()
            .unwrap_or_else(|| {
                self.file_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            })
    }

    pub fn display_artist(&self) -> String {
        self.metadata
            .artist
            .clone()
            .unwrap_or_else(|| "Unknown Artist".to_string())
    }

    pub fn display_album(&self) -> String {
        self.metadata
            .album
            .clone()
            .unwrap_or_else(|| "Unknown Album".to_string())
    }

    pub fn duration_seconds(&self) -> Option<u64> {
        self.duration.map(|d| d.as_secs())
    }

    pub fn is_playable(&self) -> bool {
        self.format.is_supported() && self.file_path.exists()
    }
}

impl Default for TrackMetadata {
    fn default() -> Self {
        Self {
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genre: None,
            duration_ms: None,
        }
    }
}

impl TrackMetadata {
    pub fn from_id3_tag(tag: &id3::Tag) -> Self {
        Self {
            title: tag.title().map(|s| s.to_string()),
            artist: tag.artist().map(|s| s.to_string()),
            album: tag.album().map(|s| s.to_string()),
            album_artist: tag.album_artist().map(|s| s.to_string()),
            track_number: tag.track(),
            disc_number: tag.disc(),
            year: tag.year().map(|y| y as u32),
            genre: tag.genre().map(|s| s.to_string()),
            duration_ms: tag.duration().map(|d| d as u64),
        }
    }
}
