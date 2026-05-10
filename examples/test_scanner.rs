// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors

use panpipe::audio::MusicScanner;
use std::path::PathBuf;

fn find_music_directory() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let candidates = vec![
        home.join("Music"),
        home.join("Downloads"),
        home.join("BangTunes").join("downloads"),
        home.join("Builds").join("BangTunes").join("downloads"),
        // Add current directory as fallback
        std::env::current_dir().ok()?.join("downloads"),
    ];
    
    candidates.into_iter().find(|p| p.exists() && p.is_dir())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🎵 PanPipe Music Scanner Test");
    println!("============================");
    
    let music_dir = match find_music_directory() {
        Some(dir) => dir,
        None => {
            println!("❌ No music directory found. Checked:");
            if let Some(home) = dirs::home_dir() {
                println!("   - {}", home.join("Music").display());
                println!("   - {}", home.join("Downloads").display());
                println!("   - {}", home.join("BangTunes").join("downloads").display());
                println!("   - {}", home.join("Builds").join("BangTunes").join("downloads").display());
            }
            if let Ok(cwd) = std::env::current_dir() {
                println!("   - {}", cwd.join("downloads").display());
            }
            return Ok(());
        }
    };
    
    println!("📁 Scanning music directory: {:?}", music_dir);
    
    let scanner = MusicScanner::new();
    
    match scanner.scan_directory(&music_dir) {
        Ok(tracks) => {
            println!("✅ Found {} music files", tracks.len());
            println!();
            
            // Show first 10 tracks
            for (i, track) in tracks.iter().take(10).enumerate() {
                println!("{}. {}", i + 1, track.display_title());
                println!("   Artist: {}", track.display_artist());
                println!("   Album: {}", track.display_album());
                println!("   Format: {:?}", track.format);
                if let Some(duration) = track.duration_seconds() {
                    let minutes = duration / 60;
                    let seconds = duration % 60;
                    println!("   Duration: {}:{:02}", minutes, seconds);
                }
                println!("   Path: {:?}", track.file_path);
                println!();
            }
            
            if tracks.len() > 10 {
                println!("... and {} more tracks", tracks.len() - 10);
            }
            
            // Show format breakdown
            let mut format_counts = std::collections::HashMap::new();
            for track in &tracks {
                *format_counts.entry(format!("{:?}", track.format)).or_insert(0) += 1;
            }
            
            println!("\n📊 Format breakdown:");
            for (format, count) in format_counts {
                println!("   {}: {} files", format, count);
            }
        }
        Err(e) => {
            println!("❌ Error scanning directory: {}", e);
        }
    }
    
    Ok(())
}
