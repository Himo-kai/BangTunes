use super::{AppEvent, EventHandler, TerminalManager};
use crate::audio::{AudioPlayer, MusicScanner, PlaybackState, Track};
use crate::behavior::{BehaviorDatabase, BehaviorTracker, PlaybackEvent, SkipReason};
use crate::config::Config;
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph},
    Frame,
};


pub struct App {
    config: Config,
    terminal: TerminalManager,
    event_handler: EventHandler,
    audio_player: AudioPlayer,
    behavior_tracker: BehaviorTracker,
    
    // State
    pub tracks: Vec<Track>,
    pub current_track_index: Option<usize>,
    pub list_state: ListState,
    pub should_quit: bool,
    
    // UI State
    #[allow(dead_code)] // Used in interactive app tab switching
    pub current_tab: Tab,
    pub volume: f32,
    #[allow(dead_code)] // Used in interactive app shuffle functionality  
    pub is_shuffled: bool,
    pub repeat_mode: RepeatMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Library,
    #[allow(dead_code)] // Future feature: Queue management
    Queue,
    #[allow(dead_code)] // Future feature: Playlist management  
    Playlists,
    #[allow(dead_code)] // Future feature: Settings panel
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepeatMode {
    Off,
    #[allow(dead_code)] // Used in interactive app repeat functionality
    All,
    #[allow(dead_code)] // Used in interactive app repeat functionality
    One,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let terminal = TerminalManager::new()?;
        let event_handler = EventHandler::new();
        let audio_player = AudioPlayer::new(Default::default())?;
        
        // Initialize behavior database
        let database = BehaviorDatabase::new(&config.database_path)?;
        let behavior_tracker = BehaviorTracker::new(database, config.behavior.min_play_time_for_tracking);
        
        // Scan music library
        let scanner = MusicScanner::new();
        let tracks = scanner.scan_directories(&config.music_directories)?;
        
        let mut list_state = ListState::default();
        if !tracks.is_empty() {
            list_state.select(Some(0));
        }
        
        Ok(Self {
            config,
            terminal,
            event_handler,
            audio_player,
            behavior_tracker,
            tracks,
            current_track_index: None,
            list_state,
            should_quit: false,
            current_tab: Tab::Library,
            volume: 0.7,
            is_shuffled: false,
            repeat_mode: RepeatMode::Off,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        // Start event handling in background
        let _event_sender = self.event_handler.sender();
        let _event_handler_clone = self.event_handler.sender();
        
        tokio::spawn(async move {
            let handler = EventHandler::new();
            let _ = handler.handle_terminal_events().await;
        });
        
        // Main event loop
        while !self.should_quit {
            // Render UI
            let should_quit = self.should_quit;
            let current_track_index = self.current_track_index;
            let tracks = &self.tracks;
            let volume = self.volume;
            let audio_state = self.audio_player.get_state();
            let mut list_state = self.list_state.clone();
            
            self.terminal.draw(|f| {
                Self::render_ui(f, should_quit, current_track_index, tracks, volume, audio_state, &mut list_state);
            })?;
            
            self.list_state = list_state;
            
            // Handle events
            if let Some(event) = self.event_handler.next_event().await {
                self.handle_event(event).await?;
            }
        }
        
        Ok(())
    }
    
    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Quit => {
                self.should_quit = true;
            }
            AppEvent::TogglePlayPause => {
                match self.audio_player.get_state() {
                    PlaybackState::Playing => {
                        self.audio_player.pause()?;
                        if let Some(track) = self.get_current_track() {
                            let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackPaused {
                                track_id: track.id,
                                position: 0, // TODO: Get actual position
                                timestamp: chrono::Utc::now(),
                            }).await;
                        }
                    }
                    PlaybackState::Paused => {
                        self.audio_player.resume()?;
                        if let Some(track) = self.get_current_track() {
                            let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackResumed {
                                track_id: track.id,
                                position: 0, // TODO: Get actual position
                                timestamp: chrono::Utc::now(),
                            }).await;
                        }
                    }
                    PlaybackState::Stopped => {
                        self.play_current_track().await?;
                    }
                }
            }
            AppEvent::NextTrack => {
                self.next_track().await?;
            }
            AppEvent::PreviousTrack => {
                self.previous_track().await?;
            }
            AppEvent::Up => {
                self.move_selection(-1);
            }
            AppEvent::Down => {
                self.move_selection(1);
            }
            AppEvent::Enter => {
                if let Some(selected) = self.list_state.selected() {
                    self.current_track_index = Some(selected);
                    self.play_current_track().await?;
                }
            }
            AppEvent::VolumeUp => {
                self.volume = (self.volume + 0.1).min(1.0);
                self.audio_player.set_volume(self.volume)?;
            }
            AppEvent::VolumeDown => {
                self.volume = (self.volume - 0.1).max(0.0);
                self.audio_player.set_volume(self.volume)?;
            }
            AppEvent::RefreshLibrary => {
                self.refresh_library().await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn play_current_track(&mut self) -> Result<()> {
        if let Some(index) = self.current_track_index {
            if let Some(track) = self.tracks.get(index).cloned() {
                self.audio_player.play_track(track.clone())?;
                
                // Track behavior
                let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackStarted {
                    track_id: track.id,
                    timestamp: chrono::Utc::now(),
                }).await;
            }
        }
        Ok(())
    }
    
    async fn next_track(&mut self) -> Result<()> {
        if let Some(current) = self.current_track_index {
            let next_index = if current + 1 < self.tracks.len() {
                current + 1
            } else {
                match self.repeat_mode {
                    RepeatMode::All => 0,
                    _ => return Ok(()),
                }
            };
            
            // Track skip behavior
            if let Some(track) = self.get_current_track() {
                let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackSkipped {
                    track_id: track.id,
                    position: 0, // TODO: Get actual position
                    reason: SkipReason::NextTrack,
                    timestamp: chrono::Utc::now(),
                }).await;
            }
            
            self.current_track_index = Some(next_index);
            self.play_current_track().await?;
        }
        Ok(())
    }
    
    async fn previous_track(&mut self) -> Result<()> {
        if let Some(current) = self.current_track_index {
            let prev_index = if current > 0 {
                current - 1
            } else {
                match self.repeat_mode {
                    RepeatMode::All => self.tracks.len() - 1,
                    _ => return Ok(()),
                }
            };
            
            self.current_track_index = Some(prev_index);
            self.play_current_track().await?;
        }
        Ok(())
    }
    
    fn move_selection(&mut self, delta: i32) {
        if self.tracks.is_empty() {
            return;
        }
        
        let current = self.list_state.selected().unwrap_or(0);
        let new_index = if delta < 0 {
            current.saturating_sub((-delta) as usize)
        } else {
            (current + delta as usize).min(self.tracks.len() - 1)
        };
        
        self.list_state.select(Some(new_index));
    }
    
    async fn refresh_library(&mut self) -> Result<()> {
        let scanner = MusicScanner::new();
        self.tracks = scanner.scan_directories(&self.config.music_directories)?;
        
        if !self.tracks.is_empty() && self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
        
        Ok(())
    }
    
    fn get_current_track(&self) -> Option<&Track> {
        self.current_track_index
            .and_then(|index| self.tracks.get(index))
    }
    
    fn render_ui(
        f: &mut Frame,
        _should_quit: bool,
        current_track_index: Option<usize>,
        tracks: &[Track],
        volume: f32,
        audio_state: PlaybackState,
        list_state: &mut ListState,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Player controls
            ])
            .split(f.area());
        
        // Header
        Self::render_header(f, chunks[0]);
        
        // Main content
        Self::render_main_content(f, chunks[1], current_track_index, tracks, list_state);
        
        // Player controls
        Self::render_player_controls(f, chunks[2], current_track_index, tracks, volume, audio_state);
    }
    
    fn render_header(f: &mut Frame, area: Rect) {
        let title = Paragraph::new("🎵 BangTunes - Terminal Music Player")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(title, area);
    }
    
    fn render_main_content(
        f: &mut Frame,
        area: Rect,
        current_track_index: Option<usize>,
        tracks: &[Track],
        list_state: &mut ListState,
    ) {
        let items: Vec<ListItem> = tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let is_current = current_track_index == Some(i);
                let prefix = if is_current { "♪ " } else { "  " };
                
                let content = format!(
                    "{}{} - {} ({})",
                    prefix,
                    track.display_artist(),
                    track.display_title(),
                    track.display_album()
                );
                
                let style = if is_current {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                ListItem::new(content).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Library"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, list_state);
    }
    
    fn render_player_controls(
        f: &mut Frame,
        area: Rect,
        current_track_index: Option<usize>,
        tracks: &[Track],
        volume: f32,
        audio_state: PlaybackState,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Track info
                Constraint::Percentage(20), // Volume
                Constraint::Percentage(20), // Status
            ])
            .split(area);
        
        // Track info
        let track_info = if let Some(track) = current_track_index.and_then(|i| tracks.get(i)) {
            format!("♪ {} - {}", track.display_artist(), track.display_title())
        } else {
            "No track selected".to_string()
        };
        
        let info_widget = Paragraph::new(track_info)
            .block(Block::default().borders(Borders::ALL).title("Now Playing"));
        f.render_widget(info_widget, chunks[0]);
        
        // Volume
        let volume_widget = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Volume"))
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(volume as f64);
        f.render_widget(volume_widget, chunks[1]);
        
        // Status
        let state_text = match audio_state {
            PlaybackState::Playing => "▶ Playing",
            PlaybackState::Paused => "⏸ Paused",
            PlaybackState::Stopped => "⏹ Stopped",
        };
        
        let status_widget = Paragraph::new(state_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[2]);
    }
}
