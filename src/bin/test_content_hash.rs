use panpipe::audio::{MusicScanner, Track};
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("🔍 Content Hash Test Utility");
    println!("============================");
    
    // Get music directory from args or use default
    let music_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Try common music directories
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(format!("{}/Music", home))
        });
    
    if !music_dir.exists() {
        println!("❌ Music directory not found: {}", music_dir.display());
        println!("Usage: cargo run --bin test_content_hash [music_directory]");
        return Ok(());
    }
    
    println!("📁 Scanning directory: {}", music_dir.display());
    
    // Scan for tracks
    let scanner = MusicScanner::new();
    let tracks = scanner.scan_directories(&[music_dir])?;
    
    println!("🎵 Found {} tracks", tracks.len());
    println!();
    
    // Group tracks by content hash
    let mut hash_groups: HashMap<Option<u64>, Vec<&Track>> = HashMap::new();
    let mut tracks_with_hash = 0;
    let mut tracks_without_hash = 0;
    
    for track in &tracks {
        if track.content_hash.is_some() {
            tracks_with_hash += 1;
        } else {
            tracks_without_hash += 1;
        }
        
        hash_groups.entry(track.content_hash)
            .or_insert_with(Vec::new)
            .push(track);
    }
    
    // Report statistics
    println!("📈 Content Hash Statistics:");
    println!("  ✅ Tracks with hash: {}", tracks_with_hash);
    println!("  ❌ Tracks without hash: {}", tracks_without_hash);
    println!("  🔢 Unique hash groups: {}", hash_groups.len());
    println!();
    
    // Find potential duplicates (same hash, different paths)
    println!("🔍 Potential Duplicates (same content hash):");
    let mut duplicates_found = false;
    
    for (hash, group) in &hash_groups {
        if let Some(hash_value) = hash {
            if group.len() > 1 {
                duplicates_found = true;
                println!("  Hash: {:016x} ({} files)", hash_value, group.len());
                for track in group {
                    let filename = track.file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    println!("    📄 {}", filename);
                }
                println!();
            }
        }
    }
    
    if !duplicates_found {
        println!("  ✨ No duplicates found (all files have unique content)");
    }
    
    // Test specific duplicate file if it exists
    println!("🎯 Testing Your Duplicate File:");
    let original_name = "The Maine, Beach Weather - thoughts i have while lying in bed.mp3";
    let duplicate_name = "duplicate_music_file.mp3";
    
    let original = tracks.iter().find(|t| {
        t.file_path.file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |n| n == original_name)
    });
    
    let duplicate = tracks.iter().find(|t| {
        t.file_path.file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |n| n == duplicate_name)
    });
    
    match (original, duplicate) {
        (Some(orig), Some(dup)) => {
            println!("  📄 Original: {}", original_name);
            println!("     Hash: {:?}", orig.content_hash);
            println!("  📄 Duplicate: {}", duplicate_name);
            println!("     Hash: {:?}", dup.content_hash);
            
            if orig.is_same_content(dup) {
                println!("  ✅ SUCCESS: Files have identical content hashes!");
            } else {
                println!("  ❌ ISSUE: Files have different content hashes");
            }
        }
        (None, None) => println!("  ⚠️  Neither original nor duplicate file found"),
        (Some(_), None) => println!("  ⚠️  Original found but duplicate not found"),
        (None, Some(_)) => println!("  ⚠️  Duplicate found but original not found"),
    }
    
    Ok(())
}
