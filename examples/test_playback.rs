// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors

use panpipe::audio::{AudioPlayer, MusicScanner};
use panpipe::config::Config;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

fn find_music_directory() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let candidates = vec![
        home.join("Music"),
        home.join("Downloads"),
        home.join("BangTunes").join("downloads"),
        home.join("Builds").join("BangTunes").join("downloads"),
        std::env::current_dir().ok()?.join("downloads"),
    ];
    
    candidates.into_iter().find(|p| p.exists() && p.is_dir())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🎵 PanPipe Audio Playback Test");
    println!("==============================");
    
    let music_dir = match find_music_directory() {
        Some(dir) => dir,
        None => {
            println!("❌ No music directory found. Use the scanner test to see checked locations.");
            return Ok(());
        }
    };
    
    println!("📁 Scanning for music files...");
    let scanner = MusicScanner::new();
    let tracks = scanner.scan_directory(&music_dir)?;
    
    if tracks.is_empty() {
        println!("❌ No music files found");
        return Ok(());
    }
    
    // Get the first playable track
    let test_track = tracks.into_iter()
        .find(|track| track.is_playable())
        .ok_or_else(|| anyhow::anyhow!("No playable tracks found"))?;
    
    println!("🎧 Testing playback with:");
    println!("   Title: {}", test_track.display_title());
    println!("   Artist: {}", test_track.display_artist());
    println!("   Path: {:?}", test_track.file_path);
    
    // Initialize audio player
    let config = Config::default();
    let mut player = AudioPlayer::new(config.into())?;
    
    println!("\n▶️  Starting playback...");
    player.play_track(test_track.clone())?;
    
    // Play for 10 seconds
    println!("🎶 Playing for 10 seconds...");
    sleep(Duration::from_secs(10)).await;
    
    // Test pause
    println!("⏸️  Pausing...");
    player.pause()?;
    sleep(Duration::from_secs(2)).await;
    
    // Test resume
    println!("▶️  Resuming...");
    player.resume()?;
    sleep(Duration::from_secs(5)).await;
    
    // Test volume control
    println!("🔊 Testing volume control...");
    player.set_volume(0.3)?;
    println!("   Volume set to 30%");
    sleep(Duration::from_secs(3)).await;
    
    player.set_volume(0.8)?;
    println!("   Volume set to 80%");
    sleep(Duration::from_secs(3)).await;
    
    // Stop playback
    println!("⏹️  Stopping playback...");
    player.stop()?;
    
    println!("✅ Playback test completed successfully!");
    println!("🎉 PanPipe audio engine is working!");
    
    Ok(())
}
