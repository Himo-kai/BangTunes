# Bang Tunes — Termux MVP

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

A polished, extensible music discovery and download tool designed for Termux and Arch Linux. Bang Tunes uses a seed-based approach to discover similar music, organizes downloads with rich metadata, and maintains a persistent SQLite library.

## Features

- **Persistent SQLite Library**: Dedupe by `youtube_id`, query by artist/album, track file paths
- **Rich Visual Interface**: Styled messages + progress bars during downloads
- **Metadata Enhancement**: YouTube Music API integration via `ytmusicapi`, file tagging with `mutagen`
- **Clean Architecture**: Single-file MVP ready for modular expansion
- **Batch Processing**: Seed→pool→50-track batches workflow, tracked in DB and resumable
- **Audio Quality**: High-quality opus downloads with embedded thumbnails and metadata

## Architecture

- **SQLite DB**: Persistent storage with `youtube_id` unique index for deduplication
- **Rich Library**: Colored output, progress bars, and styled tables
- **ytmusicapi**: YouTube Music search and metadata enrichment
- **mutagen**: Audio file tagging and cover art embedding
- **yt-dlp**: Reliable audio extraction and conversion
- **rapidfuzz**: Fuzzy matching for music discovery

## Installation

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

```bash
# Install system dependencies
sudo pacman -S python python-pip ffmpeg

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
