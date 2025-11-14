// Terminal UI - the face of PanPipe
// Built with ratatui for a modern, responsive terminal interface

mod app;        // main application state and event loop
mod components; // reusable UI widgets
pub mod events; // keyboard/mouse event handling

pub use app::App;
pub use events::{AppEvent, EventHandler};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;


pub struct TerminalManager {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    _cleanup_guard: CleanupGuard,
}

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // Force terminal cleanup - NO stdout usage to avoid stream conflicts!
        let _ = disable_raw_mode();
        
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        
        // Force cursor show
        let _ = execute!(stdout, cursor::Show);
    }
}

impl TerminalManager {
    pub fn new() -> Result<Self> {
        // Ensure clean terminal state first
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        
        // Now set up terminal properly
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        
        Ok(Self { 
            terminal,
            _cleanup_guard: CleanupGuard,
        })
    }

    pub fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    pub fn size(&self) -> Result<ratatui::layout::Rect> {
        let size = self.terminal.size()?;
        Ok(ratatui::layout::Rect::new(0, 0, size.width, size.height))
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        // Explicit cleanup - NO stdout usage to avoid stream conflicts!
        let _ = self.terminal.clear();
        let _ = self.terminal.show_cursor();
        
        // CleanupGuard will handle the rest
    }
}
