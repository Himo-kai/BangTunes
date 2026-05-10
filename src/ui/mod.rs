// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors

// Terminal UI - the face of BangTunes
// Built with ratatui because ncurses is a pain and this actually works
// Note: Main TUI app is in src/bin/panpipe_interactive.rs

pub mod events; // keyboard/mouse event handling

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


/// Owns the ratatui terminal handle and restores the terminal on drop via `CleanupGuard`.
pub struct TerminalManager {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    _cleanup_guard: CleanupGuard,
}

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // Force terminal cleanup - NO stdout usage to avoid stream conflicts!
        // This runs even if the app panics, ensuring terminal restoration
        let _ = disable_raw_mode();
        
        // Use separate stdout instance to avoid conflicts
        let mut stdout = io::stdout();
        
        // Best effort to restore everything
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = execute!(stdout, DisableMouseCapture);
        let _ = execute!(stdout, cursor::Show);
        
        // Flush to ensure commands are sent
        use std::io::Write;
        let _ = stdout.flush();
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

    /// Force a full terminal redraw (clears screen; next draw() rebuilds everything).
    pub fn clear(&mut self) -> Result<()> {
        self.terminal.clear()?;
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
