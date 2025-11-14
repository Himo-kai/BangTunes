use anyhow::Result;
use tracing::{debug, info, error};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
};
use fuzzy_matcher::{clangd::ClangdMatcher, FuzzyMatcher};
use panpipe::{
    audio::{AudioPlayer, MusicScanner, metadata_parser::MetadataParser, scanner::ScanProgress, playlist::PlaylistManager, player::PlayerEvent},
    behavior::{BehaviorDatabase, BehaviorTracker, PlaybackEvent, SkipReason},
    config::Config,
    ui::TerminalManager,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::{
    sync::mpsc,
    time::sleep,
};

use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "panpipe_interactive")]
#[command(about = "A terminal-based music player with intelligent behavior tracking")]
struct Args {
    /// Enable developer logging (stderr + debug output)
    #[arg(long)]
    dev: bool,
}

fn init_logging(dev: bool) -> Result<()> {
    // Create logs directory in project root
    let log_dir = PathBuf::from("logs");
    std::fs::create_dir_all(&log_dir)?;

    // Daily rotating file appender
    let file_appender = tracing_appender::rolling::daily(&log_dir, "panpipe.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    // Base filter: info level for general logs, debug for panpipe
    let base_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,panpipe=debug"));

    // Build subscriber with conditional stderr layer
    let subscriber = tracing_subscriber::fmt()
        .with_writer(file_writer)
        .with_target(true)
        .with_level(true)
        .with_ansi(false)
        .with_env_filter(base_filter)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // If dev mode, also log to stderr (this will be in addition to file)
    if dev {
        eprintln!("🔧 Dev mode: Debug output enabled to stderr + file");
    }
    
    // Prevent the guard from being dropped
    std::mem::forget(_guard);
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();
    
    // Initialize logging system
    init_logging(args.dev)?;
    
    info!("🎵 PanPipe Interactive starting up");
    
    // Only redirect stderr if NOT in dev mode (dev mode needs stderr for debug output)
    let _stderr_redirect = if !args.dev {
        debug!("Redirecting stderr to suppress ALSA errors");
        Some(redirect_stderr_to_null())
    } else {
        debug!("Dev mode: keeping stderr for debug output");
        None
    };
    
    // Initialize configuration
    let config = Config::load()?;
    
    // Print startup banner
    println!("🎵 BangTunes - Terminal Music Player");
    println!("===================================");
    println!("Loading your music library...");
    
    // Initialize music scanner with incremental loading
    let scanner = MusicScanner::new();
    let (progress_tx, mut progress_rx) = mpsc::channel(128); // Bounded channel per analysis
    
    println!("📁 Scanning music directories...");
    
    // Start incremental scanning in background
    let scanner_task = {
        let scanner = scanner.clone();
        let directories = config.music_directories.clone();
        tokio::spawn(async move {
            scanner.scan_directories_incremental(&directories, progress_tx).await
        })
    };
    
    // Process scan progress with live updates
    let mut all_tracks = Vec::new();
    
    while let Some(progress) = progress_rx.recv().await {
        match progress {
            ScanProgress::Started { total_directories } => {
                println!("🔍 Starting scan of {} directories", total_directories);
            }
            ScanProgress::DirectoryStarted { path } => {
                println!("📂 Scanning: {:?}", path);
            }
            ScanProgress::TrackFound { track, progress, .. } => {
                all_tracks.push(track);
                
                // Update progress every 50 tracks for smooth feedback
                if progress % 50 == 0 {
                    println!("   📀 Found {} tracks so far...", progress);
                }
            }
            ScanProgress::DirectoryCompleted { path, tracks_found } => {
                println!("   ✅ {:?}: {} tracks", path, tracks_found);
            }
            ScanProgress::Completed { total_tracks } => {
                println!("🎵 Scan complete: {} tracks total", total_tracks);
                break;
            }
            ScanProgress::Error { path, error } => {
                eprintln!("   ⚠️  Error scanning {:?}: {}", path, error);
            }
        }
    }
    
    // Wait for scanner task to complete and get final results
    match scanner_task.await {
        Ok(Ok(final_tracks)) => {
            all_tracks = final_tracks; // Use final results to ensure consistency
        }
        Ok(Err(e)) => {
            eprintln!("❌ Scanner error: {}", e);
        }
        Err(e) => {
            eprintln!("❌ Scanner task error: {}", e);
        }
    }
    
    if all_tracks.is_empty() {
        eprintln!("❌ No music files found in configured directories!");
        eprintln!("Please check your music directories in the config.");
        return Ok(());
    }
    
    println!("✅ Loaded {} tracks total", all_tracks.len());
    println!("🚀 Starting BangTunes...\n");
    
    // Small delay to let user see the loading info
    sleep(Duration::from_millis(1500)).await;
    
    // Initialize the interactive app
    let mut app = InteractiveApp::new(config, all_tracks).await?;
    
    // Run the interactive interface
    app.run().await?;
    
    println!("\n👋 Thanks for using BangTunes!");
    Ok(())
}

struct InteractiveApp {
    #[allow(dead_code)] // Used in initialization and throughout app lifecycle
    config: Config,
    terminal: TerminalManager,
    audio_player: AudioPlayer,
    behavior_tracker: BehaviorTracker,
    
    // Music library
    tracks: Vec<panpipe::Track>,
    filtered_tracks: Vec<usize>, // indices into tracks
    
    // UI state
    list_state: ListState,
    current_track_index: Option<usize>,
    should_quit: bool,
    current_tab: AppTab,
    
    // Playback state
    volume: f32,
    is_playing: bool,
    is_shuffled: bool,
    repeat_mode: RepeatMode,
    
    // Time tracking
    current_position: Duration,
    total_duration: Option<Duration>,
    last_position_update: Instant,
    
    // Visualizer removed for performance optimization
    
    // Metadata editor state
    metadata_parser: MetadataParser,
    metadata_list_state: ListState,
    editing_track_index: Option<usize>,
    edit_title: String,
    edit_artist: String,
    edit_mode: EditMode,
    
    // Event handling
    event_rx: mpsc::UnboundedReceiver<InteractiveEvent>,
    _event_tx: mpsc::UnboundedSender<InteractiveEvent>,
    audio_event_rx: mpsc::UnboundedReceiver<PlayerEvent>,
    
    // Status messages
    status_message: Option<(String, Instant)>,
    
    // Help overlay
    show_help: bool,
    
    // Search functionality
    search_mode: bool,
    search_query: String,
    fuzzy_matcher: ClangdMatcher,
    
    // Playlist functionality
    playlist_manager: PlaylistManager,
    playlist_list_state: ListState,
    current_playlist_id: Option<String>,
    playlist_tracks: Vec<usize>, // indices into tracks for current playlist
    playlist_creation_mode: bool,
    playlist_name_input: String,
    expanded_playlists: std::collections::HashSet<String>, // Track which playlists are expanded
    playlist_track_states: std::collections::HashMap<String, ListState>, // Per-playlist navigation state
    
    // Playlist selector overlay (for Library tab 'a' key)
    show_playlist_selector: bool,
    playlist_selector_state: ListState,
    selected_track_for_playlist: Option<usize>, // Track index to add to selected playlist
}

#[derive(Debug, Clone, PartialEq)]
enum AppTab {
    Library,
    Playlists,
    MetadataEditor,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
enum EditMode {
    None,
    Title,
    Artist,
}

#[derive(Debug, Clone, PartialEq)]
enum RepeatMode {
    Off,
    All,
    One,
}

// Visualizer enum removed for performance optimization

impl InteractiveApp {
    async fn new(config: Config, tracks: Vec<panpipe::Track>) -> Result<Self> {
        let terminal = TerminalManager::new()?;
        let mut audio_player = AudioPlayer::new(config.clone().into())?;
        
        // Initialize behavior database and tracker
        let behavior_db = BehaviorDatabase::new(&config.database_path)?;
        let behavior_tracker = BehaviorTracker::new(
            behavior_db,
            config.behavior.min_play_time_for_tracking,
        );
        
        // Create event channel (revert to unbounded for stability)
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // Create audio event channel for duration learning
        let (audio_event_tx, audio_event_rx) = mpsc::unbounded_channel();
        audio_player.set_event_sender(audio_event_tx);
        
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
            audio_player,
            behavior_tracker,
            tracks,
            filtered_tracks,
            list_state,
            current_track_index: None,
            should_quit: false,
            current_tab: AppTab::Library,
            volume: 0.7,
            is_playing: false,
            is_shuffled: false,
            repeat_mode: RepeatMode::Off,
            current_position: Duration::from_secs(0),
            total_duration: None,
            last_position_update: Instant::now(),
            // Visualizer initialization removed
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
            fuzzy_matcher: ClangdMatcher::default(),
            
            // Initialize playlist functionality
            playlist_manager: PlaylistManager::new("playlists".into()).map_err(|e| anyhow::anyhow!("{}", e))?,
            playlist_list_state: ListState::default(),
            current_playlist_id: None,
            playlist_tracks: Vec::new(),
            playlist_creation_mode: false,
            playlist_name_input: String::new(),
            expanded_playlists: std::collections::HashSet::new(),
            playlist_track_states: std::collections::HashMap::new(),
            
            // Initialize playlist selector overlay
            show_playlist_selector: false,
            playlist_selector_state: ListState::default(),
            selected_track_for_playlist: None,
        })
    }
    
    async fn run(&mut self) -> Result<()> {
        // SYNCHRONOUS event handling - no separate async tasks for terminal I/O
        // This prevents race conditions that cause "Error: end of stream"
        
        let _last_update = Instant::now();
        
        while !self.should_quit {
            // Handle input events with balanced polling for responsive UI
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(event) = event::read() {
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press {
                            let app_event = if self.search_mode {
                                Self::key_to_search_event(key)
                            } else if self.playlist_creation_mode {
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
            sleep(Duration::from_millis(100)).await; // ~10 FPS (balanced UI/audio)
        }
        
        Ok(())
    }
    
    fn key_to_search_event(key: KeyEvent) -> Option<InteractiveEvent> {
        use crossterm::event::KeyModifiers;
        
        match (key.code, key.modifiers) {
            // Exit search mode
            (KeyCode::Esc, _) => Some(InteractiveEvent::ExitSearch),
            (KeyCode::Enter, _) => Some(InteractiveEvent::ExitSearch),
            
            // Search input handling
            (KeyCode::Backspace, _) => Some(InteractiveEvent::SearchBackspace),
            (KeyCode::Char(c), KeyModifiers::NONE) if !c.is_control() => Some(InteractiveEvent::SearchInput(c)),
            
            // Allow navigation in search results
            (KeyCode::Up, _) => Some(InteractiveEvent::Up),
            (KeyCode::Down, _) => Some(InteractiveEvent::Down),
            
            // Global quit still works
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit),
            
            _ => None,
        }
    }
    
    fn key_to_playlist_event(key: KeyEvent) -> Option<InteractiveEvent> {
        use crossterm::event::KeyModifiers;
        
        match (key.code, key.modifiers) {
            // Confirm playlist creation
            (KeyCode::Enter, _) => Some(InteractiveEvent::ConfirmPlaylistCreation),
            // Cancel playlist creation
            (KeyCode::Esc, _) => Some(InteractiveEvent::CancelPlaylistCreation),
            
            // Playlist name input handling
            (KeyCode::Backspace, _) => Some(InteractiveEvent::PlaylistBackspace),
            (KeyCode::Char(c), KeyModifiers::NONE) if !c.is_control() => Some(InteractiveEvent::PlaylistInput(c)),
            
            // Global quit still works
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit),
            
            _ => None,
        }
    }
    
    fn key_to_playlist_selector_event(key: KeyEvent) -> Option<InteractiveEvent> {
        use crossterm::event::KeyModifiers;
        
        match (key.code, key.modifiers) {
            // Navigation in playlist selector
            (KeyCode::Up, _) => Some(InteractiveEvent::Up),
            (KeyCode::Down, _) => Some(InteractiveEvent::Down),
            
            // Select playlist or create new one
            (KeyCode::Enter, _) => Some(InteractiveEvent::SelectPlaylistFromSelector),
            
            // Cancel playlist selection
            (KeyCode::Esc, _) => Some(InteractiveEvent::CancelPlaylistSelector),
            
            // Global quit still works
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit),
            
            _ => None,
        }
    }
    
    fn key_to_app_event_basic(&self, key: KeyEvent) -> Option<InteractiveEvent> {
        use crossterm::event::KeyModifiers;
        
        match (key.code, key.modifiers) {
            // Ctrl combinations for ergonomic shortcuts
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(InteractiveEvent::SaveMetadata),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(InteractiveEvent::Quit), // Ctrl+C
            
            // Regular key mappings
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(InteractiveEvent::Quit),
            (KeyCode::Char('1'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToLibrary),
            (KeyCode::Char('2'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToPlaylists),
            (KeyCode::Char('3'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToMetadataEditor),
            (KeyCode::Char('4'), KeyModifiers::NONE) => Some(InteractiveEvent::SwitchToSettings),
            (KeyCode::Char(' '), KeyModifiers::NONE) => Some(InteractiveEvent::TogglePlayPause),
            (KeyCode::Char('n'), KeyModifiers::NONE) => Some(InteractiveEvent::NextTrack),
            (KeyCode::Char('p'), KeyModifiers::NONE) => Some(InteractiveEvent::PreviousTrack),
            (KeyCode::Char('s'), KeyModifiers::NONE) => Some(InteractiveEvent::Stop),
            (KeyCode::Char('+'), KeyModifiers::NONE) | (KeyCode::Char('='), KeyModifiers::NONE) => Some(InteractiveEvent::VolumeUp),
            (KeyCode::Char('-'), KeyModifiers::NONE) => Some(InteractiveEvent::VolumeDown),
            (KeyCode::Char('z'), KeyModifiers::NONE) => Some(InteractiveEvent::ToggleShuffle),

            (KeyCode::Up, _) => Some(InteractiveEvent::Up),
            (KeyCode::Down, _) => Some(InteractiveEvent::Down),
            (KeyCode::Esc, _) => Some(InteractiveEvent::CancelEdit),
            (KeyCode::Backspace, _) => Some(InteractiveEvent::Backspace),
            // Context-sensitive key bindings based on current tab
            (KeyCode::Char('c'), KeyModifiers::NONE) => {
                match self.current_tab {
                    AppTab::MetadataEditor => Some(InteractiveEvent::ClearMetadata),
                    _ => None,
                }
            }
            (KeyCode::Char('a'), KeyModifiers::NONE) => {
                match self.current_tab {
                    AppTab::Library => Some(InteractiveEvent::AddToPlaylist),
                    AppTab::MetadataEditor => Some(InteractiveEvent::EditArtist),
                    _ => None,
                }
            }
            (KeyCode::Char('l'), KeyModifiers::NONE) => {
                match self.current_tab {
                    AppTab::Playlists => Some(InteractiveEvent::LoadPlaylist),
                    _ => None,
                }
            }
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                match self.current_tab {
                    AppTab::Playlists => Some(InteractiveEvent::RenamePlaylist),
                    _ => Some(InteractiveEvent::ToggleRepeat), // Default behavior for other tabs
                }
            }
            (KeyCode::Char('x'), KeyModifiers::NONE) => {
                match self.current_tab {
                    AppTab::Playlists => Some(InteractiveEvent::RemoveFromPlaylist),
                    _ => None,
                }
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                match self.current_tab {
                    AppTab::Playlists => Some(InteractiveEvent::TogglePlaylistExpansion),
                    _ => Some(InteractiveEvent::Play), // Default behavior for other tabs
                }
            }
            
            // Metadata editor specific keys (only work in metadata editor tab)
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
            
            // Global keys that work everywhere
            (KeyCode::Delete, KeyModifiers::NONE) => {
                if self.current_tab == AppTab::Playlists {
                    Some(InteractiveEvent::DeletePlaylist)
                } else {
                    None
                }
            }
            
            // Search mode - forward slash to enter search
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(InteractiveEvent::EnterSearch),
            
            // Help overlay - handle ? key (Shift+/ produces '?' character)
            (KeyCode::Char('?'), KeyModifiers::NONE) => Some(InteractiveEvent::ShowHelp),
            (KeyCode::Char('?'), KeyModifiers::SHIFT) => Some(InteractiveEvent::ShowHelp),
            
            // Catch-all for text input (exclude ? to avoid conflict with help)
            (KeyCode::Char(c), KeyModifiers::NONE) if !c.is_control() && c != '?' => Some(InteractiveEvent::Input(c)),
            _ => None,
        }
    }
    
    async fn handle_event(&mut self, event: InteractiveEvent) -> Result<()> {
        // Context-aware event filtering
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
            (InteractiveEvent::ShowHelp, _, _) => true, // Help overlay should work globally
            
            // Search events - should work globally
            (InteractiveEvent::EnterSearch, _, _) => true,
            (InteractiveEvent::ExitSearch, _, _) => true,
            (InteractiveEvent::SearchInput(_), _, _) => true,
            (InteractiveEvent::SearchBackspace, _, _) => true,
            
            // Playlist creation input events - should work when in playlist creation mode
            (InteractiveEvent::PlaylistInput(_), _, _) => true,
            (InteractiveEvent::PlaylistBackspace, _, _) => true,
            (InteractiveEvent::ConfirmPlaylistCreation, _, _) => true,
            (InteractiveEvent::CancelPlaylistCreation, _, _) => true,
            
            // Playlist selector overlay events - should work when overlay is shown
            (InteractiveEvent::SelectPlaylistFromSelector, _, _) => true,
            (InteractiveEvent::CancelPlaylistSelector, _, _) => true,
            
            // Editing mode events (highest priority)
            (InteractiveEvent::SaveMetadata, _, EditMode::Title | EditMode::Artist) => true,
            (InteractiveEvent::CancelEdit, _, EditMode::Title | EditMode::Artist) => true,
            (InteractiveEvent::Backspace, _, EditMode::Title | EditMode::Artist) => true,
            (InteractiveEvent::Input(_), _, EditMode::Title | EditMode::Artist) => true,
            
            // Metadata editor events (when not editing)
            (InteractiveEvent::EditTitle, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::EditArtist, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::ApplySuggestion, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::ResetToOriginal, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::BulkApplySuggestions, AppTab::MetadataEditor, EditMode::None) => true,
            (InteractiveEvent::ClearMetadata, AppTab::MetadataEditor, EditMode::None) => true,
            
            // Playlist events (when not editing)
            (InteractiveEvent::LoadPlaylist, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::TogglePlaylistExpansion, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::DeletePlaylist, AppTab::Playlists, EditMode::None) => true,
            (InteractiveEvent::AddToPlaylist, AppTab::Library, EditMode::None) => true,
            
            // 'r' key context-sensitive handling
            (InteractiveEvent::ToggleRepeat, AppTab::Library, EditMode::None) => true,
            (InteractiveEvent::ToggleRepeat, AppTab::MetadataEditor, EditMode::None) => false, // Block in metadata editor
            
            // Playback controls work in both tabs when not editing
            (InteractiveEvent::TogglePlayPause, _, EditMode::None) => true,
            (InteractiveEvent::Play, _, EditMode::None) => true,
            (InteractiveEvent::NextTrack, _, EditMode::None) => true,
            (InteractiveEvent::PreviousTrack, _, EditMode::None) => true,
            (InteractiveEvent::Stop, _, EditMode::None) => true,
            (InteractiveEvent::ToggleShuffle, _, EditMode::None) => true,
            (InteractiveEvent::VolumeUp, _, EditMode::None) => true,
            (InteractiveEvent::VolumeDown, _, EditMode::None) => true,
            
            // Visualizer event filtering removed
            
            // Block other events when editing or in wrong context
            _ => false,
        };
        
        if !should_process {
            return Ok(());
        }
        
        match event {
            InteractiveEvent::Quit => {
                self.should_quit = true;
            }
            InteractiveEvent::Up => {
                self.move_selection(-1);
            }
            InteractiveEvent::Down => {
                self.move_selection(1);
            }
            InteractiveEvent::Play => {
                // Check if we're in playlist context first
                if let Some((playlist_id, track_idx_in_playlist)) = self.get_playlist_selection_context() {
                    // Playing from playlist - get the actual track index
                    debug!("🎵 Playlist context detected: playlist={}, track_idx={}", playlist_id, track_idx_in_playlist);
                    if let Some(playlist) = self.playlist_manager.get_playlist(&playlist_id) {
                        let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                        debug!("🎵 Valid tracks in playlist: {:?}", valid_tracks);
                        if let Some(&actual_track_idx) = valid_tracks.get(track_idx_in_playlist) {
                            debug!("🎵 Playing track {} from playlist", actual_track_idx);
                            self.play_track(actual_track_idx).await?;
                        } else {
                            debug!("❌ Track index {} not found in valid tracks", track_idx_in_playlist);
                        }
                    } else {
                        debug!("❌ Playlist {} not found", playlist_id);
                    }
                } else {
                    debug!("🎵 No playlist context, checking library selection");
                    if let Some(selected) = self.list_state.selected() {
                        // Playing from library
                        if selected < self.filtered_tracks.len() {
                            let track_idx = self.filtered_tracks[selected];
                            debug!("🎵 Playing track {} from library", track_idx);
                            self.play_track(track_idx).await?;
                        }
                    } else {
                        debug!("❌ No selection found in library");
                    }
                }
            }
            InteractiveEvent::TogglePlayPause => {
                if self.is_playing {
                    self.audio_player.pause()?;
                    self.is_playing = false;
                    self.set_status("⏸️ Paused");
                } else {
                    if self.current_track_index.is_some() {
                        self.audio_player.resume()?;
                        self.is_playing = true;
                        self.set_status("▶️ Resumed");
                    } else {
                        // Check if we're in playlist context first
                        if let Some((playlist_id, track_idx_in_playlist)) = self.get_playlist_selection_context() {
                            // Playing from playlist - get the actual track index
                            debug!("🎵 TogglePlayPause: Playlist context detected: playlist={}, track_idx={}", playlist_id, track_idx_in_playlist);
                            if let Some(playlist) = self.playlist_manager.get_playlist(&playlist_id) {
                                let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                                debug!("🎵 TogglePlayPause: Valid tracks in playlist: {:?}", valid_tracks);
                                if let Some(&actual_track_idx) = valid_tracks.get(track_idx_in_playlist) {
                                    debug!("🎵 TogglePlayPause: Playing track {} from playlist", actual_track_idx);
                                    self.play_track(actual_track_idx).await?;
                                } else {
                                    debug!("❌ TogglePlayPause: Track index {} not found in valid tracks", track_idx_in_playlist);
                                }
                            } else {
                                debug!("❌ TogglePlayPause: Playlist {} not found", playlist_id);
                            }
                        } else {
                            debug!("🎵 TogglePlayPause: No playlist context, checking library selection");
                            if let Some(selected) = self.list_state.selected() {
                                // Playing from library
                                if selected < self.filtered_tracks.len() {
                                    let track_idx = self.filtered_tracks[selected];
                                    debug!("🎵 TogglePlayPause: Playing track {} from library", track_idx);
                                    self.play_track(track_idx).await?;
                                }
                            } else {
                                debug!("❌ TogglePlayPause: No selection found in library");
                            }
                        }
                    }
                }
            }
            InteractiveEvent::NextTrack => {
                self.next_track().await?;
            }
            InteractiveEvent::PreviousTrack => {
                self.previous_track().await?;
            }
            InteractiveEvent::Stop => {
                self.audio_player.stop()?;
                self.is_playing = false;
                self.current_track_index = None;
                self.set_status("⏹️ Stopped");
            }
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
                if self.is_shuffled {
                    self.set_status("🔀 Shuffle: On");
                } else {
                    self.set_status("🔀 Shuffle: Off");
                }
            }
            InteractiveEvent::Tick => {
                // Handle periodic updates
                self.update_playback_status().await?;
            }
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
            // Visualizer event handling removed
            InteractiveEvent::Input(c) => {
                match self.edit_mode {
                    EditMode::Title => {
                        self.edit_title.push(c);
                    }
                    EditMode::Artist => {
                        self.edit_artist.push(c);
                    }
                    EditMode::None => {
                        // No special input handling needed in non-edit mode
                    }
                }
            }
            InteractiveEvent::Backspace => {
                match self.edit_mode {
                    EditMode::Title => {
                        self.edit_title.pop();
                    }
                    EditMode::Artist => {
                        self.edit_artist.pop();
                    }
                    EditMode::None => {}
                }
            }
            InteractiveEvent::ShowHelp => {
                self.show_help = !self.show_help;
                self.set_status("❓ Help overlay toggled");
            }
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
                self.set_status("🔍 Search exited");
            }
            InteractiveEvent::SearchInput(c) => {
                debug!("🔍 Search input: '{}' (char code: {})", c, c as u32);
                self.search_query.push(c);
                debug!("🔍 Search query now: '{}' (len={})", self.search_query, self.search_query.len());
                self.update_search_results();
                self.set_status(&format!("🔍 Searching: '{}' ({} results)", self.search_query, self.filtered_tracks.len()));
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
            // Playlist events

            InteractiveEvent::DeletePlaylist => {
                if self.current_tab == AppTab::Playlists {
                    if let Some(selected) = self.playlist_list_state.selected() {
                        let playlists = self.playlist_manager.list_playlists();
                        if let Some(playlist) = playlists.get(selected) {
                            let playlist_id = playlist.id.clone();
                            let playlist_count = playlists.len();
                            drop(playlists); // Release the immutable borrow
                            
                            match self.playlist_manager.delete_playlist(&playlist_id) {
                                Ok(deleted) => {
                                    self.set_status("🗑️ Playlist deleted");
                                    info!("Deleted playlist: {}", playlist_id);
                                    if deleted {
                                        // Reset selection if we deleted the last item
                                        if selected >= playlist_count.saturating_sub(1) && selected > 0 {
                                            self.playlist_list_state.select(Some(selected - 1));
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.set_status(&format!("❌ Failed to delete playlist: {}", e));
                                    error!("Failed to delete playlist: {}", e);
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
                            // Clone necessary data before making mutable borrows
                            let playlist_id = playlist.id.clone();
                            let playlist_name = playlist.name.clone();
                            let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                            
                            // Load playlist tracks
                            self.playlist_tracks = valid_tracks;
                            self.current_playlist_id = Some(playlist_id);
                            
                            // Update filtered tracks to show playlist content
                            self.filtered_tracks = (0..self.playlist_tracks.len()).collect();
                            if !self.filtered_tracks.is_empty() {
                                self.list_state.select(Some(0));
                            }
                            
                            self.set_status(&format!("🎵 Loaded playlist: {}", playlist_name));
                            info!("Loaded playlist: {} ({} tracks)", playlist_name, self.filtered_tracks.len());
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
                            
                            // Single playlist expansion: only one playlist can be expanded at a time
                            if self.expanded_playlists.contains(&playlist_id) {
                                // Collapse the currently expanded playlist
                                self.expanded_playlists.clear();
                                self.playlist_track_states.clear();
                                self.set_status(&format!("📁 Collapsed playlist: {}", playlist_name));
                                debug!("🔍 Collapsed playlist: {}", playlist_name);
                            } else {
                                // Collapse any previously expanded playlists first
                                self.expanded_playlists.clear();
                                self.playlist_track_states.clear();
                                
                                // Expand only this playlist
                                self.expanded_playlists.insert(playlist_id.clone());
                                
                                // Initialize track navigation state for this playlist
                                let mut track_state = ListState::default();
                                let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                                if !valid_tracks.is_empty() {
                                    track_state.select(Some(0));
                                }
                                self.playlist_track_states.insert(playlist_id.clone(), track_state);
                                
                                self.set_status(&format!("📂 Expanded playlist: {} ({} tracks)", playlist_name, valid_tracks.len()));
                                debug!("🔍 Expanded playlist: {} ({} tracks) - all others collapsed", playlist_name, valid_tracks.len());
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
                            
                            // Show playlist selector overlay instead of auto-adding to first playlist
                            self.show_playlist_selector = true;
                            self.selected_track_for_playlist = Some(track_idx);
                            
                            // Initialize selector state
                            let playlists = self.playlist_manager.list_playlists();
                            let total_options = playlists.len() + 1; // +1 for "Create New Playlist" option
                            
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
                if self.playlist_creation_mode {
                    self.playlist_name_input.push(c);
                    self.set_status(&format!("🎵 Playlist name: {}", self.playlist_name_input));
                }
            }
            InteractiveEvent::PlaylistBackspace => {
                if self.playlist_creation_mode {
                    self.playlist_name_input.pop();
                    self.set_status(&format!("🎵 Playlist name: {}", self.playlist_name_input));
                }
            }
            InteractiveEvent::ConfirmPlaylistCreation => {
                if self.playlist_creation_mode && !self.playlist_name_input.is_empty() {
                    match self.playlist_manager.create_playlist(self.playlist_name_input.clone(), None) {
                        Ok(playlist_id) => {
                            self.set_status(&format!("✅ Created playlist: {}", self.playlist_name_input));
                            info!("Created playlist: {} (ID: {})", self.playlist_name_input, playlist_id);
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
                self.playlist_name_input.clear();
                self.set_status("❌ Playlist creation cancelled");
            }
            // Placeholder events for future implementation
            InteractiveEvent::RenamePlaylist => {
                self.set_status("🚧 Rename playlist - not yet implemented");
            }
            InteractiveEvent::RemoveFromPlaylist => {
                self.set_status("🚧 Remove from playlist - not yet implemented");
            }
            InteractiveEvent::SelectPlaylistFromSelector => {
                if self.show_playlist_selector {
                    if let Some(selected) = self.playlist_selector_state.selected() {
                        if let Some(track_idx) = self.selected_track_for_playlist {
                            let playlists = self.playlist_manager.list_playlists();
                            let track_path = self.tracks[track_idx].file_path.clone();
                            let track_title = self.tracks[track_idx].display_title();
                            
                            if selected < playlists.len() {
                                // Selected existing playlist
                                let playlist_id = playlists[selected].id.clone();
                                let playlist_name = playlists[selected].name.clone();
                                drop(playlists); // Release the immutable borrow
                                
                                match self.playlist_manager.add_track_to_playlist(&playlist_id, &track_path) {
                                    Ok(_) => {
                                        self.set_status(&format!("➕ Added '{}' to '{}'", track_title, playlist_name));
                                        debug!("🎵 Added track to existing playlist: {}", playlist_name);
                                    }
                                    Err(e) => {
                                        self.set_status(&format!("❌ Failed to add track: {}", e));
                                    }
                                }
                            } else {
                                // Selected "Create New Playlist" option
                                drop(playlists); // Release the immutable borrow
                                self.playlist_creation_mode = true;
                                self.playlist_name_input.clear();
                                self.set_status("📝 Enter new playlist name:");
                                debug!("🎵 Starting playlist creation from selector");
                            }
                            
                            // Close the selector overlay
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
    
    async fn play_track(&mut self, track_idx: usize) -> Result<()> {
        if track_idx >= self.tracks.len() {
            return Ok(());
        }
        
        let track = self.tracks[track_idx].clone();
        
        // Record behavior tracking event
        let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackStarted {
            track_id: track.id,
            timestamp: chrono::Utc::now(),
        }).await;
        
        // Play the track with graceful error handling
        self.set_status(&format!("🔄 Attempting to play: {}", track.display_title()));
        
        match self.audio_player.play_track(track.clone()) {
            Ok(()) => {
                self.current_track_index = Some(track_idx);
                self.is_playing = true;
    
                
                // Reset time tracking
                self.current_position = Duration::from_secs(0);
                self.total_duration = track.duration;
                self.last_position_update = Instant::now();
                
                self.set_status(&format!("✅ SUCCESS: Playing {} | idx={} | is_playing={}", 
                    track.display_title(), track_idx, self.is_playing));
            }
            Err(e) => {
                // Don't crash the TUI - just show error and continue
                self.set_status(&format!("❌ AUDIO PLAYER FAILED: {} | Error: {}", track.display_title(), e));
                self.is_playing = false;
                self.current_track_index = None;
            }
        }
        
        Ok(())
    }
    
    /// Get the current playlist selection context (playlist_id, track_index_in_playlist)
    fn get_playlist_selection_context(&self) -> Option<(String, usize)> {
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
                debug!("🔍 Checking playlist '{}': current_index={}, is_expanded={}", playlist.name, current_index, is_expanded);
                
                if current_index == selected {
                    // Selected the playlist header itself
                    debug!("🔍 Selected playlist header: {}", playlist.name);
                    return Some((playlist.id.clone(), 0));
                }
                current_index += 1;
                
                if is_expanded {
                    let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                    debug!("🔍 Expanded playlist has {} valid tracks", valid_tracks.len());
                    for (track_idx_in_playlist, _) in valid_tracks.iter().enumerate() {
                        debug!("🔍 Checking track {}: current_index={}", track_idx_in_playlist, current_index);
                        if current_index == selected {
                            // Selected a track within the expanded playlist
                            debug!("🔍 Selected track {} in playlist '{}'", track_idx_in_playlist, playlist.name);
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

    async fn next_track(&mut self) -> Result<()> {
        if let Some(current_idx) = self.current_track_index {
            // Record skip event
            let track = &self.tracks[current_idx];
            let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackSkipped {
                track_id: track.id,
                position: 0, // TODO: get actual position
                reason: SkipReason::NextTrack,
                timestamp: chrono::Utc::now(),
            }).await;
        }
        
        // Check if we're in playlist context first
        if self.current_tab == AppTab::Playlists && !self.expanded_playlists.is_empty() {
            // Get the currently expanded playlist (only one can be expanded)
            let expanded_playlist_id = self.expanded_playlists.iter().next().unwrap().clone();
            debug!("🎵 Next track in playlist context: playlist={}", expanded_playlist_id);
            
            if let Some(playlist) = self.playlist_manager.get_playlist(&expanded_playlist_id) {
                let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                
                // Get current track state for this playlist
                if let Some(track_state) = self.playlist_track_states.get_mut(&expanded_playlist_id) {
                    let current_track_idx = track_state.selected().unwrap_or(0);
                    let next_track_idx = (current_track_idx + 1) % valid_tracks.len();
                    
                    // Update playlist track selection
                    track_state.select(Some(next_track_idx));
                    
                    if let Some(&actual_track_idx) = valid_tracks.get(next_track_idx) {
                        debug!("🎵 Playing next track {} from playlist (track {} of {})", actual_track_idx, next_track_idx + 1, valid_tracks.len());
                        self.play_track(actual_track_idx).await?;
                    } else {
                        debug!("❌ Next track index {} not found in playlist", next_track_idx);
                    }
                } else {
                    debug!("❌ No track state found for expanded playlist");
                }
            }
        } else {
            // Next track in library
            debug!("🎵 Next track in library context");
            if let Some(selected) = self.list_state.selected() {
                let next_idx = (selected + 1) % self.filtered_tracks.len();
                self.list_state.select(Some(next_idx));
                
                let track_idx = self.filtered_tracks[next_idx];
                self.play_track(track_idx).await?;
            }
        }
        
        Ok(())
    }
    
    async fn previous_track(&mut self) -> Result<()> {
        // Check if we're in playlist context first
        if self.current_tab == AppTab::Playlists && !self.expanded_playlists.is_empty() {
            // Get the currently expanded playlist (only one can be expanded)
            let expanded_playlist_id = self.expanded_playlists.iter().next().unwrap().clone();
            debug!("🎵 Previous track in playlist context: playlist={}", expanded_playlist_id);
            
            if let Some(playlist) = self.playlist_manager.get_playlist(&expanded_playlist_id) {
                let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                
                // Get current track state for this playlist
                if let Some(track_state) = self.playlist_track_states.get_mut(&expanded_playlist_id) {
                    let current_track_idx = track_state.selected().unwrap_or(0);
                    let prev_track_idx = if current_track_idx == 0 {
                        valid_tracks.len() - 1
                    } else {
                        current_track_idx - 1
                    };
                    
                    // Update playlist track selection
                    track_state.select(Some(prev_track_idx));
                    
                    if let Some(&actual_track_idx) = valid_tracks.get(prev_track_idx) {
                        debug!("🎵 Playing previous track {} from playlist (track {} of {})", actual_track_idx, prev_track_idx + 1, valid_tracks.len());
                        self.play_track(actual_track_idx).await?;
                    } else {
                        debug!("❌ Previous track index {} not found in playlist", prev_track_idx);
                    }
                } else {
                    debug!("❌ No track state found for expanded playlist");
                }
            }
        } else {
            // Previous track in library
            debug!("🎵 Previous track in library context");
            if let Some(selected) = self.list_state.selected() {
                let prev_idx = if selected == 0 {
                    self.filtered_tracks.len() - 1
                } else {
                    selected - 1
                };
                self.list_state.select(Some(prev_idx));
                
                let track_idx = self.filtered_tracks[prev_idx];
                self.play_track(track_idx).await?;
            }
        }
        
        Ok(())
    }
    
    fn update_search_results(&mut self) {
        if self.search_query.is_empty() {
            debug!("🔍 Empty search query, showing all {} tracks", self.tracks.len());
            self.filtered_tracks = (0..self.tracks.len()).collect();
        } else {
            debug!("🔍 Fuzzy searching for: '{}'", self.search_query);
            
            // CRITICAL: ClangdMatcher parameter order is fuzzy_match(pattern, choice) NOT (choice, pattern)!
        // This was the root cause of typo tolerance failing - we had the parameters backwards.
        // The search query is the "pattern" and the track field is the "choice".
        // Test results: "the ouytside" vs "The Outside" works in reverse order (Some(290))
        // but returns None in forward order. Always use fuzzy_match(search_query, track_field)!
        
        // Fuzzy matcher testing
        let test_cases = [
            ("the outside", "the outside"),     // Exact match
            ("the outside", "the ouytside"),    // Original typo
            ("the outside", "the outsyde"),     // Different typo
            ("the outside", "outside"),         // Partial match
            ("the outside", "the out"),         // Prefix match
            ("the outside", "outside the"),     // Reversed
        ];
        
        for (query, target) in &test_cases {
                let score1 = self.fuzzy_matcher.fuzzy_match(query, target);
                let score2 = self.fuzzy_matcher.fuzzy_match(target, query);
                
                // Simple similarity test for comparison
                let simple_similarity = if target.contains(query) || query.contains(target) {
                    100
                } else if target.to_lowercase().contains(&query.to_lowercase()) || query.to_lowercase().contains(&target.to_lowercase()) {
                    50
                } else {
                    0
                };
                
                debug!("🔍 Fuzzy test: '{}' vs '{}' = {:?} (forward)", target, query, score1);
                debug!("🔍 Fuzzy test: '{}' vs '{}' = {:?} (reverse)", query, target, score2);
                debug!("🔍 Simple similarity: '{}' vs '{}' = {}", target, query, simple_similarity);
            }
            
            let mut scored_results: Vec<(usize, i64)> = Vec::new();
            let mut match_count = 0;
            
            for (idx, track) in self.tracks.iter().enumerate() {
                let mut best_score = 0i64;
                let mut match_field = "none";
                
                // Try matching against title
                if let Some(title) = &track.metadata.title {
                    if let Some(score) = self.fuzzy_matcher.fuzzy_match(&self.search_query, title) {
                        if score > best_score {
                            best_score = score;
                            match_field = "title";
                        }
                    }
                }
                
                // Try matching against display title
                let display_title = track.display_title();
                if let Some(score) = self.fuzzy_matcher.fuzzy_match(&self.search_query, &display_title) {
                    if score > best_score {
                        best_score = score;
                        match_field = "display_title";
                    }
                }
                
                // Try matching against artist
                if let Some(artist) = &track.metadata.artist {
                    if let Some(score) = self.fuzzy_matcher.fuzzy_match(&self.search_query, artist) {
                        if score > best_score {
                            best_score = score;
                            match_field = "artist";
                        }
                    }
                }
                
                // Try matching against filename
                if let Some(filename) = track.file_path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    if let Some(score) = self.fuzzy_matcher.fuzzy_match(&self.search_query, &filename_str) {
                        if score > best_score {
                            best_score = score;
                            match_field = "filename";
                        }
                    }
                }
                
                // Log detailed info for first few matches
                if idx < 3 {
                    debug!("🔍 Track {}: '{}' -> score {} (via {})", idx, track.display_title(), best_score, match_field);
                }
                
                if best_score > 0 {
                    scored_results.push((idx, best_score));
                    match_count += 1;
                }
            }
            
            // Sort by score (highest first)
            scored_results.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_tracks = scored_results.into_iter().map(|(idx, _)| idx).collect();
            
            debug!("🔍 Search complete: {} matches found out of {} tracks", match_count, self.tracks.len());
            if !self.filtered_tracks.is_empty() {
                let top_track = &self.tracks[self.filtered_tracks[0]];
                debug!("🔍 Top match: '{}'", top_track.display_title());
            }
        }
        
        // Reset selection to first result
        if !self.filtered_tracks.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }
    
    fn reset_to_full_library(&mut self) {
        // Reset to show all tracks
        self.filtered_tracks = (0..self.tracks.len()).collect();
        
        // Reset selection to first item
        if !self.filtered_tracks.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }
    
    fn move_selection(&mut self, delta: i32) {
        // Handle playlist selector overlay first (highest priority)
        if self.show_playlist_selector {
            let playlists = self.playlist_manager.list_playlists();
            let total_options = playlists.len() + 1; // +1 for "Create New Playlist" option
            
            if total_options == 0 {
                return;
            }
            
            let current = self.playlist_selector_state.selected().unwrap_or(0);
            let new_index = if delta > 0 {
                (current + delta as usize) % total_options
            } else {
                if current == 0 {
                    total_options - 1
                } else {
                    current.saturating_sub((-delta) as usize)
                }
            };
            
            self.playlist_selector_state.select(Some(new_index));
            debug!("🔍 Playlist selector navigation: moved from {} to {} (total options: {})", current, new_index, total_options);
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
                } else {
                    if current == 0 {
                        self.filtered_tracks.len() - 1
                    } else {
                        current.saturating_sub((-delta) as usize)
                    }
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
                } else {
                    if current == 0 {
                        self.tracks.len() - 1
                    } else {
                        current.saturating_sub((-delta) as usize)
                    }
                };
                
                self.metadata_list_state.select(Some(new_index));
            }
            AppTab::Playlists => {
                // Tree-view navigation: calculate total items (playlists + expanded tracks)
                let playlists = self.playlist_manager.list_playlists();
                if playlists.is_empty() {
                    return;
                }
                
                // Calculate total items in the tree view
                let mut total_items = 0;
                for playlist in &playlists {
                    total_items += 1; // Playlist header
                    if self.expanded_playlists.contains(&playlist.id) {
                        let valid_tracks = playlist.get_valid_tracks(&self.tracks);
                        total_items += valid_tracks.len(); // Expanded tracks
                    }
                }
                
                if total_items == 0 {
                    return;
                }
                
                let current = self.playlist_list_state.selected().unwrap_or(0);
                let new_index = if delta > 0 {
                    (current + delta as usize) % total_items
                } else {
                    if current == 0 {
                        total_items - 1
                    } else {
                        current.saturating_sub((-delta) as usize)
                    }
                };
                
                self.playlist_list_state.select(Some(new_index));
                debug!("🔍 Tree navigation: moved from {} to {} (total items: {})", current, new_index, total_items);
            }
            AppTab::Settings => {
                // Settings tab has no navigable list - do nothing
            }
        }
    }
    
    async fn save_current_edit(&mut self) -> Result<()> {
        if let Some(track_idx) = self.editing_track_index {
            if track_idx < self.tracks.len() {
                let track = &mut self.tracks[track_idx];
                
                match self.edit_mode {
                    EditMode::Title => {
                        track.metadata.title = Some(self.edit_title.clone());
                        self.set_status(&format!("✅ Title updated: {}", self.edit_title));
                    }
                    EditMode::Artist => {
                        track.metadata.artist = Some(self.edit_artist.clone());
                        self.set_status(&format!("✅ Artist updated: {}", self.edit_artist));
                    }
                    EditMode::None => {}
                }
                
                // TODO: Save to file tags and database
                // For now, just update in memory
                
                self.edit_mode = EditMode::None;
                self.editing_track_index = None;
                self.edit_title.clear();
                self.edit_artist.clear();
            }
        }
        
        Ok(())
    }
    
    async fn apply_filename_suggestion(&mut self, track_idx: usize) -> Result<()> {
        if track_idx < self.tracks.len() {
            let track = &self.tracks[track_idx];
            let filename = track.file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            
            let parsed = self.metadata_parser.parse_filename(filename);
            
            // Update the track metadata with suggestions
            self.tracks[track_idx].metadata.title = Some(parsed.suggested_title.clone());
            self.tracks[track_idx].metadata.artist = Some(parsed.suggested_artist.clone());
            
            self.set_status(&format!(
                "🤖 Applied suggestion: {} - {} (confidence: {:.0}%)", 
                parsed.suggested_title, 
                parsed.suggested_artist,
                parsed.confidence * 100.0
            ));
        }
        
        Ok(())
    }
    
    async fn reset_track_metadata(&mut self, track_idx: usize) -> Result<()> {
        if track_idx < self.tracks.len() {
            // Reset to original metadata from file tags
            let track = &mut self.tracks[track_idx];
            // For now, just clear the metadata - in a full implementation, 
            // we'd reload from the original file tags
            track.metadata.title = None;
            track.metadata.artist = None;
            
            self.set_status("🔄 Reset to original metadata");
        }
        Ok(())
    }
    
    async fn bulk_apply_suggestions(&mut self) -> Result<()> {
        let mut applied_count = 0;
        let total_tracks = self.tracks.len();
        
        for i in 0..total_tracks {
            let track = &self.tracks[i];
            let filename = track.file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            
            let parsed = self.metadata_parser.parse_filename(filename);
            
            // Only apply if confidence is reasonable (>50%)
            if parsed.confidence > 0.5 {
                self.tracks[i].metadata.title = Some(parsed.suggested_title);
                self.tracks[i].metadata.artist = Some(parsed.suggested_artist);
                applied_count += 1;
            }
        }
        
        self.set_status(&format!(
            "🚀 Bulk applied suggestions to {}/{} tracks (confidence >50%)", 
            applied_count, 
            total_tracks
        ));
        
        Ok(())
    }
    
    async fn clear_track_metadata(&mut self, track_idx: usize) -> Result<()> {
        if track_idx < self.tracks.len() {
            let track = &mut self.tracks[track_idx];
            track.metadata.title = None;
            track.metadata.artist = None;
            
            self.set_status("🗑️ Cleared track metadata");
        }
        Ok(())
    }
    
    // All visualizer methods removed for performance optimization
    
    async fn update_playback_status(&mut self) -> Result<()> {
        

        
        // Update time tracking if playing
        if self.is_playing {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_position_update);
            self.current_position += elapsed;
            self.last_position_update = now;
        }
        
        // Update visualizer data
        // Visualizer removed for performance optimization
        
        // NOTE: Removed problematic UI-based completion detection
        // The is_finished() check was returning true immediately due to sink.empty()
        // causing premature track advancement and state resets
        // Track completion will be handled by PlayerEvent::TrackFinished events
        
        Ok(())
    }
    
    fn set_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), Instant::now()));
    }
    
    fn render(&mut self) -> Result<()> {
        let current_track_index = self.current_track_index;
        let is_playing = self.is_playing;
        let volume = self.volume;
        let repeat_mode = self.repeat_mode.clone();
        let is_shuffled = self.is_shuffled;
        let status_message = self.status_message.clone();
        
        // Attempt render with error recovery
        match self.terminal.draw(|f| {
            let size = f.area();
            
            // Create main layout (visualizer removed)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(6),    // Content (reduced to make room)
                    Constraint::Length(4), // Player controls
                    Constraint::Length(3), // Status bar (increased for visibility)
                ])
                .split(size);
            
            // Render header with tabs
            Self::render_header_with_tabs(f, chunks[0], &self.current_tab);
            
            // Render content based on current tab
            match &self.current_tab {
                AppTab::Library => {
                    Self::render_track_list(f, chunks[1], &self.tracks, &self.filtered_tracks, current_track_index, is_playing, &mut self.list_state);
                }
                AppTab::Playlists => {
                    Self::render_playlists_tree_view(f, chunks[1], &self.playlist_manager, &mut self.playlist_list_state, &self.expanded_playlists, &self.tracks, &self.playlist_track_states, current_track_index, is_playing);
                }
                AppTab::MetadataEditor => {
                    Self::render_metadata_editor(f, chunks[1], &self.tracks, &self.metadata_parser, &mut self.metadata_list_state, &self.edit_mode, &self.edit_title, &self.edit_artist, self.editing_track_index);
                }
                AppTab::Settings => {
                    Self::render_settings(f, chunks[1]);
                }
            }
            
            // Render player controls (visualizer removed)
            Self::render_player_controls(f, chunks[2], &self.tracks, current_track_index, is_playing, volume, repeat_mode, is_shuffled, self.current_position, self.total_duration);
            
            // Render status bar
            Self::render_status_bar(f, chunks[3], status_message);
            
            // Render search input if in search mode
            if self.search_mode {
                Self::render_search_input(f, size, &self.search_query, self.filtered_tracks.len());
            }
            
            // Render playlist creation input if in playlist creation mode
            if self.playlist_creation_mode {
                Self::render_playlist_input(f, size, &self.playlist_name_input);
            }
            
            // Render playlist selector overlay if active
            if self.show_playlist_selector {
                if let Some(track_idx) = self.selected_track_for_playlist {
                    let track_title = self.tracks[track_idx].display_title();
                    Self::render_playlist_selector_overlay(f, size, &self.playlist_manager, &mut self.playlist_selector_state, &track_title);
                }
            }
            
            // Render help overlay if active
            if self.show_help {
                Self::render_help_overlay(f, size);
            }
        }) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Terminal corruption detected - no stdout usage during TUI!
                // Recovery will be handled by CleanupGuard on exit
                Err(e)
            }
        }
    }
    
    fn render_header_with_tabs(f: &mut Frame, area: Rect, current_tab: &AppTab) {
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
    
    fn render_metadata_editor(
        f: &mut Frame,
        area: Rect,
        tracks: &[panpipe::Track],
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
        
        // Left side: Track list with metadata
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
                    .title("Metadata Editor (🟢=Good 🟡=OK 🔴=Poor)")
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("→ ");
        
        f.render_stateful_widget(list, chunks[0], list_state);
        
        // Right side: Edit panel
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
                        
                        // Create owned strings to avoid borrowing issues
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
    
    // All visualizer rendering methods removed for performance optimization
    
    fn render_track_list(
        f: &mut Frame,
        area: Rect,
        tracks: &[panpipe::Track],
        filtered_tracks: &[usize],
        current_track_index: Option<usize>,
        is_playing: bool,
        list_state: &mut ListState
    ) {
        let items: Vec<ListItem> = filtered_tracks
            .iter()
            .enumerate()
            .map(|(_i, &track_idx)| {
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
                
                let content = format!(
                    "{}{} - {} - {}",
                    prefix,
                    track.display_artist(),
                    track.display_title(),
                    track.display_album()
                );
                
                ListItem::new(content).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Library ({} tracks)", filtered_tracks.len()))
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("→ ");
        
        f.render_stateful_widget(list, area, list_state);
    }
    
    // All remaining visualizer rendering methods removed for performance optimization
    
    fn render_player_controls(
        f: &mut Frame, 
        area: Rect, 
        tracks: &[panpipe::Track], 
        current_track_index: Option<usize>, 
        is_playing: bool, 
        volume: f32, 
        repeat_mode: RepeatMode, 
        is_shuffled: bool,
        current_position: Duration,
        total_duration: Option<Duration>
    ) {
        // Create layout for progress bar and controls
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
            let time_str = format!("{} / {}", current_time, total_time);
            
            (ratio, time_str)
        } else {
            let current_secs = current_position.as_secs();
            let current_time = format!("{}:{:02}", current_secs / 60, current_secs % 60);
            (0.0, format!("{} / --:--", current_time))
        };
        
        // Animated progress bar with visual effects
        let progress_color = if is_playing {
            Color::Green // Pulsing green when playing
        } else {
            Color::Yellow // Yellow when paused
        };
        
        let progress_bar = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(Style::default().fg(progress_color).add_modifier(Modifier::BOLD))
            .ratio(progress_ratio)
            .label(time_display);
        
        f.render_widget(progress_bar, chunks[0]);
        
        // Player info and controls
        let current_track_info = if let Some(idx) = current_track_index {
            let track = &tracks[idx];
            format!("♪ {} - {}", track.display_artist(), track.display_title())
        } else {
            "No track selected".to_string()
        };
        
        // Animated status with visual effects
        let status_symbol = if is_playing { "▶" } else { "⏸" };
        let status_text = if is_playing { "Playing" } else { "Paused" };
        let status_color = if is_playing { Color::Green } else { Color::Yellow };
        
        let volume_bar = "█".repeat((volume * 10.0) as usize);
        let volume_empty = "░".repeat(10 - (volume * 10.0) as usize);
        
        let repeat_symbol = match repeat_mode {
            RepeatMode::Off => "🔁",
            RepeatMode::All => "🔁",
            RepeatMode::One => "🔂",
        };
        
        let shuffle_symbol = if is_shuffled { "🔀" } else { "🔀" };
        
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
    
    fn render_settings(f: &mut Frame, area: Rect) {
        let settings_content = vec![
            Line::from(vec![Span::styled("⚙️ Settings", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("⌨️ Keyboard Controls:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Space         Toggle play/pause"),
            Line::from("  p             Play current track"),
            Line::from("  s             Stop playback"),
            Line::from("  n / →         Next track"),
            Line::from("  b / ←         Previous track"),
            Line::from("  ↑ / ↓         Navigate track list"),
            Line::from("  Enter         Select/play highlighted track"),
            Line::from("  + / =         Volume up"),
            Line::from("  -             Volume down"),
            Line::from("  z             Toggle shuffle mode"),
            Line::from("  r             Toggle repeat mode"),
            Line::from("  F5            Refresh library"),
            Line::from("  q / Esc       Quit player"),
            Line::from(""),
            Line::from(vec![Span::styled("🎵 Audio Configuration:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Volume: Controlled via +/- keys"),
            Line::from("  Repeat Mode: Controlled via 'r' key"),
            Line::from("  Shuffle: Controlled via 'z' key"),
            Line::from(""),
            Line::from(vec![Span::styled("📁 Library Management:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Music Directory: Scanned on startup"),
            Line::from("  Metadata Editor: Available in tab 2"),
            Line::from(""),
            Line::from(vec![Span::styled("🔮 Future Features:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  ⭐ Favorites System - Coming Soon"),
            Line::from("  📋 Custom Playlists - Coming Soon"),
            Line::from("  🎯 Queue Management - Coming Soon"),
            Line::from("  💾 Persistent Settings - Coming Soon"),
            Line::from(""),
            Line::from(vec![Span::styled("🔧 Configuration:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Audio Buffer: 65KB (optimized for stability)"),
            Line::from("  Sample Rate: 44.1kHz"),
            Line::from("  Channels: Stereo"),
            Line::from(""),
            Line::from(vec![Span::styled("💡 Tips:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
            Line::from("  • Press ? for help overlay with all keybindings"),
            Line::from("  • Use 1/2/3 to switch between tabs"),
            Line::from("  • Lower system volume to ~75% for best audio quality"),
        ];
        
        let settings_paragraph = Paragraph::new(settings_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Settings & Configuration")
                    .border_style(Style::default().fg(Color::Yellow))
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        
        f.render_widget(settings_paragraph, area);
    }
    
    fn render_status_bar(f: &mut Frame, area: Rect, status_message: Option<(String, Instant)>) {
        let status_text = if let Some((message, timestamp)) = status_message {
            // Show status message for 3 seconds
            if timestamp.elapsed() < Duration::from_secs(3) {
                message
            } else {
                "Ready".to_string()
            }
        } else {
            "Ready".to_string()
        };
        
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, area);
    }
    
    fn render_search_input(f: &mut Frame, area: Rect, search_query: &str, results_count: usize) {
        // Create a centered popup for search input
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
                    .border_style(Style::default().fg(Color::Green))
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));
        
        // Clear the area and render the search input
        f.render_widget(Clear, popup_area);
        f.render_widget(search_input, popup_area);
    }
    
    fn render_playlist_input(f: &mut Frame, area: Rect, playlist_name: &str) {
        // Create a centered popup for playlist name input
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
                    .border_style(Style::default().fg(Color::Blue))
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));
        
        // Clear the area and render the playlist input
        f.render_widget(Clear, popup_area);
        f.render_widget(playlist_input, popup_area);
    }
    
    fn render_playlists_tree_view(
        f: &mut Frame,
        area: Rect,
        playlist_manager: &PlaylistManager,
        playlist_list_state: &mut ListState,
        expanded_playlists: &std::collections::HashSet<String>,
        tracks: &[panpipe::Track],
        _playlist_track_states: &std::collections::HashMap<String, ListState>,
        current_track_index: Option<usize>,
        is_playing: bool,
    ) {
        let playlists = playlist_manager.list_playlists();
        
        // Build tree-view items: playlists + their expanded tracks
        let mut tree_items: Vec<ListItem> = Vec::new();
        
        for (_playlist_idx, playlist) in playlists.iter().enumerate() {
            let stats = playlist_manager.get_playlist_stats(&playlist.id, tracks).unwrap_or_default();
            let is_expanded = expanded_playlists.contains(&playlist.id);
            
            // Playlist header with expand/collapse indicator
            let expand_icon = if is_expanded { "▼" } else { "▶" };
            let playlist_content = format!(
                "{} {} ({} tracks, {})",
                expand_icon,
                playlist.name,
                stats.track_count,
                Self::format_duration(std::time::Duration::from_millis(stats.total_duration))
            );
            
            let playlist_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            
            tree_items.push(ListItem::new(playlist_content).style(playlist_style));
            
            // If expanded, add indented track items
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
        
        // Render the tree-view list
        let tree_list = List::new(tree_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🎵 Playlists (Tree View)")
                    .title_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("→ ");
        
        f.render_stateful_widget(tree_list, area, playlist_list_state);
        
        // Show tree-view instructions at the bottom
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
    
    fn format_duration(duration: std::time::Duration) -> String {
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
    
    fn render_playlist_selector_overlay(f: &mut Frame, area: Rect, playlist_manager: &PlaylistManager, list_state: &mut ListState, track_title: &str) {
        // Create centered popup area
        let popup_area = Self::centered_rect(60, 70, area);
        
        // Clear the background
        f.render_widget(Clear, popup_area);
        
        // Create the popup block
        let block = Block::default()
            .title(format!(" Select Playlist for '{}' ", track_title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        
        f.render_widget(block, popup_area);
        
        // Create inner area for the list
        let inner_area = popup_area.inner(Margin { horizontal: 1, vertical: 1 });
        
        // Get playlists and create items
        let playlists = playlist_manager.list_playlists();
        let mut items = Vec::new();
        
        // Add existing playlists
        for playlist in &playlists {
            // Simple playlist display without duration for now (to avoid complexity)
            items.push(ListItem::new(format!("📋 {}", playlist.name)));
        }
        
        // Add "Create New Playlist" option
        items.push(ListItem::new("➕ Create New Playlist"));
        
        // Create the list widget
        let list = List::new(items)
            .block(Block::default())
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ ");
        
        // Render the list
        f.render_stateful_widget(list, inner_area, list_state);
        
        // Add instructions at the bottom
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
    
    fn render_help_overlay(f: &mut Frame, area: Rect) {
        // Create centered popup area
        let popup_area = Self::centered_rect(80, 70, area);
        
        let help_text = vec![
            Line::from(vec![Span::styled("🎵 BangTunes Help", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("Navigation:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  ↑/↓           Navigate tracks (no auto-play)"),
            Line::from("  1/2/3         Switch tabs (Library/Metadata Editor/Settings)"),
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
            Line::from(vec![Span::styled("Playlists:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  c             Create playlist"),
            Line::from("  Del           Delete playlist"),
            Line::from("  l/Enter       Load playlist"),
            Line::from("  a             Add track to playlist (from Library)"),
            Line::from(""),
            Line::from(vec![Span::styled("Metadata Editor:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from("  Enter         Edit selected track"),
            Line::from("  Ctrl+S        Save changes"),
            Line::from("  Esc           Cancel edit"),
            Line::from("  Ctrl+R        Reset to original"),
            Line::from("  Ctrl+A        Apply suggestions"),
            Line::from(""),
            Line::from(vec![Span::styled("Press ? again to close", Style::default().fg(Color::Yellow))]),
        ];
        
        // Clear the entire screen background first
        let clear_all = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(clear_all, area);
        
        // Create a completely solid background that fills the entire popup area
        use ratatui::widgets::Clear;
        f.render_widget(Clear, popup_area); // This clears the area completely
        
        // Render a solid background block to ensure complete opacity
        let solid_background = Block::default()
            .style(Style::default().bg(Color::Black));
        f.render_widget(solid_background, popup_area);
        
        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .border_style(Style::default().fg(Color::Yellow))
            )
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .wrap(Wrap { trim: true });
        
        f.render_widget(help_paragraph, popup_area);
    }
    
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

    /// Handle audio events from the player (duration learning, track finished, etc.)
    async fn handle_audio_event(&mut self, event: PlayerEvent) -> Result<()> {
        // PlayerEvent already imported at top
        
        match event {
            PlayerEvent::TrackStarted(track) => {
                self.set_status(&format!("▶️ Playing: {}", self.format_track_title(&track)));
            }
            PlayerEvent::TrackFinished(track) => {
                self.set_status(&format!("🔧 DEBUG: TrackFinished set is_playing=false for {}", self.format_track_title(&track)));
                // Just stop playing - don't auto-advance or reset track index
                // This preserves the current track display and progress bar state
                self.is_playing = false;
            }
            PlayerEvent::DurationLearned(learned_track, actual_duration) => {
                // Find the track in our library and update its duration
                if let Some(track_index) = self.tracks.iter().position(|t| t.id == learned_track.id) {
                    // Update the track in our library
                    self.tracks[track_index].duration = Some(actual_duration);
                    self.tracks[track_index].metadata.duration_ms = Some(actual_duration.as_millis() as u64);
                    
                    // Show success message
                    let duration_str = format!("{}:{:02}", 
                        actual_duration.as_secs() / 60, 
                        actual_duration.as_secs() % 60
                    );
                    self.set_status(&format!("📏 Learned duration: {} ({})", 
                        self.format_track_title(&learned_track), 
                        duration_str
                    ));
                    
                    // TODO: Persist the learned duration to database/file for future sessions
                    // This could be done via the behavior tracker or a separate metadata store
                }
            }
            PlayerEvent::TrackPaused => {
                // Ignore premature pause events - only pause if explicitly requested by user
                // The audio player sends false pause events due to sink.empty() immediately after start
                // NOTE: Don't set is_playing = false here - let explicit user pause actions handle that
            }
            PlayerEvent::TrackResumed => {
                self.is_playing = true;
                self.set_status("▶️ Resumed");
            }
            PlayerEvent::TrackStopped => {
                // Implement autoplay logic with false positive protection
                // Only autoplay if we're currently playing and have been for a reasonable duration
                if self.is_playing && self.current_track_index.is_some() {
                    let elapsed = self.last_position_update.elapsed();
                    
                    // Only autoplay if track has been playing for more than 2 seconds
                    // This prevents false positives from sink.empty() immediately after start
                    if elapsed.as_secs() >= 2 {
                        debug!("🎵 Track completed after {}s, triggering autoplay", elapsed.as_secs());
                        
                        // Record track completion
                        if let Some(current_idx) = self.current_track_index {
                            let track = &self.tracks[current_idx];
                            let _ = self.behavior_tracker.handle_event(PlaybackEvent::TrackCompleted {
                                track_id: track.id,
                                timestamp: chrono::Utc::now(),
                            }).await;
                        }
                        
                        // Autoplay next track with strict playlist isolation
                        if self.current_tab == AppTab::Playlists && !self.expanded_playlists.is_empty() {
                            // Autoplay within the expanded playlist only
                            match self.next_track().await {
                                Ok(()) => {
                                    debug!("🎵 Autoplay: Successfully started next track in playlist");
                                }
                                Err(e) => {
                                    debug!("❌ Autoplay failed in playlist: {}", e);
                                    self.is_playing = false;
                                    self.current_track_index = None;
                                    self.set_status("⏹️ Playback stopped - end of playlist");
                                }
                            }
                        } else {
                            // Autoplay in library context
                            match self.next_track().await {
                                Ok(()) => {
                                    debug!("🎵 Autoplay: Successfully started next track in library");
                                }
                                Err(e) => {
                                    debug!("❌ Autoplay failed in library: {}", e);
                                    self.is_playing = false;
                                    self.current_track_index = None;
                                    self.set_status("⏹️ Playback stopped - end of library");
                                }
                            }
                        }
                    } else {
                        debug!("🔍 Ignoring premature TrackStopped event ({}ms elapsed)", elapsed.as_millis());
                    }
                } else {
                    debug!("🔍 Ignoring TrackStopped - not currently playing");
                }
            }
            PlayerEvent::VolumeChanged(volume) => {
                self.volume = volume;
                self.set_status(&format!("🔊 Volume: {}%", (volume * 100.0) as u32));
            }
            PlayerEvent::Error(error) => {
                // Filter out known ALSA underrun errors to avoid UI spam
                let error_str = error.to_string();
                if error_str.contains("underrun occurred") || error_str.contains("snd_pcm_recover") {
                    // Log ALSA underruns but don't show in UI (these are common and non-critical)
                    debug!("🔊 ALSA underrun occurred (audio buffer issue, non-critical)");
                } else {
                    // Show other audio errors in UI
                    self.set_status(&format!("❌ Audio Error: {}", error));
                }
            }
            PlayerEvent::PositionChanged(_position) => {
                // Position updates are handled by update_playback_status
            }
        }
        
        Ok(())
    }

    /// Format track title for display in status messages
    fn format_track_title(&self, track: &panpipe::Track) -> String {
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

// Define AppEvent enum for the interactive client
#[derive(Debug, Clone)]
enum InteractiveEvent {
    Quit,
    Tick,
    Play,
    TogglePlayPause,
    NextTrack,
    PreviousTrack,
    Stop,
    Up,
    Down,
    VolumeUp,
    VolumeDown,
    ToggleRepeat,
    ToggleShuffle,
    // Tab navigation
    SwitchToLibrary,
    SwitchToPlaylists,
    SwitchToMetadataEditor,
    SwitchToSettings,
    // Metadata editor events
    EditTitle,
    EditArtist,
    SaveMetadata,
    CancelEdit,
    ApplySuggestion,
    #[allow(dead_code)] // Used in metadata editor event handling (line 516)
    ResetToOriginal,
    BulkApplySuggestions,
    ClearMetadata,
    // Visualizer events removed
    // UI events
    ShowHelp,
    Input(char),
    Backspace,
    // Search events
    EnterSearch,
    ExitSearch,
    SearchInput(char),
    SearchBackspace,
    // Playlist events

    DeletePlaylist,
    RenamePlaylist,
    AddToPlaylist,
    RemoveFromPlaylist,
    LoadPlaylist,
    TogglePlaylistExpansion, // New: Toggle expand/collapse playlist in tree view
    PlaylistInput(char),
    PlaylistBackspace,
    ConfirmPlaylistCreation,
    CancelPlaylistCreation,
    // Playlist selector overlay events
    SelectPlaylistFromSelector,
    CancelPlaylistSelector,
}

/// Redirect stderr to /dev/null to suppress ALSA error messages that interfere with TUI
fn redirect_stderr_to_null() -> Result<()> {
    
    unsafe {
        // Open /dev/null for writing
        let null_fd = libc::open(
            b"/dev/null\0".as_ptr() as *const libc::c_char,
            libc::O_WRONLY,
        );
        
        if null_fd == -1 {
            return Err(anyhow::anyhow!("Failed to open /dev/null"));
        }
        
        // Duplicate stderr to save original
        let stderr_backup = libc::dup(libc::STDERR_FILENO);
        if stderr_backup == -1 {
            libc::close(null_fd);
            return Err(anyhow::anyhow!("Failed to backup stderr"));
        }
        
        // Redirect stderr to /dev/null
        if libc::dup2(null_fd, libc::STDERR_FILENO) == -1 {
            libc::close(null_fd);
            libc::close(stderr_backup);
            return Err(anyhow::anyhow!("Failed to redirect stderr"));
        }
        
        libc::close(null_fd);
    }
    
    Ok(())
}
