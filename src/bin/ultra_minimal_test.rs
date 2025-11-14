use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders},
    Terminal,
};
use std::{io, time::Duration};

fn main() -> Result<()> {
    // Ultra-minimal test - no async, no tokio, no complex event handling
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Clear and draw once
    terminal.clear()?;
    
    // Single render
    terminal.draw(|f| {
        let size = f.area();
        let block = Block::default()
            .title("Ultra Minimal Test - Press 'q' to quit")
            .borders(Borders::ALL);
        f.render_widget(block, size);
    })?;
    
    // Simple event loop - no async
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }
    
    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    
    Ok(())
}
