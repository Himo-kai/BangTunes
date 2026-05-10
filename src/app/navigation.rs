// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors
//
// Selection movement, fuzzy search, and all key-event → InteractiveEvent mappers.

use super::{AppTab, EditMode, InteractiveApp, InteractiveEvent};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::FuzzyMatcher;
use tracing::debug;

impl InteractiveApp {
    // ──────────────────────────────────────────────────────────────────
    // Cursor movement (wraps at boundaries)
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn move_selection(&mut self, delta: i32) {
        // Playlist selector overlay takes highest priority
        if self.show_playlist_selector {
            let playlists = self.playlist_manager.list_playlists();
            let total_options = playlists.len() + 1; // +1 for "Create New Playlist"

            if total_options == 0 {
                return;
            }

            let current = self.playlist_selector_state.selected().unwrap_or(0);
            let new_index = if delta > 0 {
                (current + delta as usize) % total_options
            } else if current == 0 {
                total_options - 1
            } else {
                current.saturating_sub((-delta) as usize)
            };

            self.playlist_selector_state.select(Some(new_index));
            debug!(
                "🔍 Playlist selector navigation: moved from {} to {} (total options: {})",
                current, new_index, total_options
            );
            return;
        }

        match self.current_tab {
            AppTab::Library => {
                if self.filtered_tracks.is_empty() {
                    return;
                }
                let current = self.list_state.selected().unwrap_or(0);
                let new_index = if delta > 0 {
                    (current + delta as usize) % self.filtered_tracks.len()
                } else if current == 0 {
                    self.filtered_tracks.len() - 1
                } else {
                    current.saturating_sub((-delta) as usize)
                };
                self.list_state.select(Some(new_index));
            }
            AppTab::MetadataEditor => {
                if self.tracks.is_empty() {
                    return;
                }
                let current = self.metadata_list_state.selected().unwrap_or(0);
                let new_index = if delta > 0 {
                    (current + delta as usize) % self.tracks.len()
                } else if current == 0 {
                    self.tracks.len() - 1
                } else {
                    current.saturating_sub((-delta) as usize)
                };
                self.metadata_list_state.select(Some(new_index));
            }
            AppTab::Playlists => {
                let playlists = self.playlist_manager.list_playlists();
                if playlists.is_empty() {
                    return;
                }

                let mut total_items = 0;
                for playlist in &playlists {
                    total_items += 1;
                    if self.expanded_playlists.contains(&playlist.id) {
                        let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                        total_items += valid_tracks.len();
                    }
                }

                if total_items == 0 {
                    return;
                }

                let current = self.playlist_list_state.selected().unwrap_or(0);
                let new_index = if delta > 0 {
                    (current + delta as usize) % total_items
                } else if current == 0 {
                    total_items - 1
                } else {
                    current.saturating_sub((-delta) as usize)
                };

                self.playlist_list_state.select(Some(new_index));
                debug!(
                    "🔍 Tree navigation: moved from {} to {} (total items: {})",
                    current, new_index, total_items
                );
            }
            AppTab::Settings => {
                // Settings has no navigable list
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────
    // Fuzzy search
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn update_search_results(&mut self) {
        if self.search_query.is_empty() {
            debug!("🔍 Empty search query, showing all {} tracks", self.tracks.len());
            self.filtered_tracks = (0..self.tracks.len()).collect();
        } else {
            debug!("🔍 Fuzzy searching for: '{}'", self.search_query);

            // fuzzy_match(choice, pattern): choice = text to search IN, pattern = query.
            // Arg order matters: reversed args cause zero results when query < title length.
            let mut scored_results: Vec<(usize, i64)> = Vec::new();
            let mut match_count = 0;

            for (idx, track) in self.tracks.iter().enumerate() {
                let mut best_score = 0i64;
                let mut match_field = "none";

                if let Some(title) = &track.metadata.title {
                    if let Some(score) = self.fuzzy_matcher.fuzzy_match(title, &self.search_query) {
                        if score > best_score {
                            best_score = score;
                            match_field = "title";
                        }
                    }
                }

                let display_title = track.display_title();
                if let Some(score) = self.fuzzy_matcher.fuzzy_match(&display_title, &self.search_query) {
                    if score > best_score {
                        best_score = score;
                        match_field = "display_title";
                    }
                }

                if let Some(artist) = &track.metadata.artist {
                    if let Some(score) = self.fuzzy_matcher.fuzzy_match(artist, &self.search_query) {
                        if score > best_score {
                            best_score = score;
                            match_field = "artist";
                        }
                    }
                }

                if let Some(filename) = track.file_path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    if let Some(score) = self.fuzzy_matcher.fuzzy_match(&filename_str, &self.search_query) {
                        if score > best_score {
                            best_score = score;
                            match_field = "filename";
                        }
                    }
                }

                if idx < 3 {
                    debug!(
                        "🔍 Track {}: '{}' -> score {} (via {})",
                        idx, track.display_title(), best_score, match_field
                    );
                }

                if best_score > 0 {
                    scored_results.push((idx, best_score));
                    match_count += 1;
                }
            }

            scored_results.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_tracks = scored_results.into_iter().map(|(idx, _)| idx).collect();

            debug!(
                "🔍 Search complete: {} matches found out of {} tracks",
                match_count, self.tracks.len()
            );
            if !self.filtered_tracks.is_empty() {
                let top_track = &self.tracks[self.filtered_tracks[0]];
                debug!("🔍 Top match: '{}'", top_track.display_title());
            }
        }

        if !self.filtered_tracks.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    pub(super) fn reset_to_full_library(&mut self) {
        self.filtered_tracks = (0..self.tracks.len()).collect();
        if !self.filtered_tracks.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    // ──────────────────────────────────────────────────────────────────
    // Key → event mappers
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn key_to_search_event(key: KeyEvent) -> Option<InteractiveEvent> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => Some(InteractiveEvent::ExitSearch),
            (KeyCode::Enter, _) => Some(InteractiveEvent::ConfirmSearch),
            (KeyCode::Backspace, _) => Some(InteractiveEvent::SearchBackspace),
            (KeyCode::Char(c), KeyModifiers::NONE) if !c.is_control() => {
                Some(InteractiveEvent::SearchInput(c))
            }
            (KeyCode::Up, _) => Some(InteractiveEvent::Up),
            (KeyCode::Down, _) => Some(InteractiveEvent::Down),
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => Some(InteractiveEvent::ForceRedraw),
            _ => None,
        }
    }

    pub(super) fn key_to_playlist_event(key: KeyEvent) -> Option<InteractiveEvent> {
        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => Some(InteractiveEvent::ConfirmPlaylistCreation),
            (KeyCode::Esc, _) => Some(InteractiveEvent::CancelPlaylistCreation),
            (KeyCode::Backspace, _) => Some(InteractiveEvent::PlaylistBackspace),
            (KeyCode::Char(c), KeyModifiers::NONE) if !c.is_control() => {
                Some(InteractiveEvent::PlaylistInput(c))
            }
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => Some(InteractiveEvent::ForceRedraw),
            _ => None,
        }
    }

    pub(super) fn key_to_playlist_selector_event(key: KeyEvent) -> Option<InteractiveEvent> {
        match (key.code, key.modifiers) {
            (KeyCode::Up, _) => Some(InteractiveEvent::Up),
            (KeyCode::Down, _) => Some(InteractiveEvent::Down),
            (KeyCode::Enter, _) => Some(InteractiveEvent::SelectPlaylistFromSelector),
            (KeyCode::Esc, _) => Some(InteractiveEvent::CancelPlaylistSelector),
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => Some(InteractiveEvent::ForceRedraw),
            _ => None,
        }
    }

    pub(super) fn key_to_app_event_basic(&self, key: KeyEvent) -> Option<InteractiveEvent> {
        match (key.code, key.modifiers) {
            // Ctrl shortcuts
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(InteractiveEvent::SaveMetadata),
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => Some(InteractiveEvent::ResetToOriginal),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => Some(InteractiveEvent::ForceRedraw),

            // Regular keys
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('1'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToLibrary),
            (KeyCode::Char('2'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToPlaylists),
            (KeyCode::Char('3'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToMetadataEditor),
            (KeyCode::Char('4'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToSettings),
            (KeyCode::Char(' '), KeyModifiers::NONE) => Some(InteractiveEvent::TogglePlayPause),
            (KeyCode::Char('n'), KeyModifiers::NONE) => match self.current_tab {
                AppTab::MetadataEditor => None,
                _ => Some(InteractiveEvent::NextTrack),
            },
            (KeyCode::Char('p'), KeyModifiers::NONE) => Some(InteractiveEvent::PreviousTrack),
            (KeyCode::Char('s'), KeyModifiers::NONE) => Some(InteractiveEvent::ToggleShuffle),
            (KeyCode::Char('+'), KeyModifiers::NONE) | (KeyCode::Char('='), KeyModifiers::NONE) => {
                Some(InteractiveEvent::VolumeUp)
            }
            (KeyCode::Char('-'), KeyModifiers::NONE) => Some(InteractiveEvent::VolumeDown),
            (KeyCode::Char('z'), KeyModifiers::NONE) => Some(InteractiveEvent::ToggleShuffle),
            (KeyCode::Char('A'), KeyModifiers::NONE) => Some(InteractiveEvent::ToggleAutoplay),
            // Queue management
            (KeyCode::Char('e'), KeyModifiers::NONE) => Some(InteractiveEvent::QueuePlayNext),
            (KeyCode::Char('E'), KeyModifiers::SHIFT) => Some(InteractiveEvent::QueueAddToEnd),
            (KeyCode::Char('C'), KeyModifiers::SHIFT) => Some(InteractiveEvent::QueueClear),
            (KeyCode::Char('Q'), KeyModifiers::SHIFT) => match self.current_tab {
                AppTab::Playlists => Some(InteractiveEvent::LoadPlaylistToQueue),
                _ => Some(InteractiveEvent::ToggleQueue),
            },
            (KeyCode::Char('N'), KeyModifiers::SHIFT) => match self.current_tab {
                AppTab::Playlists => Some(InteractiveEvent::StartPlaylistCreation),
                _ => None,
            },
            // Favorites
            (KeyCode::Char('f'), KeyModifiers::NONE) => Some(InteractiveEvent::ToggleFavorite),

            (KeyCode::Up, _) => Some(InteractiveEvent::Up),
            (KeyCode::Down, _) => Some(InteractiveEvent::Down),
            (KeyCode::Esc, _) => Some(InteractiveEvent::CancelEdit),
            (KeyCode::Backspace, _) => Some(InteractiveEvent::Backspace),

            // Context-sensitive
            (KeyCode::Char('c'), KeyModifiers::NONE) => match self.current_tab {
                AppTab::MetadataEditor => Some(InteractiveEvent::ClearMetadata),
                _ => None,
            },
            (KeyCode::Char('a'), KeyModifiers::NONE) => match self.current_tab {
                AppTab::Library => Some(InteractiveEvent::AddToPlaylist),
                AppTab::MetadataEditor => Some(InteractiveEvent::EditArtist),
                _ => None,
            },
            (KeyCode::Char('l'), KeyModifiers::NONE) => match self.current_tab {
                AppTab::Playlists => Some(InteractiveEvent::LoadPlaylist),
                _ => None,
            },
            (KeyCode::Char('r'), KeyModifiers::NONE) => match self.current_tab {
                AppTab::Playlists => Some(InteractiveEvent::RenamePlaylist),
                AppTab::Library => Some(InteractiveEvent::ToggleRepeat),
                _ => None,
            },
            (KeyCode::Char('R'), KeyModifiers::SHIFT) => Some(InteractiveEvent::ToggleRepeat),
            (KeyCode::Char('x'), KeyModifiers::NONE) => {
                if self.queue_visible {
                    Some(InteractiveEvent::QueueRemove)
                } else {
                    match self.current_tab {
                        AppTab::Playlists => Some(InteractiveEvent::RemoveFromPlaylist),
                        _ => None,
                    }
                }
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                match (self.current_tab, self.edit_mode) {
                    (AppTab::Playlists, EditMode::None) => {
                        Some(InteractiveEvent::TogglePlaylistExpansion)
                    }
                    (AppTab::MetadataEditor, EditMode::Title | EditMode::Artist) => {
                        Some(InteractiveEvent::SaveMetadata)
                    }
                    (AppTab::MetadataEditor, EditMode::None) => None,
                    (_, EditMode::None) => Some(InteractiveEvent::Play),
                    _ => None,
                }
            }
            (KeyCode::Char('t'), KeyModifiers::NONE) => {
                if self.current_tab == AppTab::MetadataEditor {
                    Some(InteractiveEvent::EditTitle)
                } else {
                    None
                }
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                if self.current_tab == AppTab::MetadataEditor {
                    Some(InteractiveEvent::ApplySuggestion)
                } else {
                    None
                }
            }
            (KeyCode::Char('b'), KeyModifiers::NONE) => {
                if self.current_tab == AppTab::MetadataEditor {
                    Some(InteractiveEvent::BulkApplySuggestions)
                } else {
                    None
                }
            }
            (KeyCode::Delete, KeyModifiers::NONE) => {
                if self.current_tab == AppTab::Playlists {
                    Some(InteractiveEvent::DeletePlaylist)
                } else {
                    None
                }
            }
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(InteractiveEvent::EnterSearch),
            (KeyCode::Char('?'), KeyModifiers::NONE) | (KeyCode::Char('?'), KeyModifiers::SHIFT) => {
                Some(InteractiveEvent::ShowHelp)
            }
            (KeyCode::Char(c), KeyModifiers::NONE) if !c.is_control() && c != '?' => {
                Some(InteractiveEvent::Input(c))
            }
            _ => None,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

    /// Bug C regression: fuzzy_match(choice, pattern) — choice = text to search IN.
    /// When args are reversed, short queries fail against long titles.
    #[test]
    fn fuzzy_search_correct_arg_order_finds_track_by_substring() {
        let matcher = SkimMatcherV2::default();
        let title = "Bring Me The Horizon - Parasite Eve";
        let query = "parasite eve";

        assert!(
            matcher.fuzzy_match(title, query).is_some(),
            "fuzzy_match(title, query) must find a match"
        );
        assert!(
            matcher.fuzzy_match(query, title).is_none(),
            "fuzzy_match(query, title) should NOT match when query is shorter than title pattern"
        );
    }

    #[test]
    fn fuzzy_search_short_query_against_exact_title_matches() {
        let matcher = SkimMatcherV2::default();
        let title = "Parasite Eve";
        let query = "parasite eve";
        assert!(
            matcher.fuzzy_match(title, query).is_some(),
            "exact case-insensitive match must always succeed"
        );
    }
}
