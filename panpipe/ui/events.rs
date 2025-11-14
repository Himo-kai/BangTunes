use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AppEvent {
    // UI Events
    Quit,
    Tick,
    Render,
    
    // Playback Events
    Play,
    Stop,
    NextTrack,
    PreviousTrack,
    TogglePlayPause,
    
    // Navigation Events
    Up,
    Down,
    Enter,
    Back,
    
    // Volume Events
    VolumeUp,
    VolumeDown,
    
    // Playlist Events
    ToggleShuffle,
    ToggleRepeat,
    
    // Library Events
    RefreshLibrary,
}

pub struct EventHandler {
    event_sender: mpsc::UnboundedSender<AppEvent>,
    event_receiver: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        Self {
            event_sender,
            event_receiver,
        }
    }
    
    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.event_sender.clone()
    }
    
    pub async fn next_event(&mut self) -> Option<AppEvent> {
        self.event_receiver.recv().await
    }
    
    pub async fn handle_terminal_events(&self) -> Result<()> {
        loop {
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            if let Some(app_event) = self.key_to_app_event(key) {
                                let _ = self.event_sender.send(app_event);
                            }
                        }
                    }
                    Event::Resize(_, _) => {
                        let _ = self.event_sender.send(AppEvent::Render);
                    }
                    _ => {}
                }
            }
            
            // Send periodic tick events
            let _ = self.event_sender.send(AppEvent::Tick);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    fn key_to_app_event(&self, key: KeyEvent) -> Option<AppEvent> {
        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => Some(AppEvent::Quit),
            
            // Playback controls
            KeyCode::Char(' ') => Some(AppEvent::TogglePlayPause),
            KeyCode::Char('p') => Some(AppEvent::Play),
            KeyCode::Char('s') => Some(AppEvent::Stop),
            KeyCode::Char('n') | KeyCode::Right => Some(AppEvent::NextTrack),
            KeyCode::Char('b') | KeyCode::Left => Some(AppEvent::PreviousTrack),
            
            // Navigation
            KeyCode::Up => Some(AppEvent::Up),
            KeyCode::Down => Some(AppEvent::Down),
            KeyCode::Enter => Some(AppEvent::Enter),
            KeyCode::Backspace => Some(AppEvent::Back),
            
            // Volume
            KeyCode::Char('+') | KeyCode::Char('=') => Some(AppEvent::VolumeUp),
            KeyCode::Char('-') => Some(AppEvent::VolumeDown),
            
            // Playlist controls
            KeyCode::Char('z') => Some(AppEvent::ToggleShuffle),
            KeyCode::Char('r') => Some(AppEvent::ToggleRepeat),
            
            // Library
            KeyCode::F(5) => Some(AppEvent::RefreshLibrary),
            
            _ => None,
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
