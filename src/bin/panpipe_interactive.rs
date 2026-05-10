// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors

use anyhow::Result;
use clap::Parser;
use crossterm::{
    cursor,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use panpipe::{
    audio::{MusicScanner, scanner::ScanProgress},
    config::Config,
};
use std::{io, panic, path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time::sleep};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "panpipe_interactive")]
#[command(about = "A terminal-based music player with intelligent behavior tracking")]
struct Args {
    /// Enable developer logging (stderr + debug output)
    #[arg(long)]
    dev: bool,

    /// Optional music directory to scan (overrides config music_directories)
    /// This exists to support BangTunes launching PanPipe with its downloads folder
    #[arg(value_name = "MUSIC_DIR")]
    music_dir: Option<PathBuf>,

    /// Start playing a specific track file
    #[arg(long, value_name = "PATH")]
    play_track: Option<PathBuf>,
}

fn init_logging(dev: bool) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    // Create logs directory in project root
    let log_dir = PathBuf::from("logs");
    std::fs::create_dir_all(&log_dir)?;

    // Daily rotating file appender
    let file_appender = tracing_appender::rolling::daily(&log_dir, "panpipe.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // Base filter: info level for general logs, debug for panpipe
    let base_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,panpipe=debug"));

    // Build subscriber with conditional stderr layer
    let subscriber = tracing_subscriber::fmt()
        .with_writer(file_writer)
        .with_target(true)
        .with_level(true)
        .with_ansi(false)
        .with_env_filter(base_filter)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    if dev {
        eprintln!("🔧 Dev mode: Debug output enabled to stderr + file");
    }

    // Return guard so it is dropped at the end of main, flushing buffered log lines
    Ok(guard)
}

/// Force terminal restoration - called on panic and normal exit
fn restore_terminal() {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture, cursor::Show);
}

/// Redirect stderr to /dev/null to suppress ALSA error messages that interfere with TUI
fn redirect_stderr_to_null() -> Result<()> {
    unsafe {
        // Open /dev/null for writing
        let null_fd = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);

        if null_fd == -1 {
            return Err(anyhow::anyhow!("Failed to open /dev/null"));
        }

        // Duplicate stderr to save original
        let stderr_backup = libc::dup(libc::STDERR_FILENO);
        if stderr_backup == -1 {
            libc::close(null_fd);
            return Err(anyhow::anyhow!("Failed to backup stderr"));
        }

        // Redirect stderr to /dev/null
        if libc::dup2(null_fd, libc::STDERR_FILENO) == -1 {
            libc::close(null_fd);
            libc::close(stderr_backup);
            return Err(anyhow::anyhow!("Failed to redirect stderr"));
        }

        libc::close(null_fd);
        // stderr_backup is no longer needed once dup2 redirected stderr;
        // close it to avoid leaking the fd for the life of the process.
        libc::close(stderr_backup);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook FIRST to ensure terminal cleanup on panic
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        // Restore terminal BEFORE printing panic info
        restore_terminal();

        // Print panic info to stderr (which will be visible after terminal restore)
        eprintln!("\n❌ PanPipe crashed! Terminal restored.");
        eprintln!("Error details: {}", info);

        // Call the default panic handler for backtrace
        default_panic(info);
    }));

    // Parse CLI arguments
    let args = Args::parse();

    // Initialize logging system. Guard must live until end of main to flush buffered log lines.
    let _log_guard = init_logging(args.dev)?;

    info!("🎵 PanPipe Interactive starting up");

    // Only redirect stderr if NOT in dev mode (dev mode needs stderr for debug output)
    let _stderr_redirect = if !args.dev {
        info!("Redirecting stderr to suppress ALSA errors");
        Some(redirect_stderr_to_null())
    } else {
        info!("Dev mode: keeping stderr for debug output");
        None
    };

    // Initialize configuration
    let mut config = Config::load()?;

    // If BangTunes supplied a music dir, honor it (override config)
    if let Some(dir) = args.music_dir.clone() {
        info!("Using command-line music directory: {:?}", dir);
        config.music_directories = vec![dir];
    }

    // Print startup banner
    println!("🎵 BangTunes - Terminal Music Player");
    println!("===================================");
    println!("Loading your music library...");

    // Initialize music scanner with BangTunes database integration
    let scanner = MusicScanner::with_database();
    let (progress_tx, mut progress_rx) = mpsc::channel(128);

    println!("📁 Scanning music directories...");

    // Start incremental scanning in background
    let scanner_task = {
        let scanner = scanner.clone();
        let directories = config.music_directories.clone();
        tokio::spawn(async move {
            scanner.scan_directories_incremental(&directories, progress_tx).await
        })
    };

    // Process scan progress with live updates
    let mut all_tracks = Vec::new();
    let mut scan_error_count: usize = 0;

    while let Some(progress) = progress_rx.recv().await {
        match progress {
            ScanProgress::Started { total_directories } => {
                println!("🔍 Starting scan of {} directories", total_directories);
            }
            ScanProgress::DirectoryStarted { path } => {
                println!("📂 Scanning: {:?}", path);
            }
            ScanProgress::TrackFound { track, progress, .. } => {
                all_tracks.push(*track);

                // Update progress every 50 tracks for smooth feedback
                if progress % 50 == 0 {
                    println!("   📀 Found {} tracks so far...", progress);
                }
            }
            ScanProgress::DirectoryCompleted { path, tracks_found } => {
                println!("   ✅ {:?}: {} tracks", path, tracks_found);
            }
            ScanProgress::Completed { total_tracks } => {
                println!("🎵 Scan complete: {} tracks total", total_tracks);
                break;
            }
            ScanProgress::Error { path, error } => {
                info!("⚠️  Scan error {:?}: {}", path, error);
                scan_error_count += 1;
            }
        }
    }

    // Wait for scanner task to complete and get final results
    match scanner_task.await {
        Ok(Ok(final_tracks)) => {
            all_tracks = final_tracks;
        }
        Ok(Err(e)) => {
            eprintln!("❌ Scanner error: {}", e);
        }
        Err(e) => {
            eprintln!("❌ Scanner task error: {}", e);
        }
    }

    if all_tracks.is_empty() {
        eprintln!("❌ No music files found in configured directories!");
        eprintln!("Please check your music directories in the config.");
        return Ok(());
    }

    println!("✅ Loaded {} tracks total", all_tracks.len());
    println!("🚀 Starting BangTunes...\n");

    // Small delay to let user see the loading info
    sleep(Duration::from_millis(1500)).await;

    // Initialize the interactive app
    let mut app = panpipe::app::InteractiveApp::new(config, all_tracks).await?;
    app.scan_skipped_count = scan_error_count;

    // If a specific track was requested, try to play it
    if let Some(track_path) = args.play_track {
        app.play_specific_track(&track_path).await?;
    }

    // Run the interactive interface
    app.run().await?;

    println!("\n👋 Thanks for using BangTunes!");
    Ok(())
}
