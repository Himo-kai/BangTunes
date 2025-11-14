use super::{AudioConfig, Track};
use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackStarted(Track),
    TrackPaused,
    TrackResumed,
    TrackStopped,
    TrackFinished(Track),
    DurationLearned(Track, Duration), // Track with learned duration from actual playback
    PositionChanged(Duration),
    VolumeChanged(f32),
    Error(String),
}

pub struct AudioPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Arc<Mutex<Option<Sink>>>,
    current_track: Arc<Mutex<Option<Track>>>,
    state: Arc<Mutex<PlaybackState>>,
    config: AudioConfig,
    event_sender: Option<mpsc::UnboundedSender<PlayerEvent>>,
    // Duration learning fields
    playback_start_time: Arc<Mutex<Option<std::time::Instant>>>,
    track_for_learning: Arc<Mutex<Option<Track>>>, // Track to learn duration for
}

impl AudioPlayer {
    pub fn new(config: AudioConfig) -> Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        
        Ok(Self {
            _stream: stream,
            stream_handle,
            sink: Arc::new(Mutex::new(None)),
            current_track: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            config,
            event_sender: None,
            playback_start_time: Arc::new(Mutex::new(None)),
            track_for_learning: Arc::new(Mutex::new(None)),
        })
    }
    
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<PlayerEvent>) {
        self.event_sender = Some(sender);
    }
    
    pub fn play_track(&self, track: Track) -> Result<()> {
        // Stop current playback
        self.stop()?;
        
        // Create new sink
        let sink = Sink::try_new(&self.stream_handle)?;
        sink.set_volume(self.config.volume);
        
        // Load and decode the audio file with robust error handling
        let file = match File::open(&track.file_path) {
            Ok(f) => f,
            Err(e) => {
                // Send error event instead of crashing
                if let Some(sender) = &self.event_sender {
                    let _ = sender.send(PlayerEvent::Error(format!("Failed to open file: {}", e)));
                }
                return Err(anyhow::anyhow!("Failed to open audio file: {}", e));
            }
        };
        
        // Decode audio file - now with proper M4A/AAC codec support via Symphonia
        let source = match Decoder::new(BufReader::new(file)) {
            Ok(s) => s,
            Err(e) => {
                // Send error event instead of crashing
                if let Some(sender) = &self.event_sender {
                    let _ = sender.send(PlayerEvent::Error(format!("Unsupported audio format or corrupted file: {}", e)));
                }
                return Err(anyhow::anyhow!("Failed to decode audio file '{}': {}. This file may be corrupted or use an unsupported format.", track.file_path.display(), e));
            }
        };
        
        // Start playback with fade in
        sink.append(source);
        
        // Apply fade in effect for smooth start
        self.fade_in(&sink)?;
        
        // Update state
        {
            let mut sink_guard = self.sink.lock().unwrap();
            *sink_guard = Some(sink);
        }
        
        {
            let mut track_guard = self.current_track.lock().unwrap();
            *track_guard = Some(track.clone());
        }
        
        {
            let mut state_guard = self.state.lock().unwrap();
            *state_guard = PlaybackState::Playing;
        }
        
        // Start duration learning if track has no duration
        if track.duration.is_none() {
            let mut start_time_guard = self.playback_start_time.lock().unwrap();
            *start_time_guard = Some(std::time::Instant::now());
            
            let mut learning_track_guard = self.track_for_learning.lock().unwrap();
            *learning_track_guard = Some(track.clone());
        }
        
        // NOTE: Removed problematic completion monitor that was interfering with playback
        // Duration learning will be handled by the existing UI-based completion detection
        
        // Send success event
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(PlayerEvent::TrackStarted(track));
        }
        
        Ok(())
    }
    
    pub fn pause(&self) -> Result<()> {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            // Apply quick fade out before pausing for smooth transition
            let _ = self.fade_out_quick(sink);
            
            sink.pause();
            
            let mut state_guard = self.state.lock().unwrap();
            *state_guard = PlaybackState::Paused;
            
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(PlayerEvent::TrackPaused);
            }
        }
        
        Ok(())
    }
    
    pub fn resume(&self) -> Result<()> {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.play();
            
            // Apply fade in effect when resuming for smooth transition
            let _ = self.fade_in(sink);
            
            let mut state_guard = self.state.lock().unwrap();
            *state_guard = PlaybackState::Playing;
            
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(PlayerEvent::TrackResumed);
            }
        }
        
        Ok(())
    }
    
    pub fn stop(&self) -> Result<()> {
        {
            let mut sink_guard = self.sink.lock().unwrap();
            if let Some(sink) = sink_guard.as_ref() {
                // Apply fade out effect before stopping
                let _ = self.fade_out(sink);
                
                // Now stop the sink
                sink.stop();
            }
            // Take the sink to remove it
            sink_guard.take();
        }
        
        {
            let mut state_guard = self.state.lock().unwrap();
            *state_guard = PlaybackState::Stopped;
        }
        
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(PlayerEvent::TrackStopped);
        }
        
        Ok(())
    }
    
    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        let clamped_volume = volume.clamp(0.0, 1.0);
        self.config.volume = clamped_volume;
        
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.set_volume(clamped_volume);
        }
        
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(PlayerEvent::VolumeChanged(clamped_volume));
        }
        
        Ok(())
    }
    
    pub fn get_state(&self) -> PlaybackState {
        self.state.lock().unwrap().clone()
    }
    
    pub fn get_current_track(&self) -> Option<Track> {
        self.current_track.lock().unwrap().clone()
    }
    
    pub fn is_finished(&self) -> bool {
        self.sink.lock().unwrap()
            .as_ref()
            .map(|sink| sink.empty())
            .unwrap_or(true)
    }
    
    pub fn get_volume(&self) -> f32 {
        self.config.volume
    }

    /// Smooth fade in effect for professional track start
    fn fade_in(&self, sink: &Sink) -> Result<()> {
        let target_volume = self.config.volume;
        let fade_duration = self.config.fade_in_duration;
        
        if fade_duration == 0 {
            // No fade - set volume immediately
            sink.set_volume(target_volume);
            return Ok(());
        }
        
        // Start from silence and perform immediate fade
        sink.set_volume(0.0);
        
        let fade_steps = 10; // Fewer steps for immediate effect
        let step_duration = fade_duration / fade_steps;
        let volume_step = target_volume / fade_steps as f32;
        
        // Perform fade synchronously for immediate effect
        for step in 1..=fade_steps {
            let current_volume = volume_step * step as f32;
            sink.set_volume(current_volume);
            
            // Small delay for smooth transition
            std::thread::sleep(std::time::Duration::from_millis(step_duration));
        }
        
        // Ensure final volume is exact
        sink.set_volume(target_volume);
        
        Ok(())
    }
    
    /// Smooth fade out effect for professional track stop
    fn fade_out(&self, sink: &Sink) -> Result<()> {
        let current_volume = self.config.volume;
        let fade_duration = self.config.fade_out_duration;
        
        if fade_duration == 0 {
            // No fade - stop immediately
            return Ok(());
        }
        
        let fade_steps = 15; // 15 steps for quick but smooth fade out
        let step_duration = fade_duration / fade_steps;
        let volume_step = current_volume / fade_steps as f32;
        
        // Perform fade out synchronously for immediate effect
        for step in 1..=fade_steps {
            let new_volume = current_volume - (volume_step * step as f32);
            sink.set_volume(new_volume.max(0.0));
            
            std::thread::sleep(std::time::Duration::from_millis(step_duration));
        }
        
        // Ensure final silence
        sink.set_volume(0.0);
        
        Ok(())
    }
    
    /// Quick fade out for pause transitions (shorter duration)
    fn fade_out_quick(&self, sink: &Sink) -> Result<()> {
        let current_volume = self.config.volume;
        let fade_duration = 100; // Quick 100ms fade for pause
        
        let fade_steps = 10; // 10 steps for quick fade
        let step_duration = fade_duration / fade_steps;
        let volume_step = current_volume / fade_steps as f32;
        
        // Perform quick fade out synchronously
        for step in 1..=fade_steps {
            let new_volume = current_volume - (volume_step * step as f32);
            sink.set_volume(new_volume.max(0.0));
            
            std::thread::sleep(std::time::Duration::from_millis(step_duration));
        }
        
        Ok(())
    }

}
