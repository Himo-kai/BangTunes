// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors
//
// handle_event: the single dispatch function that translates every
// InteractiveEvent into state mutations and side-effects.

use super::{AppTab, EditMode, InteractiveApp, InteractiveEvent, RepeatMode};
use anyhow::Result;
use tracing::debug;

impl InteractiveApp {
    pub(super) async fn handle_event(&mut self, event: InteractiveEvent) -> Result<()> {
        // ── Context-aware gate ─────────────────────────────────────────────
        let should_process = match (&event, &self.current_tab, &self.edit_mode) {
            // Global events always process
            (InteractiveEvent::Quit, _, _) => true,
            (InteractiveEvent::SwitchToLibrary, _, _) => true,
            (InteractiveEvent::SwitchToPlaylists, _, _) => true,
            (InteractiveEvent::SwitchToMetadataEditor, _, _) => true,
            (InteractiveEvent::SwitchToSettings, _, _) => true,
            (InteractiveEvent::Up, _, _) => true,
            (InteractiveEvent::Down, _, _) => true,
            (InteractiveEvent::Tick, _, _) => true,
            (InteractiveEvent::ShowHelp, _, _) => true,
            // Search
            (InteractiveEvent::EnterSearch, _, _) => true,
            (InteractiveEvent::ExitSearch, _, _) => true,
            (InteractiveEvent::ConfirmSearch, _, _) => true,
            (InteractiveEvent::SearchInput(_), _, _) => true,
            (InteractiveEvent::SearchBackspace, _, _) => true,
            // Playlist creation / rename input
            (InteractiveEvent::StartPlaylistCreation, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::PlaylistInput(_), _, _) => true,
            (InteractiveEvent::PlaylistBackspace, _, _) => true,
            (InteractiveEvent::ConfirmPlaylistCreation, _, _) => true,
            (InteractiveEvent::CancelPlaylistCreation, _, _) => true,
            // Playlist selector overlay
            (InteractiveEvent::SelectPlaylistFromSelector, _, _) => true,
            (InteractiveEvent::CancelPlaylistSelector, _, _) => true,
            // Editing mode (highest priority)
            (InteractiveEvent::SaveMetadata, _, EditMode::Title | EditMode::Artist) => true,
            (InteractiveEvent::CancelEdit, _, EditMode::Title | EditMode::Artist) => true,
            (InteractiveEvent::CancelEdit, _, EditMode::None) => !self.search_query.is_empty(),
            (InteractiveEvent::Backspace, _, EditMode::Title | EditMode::Artist) => true,
            (InteractiveEvent::Input(_), _, EditMode::Title | EditMode::Artist) => true,
            // Metadata editor (not editing)
            (InteractiveEvent::EditTitle, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::EditArtist, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::ApplySuggestion, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::ResetToOriginal, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::BulkApplySuggestions, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::ClearMetadata, AppTab::MetadataEditor, EditMode::None) => true,
            // Playlists (not editing)
            (InteractiveEvent::LoadPlaylist, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::TogglePlaylistExpansion, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::DeletePlaylist, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::AddToPlaylist, AppTab::Library, EditMode::None) => true,
            // Repeat: 'r' in Library, Shift+R everywhere except MetadataEditor
            (InteractiveEvent::ToggleRepeat, AppTab::Library, EditMode::None) => true,
            (InteractiveEvent::ToggleRepeat, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::ToggleRepeat, AppTab::Settings, EditMode::None) => true,
            (InteractiveEvent::ToggleRepeat, AppTab::MetadataEditor, EditMode::None) => false,
            // Playback controls (not editing)
            (InteractiveEvent::TogglePlayPause, _, EditMode::None) => true,
            (InteractiveEvent::Play, _, EditMode::None) => true,
            (InteractiveEvent::NextTrack, _, EditMode::None) => true,
            (InteractiveEvent::PreviousTrack, _, EditMode::None) => true,
            (InteractiveEvent::ToggleShuffle, _, EditMode::None) => true,
            (InteractiveEvent::ToggleAutoplay, _, EditMode::None) => true,
            (InteractiveEvent::VolumeUp, _, EditMode::None) => true,
            (InteractiveEvent::VolumeDown, _, EditMode::None) => true,
            // Queue management (not editing)
            (InteractiveEvent::QueuePlayNext, _, EditMode::None) => true,
            (InteractiveEvent::QueueAddToEnd, _, EditMode::None) => true,
            (InteractiveEvent::QueueClear, _, EditMode::None) => true,
            (InteractiveEvent::ToggleQueue, _, _) => true,
            (InteractiveEvent::QueueRemove, _, EditMode::None) => true,
            (InteractiveEvent::LoadPlaylistToQueue, _, EditMode::None) => true,
            (InteractiveEvent::ConfirmQueueReplace, _, EditMode::None) => true,
            (InteractiveEvent::CancelQueueReplace, _, EditMode::None) => true,
            // Favorites (not editing)
            (InteractiveEvent::ToggleFavorite, _, EditMode::None) => true,
            // Recovery — always works
            (InteractiveEvent::ForceRedraw, _, _) => true,
            _ => false,
        };

        if !should_process {
            return Ok(());
        }

        // ── Queue overlay navigation intercept ────────────────────────────
        if self.queue_visible {
            match &event {
                InteractiveEvent::Up => {
                    if let Some(selected) = self.queue_list_state.selected() {
                        if selected > 0 {
                            self.queue_list_state.select(Some(selected - 1));
                        }
                    }
                    return Ok(());
                }
                InteractiveEvent::Down => {
                    if let Some(selected) = self.queue_list_state.selected() {
                        let max = self.queue.len().saturating_sub(1);
                        if selected < max {
                            self.queue_list_state.select(Some(selected + 1));
                        }
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        // ── Main dispatch ──────────────────────────────────────────────────
        match event {
            InteractiveEvent::Quit => {
                self.should_quit = true;
                self.event_handler
                    .quit_flag()
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            InteractiveEvent::Up => self.move_selection(-1),
            InteractiveEvent::Down => self.move_selection(1),

            InteractiveEvent::Play => {
                if let Some((playlist_id, track_idx_in_playlist)) =
                    self.get_playlist_selection_context()
                {
                    debug!(
                        "🎵 Playlist context detected: playlist={}, track_idx={}",
                        playlist_id, track_idx_in_playlist
                    );
                    if let Some(playlist) = self.playlist_manager.get_playlist(&playlist_id) {
                        let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                        if let Some(&actual_track_idx) = valid_tracks.get(track_idx_in_playlist) {
                            self.play_track(actual_track_idx).await?;
                        }
                    }
                } else if let Some(selected) = self.list_state.selected() {
                    if selected < self.filtered_tracks.len() {
                        let track_idx = self.filtered_tracks[selected];
                        self.play_track(track_idx).await?;
                    }
                }
            }

            InteractiveEvent::TogglePlayPause => {
                if self.is_playing {
                    tokio::task::block_in_place(|| self.audio_player.pause())?;
                    self.is_playing = false;
                    self.set_status("⏸️ Paused");
                } else if self.current_track_index.is_some() {
                    tokio::task::block_in_place(|| self.audio_player.resume())?;
                    self.is_playing = true;
                    self.set_status("▶️ Resumed");
                } else {
                    if let Some((playlist_id, track_idx_in_playlist)) =
                        self.get_playlist_selection_context()
                    {
                        if let Some(playlist) = self.playlist_manager.get_playlist(&playlist_id) {
                            let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                            if let Some(&actual_track_idx) = valid_tracks.get(track_idx_in_playlist) {
                                self.play_track(actual_track_idx).await?;
                            }
                        }
                    } else if let Some(selected) = self.list_state.selected() {
                        if selected < self.filtered_tracks.len() {
                            let track_idx = self.filtered_tracks[selected];
                            self.play_track(track_idx).await?;
                        }
                    }
                }
            }

            InteractiveEvent::NextTrack => self.next_track().await?,
            InteractiveEvent::PreviousTrack => self.previous_track().await?,

            InteractiveEvent::VolumeUp => {
                self.volume = (self.volume + 0.1).min(1.0);
                self.audio_player.set_volume(self.volume)?;
                self.set_status(&format!("🔊 Volume: {}%", (self.volume * 100.0) as u32));
            }
            InteractiveEvent::VolumeDown => {
                self.volume = (self.volume - 0.1).max(0.0);
                self.audio_player.set_volume(self.volume)?;
                self.set_status(&format!("🔉 Volume: {}%", (self.volume * 100.0) as u32));
            }

            InteractiveEvent::ToggleRepeat => {
                self.repeat_mode = match self.repeat_mode {
                    RepeatMode::Off => RepeatMode::All,
                    RepeatMode::All => RepeatMode::One,
                    RepeatMode::One => RepeatMode::Off,
                };
                let mode_str = match self.repeat_mode {
                    RepeatMode::Off => "🔁 Repeat: Off",
                    RepeatMode::All => "🔁 Repeat: All",
                    RepeatMode::One => "🔂 Repeat: One",
                };
                self.set_status(mode_str);
            }

            InteractiveEvent::ToggleShuffle => {
                self.is_shuffled = !self.is_shuffled;
                self.set_status(if self.is_shuffled {
                    "🔀 Shuffle: On"
                } else {
                    "🔀 Shuffle: Off"
                });
            }

            InteractiveEvent::ToggleAutoplay => {
                self.autoplay = !self.autoplay;
                self.set_status(if self.autoplay {
                    "🔄 Autoplay: On - tracks will advance automatically"
                } else {
                    "⏸️ Autoplay: Off - tracks will stop after finishing"
                });
            }

            // ── Queue ──────────────────────────────────────────────────────
            InteractiveEvent::QueuePlayNext => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.filtered_tracks.len() {
                        let track_idx = self.filtered_tracks[selected];
                        self.queue.push_front(track_idx);
                        let title = self.tracks[track_idx].display_title();
                        self.set_status(&format!("⏭ Playing next: {}", title));
                    }
                }
            }
            InteractiveEvent::QueueAddToEnd => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.filtered_tracks.len() {
                        let track_idx = self.filtered_tracks[selected];
                        self.queue.push_back(track_idx);
                        let title = self.tracks[track_idx].display_title();
                        self.set_status(&format!(
                            "➕ Added to queue: {} ({} in queue)",
                            title,
                            self.queue.len()
                        ));
                    }
                }
            }
            InteractiveEvent::QueueClear => {
                let count = self.queue.len();
                self.queue.clear();
                self.set_status(&format!("🗑 Queue cleared ({} tracks removed)", count));
            }
            InteractiveEvent::ToggleQueue => {
                self.queue_visible = !self.queue_visible;
                if self.queue_visible && !self.queue.is_empty() {
                    self.queue_list_state.select(Some(0));
                }
                self.set_status(if self.queue_visible {
                    "📋 Queue open"
                } else {
                    "📋 Queue closed"
                });
            }
            InteractiveEvent::QueueRemove => {
                if self.queue_visible {
                    if let Some(selected) = self.queue_list_state.selected() {
                        let mut items: Vec<usize> = self.queue.drain(..).collect();
                        if selected < items.len() {
                            let title = self.tracks[items[selected]].display_title();
                            items.remove(selected);
                            self.queue = items.into_iter().collect();
                            let new_len = self.queue.len();
                            if new_len == 0 {
                                self.queue_list_state.select(None);
                                self.queue_visible = false;
                            } else {
                                self.queue_list_state.select(Some(selected.min(new_len - 1)));
                            }
                            self.set_status(&format!("🗑 Removed from queue: {}", title));
                        }
                    }
                }
            }

            InteractiveEvent::ToggleFavorite => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.filtered_tracks.len() {
                        let track_idx = self.filtered_tracks[selected];
                        let track_id = self.tracks[track_idx].id;
                        match self.behavior_tracker.toggle_favorite(track_id).await {
                            Ok(true) => {
                                self.favorites.insert(track_id);
                                self.set_status(&format!(
                                    "⭐ Favorited: {}",
                                    self.tracks[track_idx].display_title()
                                ));
                            }
                            Ok(false) => {
                                self.favorites.remove(&track_id);
                                self.set_status(&format!(
                                    "☆ Removed favorite: {}",
                                    self.tracks[track_idx].display_title()
                                ));
                            }
                            Err(e) => {
                                self.set_status(&format!("❌ Failed to toggle favorite: {}", e));
                            }
                        }
                    }
                }
            }

            InteractiveEvent::LoadPlaylistToQueue => {
                if self.current_tab == AppTab::Playlists {
                    if let Some(selected) = self.playlist_list_state.selected() {
                        let playlists = self.playlist_manager.list_playlists();
                        if let Some(playlist) = playlists.get(selected) {
                            let playlist_id = playlist.id.clone();
                            let playlist_name = playlist.name.clone();
                            let playlist_track_count = playlist.track_count;
                            drop(playlists);

                            if self.queue.is_empty() {
                                self.load_playlist_to_queue(&playlist_id);
                            } else {
                                self.queue_replace_confirmation = true;
                                self.queue_replace_playlist_id = Some(playlist_id);
                                self.set_status(&format!(
                                    "⚠️ Replace queue ({} tracks) with '{}' ({} tracks)? Y/N",
                                    self.queue.len(),
                                    playlist_name,
                                    playlist_track_count
                                ));
                            }
                        }
                    }
                }
            }
            InteractiveEvent::ConfirmQueueReplace => {
                if self.queue_replace_confirmation {
                    if let Some(playlist_id) = self.queue_replace_playlist_id.take() {
                        self.load_playlist_to_queue(&playlist_id);
                    }
                    self.queue_replace_confirmation = false;
                }
            }
            InteractiveEvent::CancelQueueReplace => {
                self.queue_replace_confirmation = false;
                self.queue_replace_playlist_id = None;
                self.set_status("❌ Queue replacement cancelled");
            }

            InteractiveEvent::ForceRedraw => {
                if let Err(e) = self.terminal.clear() {
                    debug!("⚠️ Force redraw clear failed: {}", e);
                }
                self.set_status("🔄 Display refreshed");
            }

            InteractiveEvent::Tick => {
                self.update_playback_status().await?;
            }

            // ── Tab switches ───────────────────────────────────────────────
            InteractiveEvent::SwitchToLibrary => {
                self.current_tab = AppTab::Library;
                self.set_status("📚 Library Tab");
            }
            InteractiveEvent::SwitchToPlaylists => {
                self.current_tab = AppTab::Playlists;
                self.set_status("🎵 Playlists Tab");
            }
            InteractiveEvent::SwitchToMetadataEditor => {
                self.current_tab = AppTab::MetadataEditor;
                self.set_status("🏷️ Metadata Editor Tab");
            }
            InteractiveEvent::SwitchToSettings => {
                self.current_tab = AppTab::Settings;
                self.set_status("⚙️ Settings Tab");
            }

            // ── Metadata editor ────────────────────────────────────────────
            InteractiveEvent::EditTitle => {
                if self.current_tab == AppTab::MetadataEditor {
                    if let Some(selected) = self.metadata_list_state.selected() {
                        if selected < self.tracks.len() {
                            self.editing_track_index = Some(selected);
                            self.edit_mode = EditMode::Title;
                            self.edit_title = self.tracks[selected].display_title();
                            self.set_status("✏️ Editing title - Press Enter to save, Esc to cancel");
                        }
                    }
                }
            }
            InteractiveEvent::EditArtist => {
                if self.current_tab == AppTab::MetadataEditor {
                    if let Some(selected) = self.metadata_list_state.selected() {
                        if selected < self.tracks.len() {
                            self.editing_track_index = Some(selected);
                            self.edit_mode = EditMode::Artist;
                            self.edit_artist = self.tracks[selected].display_artist();
                            self.set_status("✏️ Editing artist - Press Enter to save, Esc to cancel");
                        }
                    }
                }
            }
            InteractiveEvent::SaveMetadata => {
                if self.edit_mode != EditMode::None {
                    self.save_current_edit().await?;
                }
            }
            InteractiveEvent::CancelEdit => {
                if self.queue_visible {
                    self.queue_visible = false;
                    self.set_status("📋 Queue closed");
                    return Ok(());
                }
                if !self.search_query.is_empty() && !self.search_mode {
                    self.search_query.clear();
                    self.reset_to_full_library();
                    self.set_status("🔍 Search cleared");
                    return Ok(());
                }
                self.edit_mode = EditMode::None;
                self.editing_track_index = None;
                self.edit_title.clear();
                self.edit_artist.clear();
                self.set_status("❌ Edit cancelled");
            }
            InteractiveEvent::ApplySuggestion => {
                if self.current_tab == AppTab::MetadataEditor {
                    if let Some(selected) = self.metadata_list_state.selected() {
                        if selected < self.tracks.len() {
                            self.apply_filename_suggestion(selected).await?;
                        }
                    }
                }
            }
            InteractiveEvent::ResetToOriginal => {
                if self.current_tab == AppTab::MetadataEditor {
                    if let Some(selected) = self.metadata_list_state.selected() {
                        if selected < self.tracks.len() {
                            self.reset_track_metadata(selected).await?;
                        }
                    }
                }
            }
            InteractiveEvent::BulkApplySuggestions => {
                if self.current_tab == AppTab::MetadataEditor {
                    self.bulk_apply_suggestions().await?;
                }
            }
            InteractiveEvent::ClearMetadata => {
                if self.current_tab == AppTab::MetadataEditor {
                    if let Some(selected) = self.metadata_list_state.selected() {
                        if selected < self.tracks.len() {
                            self.clear_track_metadata(selected).await?;
                        }
                    }
                }
            }

            // ── Text input (edit fields) ───────────────────────────────────
            InteractiveEvent::Input(c) => match self.edit_mode {
                EditMode::Title => self.edit_title.push(c),
                EditMode::Artist => self.edit_artist.push(c),
                EditMode::None => {}
            },
            InteractiveEvent::Backspace => match self.edit_mode {
                EditMode::Title => { self.edit_title.pop(); }
                EditMode::Artist => { self.edit_artist.pop(); }
                EditMode::None => {}
            },

            InteractiveEvent::ShowHelp => {
                self.show_help = !self.show_help;
                self.set_status("❓ Help overlay toggled");
            }

            // ── Search ─────────────────────────────────────────────────────
            InteractiveEvent::EnterSearch => {
                self.search_mode = true;
                self.search_query.clear();
                self.update_search_results();
                debug!("🔍 Search mode activated");
                self.set_status("🔍 Search mode - type to search, Esc to exit");
            }
            InteractiveEvent::ExitSearch => {
                self.search_mode = false;
                self.search_query.clear();
                self.reset_to_full_library();
                debug!("🔍 Search mode exited");
                self.set_status("🔍 Search cleared");
            }
            InteractiveEvent::ConfirmSearch => {
                self.search_mode = false;
                let count = self.filtered_tracks.len();
                debug!("🔍 Search confirmed: {} results for '{}'", count, self.search_query);
                if count == 0 {
                    self.set_status("🔍 No results — press / to search again, Esc to clear");
                } else {
                    self.set_status(&format!(
                        "🔍 {} result{} for '{}' — Esc to clear, / to refine",
                        count,
                        if count == 1 { "" } else { "s" },
                        self.search_query
                    ));
                }
            }
            InteractiveEvent::SearchInput(c) => {
                debug!("🔍 Search input: '{}' (char code: {})", c, c as u32);
                self.search_query.push(c);
                debug!(
                    "🔍 Search query now: '{}' (len={})",
                    self.search_query, self.search_query.len()
                );
                self.update_search_results();
                self.set_status(&format!(
                    "🔍 Searching: '{}' ({} results)",
                    self.search_query,
                    self.filtered_tracks.len()
                ));
            }
            InteractiveEvent::SearchBackspace => {
                self.search_query.pop();
                self.update_search_results();
                if self.search_query.is_empty() {
                    self.set_status("🔍 Search mode - type to search, Esc to exit");
                } else {
                    self.set_status(&format!("🔍 Searching: '{}'", self.search_query));
                }
            }

            // ── Playlists ──────────────────────────────────────────────────
            InteractiveEvent::DeletePlaylist => {
                if self.current_tab == AppTab::Playlists {
                    if let Some(selected) = self.playlist_list_state.selected() {
                        let playlists = self.playlist_manager.list_playlists();
                        if let Some(playlist) = playlists.get(selected) {
                            let playlist_id = playlist.id.clone();
                            let playlist_count = playlists.len();
                            drop(playlists);

                            match self.playlist_manager.delete_playlist(&playlist_id) {
                                Ok(deleted) => {
                                    self.set_status("🗑️ Playlist deleted");
                                    tracing::info!("Deleted playlist: {}", playlist_id);
                                    if deleted
                                        && selected >= playlist_count.saturating_sub(1)
                                        && selected > 0
                                    {
                                        self.playlist_list_state.select(Some(selected - 1));
                                    }
                                }
                                Err(e) => {
                                    self.set_status(&format!("❌ Failed to delete playlist: {}", e));
                                    tracing::error!("Failed to delete playlist: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            InteractiveEvent::LoadPlaylist => {
                if self.current_tab == AppTab::Playlists {
                    if let Some(selected) = self.playlist_list_state.selected() {
                        let playlists = self.playlist_manager.list_playlists();
                        if let Some(playlist) = playlists.get(selected) {
                            let playlist_id = playlist.id.clone();
                            let playlist_name = playlist.name.clone();
                            let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                            self.current_playlist_id = Some(playlist_id);
                            self.filtered_tracks = valid_tracks;
                            if !self.filtered_tracks.is_empty() {
                                self.list_state.select(Some(0));
                            }
                            self.set_status(&format!("🎵 Loaded playlist: {}", playlist_name));
                            tracing::info!(
                                "Loaded playlist: {} ({} tracks)",
                                playlist_name,
                                self.filtered_tracks.len()
                            );
                        }
                    }
                }
            }
            InteractiveEvent::TogglePlaylistExpansion => {
                if self.current_tab == AppTab::Playlists {
                    if let Some(selected) = self.playlist_list_state.selected() {
                        let playlists = self.playlist_manager.list_playlists();
                        if let Some(playlist) = playlists.get(selected) {
                            let playlist_id = playlist.id.clone();
                            let playlist_name = playlist.name.clone();

                            if self.expanded_playlists.contains(&playlist_id) {
                                self.expanded_playlists.clear();
                                self.playlist_track_states.clear();
                                self.set_status(&format!("📁 Collapsed playlist: {}", playlist_name));
                                debug!("🔍 Collapsed playlist: {}", playlist_name);
                            } else {
                                self.expanded_playlists.clear();
                                self.playlist_track_states.clear();
                                self.expanded_playlists.insert(playlist_id.clone());

                                let mut track_state = ratatui::widgets::ListState::default();
                                let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                                if !valid_tracks.is_empty() {
                                    track_state.select(Some(0));
                                }
                                self.playlist_track_states.insert(playlist_id.clone(), track_state);

                                self.set_status(&format!(
                                    "📂 Expanded playlist: {} ({} tracks)",
                                    playlist_name,
                                    valid_tracks.len()
                                ));
                                debug!(
                                    "🔍 Expanded playlist: {} ({} tracks) - all others collapsed",
                                    playlist_name,
                                    valid_tracks.len()
                                );
                            }
                        }
                    }
                }
            }
            InteractiveEvent::AddToPlaylist => {
                if self.current_tab == AppTab::Library {
                    if let Some(selected) = self.list_state.selected() {
                        if selected < self.filtered_tracks.len() {
                            let track_idx = self.filtered_tracks[selected];
                            self.show_playlist_selector = true;
                            self.selected_track_for_playlist = Some(track_idx);
                            let playlists = self.playlist_manager.list_playlists();
                            let total_options = playlists.len() + 1;
                            if total_options > 0 {
                                self.playlist_selector_state.select(Some(0));
                            }
                            let track_title = self.tracks[track_idx].display_title();
                            self.set_status(&format!("📋 Select playlist for '{}'", track_title));
                            debug!("🎵 Showing playlist selector for track: {}", track_title);
                        }
                    }
                }
            }
            InteractiveEvent::PlaylistInput(c) => {
                if self.playlist_creation_mode || self.playlist_rename_mode {
                    self.playlist_name_input.push(c);
                    let prefix = if self.playlist_rename_mode { "✏️" } else { "🎵" };
                    self.set_status(&format!(
                        "{} Playlist name: {}",
                        prefix, self.playlist_name_input
                    ));
                }
            }
            InteractiveEvent::PlaylistBackspace => {
                if self.playlist_creation_mode || self.playlist_rename_mode {
                    self.playlist_name_input.pop();
                    let prefix = if self.playlist_rename_mode { "✏️" } else { "🎵" };
                    self.set_status(&format!(
                        "{} Playlist name: {}",
                        prefix, self.playlist_name_input
                    ));
                }
            }
            InteractiveEvent::ConfirmPlaylistCreation => {
                if self.playlist_rename_mode && !self.playlist_name_input.is_empty() {
                    if let Some(rename_id) = self.playlist_rename_id.take() {
                        match self
                            .playlist_manager
                            .rename_playlist(&rename_id, self.playlist_name_input.clone())
                        {
                            Ok(_) => self.set_status(&format!("✅ Renamed to '{}'", self.playlist_name_input)),
                            Err(e) => self.set_status(&format!("❌ Rename failed: {}", e)),
                        }
                    }
                    self.playlist_rename_mode = false;
                    self.playlist_name_input.clear();
                } else if self.playlist_creation_mode && !self.playlist_name_input.is_empty() {
                    match self
                        .playlist_manager
                        .create_playlist(self.playlist_name_input.clone(), None)
                    {
                        Ok(playlist_id) => {
                            self.set_status(&format!("✅ Created playlist: {}", self.playlist_name_input));
                            tracing::info!(
                                "Created playlist: {} (ID: {})",
                                self.playlist_name_input,
                                playlist_id
                            );
                        }
                        Err(e) => {
                            self.set_status(&format!("❌ Failed to create playlist: {}", e));
                        }
                    }
                    self.playlist_creation_mode = false;
                    self.playlist_name_input.clear();
                }
            }
            InteractiveEvent::CancelPlaylistCreation => {
                self.playlist_creation_mode = false;
                self.playlist_rename_mode = false;
                self.playlist_rename_id = None;
                self.playlist_name_input.clear();
                self.set_status("❌ Cancelled");
            }
            InteractiveEvent::StartPlaylistCreation => {
                if self.current_tab == AppTab::Playlists {
                    self.playlist_creation_mode = true;
                    self.playlist_name_input.clear();
                    self.set_status(
                        "📝 New Playlist — Enter name, then Enter to confirm, Esc to cancel",
                    );
                }
            }
            InteractiveEvent::RenamePlaylist => {
                if self.current_tab == AppTab::Playlists {
                    if let Some(selected) = self.playlist_list_state.selected() {
                        let playlists = self.playlist_manager.list_playlists();
                        if let Some(playlist) = playlists.get(selected) {
                            self.playlist_rename_id = Some(playlist.id.clone());
                            self.playlist_name_input = playlist.name.clone();
                            self.playlist_rename_mode = true;
                            self.set_status(&format!(
                                "✏️ Renaming '{}' — Enter to confirm, Esc to cancel",
                                playlist.name
                            ));
                        }
                    }
                }
            }
            InteractiveEvent::RemoveFromPlaylist => {
                if self.current_tab == AppTab::Playlists {
                    if let Some((playlist_id, track_idx_in_playlist)) =
                        self.get_playlist_selection_context()
                    {
                        if let Some(playlist) = self.playlist_manager.get_playlist(&playlist_id) {
                            let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                            if let Some(&actual_track_idx) = valid_tracks.get(track_idx_in_playlist) {
                                let track_path = self.tracks[actual_track_idx].file_path.clone();
                                let track_title = self.tracks[actual_track_idx].display_title();
                                let _ = playlist; // release immutable borrow

                                match self
                                    .playlist_manager
                                    .remove_track_from_playlist(&playlist_id, &track_path)
                                {
                                    Ok(_) => self.set_status(&format!(
                                        "🗑 Removed '{}' from playlist",
                                        track_title
                                    )),
                                    Err(e) => self.set_status(&format!(
                                        "❌ Failed to remove track: {}",
                                        e
                                    )),
                                }
                            }
                        }
                    }
                }
            }
            InteractiveEvent::SelectPlaylistFromSelector => {
                if self.show_playlist_selector {
                    if let Some(selected) = self.playlist_selector_state.selected() {
                        if let Some(track_idx) = self.selected_track_for_playlist {
                            let playlists = self.playlist_manager.list_playlists();
                            let track_path = self.tracks[track_idx].file_path.clone();
                            let track_title = self.tracks[track_idx].display_title();

                            if selected < playlists.len() {
                                let playlist_id = playlists[selected].id.clone();
                                let playlist_name = playlists[selected].name.clone();
                                drop(playlists);

                                match self
                                    .playlist_manager
                                    .add_track_to_playlist(&playlist_id, &track_path)
                                {
                                    Ok(_) => {
                                        self.set_status(&format!(
                                            "➕ Added '{}' to '{}'",
                                            track_title, playlist_name
                                        ));
                                        debug!(
                                            "🎵 Added track to existing playlist: {}",
                                            playlist_name
                                        );
                                    }
                                    Err(e) => {
                                        self.set_status(&format!("❌ Failed to add track: {}", e));
                                    }
                                }
                            } else {
                                drop(playlists);
                                self.playlist_creation_mode = true;
                                self.playlist_name_input.clear();
                                self.set_status("📝 Enter new playlist name:");
                                debug!("🎵 Starting playlist creation from selector");
                            }

                            self.show_playlist_selector = false;
                            self.selected_track_for_playlist = None;
                        }
                    }
                }
            }
            InteractiveEvent::CancelPlaylistSelector => {
                self.show_playlist_selector = false;
                self.selected_track_for_playlist = None;
                self.set_status("❌ Playlist selection cancelled");
                debug!("🎵 Playlist selector cancelled");
            }
        }

        Ok(())
    }
}
