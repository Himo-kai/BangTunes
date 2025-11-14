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
    // Initialize configuration
    let config = Config::load()?;
    
    // Initialize and run the application
    let mut app = App::new(config).await?;
    app.run().await?;
    
    Ok(())
}
