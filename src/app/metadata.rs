// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors
//
// Metadata editor actions: save, apply filename suggestion, reset, bulk apply, clear.

use super::{EditMode, InteractiveApp};
use anyhow::Result;

impl InteractiveApp {
    pub(super) async fn save_current_edit(&mut self) -> Result<()> {
        if let Some(track_idx) = self.editing_track_index {
            if track_idx < self.tracks.len() {
                let file_path = self.tracks[track_idx].file_path.clone();
                let edit_mode = self.edit_mode;
                let edit_title = self.edit_title.clone();
                let edit_artist = self.edit_artist.clone();

                {
                    let track = &mut self.tracks[track_idx];
                    match edit_mode {
                        EditMode::Title => {
                            track.metadata.title = Some(edit_title.clone());
                        }
                        EditMode::Artist => {
                            track.metadata.artist = Some(edit_artist.clone());
                        }
                        EditMode::None => {}
                    }
                }

                match edit_mode {
                    EditMode::Title => {
                        self.set_status(&format!("✅ Title updated: {}", edit_title));
                    }
                    EditMode::Artist => {
                        self.set_status(&format!("✅ Artist updated: {}", edit_artist));
                    }
                    EditMode::None => {}
                }

                // Persist to BangTunes database if available
                if let Some(ref database) = self.database {
                    if let Ok(Some(db_track)) = database.find_track_by_path(&file_path) {
                        let track = &self.tracks[track_idx];
                        let title = track.metadata.title.as_deref().unwrap_or("Unknown Title");
                        let artist = track.metadata.artist.as_deref().unwrap_or("Unknown Artist");
                        let album = track.metadata.album.as_deref().unwrap_or("Unknown Album");

                        if let Err(e) = database.update_track_metadata(db_track.id, title, artist, album) {
                            self.set_status(&format!("⚠️ Database update failed: {}", e));
                        } else {
                            self.set_status("💾 Saved to BangTunes database");
                        }
                    } else {
                        self.set_status("⚠️ Track not found in BangTunes database");
                    }
                }

                self.edit_mode = EditMode::None;
                self.editing_track_index = None;
                self.edit_title.clear();
                self.edit_artist.clear();
            }
        }
        Ok(())
    }

    pub(super) async fn apply_filename_suggestion(&mut self, track_idx: usize) -> Result<()> {
        if track_idx < self.tracks.len() {
            let file_path = self.tracks[track_idx].file_path.clone();
            let filename = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");

            let parsed = self.metadata_parser.parse_filename(filename);

            self.tracks[track_idx].metadata.title = Some(parsed.suggested_title.clone());
            self.tracks[track_idx].metadata.artist = Some(parsed.suggested_artist.clone());

            if let Some(ref database) = self.database {
                if let Ok(Some(db_track)) = database.find_track_by_path(&file_path) {
                    let track = &self.tracks[track_idx];
                    let title = track.metadata.title.as_deref().unwrap_or("Unknown Title");
                    let artist = track.metadata.artist.as_deref().unwrap_or("Unknown Artist");
                    let album = track.metadata.album.as_deref().unwrap_or("Unknown Album");
                    let _ = database.update_track_metadata(db_track.id, title, artist, album);
                }
            }

            self.set_status(&format!(
                "🤖 Applied suggestion: {} - {} (confidence: {:.0}%)",
                parsed.suggested_title,
                parsed.suggested_artist,
                parsed.confidence * 100.0
            ));
        }
        Ok(())
    }

    pub(super) async fn reset_track_metadata(&mut self, track_idx: usize) -> Result<()> {
        if track_idx < self.tracks.len() {
            let track = &mut self.tracks[track_idx];
            track.metadata.title = None;
            track.metadata.artist = None;
            self.set_status("🔄 Reset to original metadata");
        }
        Ok(())
    }

    pub(super) async fn bulk_apply_suggestions(&mut self) -> Result<()> {
        let mut applied_count = 0;
        let total_tracks = self.tracks.len();

        for i in 0..total_tracks {
            let file_path = self.tracks[i].file_path.clone();
            let filename = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");

            let parsed = self.metadata_parser.parse_filename(filename);

            if parsed.confidence > 0.5 {
                self.tracks[i].metadata.title = Some(parsed.suggested_title.clone());
                self.tracks[i].metadata.artist = Some(parsed.suggested_artist.clone());

                if let Some(ref database) = self.database {
                    if let Ok(Some(db_track)) = database.find_track_by_path(&file_path) {
                        let track = &self.tracks[i];
                        let title = track.metadata.title.as_deref().unwrap_or("Unknown Title");
                        let artist = track.metadata.artist.as_deref().unwrap_or("Unknown Artist");
                        let album = track.metadata.album.as_deref().unwrap_or("Unknown Album");
                        let _ = database.update_track_metadata(db_track.id, title, artist, album);
                    }
                }

                applied_count += 1;
            }
        }

        self.set_status(&format!(
            "🚀 Bulk applied suggestions to {}/{} tracks (confidence >50%)",
            applied_count, total_tracks
        ));
        Ok(())
    }

    pub(super) async fn clear_track_metadata(&mut self, track_idx: usize) -> Result<()> {
        if track_idx < self.tracks.len() {
            let track = &mut self.tracks[track_idx];
            track.metadata.title = None;
            track.metadata.artist = None;
            self.set_status("🗑️ Cleared track metadata");
        }
        Ok(())
    }
}
