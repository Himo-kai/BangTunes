use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use super::track::Track;

/// Represents a single playlist with metadata and track references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub track_paths: Vec<PathBuf>,  // Store file paths instead of full Track objects
    pub track_count: usize,
    pub total_duration: Option<u64>, // Total duration in seconds
}

impl Playlist {
    /// Create a new empty playlist
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            created_at: now,
            modified_at: now,
            track_paths: Vec::new(),
            track_count: 0,
            total_duration: None,
        }
    }

    /// Add a track to the playlist
    pub fn add_track(&mut self, track_path: PathBuf) {
        if !self.track_paths.contains(&track_path) {
            info!("Added track '{}' to playlist '{}'", track_path.display(), self.name);
            self.track_paths.push(track_path);
            self.track_count = self.track_paths.len();
            self.modified_at = chrono::Utc::now();
            self.update_total_duration();
        }
    }

    /// Remove a track from the playlist by path
    pub fn remove_track(&mut self, track_path: &Path) -> bool {
        if let Some(pos) = self.track_paths.iter().position(|p| p == track_path) {
            self.track_paths.remove(pos);
            self.track_count = self.track_paths.len();
            self.modified_at = chrono::Utc::now();
            self.update_total_duration();
            info!("Removed track '{}' from playlist '{}'", track_path.display(), self.name);
            true
        } else {
            false
        }
    }

    /// Move a track to a different position in the playlist
    pub fn move_track(&mut self, from_index: usize, to_index: usize) -> bool {
        if from_index < self.track_paths.len() && to_index < self.track_paths.len() {
            let track = self.track_paths.remove(from_index);
            self.track_paths.insert(to_index, track);
            self.modified_at = chrono::Utc::now();
            info!("Moved track from position {} to {} in playlist '{}'", from_index, to_index, self.name);
            true
        } else {
            false
        }
    }

    /// Get tracks that exist and are accessible
    pub fn get_valid_tracks(&self, all_tracks: &[Track]) -> Vec<usize> {
        // Create a map from file path to track index for quick lookup
        let track_map: HashMap<&Path, usize> = all_tracks
            .iter()
            .enumerate()
            .map(|(idx, track)| (track.file_path.as_path(), idx))
            .collect();
        
        self.track_paths
            .iter()
            .filter_map(|path| track_map.get(path.as_path()))
            .copied()
            .collect()
    }

    /// Update total duration based on available tracks
    fn update_total_duration(&mut self) {
        // Note: This sets duration to None since we don't have track metadata here.
        // The actual duration will be calculated in get_playlist_stats() using calculate_duration()
        self.total_duration = None;
    }

    /// Calculate total duration from available tracks
    pub fn calculate_duration(&self, all_tracks: &[Track]) -> Option<u64> {
        let valid_tracks = self.get_valid_tracks(all_tracks);
        let total: u64 = valid_tracks
            .iter()
            .filter_map(|&idx| all_tracks.get(idx))
            .filter_map(|track| track.duration.map(|d| d.as_millis() as u64))
            .sum();
        
        if total > 0 {
            Some(total)
        } else {
            None
        }
    }

    /// Check if playlist is empty
    pub fn is_empty(&self) -> bool {
        self.track_paths.is_empty()
    }

    /// Get formatted duration string
    pub fn duration_string(&self, all_tracks: &[Track]) -> String {
        if let Some(duration) = self.calculate_duration(all_tracks) {
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;
            let seconds = duration % 60;
            
            if hours > 0 {
                format!("{}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                format!("{}:{:02}", minutes, seconds)
            }
        } else {
            "Unknown".to_string()
        }
    }
}

/// Manages all playlists - creation, loading, saving, deletion
#[derive(Debug)]
pub struct PlaylistManager {
    playlists: HashMap<String, Playlist>,
    playlists_dir: PathBuf,
}

impl PlaylistManager {
    /// Create a new playlist manager
    pub fn new(playlists_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure playlists directory exists
        if !playlists_dir.exists() {
            fs::create_dir_all(&playlists_dir)?;
            info!("Created playlists directory: {}", playlists_dir.display());
        }

        let mut manager = Self {
            playlists: HashMap::new(),
            playlists_dir,
        };

        // Load existing playlists
        manager.load_all_playlists()?;
        
        Ok(manager)
    }

    /// Create a new playlist
    pub fn create_playlist(&mut self, name: String, description: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
        // Check if playlist name already exists
        if self.playlists.values().any(|p| p.name == name) {
            return Err(format!("Playlist '{}' already exists", name).into());
        }

        let playlist = Playlist::new(name.clone(), description);
        let playlist_id = playlist.id.clone();
        
        // Save to file
        self.save_playlist(&playlist)?;
        
        // Add to memory
        self.playlists.insert(playlist_id.clone(), playlist);
        
        info!("Created new playlist: '{}'", name);
        Ok(playlist_id)
    }

    /// Get a playlist by ID
    pub fn get_playlist(&self, id: &str) -> Option<&Playlist> {
        self.playlists.get(id)
    }

    /// Get a mutable playlist by ID
    pub fn get_playlist_mut(&mut self, id: &str) -> Option<&mut Playlist> {
        self.playlists.get_mut(id)
    }



    /// Delete a playlist
    pub fn delete_playlist(&mut self, playlist_id: &str) -> anyhow::Result<bool> {
        if let Some(playlist) = self.playlists.remove(playlist_id) {
            // Delete file
            let file_path = self.get_playlist_file_path(&playlist.id);
            if file_path.exists() {
                fs::remove_file(&file_path)
                    .map_err(|e| anyhow::anyhow!("Failed to delete playlist file: {}", e))?;
                info!("Deleted playlist file: {}", file_path.display());
            }
            
            info!("Deleted playlist: '{}'", playlist.name);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Save a playlist to file
    pub fn save_playlist(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let file_path = self.get_playlist_file_path(&playlist.id);
        let json = serde_json::to_string_pretty(playlist)
            .map_err(|e| anyhow::anyhow!("Failed to serialize playlist: {}", e))?;
        fs::write(&file_path, json)
            .map_err(|e| anyhow::anyhow!("Failed to write playlist file: {}", e))?;
        info!("Saved playlist '{}' to {}", playlist.name, file_path.display());
        Ok(())
    }

    /// Load all playlists from the playlists directory
    fn load_all_playlists(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let entries = fs::read_dir(&self.playlists_dir)?;
        let mut loaded_count = 0;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_playlist_from_file(&path) {
                    Ok(playlist) => {
                        self.playlists.insert(playlist.id.clone(), playlist);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load playlist from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("Loaded {} playlists from {}", loaded_count, self.playlists_dir.display());
        Ok(())
    }

    /// Load a single playlist from file
    fn load_playlist_from_file(&self, file_path: &Path) -> anyhow::Result<Playlist> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read playlist file: {}", e))?;
        let playlist: Playlist = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse playlist JSON: {}", e))?;
        info!("Loaded playlist '{}' from {}", playlist.name, file_path.display());
        Ok(playlist)
    }

    /// Get the file path for a playlist
    fn get_playlist_file_path(&self, playlist_id: &str) -> PathBuf {
        self.playlists_dir.join(format!("{}.json", playlist_id))
    }

    /// Rename a playlist
    pub fn rename_playlist(&mut self, playlist_id: &str, new_name: String) -> Result<(), Box<dyn std::error::Error>> {
        // Check if new name already exists
        if self.playlists.values().any(|p| p.name == new_name && p.id != playlist_id) {
            return Err(format!("Playlist '{}' already exists", new_name).into());
        }

        if let Some(playlist) = self.playlists.get_mut(playlist_id) {
            let old_name = playlist.name.clone();
            playlist.name = new_name.clone();
            playlist.modified_at = chrono::Utc::now();
            
            // Clone the playlist to avoid borrow checker issues
            let playlist_clone = playlist.clone();
            self.save_playlist(&playlist_clone)?;
            info!("Renamed playlist '{}' to '{}'", old_name, new_name);
            Ok(())
        } else {
            Err(format!("Playlist with ID '{}' not found", playlist_id).into())
        }
    }

    /// Add a track to a playlist
    pub fn add_track_to_playlist(&mut self, playlist_id: &str, track_path: &Path) -> anyhow::Result<()> {
        // Check if playlist exists first
        if !self.playlists.contains_key(playlist_id) {
            return Err(anyhow::anyhow!("Playlist not found: {}", playlist_id));
        }
        
        // Update the playlist
        if let Some(playlist) = self.playlists.get_mut(playlist_id) {
            playlist.add_track(track_path.to_path_buf());
            playlist.modified_at = chrono::Utc::now();
        }
        
        // Save the playlist (clone to avoid borrow issues)
        if let Some(playlist) = self.playlists.get(playlist_id) {
            let playlist_clone = playlist.clone();
            self.save_playlist(&playlist_clone)?;
            info!("Added track to playlist {}: {}", playlist_id, track_path.display());
        }
        
        Ok(())
    }

    /// List all playlists
    pub fn list_playlists(&self) -> Vec<&Playlist> {
        self.playlists.values().collect()
    }

    /// Get playlist statistics
    pub fn get_playlist_stats(&self, playlist_id: &str, all_tracks: &[Track]) -> Option<PlaylistStats> {
        self.playlists.get(playlist_id).map(|playlist| {
            let calculated_duration = playlist.calculate_duration(all_tracks).unwrap_or(0);
            PlaylistStats {
                track_count: playlist.track_count,
                total_duration: calculated_duration,
            }
        })
    }
}

/// Statistics about playlists
#[derive(Debug, Default)]
pub struct PlaylistStats {
    pub track_count: usize,
    pub total_duration: u64,
}
