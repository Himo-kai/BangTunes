// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors
//
// InteractiveApp: struct definition, constructor, run loop, render, set_status.
// Method implementations are split across submodule files.

mod actions;
mod events;
mod metadata;
mod navigation;
mod playback;
mod queue;
mod render;

pub(crate) use events::{AppTab, EditMode, InteractiveEvent, RepeatMode};

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::ListState,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, time::sleep};

use crate::{
    audio::{
        metadata_parser::MetadataParser,
        player::PlayerEvent,
        playlist::PlaylistManager,
        AudioPlayer,
    },
    behavior::{
        weighting::ShuffleWeighting, BehaviorDatabase, BehaviorTracker,
    },
    config::Config,
    database::BangTunesDatabase,
    ui::{events::EventHandler, TerminalManager},
};

pub struct InteractiveApp {
    #[allow(dead_code)]
    pub(super) config: Config,
    pub(super) terminal: TerminalManager,
    pub(super) event_handler: EventHandler,
    pub(super) audio_player: AudioPlayer,
    pub(super) behavior_tracker: BehaviorTracker,
    pub(super) shuffle_weighting: ShuffleWeighting,

    // Music library
    pub(super) tracks: Vec<crate::Track>,
    pub(super) filtered_tracks: Vec<usize>,

    // UI state
    pub(super) list_state: ListState,
    pub(super) current_track_index: Option<usize>,
    pub(super) should_quit: bool,
    pub(super) current_tab: AppTab,

    // Playback state
    pub(super) volume: f32,
    pub(super) is_playing: bool,
    pub(super) is_shuffled: bool,
    pub(super) repeat_mode: RepeatMode,
    pub(super) autoplay: bool,
    pub(super) recently_played: VecDeque<uuid::Uuid>,
    pub(super) queue: VecDeque<usize>,
    pub(super) queue_visible: bool,
    pub(super) queue_list_state: ListState,
    pub(super) queue_replace_confirmation: bool,
    pub(super) queue_replace_playlist_id: Option<String>,
    pub(super) favorites: HashSet<uuid::Uuid>,
    pub(super) failed_tracks: HashSet<uuid::Uuid>,
    pub scan_skipped_count: usize,

    // Time tracking
    pub(super) current_position: Duration,
    pub(super) total_duration: Option<Duration>,
    pub(super) last_position_update: Instant,

    // Metadata editor state
    pub(super) metadata_parser: MetadataParser,
    pub(super) metadata_list_state: ListState,
    pub(super) editing_track_index: Option<usize>,
    pub(super) edit_title: String,
    pub(super) edit_artist: String,
    pub(super) edit_mode: EditMode,

    // Event handling
    pub(super) event_rx: mpsc::UnboundedReceiver<InteractiveEvent>,
    pub(super) _event_tx: mpsc::UnboundedSender<InteractiveEvent>,
    pub(super) audio_event_rx: mpsc::UnboundedReceiver<PlayerEvent>,

    // Status messages
    pub(super) status_message: Option<(String, Instant)>,

    // Help overlay
    pub(super) show_help: bool,

    // Search functionality
    pub(super) search_mode: bool,
    pub(super) search_query: String,
    pub(super) fuzzy_matcher: SkimMatcherV2,

    // Playlist functionality
    pub(super) playlist_manager: PlaylistManager,
    pub(super) playlist_list_state: ListState,
    pub(super) current_playlist_id: Option<String>,
    pub(super) playlist_creation_mode: bool,
    pub(super) playlist_rename_mode: bool,
    pub(super) playlist_rename_id: Option<String>,
    pub(super) playlist_name_input: String,
    pub(super) expanded_playlists: HashSet<String>,
    pub(super) playlist_track_states: HashMap<String, ListState>,

    // Playlist selector overlay
    pub(super) show_playlist_selector: bool,
    pub(super) playlist_selector_state: ListState,
    pub(super) selected_track_for_playlist: Option<usize>,

    // BangTunes database integration
    pub(super) database: Option<BangTunesDatabase>,
}

impl InteractiveApp {
    pub async fn new(config: Config, mut tracks: Vec<crate::Track>) -> Result<Self> {
        let terminal = TerminalManager::new()?;
        let event_handler = EventHandler::new();
        let mut audio_player = AudioPlayer::new(config.clone().into())?;

        // Initialize behavior database and tracker
        let behavior_db = BehaviorDatabase::new(&config.database_path)?;
        let behavior_tracker = BehaviorTracker::new(
            behavior_db,
            config.behavior.min_play_time_for_tracking,
        );

        // Initialize intelligent shuffle weighting
        let shuffle_weighting = ShuffleWeighting::new(config.behavior.weight_decay_days);

        // Load favorites cache from behavior tracker
        let favorites = behavior_tracker
            .get_all_behaviors()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|b| b.tags.contains(&"favorite".to_string()))
            .map(|b| b.track_id)
            .collect::<HashSet<_>>();

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Create audio event channel for duration learning
        let (audio_event_tx, audio_event_rx) = mpsc::unbounded_channel();
        audio_player.set_event_sender(audio_event_tx);

        // Restore learned durations from previous sessions
        for track in &mut tracks {
            if let Ok(Some(dur_secs)) = behavior_tracker.get_track_duration(track.id).await {
                track.learn_duration(std::time::Duration::from_secs(dur_secs));
            }
        }

        // Initialize filtered tracks (show all initially)
        let filtered_tracks: Vec<usize> = (0..tracks.len()).collect();

        let mut list_state = ListState::default();
        if !filtered_tracks.is_empty() {
            list_state.select(Some(0));
        }

        let mut metadata_list_state = ListState::default();
        if !tracks.is_empty() {
            metadata_list_state.select(Some(0));
        }

        Ok(Self {
            config,
            terminal,
            event_handler,
            audio_player,
            behavior_tracker,
            shuffle_weighting,
            tracks,
            filtered_tracks,
            list_state,
            current_track_index: None,
            should_quit: false,
            current_tab: AppTab::Library,
            volume: 1.0,
            is_playing: false,
            is_shuffled: false,
            repeat_mode: RepeatMode::Off,
            autoplay: true,
            recently_played: VecDeque::new(),
            queue: VecDeque::new(),
            queue_visible: false,
            queue_list_state: ListState::default(),
            queue_replace_confirmation: false,
            queue_replace_playlist_id: None,
            favorites,
            failed_tracks: HashSet::new(),
            scan_skipped_count: 0,
            current_position: Duration::from_secs(0),
            total_duration: None,
            last_position_update: Instant::now(),
            metadata_parser: MetadataParser::new(),
            metadata_list_state,
            editing_track_index: None,
            edit_title: String::new(),
            edit_artist: String::new(),
            edit_mode: EditMode::None,
            event_rx,
            _event_tx: event_tx,
            audio_event_rx,
            status_message: None,
            show_help: false,
            search_mode: false,
            search_query: String::new(),
            fuzzy_matcher: SkimMatcherV2::default(),
            playlist_manager: PlaylistManager::new("playlists".into())
                .map_err(|e| anyhow::anyhow!("{}", e))?,
            playlist_list_state: ListState::default(),
            current_playlist_id: None,
            playlist_creation_mode: false,
            playlist_rename_mode: false,
            playlist_rename_id: None,
            playlist_name_input: String::new(),
            expanded_playlists: HashSet::new(),
            playlist_track_states: HashMap::new(),
            show_playlist_selector: false,
            playlist_selector_state: ListState::default(),
            selected_track_for_playlist: None,
            database: BangTunesDatabase::find_database().ok(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        while !self.should_quit {
            // Handle input events
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        let app_event = if self.queue_replace_confirmation {
                            use crossterm::event::{KeyCode, KeyModifiers};
                            match (key.code, key.modifiers) {
                                (KeyCode::Char('y'), _) | (KeyCode::Char('Y'), _) => {
                                    Some(InteractiveEvent::ConfirmQueueReplace)
                                }
                                (KeyCode::Char('n'), _)
                                | (KeyCode::Char('N'), _)
                                | (KeyCode::Esc, _) => Some(InteractiveEvent::CancelQueueReplace),
                                (KeyCode::Char('c'), KeyModifiers::CONTROL)
                                | (KeyCode::Char('q'), KeyModifiers::NONE) => {
                                    Some(InteractiveEvent::Quit)
                                }
                                (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                                    Some(InteractiveEvent::ForceRedraw)
                                }
                                _ => None,
                            }
                        } else if self.search_mode {
                            Self::key_to_search_event(key)
                        } else if self.playlist_creation_mode || self.playlist_rename_mode {
                            Self::key_to_playlist_event(key)
                        } else if self.show_playlist_selector {
                            Self::key_to_playlist_selector_event(key)
                        } else {
                            self.key_to_app_event_basic(key)
                        };

                        if let Some(app_event) = app_event {
                            self.handle_event(app_event).await?;
                        }
                    }
                }
            }

            // Handle audio events (duration learning, track finished, etc.)
            while let Ok(audio_event) = self.audio_event_rx.try_recv() {
                self.handle_audio_event(audio_event).await?;
            }

            // Handle internal events (including Tick events for time tracking)
            while let Ok(internal_event) = self.event_rx.try_recv() {
                self.handle_event(internal_event).await?;
            }

            // Generate a Tick event for time tracking updates
            let _ = self._event_tx.send(InteractiveEvent::Tick);

            // Render UI
            self.render()?;

            // Balanced delay for smooth UI with good audio performance
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    pub(super) fn set_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), Instant::now()));
    }

    fn render(&mut self) -> Result<()> {
        let current_track_index = self.current_track_index;
        let is_playing = self.is_playing;
        let volume = self.volume;
        let repeat_mode = self.repeat_mode.clone();
        let is_shuffled = self.is_shuffled;
        let status_message = self.status_message.clone();

        // Get terminal size for responsive layout decisions
        let terminal_size = self
            .terminal
            .size()
            .unwrap_or(ratatui::layout::Rect::new(0, 0, 80, 24));
        let is_small_screen = terminal_size.height < 20;

        // Extract queue data before closure to avoid borrow checker issues
        let queue_visible = self.queue_visible;
        let queue = &self.queue;
        let tracks = &self.tracks;
        let queue_list_state = &mut self.queue_list_state;
        let scan_skipped_count = self.scan_skipped_count;

        match self.terminal.draw(|f| {
            let size = f.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(if is_small_screen {
                    [
                        Constraint::Length(2),
                        Constraint::Min(4),
                        Constraint::Length(3),
                        Constraint::Length(2),
                    ]
                } else {
                    [
                        Constraint::Length(3),
                        Constraint::Min(6),
                        Constraint::Length(4),
                        Constraint::Length(3),
                    ]
                })
                .split(size);

            Self::render_header_with_tabs(f, chunks[0], &self.current_tab);

            match &self.current_tab {
                AppTab::Library => {
                    Self::render_track_list(
                        f,
                        chunks[1],
                        &self.tracks,
                        &self.filtered_tracks,
                        current_track_index,
                        is_playing,
                        &mut self.list_state,
                        &self.favorites,
                        scan_skipped_count,
                    );
                }
                AppTab::Playlists => {
                    Self::render_playlists_tree_view(
                        f,
                        chunks[1],
                        &self.playlist_manager,
                        &mut self.playlist_list_state,
                        &self.expanded_playlists,
                        &self.tracks,
                        &self.playlist_track_states,
                        current_track_index,
                        is_playing,
                    );
                }
                AppTab::MetadataEditor => {
                    Self::render_metadata_editor(
                        f,
                        chunks[1],
                        &self.tracks,
                        &self.metadata_parser,
                        &mut self.metadata_list_state,
                        &self.edit_mode,
                        &self.edit_title,
                        &self.edit_artist,
                        self.editing_track_index,
                    );
                }
                AppTab::Settings => {
                    Self::render_settings(f, chunks[1]);
                }
            }

            Self::render_player_controls(
                f,
                chunks[2],
                &self.tracks,
                current_track_index,
                is_playing,
                volume,
                repeat_mode,
                is_shuffled,
                self.current_position,
                self.total_duration,
            );

            Self::render_status_bar(f, chunks[3], status_message, self.queue.len());

            if self.search_mode {
                Self::render_search_input(f, size, &self.search_query, self.filtered_tracks.len());
            }

            if self.playlist_creation_mode || self.playlist_rename_mode {
                Self::render_playlist_input(f, size, &self.playlist_name_input);
            }

            if self.show_playlist_selector {
                if let Some(track_idx) = self.selected_track_for_playlist {
                    let track_title = self.tracks[track_idx].display_title();
                    Self::render_playlist_selector_overlay(
                        f,
                        size,
                        &self.playlist_manager,
                        &mut self.playlist_selector_state,
                        &track_title,
                    );
                }
            }

            if self.show_help {
                Self::render_help_overlay(f, size);
            }

            if queue_visible {
                Self::render_queue_overlay(f, queue, tracks, queue_list_state, queue_visible);
            }
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
