// PanPipe - Terminal Music Player
// Started as a simple player, grew into something with smart features
// Now integrated into BangTunes for seamless music discovery -> playback

mod audio;
mod behavior;
mod config;
mod export;
mod spotify;
mod ui;

use anyhow::Result;
use config::Config;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config - falls back to defaults if missing
    let config = Config::load()?;
    
    // Fire up the TUI and let it rip
    let mut app = App::new(config).await?;
    app.run().await?;
    
    Ok(())
}
