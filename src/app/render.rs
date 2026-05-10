// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors
//
// All TUI rendering functions.  Every function is a static associated
// function on InteractiveApp (no `self`) so the master render() in mod.rs
// can call them as Self::render_*().

use super::{AppTab, EditMode, InteractiveApp, RepeatMode};
use crate::{
    audio::{metadata_parser::MetadataParser, playlist::PlaylistManager},
    Track,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
use uuid::Uuid;

impl InteractiveApp {
    // ──────────────────────────────────────────────────────────────────
    // Header / tabs
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn render_header_with_tabs(f: &mut Frame, area: Rect, current_tab: &AppTab) {
        let tab_titles = vec![
            match current_tab {
                AppTab::Library => Span::styled("1. 📚 Library", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                _ => Span::styled("1. 📚 Library", Style::default().fg(Color::Gray)),
            },
            Span::raw(" | "),
            match current_tab {
                AppTab::Playlists => Span::styled("2. 🎵 Playlists", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                _ => Span::styled("2. 🎵 Playlists", Style::default().fg(Color::Gray)),
            },
            Span::raw(" | "),
            match current_tab {
                AppTab::MetadataEditor => Span::styled("3. 🏷️ Metadata Editor", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                _ => Span::styled("3. 🏷️ Metadata Editor", Style::default().fg(Color::Gray)),
            },
            Span::raw(" | "),
            match current_tab {
                AppTab::Settings => Span::styled("4. ⚙️ Settings", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                _ => Span::styled("4. ⚙️ Settings", Style::default().fg(Color::Gray)),
            },
        ];

        let header = Paragraph::new(Line::from(tab_titles))
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title("🎵 BangTunes"));
        f.render_widget(header, area);
    }

    // ──────────────────────────────────────────────────────────────────
    // Library track list
    // ──────────────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_track_list(
        f: &mut Frame,
        area: Rect,
        tracks: &[Track],
        filtered_tracks: &[usize],
        current_track_index: Option<usize>,
        is_playing: bool,
        list_state: &mut ListState,
        favorites: &HashSet<Uuid>,
        scan_skipped_count: usize,
    ) {
        let items: Vec<ListItem> = filtered_tracks
            .iter()
            .map(|&track_idx| {
                let track = &tracks[track_idx];
                let is_current = current_track_index == Some(track_idx);

                let style = if is_current {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let prefix = if is_current && is_playing {
                    "▶ "
                } else if is_current {
                    "⏸ "
                } else {
                    "  "
                };

                let star = if favorites.contains(&track.id) { "⭐ " } else { "   " };

                let content = format!(
                    "{}{}{} - {} - {}",
                    prefix,
                    star,
                    track.display_artist(),
                    track.display_title(),
                    track.display_album()
                );

                ListItem::new(content).style(style)
            })
            .collect();

        let library_title = if scan_skipped_count > 0 {
            format!(
                "Library ({} tracks | ⚠️ {} failed to load — run --dev for details)",
                filtered_tracks.len(),
                scan_skipped_count
            )
        } else {
            format!("Library ({} tracks)", filtered_tracks.len())
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(library_title))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("→ ");

        f.render_stateful_widget(list, area, list_state);
    }

    // ──────────────────────────────────────────────────────────────────
    // Player controls bar
    // ──────────────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_player_controls(
        f: &mut Frame,
        area: Rect,
        tracks: &[Track],
        current_track_index: Option<usize>,
        is_playing: bool,
        volume: f32,
        repeat_mode: RepeatMode,
        is_shuffled: bool,
        current_position: Duration,
        total_duration: Option<Duration>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Progress bar
                Constraint::Min(2),    // Controls
            ])
            .split(area);

        // Progress bar with time tracking
        let (progress_ratio, time_display) = if let Some(total) = total_duration {
            let total_secs = total.as_secs();
            let current_secs = current_position.as_secs().min(total_secs);
            let ratio = if total_secs > 0 { current_secs as f64 / total_secs as f64 } else { 0.0 };
            let current_time = format!("{}:{:02}", current_secs / 60, current_secs % 60);
            let total_time = format!("{}:{:02}", total_secs / 60, total_secs % 60);
            (ratio, format!("{} / {}", current_time, total_time))
        } else {
            let current_secs = current_position.as_secs();
            let current_time = format!("{}:{:02}", current_secs / 60, current_secs % 60);
            (0.0, format!("{} / --:--", current_time))
        };

        let progress_color = if is_playing { Color::Green } else { Color::Yellow };

        let progress_bar = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(Style::default().fg(progress_color).add_modifier(Modifier::BOLD))
            .ratio(progress_ratio)
            .label(time_display);
        f.render_widget(progress_bar, chunks[0]);

        let current_track_info = if let Some(idx) = current_track_index {
            let track = &tracks[idx];
            format!("♪ {} - {}", track.display_artist(), track.display_title())
        } else {
            "No track selected".to_string()
        };

        let status_symbol = if is_playing { "▶" } else { "⏸" };
        let status_text = if is_playing { "Playing" } else { "Paused" };
        let status_color = if is_playing { Color::Green } else { Color::Yellow };

        let volume_bar = "█".repeat((volume * 10.0) as usize);
        let volume_empty = "░".repeat(10 - (volume * 10.0) as usize);

        let repeat_symbol = match repeat_mode {
            RepeatMode::Off => "⭘",
            RepeatMode::All => "🔁",
            RepeatMode::One => "🔂",
        };
        let shuffle_symbol = if is_shuffled { "🔀" } else { "➡" };

        let controls_text = vec![
            Line::from(vec![
                Span::styled(current_track_info, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(status_symbol, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(status_text, Style::default().fg(status_color)),
                Span::raw(" | "),
                Span::styled("Vol: ", Style::default().fg(Color::Gray)),
                Span::styled(volume_bar, Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                Span::styled(volume_empty, Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {}%", (volume * 100.0) as u32)),
                Span::raw(" | "),
                Span::styled(repeat_symbol, Style::default().fg(Color::Magenta)),
                Span::raw(" "),
                Span::styled(shuffle_symbol, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Controls: ", Style::default().fg(Color::Gray)),
                Span::styled("Space", Style::default().fg(Color::Yellow)),
                Span::raw("=Play/Pause "),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::raw("=Next "),
                Span::styled("p", Style::default().fg(Color::Yellow)),
                Span::raw("=Prev "),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw("=Quit"),
            ]),
        ];

        let controls = Paragraph::new(controls_text)
            .block(Block::default().borders(Borders::ALL).title("Player"))
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[1]);
    }

    // ──────────────────────────────────────────────────────────────────
    // Status bar
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn render_status_bar(
        f: &mut Frame,
        area: Rect,
        status_message: Option<(String, Instant)>,
        queue_len: usize,
    ) {
        let mut status_text = if let Some((message, timestamp)) = status_message {
            if timestamp.elapsed() < Duration::from_secs(3) {
                message
            } else {
                "Ready".to_string()
            }
        } else {
            "Ready".to_string()
        };

        if queue_len > 0 {
            status_text.push_str(&format!(" | 📋 Queue: {}", queue_len));
        }

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, area);
    }

    // ──────────────────────────────────────────────────────────────────
    // Settings / help pages
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn render_settings(f: &mut Frame, area: Rect) {
        let settings_content = vec![
            Line::from(vec![Span::styled("⚙️ Settings & Help", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("📑 Tabs:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  1             Library (browse all tracks)"),
            Line::from("  2             Playlists (create, manage, expand)"),
            Line::from("  3             Metadata Editor (fix track info)"),
            Line::from("  4             Settings (this page)"),
            Line::from(""),
            Line::from(vec![Span::styled("⌨️ Playback Controls:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Space         Toggle play/pause"),
            Line::from("  p             Previous track"),
            Line::from("  n / →         Next track"),
            Line::from("  b / ←         Previous track"),
            Line::from("  + / =         Volume up"),
            Line::from("  -             Volume down"),
            Line::from("  s / z         Toggle smart shuffle"),
            Line::from("  r             Toggle repeat mode (Library tab only)"),
            Line::from("  Shift+R       Toggle repeat mode (all tabs — use when in Playlists)"),
            Line::from(""),
            Line::from(vec![Span::styled("🎵 Library Features:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  ↑ / ↓         Navigate tracks"),
            Line::from("  /             Search library"),
            Line::from("  f             Toggle favorite (⭐)"),
            Line::from("  a             Add track to playlist"),
            Line::from("  F5            Refresh library scan"),
            Line::from(""),
            Line::from(vec![Span::styled("📋 Playlist Features:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Enter         Expand/collapse playlist"),
            Line::from("  Shift+N       Create new playlist"),
            Line::from("  r             Rename playlist (Playlists tab)"),
            Line::from("  Shift+Q       Load playlist to queue"),
            Line::from("  x             Remove track from playlist"),
            Line::from("  Del           Delete playlist"),
            Line::from(""),
            Line::from(vec![Span::styled("✏️ Metadata Editor:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  t             Edit title"),
            Line::from("  a             Edit artist"),
            Line::from("  Tab           Apply suggestion"),
            Line::from("  b             Bulk apply all suggestions"),
            Line::from("  Shift+S       Save all changes"),
            Line::from(""),
            Line::from(vec![Span::styled("🔧 Technical:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Smart Shuffle: Behavior-weighted track selection"),
            Line::from("  Favorites: Stored in SQLite behavior database"),
            Line::from("  Queue: Manual queue + playlist loading"),
            Line::from(""),
            Line::from(vec![Span::styled("💡 Tips:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
            Line::from("  • Press ? for context-sensitive help overlay"),
            Line::from("  • Favorites (f key) boost shuffle weight by 1.8x"),
            Line::from("  • Tracks played >90% auto-tagged 'high_completion' (+1.3x)"),
            Line::from("  • Smart shuffle avoids recently played tracks"),
            Line::from("  • Press q or Esc to quit"),
        ];

        let settings_paragraph = Paragraph::new(settings_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Settings & Configuration")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        f.render_widget(settings_paragraph, area);
    }

    // ──────────────────────────────────────────────────────────────────
    // Metadata editor
    // ──────────────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_metadata_editor(
        f: &mut Frame,
        area: Rect,
        tracks: &[Track],
        metadata_parser: &MetadataParser,
        list_state: &mut ListState,
        edit_mode: &EditMode,
        edit_title: &str,
        edit_artist: &str,
        editing_track_index: Option<usize>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let items: Vec<ListItem> = tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let filename = track.file_path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown");

                let parsed = metadata_parser.parse_filename(filename);
                let confidence_indicator = match parsed.confidence {
                    c if c > 0.8 => "🟢",
                    c if c > 0.5 => "🟡",
                    _ => "🔴",
                };

                let is_editing = editing_track_index == Some(i);
                let style = if is_editing {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let current_display = format!("{} - {}", track.display_title(), track.display_artist());
                let suggested_display = format!("{} - {}", parsed.suggested_title, parsed.suggested_artist);

                let content = if current_display == suggested_display {
                    format!("{} ✅ {}", confidence_indicator, current_display)
                } else {
                    format!("{} {} → {}", confidence_indicator, current_display, suggested_display)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Metadata Editor (🟢=Good 🟡=OK 🔴=Poor)"),
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("→ ");
        f.render_stateful_widget(list, chunks[0], list_state);

        let edit_content = match edit_mode {
            EditMode::Title => {
                vec![
                    Line::from(vec![Span::styled("Editing Title:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![Span::styled(edit_title, Style::default().fg(Color::White).add_modifier(Modifier::UNDERLINED))]),
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![Span::styled("Press Enter to save, Esc to cancel", Style::default().fg(Color::Gray))]),
                ]
            }
            EditMode::Artist => {
                vec![
                    Line::from(vec![Span::styled("Editing Artist:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![Span::styled(edit_artist, Style::default().fg(Color::White).add_modifier(Modifier::UNDERLINED))]),
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![Span::styled("Press Enter to save, Esc to cancel", Style::default().fg(Color::Gray))]),
                ]
            }
            EditMode::None => {
                if let Some(selected) = list_state.selected() {
                    if selected < tracks.len() {
                        let track = &tracks[selected];
                        let filename = track.file_path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("unknown");
                        let parsed = metadata_parser.parse_filename(filename);

                        let current_title = track.display_title();
                        let current_artist = track.display_artist();
                        let suggested_title = parsed.suggested_title.clone();
                        let suggested_artist = parsed.suggested_artist.clone();
                        let confidence_text = format!("Confidence: {:.0}%", parsed.confidence * 100.0);

                        vec![
                            Line::from(vec![Span::styled("Current Track:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
                            Line::from(vec![Span::raw("")]),
                            Line::from(vec![Span::styled("Title: ", Style::default().fg(Color::Gray)), Span::raw(current_title)]),
                            Line::from(vec![Span::styled("Artist: ", Style::default().fg(Color::Gray)), Span::raw(current_artist)]),
                            Line::from(vec![Span::raw("")]),
                            Line::from(vec![Span::styled("Suggested:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
                            Line::from(vec![Span::styled("Title: ", Style::default().fg(Color::Gray)), Span::raw(suggested_title)]),
                            Line::from(vec![Span::styled("Artist: ", Style::default().fg(Color::Gray)), Span::raw(suggested_artist)]),
                            Line::from(vec![Span::styled(confidence_text, Style::default().fg(Color::Yellow))]),
                            Line::from(vec![Span::raw("")]),
                            Line::from(vec![Span::styled("Controls:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))]),
                            Line::from(vec![Span::styled("t", Style::default().fg(Color::Yellow)), Span::raw(" = Edit Title")]),
                            Line::from(vec![Span::styled("a", Style::default().fg(Color::Yellow)), Span::raw(" = Edit Artist")]),
                            Line::from(vec![Span::styled("Tab", Style::default().fg(Color::Yellow)), Span::raw(" = Apply Suggestion")]),
                            Line::from(vec![Span::styled("r", Style::default().fg(Color::Yellow)), Span::raw(" = Reset to Original")]),
                            Line::from(vec![Span::styled("c", Style::default().fg(Color::Yellow)), Span::raw(" = Clear Metadata")]),
                            Line::from(vec![Span::raw("")]),
                            Line::from(vec![Span::styled("Bulk Operations:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
                            Line::from(vec![Span::styled("b", Style::default().fg(Color::Green)), Span::raw(" = Bulk Apply Suggestions")]),
                            Line::from(vec![Span::styled("S", Style::default().fg(Color::Green)), Span::raw(" = Save Changes")]),
                        ]
                    } else {
                        vec![Line::from(vec![Span::raw("No track selected")])]
                    }
                } else {
                    vec![Line::from(vec![Span::raw("No track selected")])]
                }
            }
        };

        let edit_panel = Paragraph::new(edit_content)
            .block(Block::default().borders(Borders::ALL).title("Edit Panel"))
            .wrap(Wrap { trim: true });
        f.render_widget(edit_panel, chunks[1]);
    }

    // ──────────────────────────────────────────────────────────────────
    // Playlists tree view
    // ──────────────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_playlists_tree_view(
        f: &mut Frame,
        area: Rect,
        playlist_manager: &PlaylistManager,
        playlist_list_state: &mut ListState,
        expanded_playlists: &std::collections::HashSet<String>,
        tracks: &[Track],
        _playlist_track_states: &HashMap<String, ListState>,
        current_track_index: Option<usize>,
        is_playing: bool,
    ) {
        let playlists = playlist_manager.list_playlists();
        let mut tree_items: Vec<ListItem> = Vec::new();

        for playlist in playlists.iter() {
            let stats = playlist_manager.get_playlist_stats(&playlist.id, tracks).unwrap_or_default();
            let is_expanded = expanded_playlists.contains(&playlist.id);

            let expand_icon = if is_expanded { "▼" } else { "▶" };
            let playlist_content = format!(
                "{} {} ({} tracks, {})",
                expand_icon,
                playlist.name,
                stats.track_count,
                Self::format_duration(std::time::Duration::from_secs(stats.total_duration))
            );

            let playlist_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
            tree_items.push(ListItem::new(playlist_content).style(playlist_style));

            if is_expanded {
                let valid_tracks = playlist.get_valid_tracks(tracks);
                for (track_idx, &track_index) in valid_tracks.iter().enumerate() {
                    if track_index < tracks.len() {
                        let track = &tracks[track_index];
                        let is_current = current_track_index == Some(track_index);

                        let mut track_content = format!("  {}. {}", track_idx + 1, track.display_title());

                        if is_current && is_playing {
                            track_content = format!("  ▶ {}", track_content.trim_start());
                        } else if is_current {
                            track_content = format!("  ⏸ {}", track_content.trim_start());
                        }

                        let track_style = if is_current {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray)
                        };

                        tree_items.push(ListItem::new(track_content).style(track_style));
                    }
                }
            }
        }

        let tree_list = List::new(tree_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🎵 Playlists (Tree View)")
                    .title_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("→ ");
        f.render_stateful_widget(tree_list, area, playlist_list_state);

        let instructions_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(3),
            width: area.width,
            height: 3,
        };

        let instructions = Paragraph::new("Del: Delete | Enter: Expand/Collapse | Space: Play/Pause")
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: true });
        f.render_widget(instructions, instructions_area);
    }

    // ──────────────────────────────────────────────────────────────────
    // Overlay widgets
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn render_search_input(
        f: &mut Frame,
        area: Rect,
        search_query: &str,
        results_count: usize,
    ) {
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(area.height.saturating_sub(4)),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area)[1];

        let search_text = if search_query.is_empty() {
            "🔍 Search (fuzzy): ".to_string()
        } else {
            format!("🔍 Search (fuzzy): {} | {} results", search_query, results_count)
        };

        let search_input = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search Mode - Esc to exit")
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(Clear, popup_area);
        f.render_widget(search_input, popup_area);
    }

    pub(super) fn render_playlist_input(f: &mut Frame, area: Rect, playlist_name: &str) {
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(area.height.saturating_sub(4)),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area)[1];

        let input_text = format!("🎵 Playlist Name: {}", playlist_name);

        let playlist_input = Paragraph::new(input_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Create Playlist - Enter to confirm, Esc to cancel")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(Clear, popup_area);
        f.render_widget(playlist_input, popup_area);
    }

    pub(super) fn render_playlist_selector_overlay(
        f: &mut Frame,
        area: Rect,
        playlist_manager: &PlaylistManager,
        list_state: &mut ListState,
        track_title: &str,
    ) {
        let popup_area = Self::centered_rect(60, 70, area);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(format!(" Select Playlist for '{}' ", track_title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        f.render_widget(block, popup_area);

        let inner_area = popup_area.inner(Margin { horizontal: 1, vertical: 1 });

        let playlists = playlist_manager.list_playlists();
        let mut items = Vec::new();
        for playlist in &playlists {
            items.push(ListItem::new(format!("📋 {}", playlist.name)));
        }
        items.push(ListItem::new("➕ Create New Playlist"));

        let list = List::new(items)
            .block(Block::default())
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, inner_area, list_state);

        let instructions_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width - 2,
            height: 1,
        };

        let instructions = Paragraph::new("↑↓: Navigate | Enter: Select | Esc: Cancel")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(instructions, instructions_area);
    }

    pub(super) fn render_help_overlay(f: &mut Frame, area: Rect) {
        let popup_area = Self::centered_rect(80, 70, area);

        let help_text = vec![
            Line::from(vec![Span::styled("🎵 BangTunes Help", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("Navigation:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  ↑/↓           Navigate tracks (no auto-play)"),
            Line::from("  1/2/3/4       Switch tabs (Library/Playlists/Metadata Editor/Settings)"),
            Line::from("  /             Enter search mode (fuzzy search)"),
            Line::from("  ?             Toggle this help"),
            Line::from("  q             Quit"),
            Line::from(""),
            Line::from(vec![Span::styled("Playback:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Space         Play/Pause"),
            Line::from("  n             Next track"),
            Line::from("  p             Previous track"),
            Line::from("  s             Toggle shuffle"),
            Line::from("  r             Cycle repeat mode"),
            Line::from("  +/-           Volume up/down"),
            Line::from(""),
            Line::from(vec![Span::styled("Queue:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  e             Play next (add to front of queue)"),
            Line::from("  E             Add to end of queue"),
            Line::from("  Q             Toggle queue overlay"),
            Line::from("  x             Remove from queue (when overlay open)"),
            Line::from("  C             Clear queue"),
            Line::from("  f             Toggle favorite (⭐ boosts shuffle weight)"),
            Line::from(""),
            Line::from(vec![Span::styled("Playlists:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  N             Create playlist"),
            Line::from("  Del           Delete playlist"),
            Line::from("  l             Load playlist  (Enter = Expand/collapse)"),
            Line::from("  a             Add track to playlist (from Library)"),
            Line::from("  Q             Load playlist into queue"),
            Line::from(""),
            Line::from(vec![Span::styled("Metadata Editor:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  t / a         Edit title / artist"),
            Line::from("  Tab           Apply suggestion"),
            Line::from("  b             Bulk apply all suggestions"),
            Line::from("  Shift+S       Save all changes"),
            Line::from("  Esc           Cancel edit"),
            Line::from(""),
            Line::from(vec![Span::styled("Press ? again to close", Style::default().fg(Color::Yellow))]),
        ];

        // Solid background behind help overlay
        let clear_all = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(clear_all, area);

        f.render_widget(Clear, popup_area);
        let solid_background = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(solid_background, popup_area);

        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .wrap(Wrap { trim: true });
        f.render_widget(help_paragraph, popup_area);
    }

    pub(super) fn render_queue_overlay(
        f: &mut Frame,
        queue: &VecDeque<usize>,
        tracks: &[Track],
        queue_list_state: &mut ListState,
        queue_visible: bool,
    ) {
        if !queue_visible || queue.is_empty() {
            if queue_visible && queue.is_empty() {
                let area = Self::centered_rect(50, 30, f.area());
                f.render_widget(Clear, area);
                let block = Block::default()
                    .title("📋 Queue (empty)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                let msg = Paragraph::new("Queue is empty\n\ne / E to add tracks")
                    .block(block)
                    .alignment(Alignment::Center);
                f.render_widget(msg, area);
            }
            return;
        }

        let area = Self::centered_rect(60, 60, f.area());
        f.render_widget(Clear, area);

        let items: Vec<ListItem> = queue
            .iter()
            .enumerate()
            .map(|(i, &track_idx)| {
                let track = &tracks[track_idx];
                let label = format!(
                    "{:2}. {} — {}",
                    i + 1,
                    track.display_artist(),
                    track.display_title()
                );
                ListItem::new(label)
            })
            .collect();

        let title = format!("📋 Queue ({} tracks) — Q:close  x:remove  C:clear", queue.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, area, queue_list_state);
    }

    // ──────────────────────────────────────────────────────────────────
    // Layout helpers
    // ──────────────────────────────────────────────────────────────────

    pub(super) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    pub(super) fn format_duration(duration: std::time::Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{}:{:02}", minutes, seconds)
        }
    }
}
