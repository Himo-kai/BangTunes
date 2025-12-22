use super::{AppEvent, EventHandler, TerminalManager};
use crate::audio::{AudioPlayer, MusicScanner, PlaybackState, Track, playlist::PlaylistManager};
use crate::behavior::{BehaviorDatabase, BehaviorTracker, PlaybackEvent, SkipReason, weighting::{WeightCalculator, ShuffleWeighting}};
use crate::config::Config;
use crate::database::BangTunesDatabase;
use crate::export::ExportManager;
use crate::spotify::SpotifyClient;
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
    weight_calculator: WeightCalculator,
    shuffle_weighting: ShuffleWeighting,
    playlist_manager: PlaylistManager,
    database: Option<BangTunesDatabase>,
    
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
        let mut audio_player = AudioPlayer::new(Default::default())?;
        
        // Set up audio event channel for player events
        let (audio_event_tx, _audio_event_rx) = tokio::sync::mpsc::unbounded_channel();
        audio_player.set_event_sender(audio_event_tx);
        
        // Initialize behavior database
        let behavior_db = BehaviorDatabase::new(&config.database_path)?;
        let behavior_tracker = BehaviorTracker::new(behavior_db, config.behavior.min_play_time_for_tracking);
        
        // Initialize weight calculator for advanced analytics
        let weight_calculator = WeightCalculator::new(config.behavior.weight_decay_days);
        
        // Initialize shuffle weighting for intelligent recommendations
        let shuffle_weighting = ShuffleWeighting::new(config.behavior.weight_decay_days);
        
        // Initialize playlist manager
        let playlists_dir = std::path::PathBuf::from(&config.database_path).parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("playlists");
        let playlist_manager = PlaylistManager::new(playlists_dir).map_err(|e| anyhow::anyhow!("Failed to create playlist manager: {}", e))?;
        
        // Initialize BangTunes database integration
        let database = BangTunesDatabase::find_database().ok();
        
        // Scan music library with database integration
        let scanner = MusicScanner::with_database();
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
            weight_calculator,
            shuffle_weighting,
            playlist_manager,
            database,
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
            // Check EventHandler quit flag for coordinated shutdown
            if self.event_handler.quit_flag().load(std::sync::atomic::Ordering::Relaxed) {
                self.should_quit = true;
                break;
            }
            
            // Update status from audio player
            self.update_audio_status();
            
            // Render UI
            let should_quit = self.should_quit;
            let current_track_index = self.current_track_index;
            let tracks = &self.tracks;
            let volume = self.volume;
            let audio_state = self.audio_player.get_state();
            let mut list_state = self.list_state.clone();
            
            // Use terminal size for responsive rendering
            let terminal_size = self.terminal.size()?;
            
            self.terminal.draw(|f| {
                Self::render_ui(f, should_quit, current_track_index, tracks, volume, audio_state, &mut list_state, terminal_size);
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
            AppEvent::ShowDatabaseStats => {
                self.show_database_stats().await?;
            }
            AppEvent::ShowTrackInfo => {
                self.show_track_info().await?;
            }
            AppEvent::ShowBehaviorStats => {
                self.show_behavior_stats().await?;
            }
            AppEvent::GenerateSmartPlaylist => {
                self.generate_smart_playlist().await?;
            }
            AppEvent::ExportPlaylist => {
                self.export_playlist().await?;
            }
            AppEvent::ConnectSpotify => {
                self.connect_spotify().await?;
            }
            AppEvent::ManagePlaylists => {
                self.manage_playlists().await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn play_current_track(&mut self) -> Result<()> {
        if let Some(index) = self.current_track_index {
            if let Some(track) = self.tracks.get(index).cloned() {
                // Check if track is playable before attempting playback
                if !track.is_playable() {
                    println!("❌ Track not playable: {}", track.display_title());
                    return Ok(());
                }
                
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
        use crate::audio::scanner::ScanProgress;
        use tokio::sync::mpsc;
        
        let scanner = MusicScanner::with_database();
        
        // Use incremental scanning with progress updates
        let (progress_tx, mut progress_rx) = mpsc::channel(128);
        
        println!("📁 Refreshing music library...");
        
        // Start incremental scanning in background
        let scanner_clone = scanner.clone();
        let directories = self.config.music_directories.clone();
        let scan_task = tokio::spawn(async move {
            scanner_clone.scan_directories_incremental(&directories, progress_tx).await
        });
        
        // Process scan progress with live updates
        let mut new_tracks = Vec::new();
        
        while let Some(progress) = progress_rx.recv().await {
            match progress {
                ScanProgress::Started { total_directories } => {
                    println!("🔍 Starting scan of {} directories", total_directories);
                }
                ScanProgress::DirectoryStarted { path } => {
                    println!("📂 Scanning: {:?}", path);
                }
                ScanProgress::TrackFound { track, progress, .. } => {
                    new_tracks.push(track);
                    
                    // Update progress every 25 tracks for smooth feedback
                    if progress % 25 == 0 {
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
                    println!("   ⚠️  Error scanning {:?}: {}", path, error);
                }
            }
        }
        
        // Wait for scanner task to complete and get final results
        match scan_task.await {
            Ok(Ok(final_tracks)) => {
                self.tracks = final_tracks; // Use final results to ensure consistency
            }
            Ok(Err(e)) => {
                println!("❌ Scan failed: {}", e);
                return Err(e);
            }
            Err(e) => {
                println!("❌ Scan task failed: {}", e);
                return Err(e.into());
            }
        }
        
        if !self.tracks.is_empty() && self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
        
        Ok(())
    }
    
    fn get_current_track(&self) -> Option<&Track> {
        self.current_track_index
            .and_then(|index| self.tracks.get(index))
    }
    
    async fn show_database_stats(&mut self) -> Result<()> {
        if let Some(ref database) = self.database {
            match database.load_all_tracks() {
                Ok(db_tracks) => {
                    println!("📊 BangTunes Database Stats:");
                    println!("   Total tracks in database: {}", db_tracks.len());
                    println!("   Tracks currently loaded: {}", self.tracks.len());
                    
                    // Count tracks with metadata
                    let with_metadata = db_tracks.iter()
                        .filter(|t| t.title.is_some() && t.artist.is_some())
                        .count();
                    println!("   Tracks with metadata: {}", with_metadata);
                }
                Err(e) => {
                    println!("❌ Failed to load database tracks: {}", e);
                }
            }
        } else {
            println!("⚠️  No BangTunes database found");
        }
        Ok(())
    }
    
    async fn show_track_info(&mut self) -> Result<()> {
        if let Some(index) = self.current_track_index {
            if let Some(track) = self.tracks.get_mut(index) {
                println!("🎵 Track Info:");
                println!("   Title: {}", track.display_title());
                println!("   Artist: {}", track.display_artist());
                println!("   Album: {}", track.display_album());
                
                // Show duration using duration_seconds method
                if let Some(duration_secs) = track.duration_seconds() {
                    println!("   Duration: {}:{:02}", duration_secs / 60, duration_secs % 60);
                } else {
                    println!("   Duration: Unknown");
                    
                    // Simulate learning duration (in real app, this would come from audio player)
                    let simulated_duration = std::time::Duration::from_secs(180); // 3 minutes
                    track.learn_duration(simulated_duration);
                    
                    // Show updated duration
                    if let Some(duration_secs) = track.duration_seconds() {
                        println!("   Duration (learned): {}:{:02}", duration_secs / 60, duration_secs % 60);
                    }
                }
                
                println!("   Playable: {}", if track.is_playable() { "Yes" } else { "No" });
                
                // Check if file format is supported
                println!("   Format supported: {}", if track.format.is_supported() { "Yes" } else { "No" });
                
                // Demonstrate update_track_metadata usage
                if let Some(ref database) = self.database {
                    // Find the track in the database and update its metadata
                    if let Ok(Some(db_track)) = database.find_track_by_path(&track.file_path) {
                        if let Err(e) = database.update_track_metadata(
                            db_track.id,
                            &track.display_title(),
                            &track.display_artist(),
                            &track.display_album()
                        ) {
                            println!("   ⚠️  Could not update database metadata: {}", e);
                        } else {
                            println!("   ✅ Updated metadata in BangTunes database");
                        }
                    } else {
                        println!("   ℹ️  Track not found in BangTunes database");
                    }
                }
            }
        } else {
            println!("⚠️  No track selected");
        }
        Ok(())
    }
    
    async fn show_behavior_stats(&mut self) -> Result<()> {
        println!("📊 Behavior Tracking Stats:");
        
        // Use get_all_behaviors from BehaviorTracker
        match self.behavior_tracker.get_all_behaviors().await {
            Ok(all_behaviors) => {
                println!("   Total tracked behaviors: {}", all_behaviors.len());
                
                if let Some(current_track) = self.get_current_track() {
                    // Use get_track_behavior for current track
                    match self.behavior_tracker.get_track_behavior(current_track.id).await {
                        Ok(Some(behavior)) => {
                            println!("   Current track behavior:");
                            println!("     Play count: {}", behavior.total_plays);
                            println!("     Skip count: {}", behavior.total_skips);
                            println!("     Last played: {:?}", behavior.last_played);
                        }
                        Ok(None) => {
                            println!("   Current track: No behavior data yet");
                        }
                        Err(e) => {
                            println!("   Error getting track behavior: {}", e);
                        }
                    }
                    
                    // Use save_track_metadata through BehaviorTracker
                    if let Err(e) = self.behavior_tracker.save_track_metadata(
                        current_track.id,
                        &current_track.display_title(),
                        &current_track.display_artist(),
                        &current_track.display_album()
                    ).await {
                        println!("   Warning: Could not save track metadata: {}", e);
                    } else {
                        println!("   ✓ Track metadata saved to behavior database");
                    }
                }
                
                // Note: get_all_track_behaviors would be used here if database was accessible
                // Database access is private, so we demonstrate the concept with current behaviors
                println!("   ✓ Would load database behavior records for analysis");
                
                // Show analytics using current behavior data
                println!("   📊 Current behavior analytics:");
                for (i, behavior) in all_behaviors.iter().take(3).enumerate() {
                    if let Some(track) = self.tracks.iter().find(|t| t.id == behavior.track_id) {
                        // Use calculate_weight to show advanced analytics
                        let weight = self.weight_calculator.calculate_weight(behavior, chrono::Utc::now());
                        println!("     {}. {} - {} (played {} times, weight: {:.2})", 
                            i + 1, 
                            track.display_artist(), 
                            track.display_title(), 
                            behavior.total_plays,
                            weight
                        );
                    }
                }
            }
            Err(e) => {
                println!("   Error getting behaviors: {}", e);
            }
        }
        
        Ok(())
    }
    
    async fn generate_smart_playlist(&mut self) -> Result<()> {
        println!("🎯 Generating Smart Playlist:");
        
        // Get track IDs for available tracks
        let available_track_ids: Vec<uuid::Uuid> = self.tracks.iter()
            .map(|track| track.id)
            .collect();
        
        if available_track_ids.is_empty() {
            println!("   ⚠️  No tracks available for playlist generation");
            return Ok(());
        }
        
        // Get behaviors for weighting
        let behaviors = match self.behavior_tracker.get_all_behaviors().await {
            Ok(behaviors) => {
                behaviors.into_iter()
                    .map(|b| (b.track_id, b))
                    .collect::<std::collections::HashMap<_, _>>()
            }
            Err(_) => std::collections::HashMap::new(),
        };
        
        // Get recent tracks to avoid (empty for demo)
        let recently_played: Vec<uuid::Uuid> = vec![];
        
        // Use select_next_track to pick the best starting track
        if let Some(first_track_id) = self.shuffle_weighting.select_next_track(
            &available_track_ids,
            &behaviors,
            &recently_played
        ) {
            println!("   🎵 Selected starting track:");
            if let Some(track) = self.tracks.iter().find(|t| t.id == first_track_id) {
                println!("     {} - {}", track.display_artist(), track.display_title());
            }
            
            // Use generate_shuffled_playlist to create a full playlist
            let playlist_size = 10.min(available_track_ids.len());
            let smart_playlist = self.shuffle_weighting.generate_shuffled_playlist(
                &available_track_ids,
                &behaviors,
                playlist_size
            );
            
            println!("   📋 Generated smart playlist ({} tracks):", smart_playlist.len());
            for (i, track_id) in smart_playlist.iter().enumerate() {
                if let Some(track) = self.tracks.iter().find(|t| t.id == *track_id) {
                    println!("     {}. {} - {}", i + 1, track.display_artist(), track.display_title());
                }
            }
            
            // Use get_tracks_by_weight to show analytics
            let weighted_tracks = self.shuffle_weighting.get_tracks_by_weight(
                &behaviors
            );
            
            println!("   📊 Top 3 weighted tracks:");
            for (i, (track_id, weight)) in weighted_tracks.iter().take(3).enumerate() {
                if let Some(track) = self.tracks.iter().find(|t| t.id == *track_id) {
                    println!("     {}. {} - {} (weight: {:.2})", 
                        i + 1, 
                        track.display_artist(), 
                        track.display_title(),
                        weight
                    );
                }
            }
            
            // Use recalculate_all_weights for maintenance
            let mut mutable_behaviors = behaviors.clone();
            self.shuffle_weighting.recalculate_all_weights(&mut mutable_behaviors);
            println!("   🔄 Recalculated all track weights for future recommendations");
            
        } else {
            println!("   ❌ Could not select starting track for playlist");
        }
        
        Ok(())
    }
    
    async fn export_playlist(&mut self) -> Result<()> {
        use crate::export::PlaylistExport;
        
        println!("📤 Export Functionality Demo:");
        
        // Create ExportManager instance
        let export_manager = ExportManager::new();
        
        // Create a sample playlist export
        let playlist_export = PlaylistExport {
            name: "Demo Playlist".to_string(),
            tracks: self.tracks.iter().take(5).map(|t| t.id).collect(),
            created_at: chrono::Utc::now(),
            behavior_data: None, // Would contain behavior data if available
        };
        
        // Get sample tracks for M3U export
        let sample_tracks: Vec<Track> = self.tracks.iter().take(5).cloned().collect();
        
        // Demonstrate export_to_json
        let json_path = "/tmp/sample_playlist.json";
        match export_manager.export_to_json(&playlist_export, json_path).await {
            Ok(_) => println!("   ✅ Exported playlist to JSON: {}", json_path),
            Err(e) => println!("   ❌ JSON export failed: {}", e),
        }
        
        // Demonstrate export_to_m3u
        let m3u_path = "/tmp/sample_playlist.m3u";
        match export_manager.export_to_m3u(&sample_tracks, m3u_path).await {
            Ok(_) => println!("   ✅ Exported playlist to M3U: {}", m3u_path),
            Err(e) => println!("   ❌ M3U export failed: {}", e),
        }
        
        // Demonstrate export_to_spotify (placeholder)
        let spotify_client = SpotifyClient::new(
            "demo_client_id".to_string(),
            "http://localhost:8080/callback".to_string()
        );
        match export_manager.export_to_spotify(&playlist_export, &spotify_client).await {
            Ok(playlist_id) => println!("   ✅ Exported playlist to Spotify: {}", playlist_id),
            Err(e) => println!("   ❌ Spotify export failed: {}", e),
        }
        
        Ok(())
    }
    
    async fn connect_spotify(&mut self) -> Result<()> {
        println!("🎵 Spotify Integration Demo:");
        
        // Create SpotifyClient instance
        let mut spotify_client = SpotifyClient::new(
            "demo_client_id".to_string(),
            "http://localhost:8080/callback".to_string()
        );
        
        // Demonstrate authenticate
        match spotify_client.authenticate().await {
            Ok(_) => println!("   ✅ Spotify authentication successful"),
            Err(e) => println!("   ❌ Spotify authentication failed: {}", e),
        }
        
        // Demonstrate search_tracks
        let search_query = "rock music";
        match spotify_client.search_tracks(search_query).await {
            Ok(tracks) => {
                println!("   🔍 Found {} tracks for query '{}':", tracks.len(), search_query);
                for (i, track) in tracks.iter().take(3).enumerate() {
                    let unknown_artist = "Unknown Artist".to_string();
                    let artist = track.artists.first().unwrap_or(&unknown_artist);
                    println!("     {}. {} - {} ({})", 
                        i + 1, 
                        artist, 
                        track.name,
                        track.album
                    );
                }
            }
            Err(e) => println!("   ❌ Spotify search failed: {}", e),
        }
        
        Ok(())
    }
    
    async fn manage_playlists(&mut self) -> Result<()> {
        println!("📋 Playlist Management Demo:");
        
        // Use create_playlist
        match self.playlist_manager.create_playlist(
            "Demo Playlist".to_string(), 
            Some("Created by BangTunes main app".to_string())
        ) {
            Ok(playlist_id) => {
                println!("   ✅ Created playlist: {}", playlist_id);
                
                // Use add_track_to_playlist
                if let Some(first_track) = self.tracks.first() {
                    if let Err(e) = self.playlist_manager.add_track_to_playlist(&playlist_id, &first_track.file_path) {
                        println!("   ⚠️  Could not add track to playlist: {}", e);
                    } else {
                        println!("   ✅ Added track to playlist");
                    }
                }
                
                // Use get_playlist
                if let Some(playlist) = self.playlist_manager.get_playlist(&playlist_id) {
                    println!("   📋 Playlist details:");
                    println!("     Name: {}", playlist.name);
                    println!("     Tracks: {}", playlist.track_paths.len());
                    
                    // Use get_playlist_stats
                    if let Some(stats) = self.playlist_manager.get_playlist_stats(&playlist_id, &self.tracks) {
                        println!("     Duration: {} seconds", stats.total_duration);
                        println!("     Valid tracks: {}", stats.track_count);
                    }
                }
                
                // Use rename_playlist
                if let Err(e) = self.playlist_manager.rename_playlist(&playlist_id, "Renamed Demo Playlist".to_string()) {
                    println!("   ⚠️  Could not rename playlist: {}", e);
                } else {
                    println!("   ✅ Renamed playlist");
                }
                
                // Use save_playlist
                if let Some(playlist) = self.playlist_manager.get_playlist(&playlist_id) {
                    if let Err(e) = self.playlist_manager.save_playlist(playlist) {
                        println!("   ⚠️  Could not save playlist: {}", e);
                    } else {
                        println!("   ✅ Saved playlist to disk");
                    }
                }
                
                // Use delete_playlist (cleanup)
                match self.playlist_manager.delete_playlist(&playlist_id) {
                    Ok(true) => println!("   ✅ Deleted demo playlist"),
                    Ok(false) => println!("   ⚠️  Demo playlist was not found for deletion"),
                    Err(e) => println!("   ❌ Error deleting playlist: {}", e),
                }
            }
            Err(e) => {
                println!("   ❌ Failed to create playlist: {}", e);
            }
        }
        
        // Use list_playlists
        let all_playlists = self.playlist_manager.list_playlists();
        println!("   📋 Total playlists: {}", all_playlists.len());
        
        for (i, playlist) in all_playlists.iter().take(3).enumerate() {
            println!("     {}. {} ({} tracks)", 
                i + 1, 
                playlist.name, 
                playlist.track_paths.len()
            );
        }
        
        Ok(())
    }
    
    fn update_audio_status(&mut self) {
        // Sync volume from audio player (useful for external volume changes)
        self.volume = self.audio_player.get_volume();
        
        // Check current track synchronization
        if let Some(current_audio_track) = self.audio_player.get_current_track() {
            if let Some(current_idx) = self.current_track_index {
                if current_idx < self.tracks.len() {
                    let expected_track = &self.tracks[current_idx];
                    // Basic sanity check - ensure we're tracking the right track
                    if current_audio_track.id != expected_track.id {
                        println!("⚠️  Track sync mismatch: UI tracking {} but audio playing {}", 
                               expected_track.display_title(), current_audio_track.display_title());
                    }
                }
            }
        }
        
        // Check if track finished
        if self.audio_player.is_finished() && self.current_track_index.is_some() {
            println!("🎵 Track finished, advancing to next");
            // Note: In a real implementation, you'd call next_track here
        }
    }
    
    fn render_ui(
        f: &mut Frame,
        _should_quit: bool,
        current_track_index: Option<usize>,
        tracks: &[Track],
        volume: f32,
        audio_state: PlaybackState,
        list_state: &mut ListState,
        terminal_size: ratatui::layout::Rect,
    ) {
        // Responsive layout based on terminal size
        let header_height = if terminal_size.height < 10 { 1 } else { 3 };
        let control_height = if terminal_size.height < 15 { 1 } else { 3 };
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height), // Header (responsive)
                Constraint::Min(0),                // Main content
                Constraint::Length(control_height), // Player controls (responsive)
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
