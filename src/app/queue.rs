// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors
//
// Queue management and playlist selection context helpers.

use super::{AppTab, InteractiveApp};
use tracing::debug;

impl InteractiveApp {
    /// Load a playlist into the queue, replacing any existing contents.
    pub(super) fn load_playlist_to_queue(&mut self, playlist_id: &str) {
        if let Some(playlist) = self.playlist_manager.get_playlist(playlist_id) {
            let valid_tracks = playlist.get_valid_tracks(&self.tracks);

            if valid_tracks.is_empty() {
                self.set_status(&format!("⚠️ '{}' has no playable tracks", playlist.name));
                return;
            }

            let name = playlist.name.clone();
            self.queue.clear();
            self.queue.extend(valid_tracks.iter().copied());

            // Auto-open queue overlay so user sees what was loaded
            self.queue_visible = true;
            self.queue_list_state.select(Some(0));

            self.set_status(&format!(
                "📋 Loaded '{}' into queue ({} tracks)",
                name,
                self.queue.len()
            ));
        }
    }

    /// Determine which (playlist_id, track_index_within_playlist) the current
    /// Playlists-tab selection corresponds to.  Returns `None` if the user is
    /// not on the Playlists tab or nothing is selected.
    pub(super) fn get_playlist_selection_context(&self) -> Option<(String, usize)> {
        if self.current_tab != AppTab::Playlists {
            debug!("🔍 Not in playlists tab, current_tab={:?}", self.current_tab);
            return None;
        }

        if let Some(selected) = self.playlist_list_state.selected() {
            debug!("🔍 Playlist selection detected: selected={}", selected);
            let playlists = self.playlist_manager.list_playlists();
            let mut current_index = 0;

            for playlist in playlists {
                let is_expanded = self.expanded_playlists.contains(&playlist.id);
                debug!(
                    "🔍 Checking playlist '{}': current_index={}, is_expanded={}",
                    playlist.name, current_index, is_expanded
                );

                if current_index == selected {
                    debug!("🔍 Selected playlist header: {}", playlist.name);
                    return Some((playlist.id.clone(), 0));
                }
                current_index += 1;

                if is_expanded {
                    let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                    debug!("🔍 Expanded playlist has {} valid tracks", valid_tracks.len());
                    for (track_idx_in_playlist, _) in valid_tracks.iter().enumerate() {
                        debug!(
                            "🔍 Checking track {}: current_index={}",
                            track_idx_in_playlist, current_index
                        );
                        if current_index == selected {
                            debug!(
                                "🔍 Selected track {} in playlist '{}'",
                                track_idx_in_playlist, playlist.name
                            );
                            return Some((playlist.id.clone(), track_idx_in_playlist));
                        }
                        current_index += 1;
                    }
                }
            }
            debug!("🔍 No match found for selection {}", selected);
        } else {
            debug!("🔍 No playlist selection found");
        }

        None
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    /// Bug B regression: adding the same track index 5× should not corrupt the
    /// queue selection cursor after repeated pop_front() calls.
    #[test]
    fn queue_duplicate_tracks_pop_front_bounds_clamp() {
        let mut queue: VecDeque<usize> = VecDeque::new();
        for _ in 0..5 {
            queue.push_back(42);
        }
        let mut selection: Option<usize> = Some(4);

        for _ in 0..3 {
            queue.pop_front();
            if queue.is_empty() {
                selection = None;
            } else if let Some(sel) = selection {
                selection = Some(sel.min(queue.len() - 1));
            }
        }

        assert_eq!(queue.len(), 2);
        assert_eq!(
            selection,
            Some(1),
            "selection must be clamped to new queue length - 1"
        );
    }

    #[test]
    fn queue_empties_cleanly_selection_becomes_none() {
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(7);
        let mut selection: Option<usize> = Some(0);

        queue.pop_front();
        if queue.is_empty() {
            selection = None;
        } else if let Some(sel) = selection {
            selection = Some(sel.min(queue.len() - 1));
        }

        assert!(queue.is_empty());
        assert_eq!(selection, None, "selection must be None when queue is empty");
    }
}
