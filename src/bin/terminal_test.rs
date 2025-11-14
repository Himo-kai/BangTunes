use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{io, time::Duration};
use tokio::time::sleep;

struct MinimalTerminal {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl MinimalTerminal {
    fn new() -> Result<Self> {
        // Clean slate approach
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        
        // Fresh setup
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        
        Ok(Self { terminal })
    }
    
    fn render_test(&mut self) -> Result<()> {
        self.terminal.draw(|f| {
            let size = f.area();
            
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(size);
            
            let items = vec![
                ListItem::new("Test Item 1"),
                ListItem::new("Test Item 2"),
                ListItem::new("Test Item 3"),
            ];
            
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Minimal Test"));
            
            f.render_widget(list, chunks[0]);
        })?;
        
        Ok(())
    }
}

impl Drop for MinimalTerminal {
    fn drop(&mut self) {
        let _ = self.terminal.clear();
        let _ = self.terminal.show_cursor();
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // NO stdout usage after terminal init - this was causing the corruption!
    
    let mut terminal = MinimalTerminal::new()?;
    
    // Simple render loop
    for _i in 0..50 {
        // Check for quit key
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
        
        // Render without any stdout interference
        terminal.render_test()?;
        
        sleep(Duration::from_millis(100)).await;
    }
    
    // Only use stdout after terminal cleanup
    Ok(())
}
