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
                                                    ╚──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╝

A complete music ecosystem combining intelligent discovery, high-quality downloads, and smart playback in one unified system. Bang Tunes uses seed-based discovery to find similar music, downloads with rich metadata, and features an integrated terminal music player with behavior tracking and smart shuffle.

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

## Architecture

- **SQLite DB**: Persistent storage with `youtube_id` unique index for deduplication
- **Rich Library**: Colored output, progress bars, and styled tables
- **ytmusicapi**: YouTube Music search and metadata enrichment
- **mutagen**: Audio file tagging and cover art embedding
- **yt-dlp**: Reliable audio extraction and conversion
- **rapidfuzz**: Fuzzy matching for music discovery

## Getting Started in 3 Commands

Jump right in with the essential workflow:

```bash
# 1. Generate discovery batches from your music taste
python bang_tunes.py build

# 2. Download your first batch of music
python bang_tunes.py download batches/mix_001.csv

# 3. Set up and launch the intelligent music player
python bang_tunes.py setup-player && python bang_tunes.py play
```

That's it! You now have a complete music discovery and playback ecosystem.

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
```

# Install Python packages

pip install -U "yt-dlp[default]" ytmusicapi mutagen rapidfuzz rich

# Create project structure

mkdir -p ~/BangTunes/{batches,downloads}

```

### Using Requirements File

```bash
cd /home/himokai/Builds/BangTunes
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
BANGTUNES_DEBUG=1 python bang_tunes.py download batch_001.csv
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

- **Curses TUI**: Full-screen terminal interface with arrow-key navigation
- **Audio Format Options**: Support for mp3, flac, and other formats
- **Smart Similarity**: Local embeddings for better music clustering
- **Dedup Sweeper**: Filesystem vs database reconciliation
- **License Filtering**: Optional Creative Commons-only mode

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

**Bang Tunes** — Because your music discovery shouldn't be drab and monotonous. 🎵
