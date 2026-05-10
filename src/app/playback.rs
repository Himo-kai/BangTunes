// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors
//
// Playback-related methods: play_track, next_track, previous_track,
// update_playback_status, handle_audio_event, plus the compute_prev_idx helper.

use super::{AppTab, InteractiveApp, RepeatMode};
use crate::{
    audio::player::PlayerEvent,
    behavior::{PlaybackEvent, SkipReason},
};
use anyhow::Result;
use std::path::PathBuf;
use tracing::debug;

impl InteractiveApp {
    // ──────────────────────────────────────────────────────────────────
    // Startup helper
    // ──────────────────────────────────────────────────────────────────

    pub async fn play_specific_track(&mut self, track_path: &PathBuf) -> Result<()> {
        for (index, track) in self.tracks.iter().enumerate() {
            if track.file_path == *track_path {
                tracing::info!("🎵 Found requested track: {:?}", track_path);
                self.play_track(index).await?;
                return Ok(());
            }
        }
        tracing::info!("⚠️ Requested track not found in library: {:?}", track_path);
        self.set_status(&format!("Track not found: {}", track_path.display()));
        Ok(())
    }

    // ──────────────────────────────────────────────────────────────────
    // Core playback
    // ──────────────────────────────────────────────────────────────────

    pub(super) async fn play_track(&mut self, track_idx: usize) -> Result<()> {
        if track_idx >= self.tracks.len() {
            return Ok(());
        }

        let track = self.tracks[track_idx].clone();

        if !track.is_playable() {
            self.set_status(&format!("❌ Track not playable: {}", track.display_title()));
            debug!("🎵 Track {} is not playable (format unsupported or file missing)", track.display_title());
            return Ok(());
        }

        // Record behavior tracking event
        let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackStarted {
            track_id: track.id,
            timestamp: chrono::Utc::now(),
        }).await;

        // Update recently-played history for intelligent shuffle
        self.recently_played.push_back(track.id);
        let max_history = (self.tracks.len() / 4).max(20);
        if self.recently_played.len() > max_history {
            self.recently_played.pop_front();
        }

        #[cfg(feature = "audio")]
        {
            self.set_status(&format!("🔄 Attempting to play: {}", track.display_title()));

            match tokio::task::block_in_place(|| self.audio_player.play_track(track.clone())) {
                Ok(()) => {
                    self.current_track_index = Some(track_idx);
                    self.is_playing = true;
                    self.current_position = std::time::Duration::from_secs(0);
                    self.total_duration = track.duration;
                    self.last_position_update = std::time::Instant::now();
                    self.set_status(&format!(
                        "✅ SUCCESS: Playing {} | idx={} | is_playing={}",
                        track.display_title(), track_idx, self.is_playing
                    ));
                }
                Err(e) => {
                    self.set_status(&format!(
                        "❌ AUDIO PLAYER FAILED: {} | Error: {}",
                        track.display_title(), e
                    ));
                    self.is_playing = false;
                    self.current_track_index = None;
                }
            }
        }

        #[cfg(not(feature = "audio"))]
        {
            self.set_status(&format!(
                "▶ {} — Audio playback unavailable. Run: python bang_tunes.py quickplay",
                track.display_title()
            ));
            self.current_track_index = Some(track_idx);
        }

        Ok(())
    }

    // ──────────────────────────────────────────────────────────────────
    // Navigation
    // ──────────────────────────────────────────────────────────────────

    pub(super) async fn next_track(&mut self) -> Result<()> {
        // Only record skip if NOT advancing due to queue consumption
        if self.queue.is_empty() {
            if let Some(current_idx) = self.current_track_index {
                let track = &self.tracks[current_idx];
                let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackSkipped {
                    track_id: track.id,
                    position: self.current_position.as_secs(),
                    reason: SkipReason::NextTrack,
                    timestamp: chrono::Utc::now(),
                }).await;
            }
        }

        // Priority 1: queue
        if let Some(queued_idx) = self.queue.pop_front() {
            if self.queue.is_empty() {
                self.queue_list_state.select(None);
            } else if let Some(sel) = self.queue_list_state.selected() {
                self.queue_list_state.select(Some(sel.min(self.queue.len() - 1)));
            }
            self.play_track(queued_idx).await?;
            return Ok(());
        }

        // Priority 2: expanded playlist context
        if self.current_tab == AppTab::Playlists && !self.expanded_playlists.is_empty() {
            let expanded_playlist_id = self.expanded_playlists.iter().next().unwrap().clone();
            debug!("🎵 Next track in playlist context: playlist={}", expanded_playlist_id);

            if let Some(playlist) = self.playlist_manager.get_playlist(&expanded_playlist_id) {
                let valid_tracks = playlist.get_valid_tracks(&self.tracks);

                if let Some(track_state) = self.playlist_track_states.get_mut(&expanded_playlist_id) {
                    let next_track_idx = if self.is_shuffled {
                        debug!("🎵 Using intelligent shuffle weighting for playlist");

                        let available_track_ids: Vec<uuid::Uuid> = valid_tracks
                            .iter()
                            .map(|&idx| self.tracks[idx].id)
                            .collect();

                        let behaviors = match self.behavior_tracker.get_all_behaviors().await {
                            Ok(behavior_vec) => behavior_vec
                                .into_iter()
                                .map(|b| (b.track_id, b))
                                .collect::<std::collections::HashMap<_, _>>(),
                            Err(e) => {
                                debug!("Failed to load behaviors for shuffle: {}", e);
                                std::collections::HashMap::new()
                            }
                        };

                        let recently_played_vec: Vec<uuid::Uuid> =
                            self.recently_played.iter().copied().collect();

                        if let Some(next_track_id) = self.shuffle_weighting.select_next_track(
                            &available_track_ids,
                            &behaviors,
                            &recently_played_vec,
                        ) {
                            valid_tracks
                                .iter()
                                .position(|&idx| self.tracks[idx].id == next_track_id)
                                .unwrap_or(0)
                        } else {
                            use rand::Rng;
                            rand::thread_rng().gen_range(0..valid_tracks.len())
                        }
                    } else {
                        let current_track_idx = track_state.selected().unwrap_or(0);
                        let next = current_track_idx + 1;
                        if next >= valid_tracks.len() {
                            if self.repeat_mode == RepeatMode::All {
                                track_state.select(Some(0));
                                if let Some(&actual_track_idx) = valid_tracks.first() {
                                    debug!("🎵 Playlist ended, repeating from start (RepeatAll)");
                                    self.play_track(actual_track_idx).await?;
                                    return Ok(());
                                }
                            } else {
                                self.is_playing = false;
                                self.set_status("⏹️ Playlist finished");
                                debug!("🎵 Playlist finished (RepeatMode::{:?})", self.repeat_mode);
                                return Ok(());
                            }
                        }
                        next
                    };

                    track_state.select(Some(next_track_idx));
                    if let Some(&actual_track_idx) = valid_tracks.get(next_track_idx) {
                        debug!("🎵 Playing track {} from playlist (shuffled: {})", actual_track_idx, self.is_shuffled);
                        self.play_track(actual_track_idx).await?;
                    } else {
                        debug!("❌ Next track index {} not found in playlist", next_track_idx);
                    }
                } else {
                    debug!("❌ No track state found for expanded playlist");
                }
            }
        } else {
            // Library context
            debug!("🎵 Next track in library context");
            if let Some(selected) = self.list_state.selected() {
                let next_idx = if self.is_shuffled {
                    debug!("🎵 Using intelligent shuffle weighting for next track");

                    let available_track_ids: Vec<uuid::Uuid> = self.filtered_tracks
                        .iter()
                        .map(|&idx| self.tracks[idx].id)
                        .filter(|id| !self.failed_tracks.contains(id))
                        .collect();

                    let behaviors = match self.behavior_tracker.get_all_behaviors().await {
                        Ok(behavior_vec) => behavior_vec
                            .into_iter()
                            .map(|b| (b.track_id, b))
                            .collect::<std::collections::HashMap<_, _>>(),
                        Err(e) => {
                            debug!("Failed to load behaviors for shuffle: {}", e);
                            std::collections::HashMap::new()
                        }
                    };

                    let recently_played_vec: Vec<uuid::Uuid> =
                        self.recently_played.iter().copied().collect();

                    if let Some(next_track_id) = self.shuffle_weighting.select_next_track(
                        &available_track_ids,
                        &behaviors,
                        &recently_played_vec,
                    ) {
                        self.filtered_tracks
                            .iter()
                            .position(|&idx| self.tracks[idx].id == next_track_id)
                            .unwrap_or((selected + 1) % self.filtered_tracks.len())
                    } else {
                        use rand::Rng;
                        rand::thread_rng().gen_range(0..self.filtered_tracks.len())
                    }
                } else {
                    let next = selected + 1;
                    if next >= self.filtered_tracks.len() {
                        if self.repeat_mode == RepeatMode::Off {
                            self.is_playing = false;
                            self.set_status("⏹️ End of library");
                            debug!("🎵 Library finished (RepeatMode::Off)");
                            return Ok(());
                        } else {
                            0
                        }
                    } else {
                        next
                    }
                };

                // Skip tracks that previously failed to decode
                let final_idx = {
                    let len = self.filtered_tracks.len();
                    let mut candidate = next_idx;
                    let mut attempts = 0;
                    while attempts < len {
                        let t_idx = self.filtered_tracks[candidate];
                        if self.failed_tracks.contains(&self.tracks[t_idx].id) {
                            candidate = (candidate + 1) % len;
                            attempts += 1;
                        } else {
                            break;
                        }
                    }
                    candidate
                };

                self.list_state.select(Some(final_idx));
                let track_idx = self.filtered_tracks[final_idx];
                self.play_track(track_idx).await?;
            }
        }

        Ok(())
    }

    pub(super) async fn previous_track(&mut self) -> Result<()> {
        if self.current_tab == AppTab::Playlists && !self.expanded_playlists.is_empty() {
            let expanded_playlist_id = self.expanded_playlists.iter().next().unwrap().clone();
            debug!("🎵 Previous track in playlist context: playlist={}", expanded_playlist_id);

            if let Some(playlist) = self.playlist_manager.get_playlist(&expanded_playlist_id) {
                let valid_tracks = playlist.get_valid_tracks(&self.tracks);

                if let Some(track_state) = self.playlist_track_states.get_mut(&expanded_playlist_id) {
                    let current_track_idx = track_state.selected().unwrap_or(0);
                    if let Some(prev_track_idx) = compute_prev_idx(&valid_tracks, current_track_idx) {
                        track_state.select(Some(prev_track_idx));
                        if let Some(&actual_track_idx) = valid_tracks.get(prev_track_idx) {
                            debug!(
                                "🎵 Playing previous track {} from playlist (track {} of {})",
                                actual_track_idx,
                                prev_track_idx + 1,
                                valid_tracks.len()
                            );
                            self.play_track(actual_track_idx).await?;
                        } else {
                            debug!("❌ Previous track index {} not found in playlist", prev_track_idx);
                        }
                    }
                } else {
                    debug!("❌ No track state found for expanded playlist");
                }
            }
        } else {
            debug!("🎵 Previous track in library context");
            if let Some(selected) = self.list_state.selected() {
                if let Some(prev_idx) = compute_prev_idx(&self.filtered_tracks, selected) {
                    self.list_state.select(Some(prev_idx));
                    let track_idx = self.filtered_tracks[prev_idx];
                    self.play_track(track_idx).await?;
                }
            }
        }
        Ok(())
    }

    // ──────────────────────────────────────────────────────────────────
    // Periodic status update
    // ──────────────────────────────────────────────────────────────────

    pub(super) async fn update_playback_status(&mut self) -> Result<()> {
        self.volume = self.audio_player.get_volume();

        if let Some(current_audio_track) = self.audio_player.get_current_track() {
            if let Some(current_idx) = self.current_track_index {
                if current_idx < self.tracks.len() {
                    let expected_track = &self.tracks[current_idx];
                    if current_audio_track.id != expected_track.id {
                        debug!(
                            "⚠️  Track sync mismatch: UI tracking {} but audio playing {}",
                            expected_track.id, current_audio_track.id
                        );
                    }
                }
            }
        }

        if self.is_playing {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(self.last_position_update);
            self.current_position += elapsed;
            self.last_position_update = now;
        }

        Ok(())
    }

    // ──────────────────────────────────────────────────────────────────
    // Audio event handler
    // ──────────────────────────────────────────────────────────────────

    pub(super) async fn handle_audio_event(&mut self, event: PlayerEvent) -> Result<()> {
        match event {
            PlayerEvent::TrackStarted(track) => {
                self.set_status(&format!("▶️ Playing: {}", self.format_track_title(&track)));
            }
            PlayerEvent::TrackFinished(_track) => {
                // Queued tracks always advance — user explicitly requested them, bypass autoplay toggle
                if !self.queue.is_empty() {
                    debug!("TrackFinished - queue has items, advancing to queued track");
                    self.next_track().await?;
                    return Ok(());
                }
                match self.repeat_mode {
                    RepeatMode::One => {
                        if let Some(idx) = self.current_track_index {
                            debug!("RepeatOne: Replaying track at index {}", idx);
                            self.play_track(idx).await?;
                        }
                    }
                    RepeatMode::All | RepeatMode::Off => {
                        if self.autoplay {
                            debug!("TrackFinished - auto-advancing (RepeatMode::{:?})", self.repeat_mode);
                            self.next_track().await?;
                        } else {
                            debug!("TrackFinished set is_playing=false");
                            self.is_playing = false;
                        }
                    }
                }
            }
            PlayerEvent::DurationLearned(learned_track, actual_duration) => {
                if let Some(track_index) = self.tracks.iter().position(|t| t.id == learned_track.id) {
                    self.tracks[track_index].learn_duration(actual_duration);
                    let duration_secs = actual_duration.as_secs();
                    let track_id = self.tracks[track_index].id;
                    let _ = self.behavior_tracker.save_learned_duration(track_id, duration_secs).await;
                    debug!(
                        "📏 Learned + persisted duration: {} ({}:{:02})",
                        self.format_track_title(&learned_track),
                        duration_secs / 60,
                        duration_secs % 60
                    );
                }
            }
            PlayerEvent::TrackPaused => {
                // Ignore — premature pause events from rodio buffering; only user action pauses UI
            }
            PlayerEvent::TrackResumed => {
                self.is_playing = true;
                self.set_status("▶️ Resumed");
            }
            PlayerEvent::TrackStopped => {
                // User-initiated stop only. Never triggers autoplay.
                self.is_playing = false;
                debug!("⏹️ TrackStopped: halted playback, no autoplay");
            }
            PlayerEvent::VolumeChanged(volume) => {
                self.volume = volume;
                self.set_status(&format!("🔊 Volume: {}%", (volume * 100.0) as u32));
            }
            PlayerEvent::Error(error) => {
                let error_str = error.to_string();
                if error_str.contains("underrun occurred") || error_str.contains("snd_pcm_recover") {
                    debug!("🔊 ALSA underrun (non-critical)");
                } else if error_str.contains("Unsupported audio format")
                    || error_str.contains("corrupted file")
                    || error_str.contains("Failed to open file")
                    || error_str.contains("NoSupportedCodec")
                    || error_str.contains("IoError")
                {
                    let (track_id, track_label) = if let Some(idx) = self.current_track_index {
                        let t = &self.tracks[idx];
                        (Some(t.id), self.format_track_title(t))
                    } else {
                        (None, "unknown track".to_string())
                    };

                    if let Some(id) = track_id {
                        self.failed_tracks.insert(id);
                        debug!("⚠️ Flagged track {} as failed: {}", id, error_str);
                    }

                    self.is_playing = false;
                    self.set_status(&format!("⚠️ Skipping '{}' — {}", track_label, error_str));

                    if self.autoplay && !self.tracks.is_empty() {
                        let _ = self.next_track().await;
                    }
                } else {
                    self.set_status(&format!("❌ Audio error: {}", error));
                }
            }
            PlayerEvent::PositionChanged(_position) => {
                // Handled by update_playback_status
            }
        }
        Ok(())
    }

    // ──────────────────────────────────────────────────────────────────
    // Display helper
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn format_track_title(&self, track: &crate::Track) -> String {
        if let (Some(title), Some(artist)) = (&track.metadata.title, &track.metadata.artist) {
            format!("{} - {}", artist, title)
        } else if let Some(title) = &track.metadata.title {
            title.clone()
        } else {
            track.file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Free helper — previous index with wrap-around
// ──────────────────────────────────────────────────────────────────────────

/// Returns the index before `selected` in `items`, wrapping 0 → last.
/// Returns `None` when `items` is empty (prevents underflow).
pub(crate) fn compute_prev_idx(items: &[usize], selected: usize) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    Some(if selected == 0 { items.len() - 1 } else { selected - 1 })
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::compute_prev_idx;

    #[test]
    fn prev_idx_returns_none_when_filtered_tracks_empty() {
        assert!(compute_prev_idx(&[], 0).is_none());
    }

    #[test]
    fn prev_idx_wraps_to_last_when_at_first() {
        let tracks = [10usize, 20, 30];
        assert_eq!(compute_prev_idx(&tracks, 0), Some(2));
    }

    #[test]
    fn prev_idx_decrements_normally() {
        let tracks = [10usize, 20, 30];
        assert_eq!(compute_prev_idx(&tracks, 2), Some(1));
    }
}
