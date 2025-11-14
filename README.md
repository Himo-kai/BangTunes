# Bang Tunes — Unified Music Discovery & Playback System

    ╔──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╗
    │                                                                                                                              │
    │   ▄▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄  ▄▄        ▄  ▄▄▄▄▄▄▄▄▄▄▄       ▄▄▄▄▄▄▄▄▄▄▄  ▄         ▄  ▄▄        ▄  ▄▄▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄▄▄   │
    │  ▐░░░░░░░░░░▌ ▐░░░░░░░░░░░▌▐░░▌      ▐░▌▐░░░░░░░░░░░▌     ▐░░░░░░░░░░░▌▐░▌       ▐░▌▐░░▌      ▐░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌  │
    │  ▐░█▀▀▀▀▀▀▀█░▌▐░█▀▀▀▀▀▀▀█░▌▐░▌░▌     ▐░▌▐░█▀▀▀▀▀▀▀▀▀       ▀▀▀▀█░█▀▀▀▀ ▐░▌       ▐░▌▐░▌░▌     ▐░▌▐░█▀▀▀▀▀▀▀▀▀ ▐░█▀▀▀▀▀▀▀▀▀   │
    │  ▐░▌       ▐░▌▐░▌       ▐░▌▐░▌▐░▌    ▐░▌▐░▌                    ▐░▌     ▐░▌       ▐░▌▐░▌▐░▌    ▐░▌▐░▌          ▐░▌            │
    │  ▐░█▄▄▄▄▄▄▄█░▌▐░█▄▄▄▄▄▄▄█░▌▐░▌ ▐░▌   ▐░▌▐░▌ ▄▄▄▄▄▄▄▄           ▐░▌     ▐░▌       ▐░▌▐░▌ ▐░▌   ▐░▌▐░█▄▄▄▄▄▄▄▄▄ ▐░█▄▄▄▄▄▄▄▄▄   │
    │  ▐░░░░░░░░░░▌ ▐░░░░░░░░░░░▌▐░▌  ▐░▌  ▐░▌▐░▌▐░░░░░░░░▌          ▐░▌     ▐░▌       ▐░▌▐░▌  ▐░▌  ▐░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌  │
    │  ▐░█▀▀▀▀▀▀▀█░▌▐░█▀▀▀▀▀▀▀█░▌▐░▌   ▐░▌ ▐░▌▐░▌ ▀▀▀▀▀▀█░▌          ▐░▌     ▐░▌       ▐░▌▐░▌   ▐░▌ ▐░▌▐░█▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀█░▌  │
    │  ▐░▌       ▐░▌▐░▌       ▐░▌▐░▌    ▐░▌▐░▌▐░▌       ▐░▌          ▐░▌     ▐░▌       ▐░▌▐░▌    ▐░▌▐░▌▐░▌                    ▐░▌  │
    │  ▐░█▄▄▄▄▄▄▄█░▌▐░▌       ▐░▌▐░▌     ▐░▐░▌▐░█▄▄▄▄▄▄▄█░▌          ▐░▌     ▐░█▄▄▄▄▄▄▄█░▌▐░▌     ▐░▐░▌▐░█▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄█░▌  │
    │  ▐░░░░░░░░░░▌ ▐░▌       ▐░▌▐░▌      ▐░░▌▐░░░░░░░░░░░▌          ▐░▌     ▐░░░░░░░░░░░▌▐░▌      ▐░░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌  │
    │   ▀▀▀▀▀▀▀▀▀▀   ▀         ▀  ▀        ▀▀  ▀▀▀▀▀▀▀▀▀▀▀            ▀       ▀▀▀▀▀▀▀▀▀▀▀  ▀        ▀▀  ▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀   │
    │                                                                                                                              │
    │                                                 created by Himokai                                                           │
    ╚──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╝

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Termux%20%7C%20WSL-lightgrey)](#installation)

Bang Tunes is a **complete music ecosystem** that helps you discover new music based on tracks you already like. It searches YouTube Music for similar songs, downloads them, and includes both basic and advanced playback capabilities - all in one unified package.

**Perfect for**: People who spend a lot of time in terminals and want to find new music without dealing with streaming apps.

**What it does**: Searches YouTube Music, downloads audio files, organizes everything in a database, and provides both basic playback (system players) and advanced playback (integrated TUI player with smart features).

## Features

### Music Discovery & Download

- **Persistent SQLite Library**: Dedupe by `youtube_id`, query by artist/album, track file paths
- **Rich Visual Interface**: Styled messages + progress bars during downloads
- **Metadata Enhancement**: YouTube Music API integration via `ytmusicapi`, file tagging with `mutagen`
- **Batch Processing**: Seed→pool→50-track batches workflow, tracked in DB and resumable
- **Audio Quality**: High-quality opus downloads with embedded thumbnails and metadata
- **Anti-Bot Protection**: Smart user agent rotation and retry logic for reliable downloads

### Integrated Music Player

- **Intelligent Behavior Tracking**: Smart shuffle based on listening habits and skip patterns
- **Terminal UI**: Modern ratatui-based interface with keyboard controls
- **Metadata Management**: In-app editing with smart filename parsing
- **Multi-Format Support**: MP3, FLAC, OGG, MP4/M4A, WAV playback
- **Content Hashing**: xxhash64-based deduplication and move detection
- **Unified Database**: Seamless sync between discovery and playback systems

- Uses yt-dlp / YouTube Music for personal library building. Please respect YouTube's Terms of Service and your local laws.

## Architecture

BangTunes is designed as a **standalone tool** with optional advanced features:

### Core Dependencies (Required)

- **Python 3.8+**: Runtime environment
- **SQLite DB**: Persistent storage with `youtube_id` unique index for deduplication
- **Rich Library**: Colored output, progress bars, and styled tables
- **ytmusicapi**: YouTube Music search and metadata enrichment
- **mutagen**: Audio file tagging and cover art embedding
- **yt-dlp**: Reliable audio extraction and conversion
- **rapidfuzz**: Fuzzy matching for music discovery

### Playback Options

- **Basic playback** (`quickplay`): Uses system audio players (ffplay, termux-media-player) - **no additional setup required**
- **Advanced playback** (`play`): Integrated TUI player with smart shuffle and behavior tracking - **built-in, requires Rust for compilation**

## Getting Started in 3 Commands

Jump right in with the essential workflow:

```bash
# 1. Build some discovery batches from your seed tracks
python bang_tunes.py build

# 2. Download your first batch
python bang_tunes.py download batches/mix_001.csv

# 3. Set up the music player and start listening
python bang_tunes.py setup-player && python bang_tunes.py play
```

That's it and now you'll have some new music to check out.

## Installation

Bang Tunes requires Python 3.8+ and works best on Linux/Termux environments.

### Termux Setup

```bash
# Enable storage access
termux-setup-storage

# Update packages
pkg update && pkg upgrade -y

# Install dependencies
pkg install -y python ffmpeg

# Install Python packages
pip install -U "yt-dlp[default]" ytmusicapi mutagen rapidfuzz rich

# Create project structure
mkdir -p ~/BangTunes/{batches,downloads}
```

### Arch Linux Setup

# Install system dependencies

```bash
sudo pacman -S python python-pip ffmpeg


# Install Python packages
pip install -U "yt-dlp[default]" ytmusicapi mutagen rapidfuzz rich

# Create project structure
mkdir -p ~/BangTunes/{batches,downloads}
```

## Using Requirements File

```bash
cd ~/BangTunes
pip install -r requirements.txt
```

## Usage

### 1. Prepare Your Seed File

Edit `seed.csv` with your musical preferences:

```csv
title,artist,notes
Phantom Bride,Deftones,alt metal
Sun,Sleeping At Last,soft indie
Bohemian Rhapsody,Queen,classic rock
```

The `notes` field helps with fuzzy matching to find similar tracks.

### 2. Build Batches

Generate discovery batches from your seed list:

```bash
# Basic batch generation (50 tracks per batch, minimum score 50)
python bang_tunes.py build

# Custom parameters
python bang_tunes.py build --prefix rock --min-score 60 --size 30
```

This creates CSV files in the `batches/` directory with discovered tracks.

### 3. Download a Batch

Download and organize audio files from a batch:

```bash
# Download a specific batch
python bang_tunes.py download batches/mix_001.csv

# Or just the filename if in batches/ directory
python bang_tunes.py download mix_001.csv
```

### 4. View Your Library

Display a summary of your music collection:

```bash
python bang_tunes.py view
```

### 5. Setup Integrated Music Player

Setup the intelligent music player integration:

```bash
# One-time setup: configure player and sync library
python bang_tunes.py setup-player
```

### 6. Play Your Music

Launch the intelligent terminal music player:

```bash
# Start the music player with your library
python bang_tunes.py play
```

### 7. Sync Library Changes

Sync new downloads with the music player:

```bash
# After downloading new batches, sync with player
python bang_tunes.py sync

# Check integration status
python bang_tunes.py player-status
```

## File Organization

Downloaded files are organized as:

```markdown
downloads/
├── batch_name/
│   ├── Artist_Name/
│   │   ├── Album_Name/
│   │   │   ├── Track_Title.opus
│   │   │   └── Another_Track.opus
│   │   └── Unknown_Album/
│   │       └── Single_Track.opus
│   └── Another_Artist/
└── library.db
```

## Database Schema

The SQLite database (`library.db`) stores:

- `id`: Primary key
- `youtube_id`: Unique YouTube video ID (prevents duplicates)
- `title`: Track title
- `artist`: Artist name(s)
- `album`: Album name
- `file_path`: Full path to audio file
- `added_on`: Timestamp when added

## Command Reference

### Build Command

```bash
python bang_tunes.py build [OPTIONS]

Options:
  --prefix TEXT     Batch file prefix (default: mix)
  --min-score INT   Minimum fuzzy match score (default: 50)
  --size INT        Tracks per batch (default: 50)
```

### Download Command

```bash
python bang_tunes.py download BATCH_CSV

Arguments:
  BATCH_CSV    Path to batch CSV file (relative to batches/ or absolute)
```

### View Command

```bash
python bang_tunes.py view
```

Shows top artists by track count in a styled table.

### Player Commands

```bash
# Setup integrated music player (one-time)
python bang_tunes.py setup-player

# Launch intelligent music player
python bang_tunes.py play

# Sync library with player database
python bang_tunes.py sync

# Show integration status
python bang_tunes.py player-status
```

The integrated player features:

- Smart shuffle based on listening habits
- Skip learning and behavior tracking
- Modern terminal UI with keyboard controls
- Metadata editing capabilities
- Multi-format audio support

## Advanced Features

### Metadata Enhancement

- Searches YouTube Music for accurate artist/album information
- Downloads highest resolution cover art available
- Embeds metadata using mutagen (title, artist, album, year)
- Supports both Vorbis comments (opus) and ID3 tags (mp3)

**Note**: Current metadata detection uses intentionally simple placeholder logic for rapid discovery. Advanced AI-powered metadata suggestions using local ML models are planned for future releases to enhance artist/album detection accuracy.

### Fuzzy Matching

Uses rapidfuzz for intelligent music discovery:

- Combines title, artist, and notes from seed
- Scores candidates using token set ratio
- Filters by minimum score threshold
- Deduplicates across all seeds

### Progress Tracking

Rich progress bars show:

- Current download progress
- Tracks completed/total
- Estimated time remaining
- Spinner animation during processing

## Troubleshooting

### Missing Dependencies

If you get import errors, ensure all dependencies are installed:

```bash
pip install -r requirements.txt
```

### yt-dlp Issues

If downloads fail, update yt-dlp:

```bash
pip install -U yt-dlp
```

### Permission Errors

Make sure the script is executable:

```bash
chmod +x bang_tunes.py
```

## Advanced / Debugging

### Debug Mode

Enable verbose debug logging for troubleshooting:

```bash
BANGTUNES_DEBUG=1 python bang_tunes.py download mix_001.csv
```

Debug mode provides:

- Detailed yt-dlp failure information
- Failed download summaries with URLs and reasons
- Enhanced error messages for PanPipe integration
- Verbose logging for API calls and file operations

### YouTube Music API Debugging

For debugging YouTube Music API issues, use the dedicated debug script:

```bash
python debug_ytm.py
```

This script helps troubleshoot:

- YTMusic authentication issues
- Search query problems
- API response parsing errors
- Rate limiting and connection issues

### Download Archive

BangTunes uses `download_archive.txt` to track downloaded videos and avoid re-downloading. If you need to reset this:

```bash
# Clear download history (will re-download everything)
rm download_archive.txt

# Or backup and restore
cp download_archive.txt download_archive.backup
```

## Future Enhancements

- **Cross-device sync (Termux ↔ Linux)**: Seamless library synchronization across platforms
- **Metadata AI suggestions**: Enhanced artist/album detection using local ML models
- **Smart Similarity**: Local embeddings for better music clustering and recommendations
- **Discovery Report**: Rich batch analysis with genre clustering and similarity scores
- **Audio Format Options**: Support for mp3, flac, and other high-quality formats
- **Curses TUI**: Full-screen terminal interface with arrow-key navigation
- **Dedup Sweeper**: Advanced filesystem vs database reconciliation
- **License Filtering**: Optional Creative Commons-only discovery mode

## Contributing

This is a single-file MVP designed for easy modification and extension. The code is structured with clear separation between:

- Database operations (`db_*` functions)
- Metadata handling (`embed_*`, `fetch_*` functions)
- Music discovery (`search_*`, `build_*` functions)
- File operations (`organize_*`, `run_*` functions)
- CLI interface (`main` function)

## License

Open source - modify and distribute as needed.

---

**Bang Tunes** — Because your music discovery shouldn't be drab and monotonous.
