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

Bang Tunes helps you discover new music based on tracks you already love. Started this during some downtime and it's grown into something I actually use daily for finding new music.

**Perfect for**: Anyone who lives in the terminal and gets tired of the same old playlists. Also great if you want to build a local music library without the streaming subscription hassle.

**How it works**: You give it some songs you like, it finds similar stuff on YouTube Music, downloads the audio, and keeps everything organized. Plus there's a pretty decent terminal player built in if you want to get fancy.

## Features

### Music Discovery & Download

- **Smart Discovery**: Feed it some tracks you like, it finds similar stuff you haven't heard
- **SQLite Database**: Keeps track of everything, no duplicates, easy to search
- **Pretty Progress Bars**: Because watching downloads is oddly satisfying
- **Good Metadata**: Pulls proper artist/album info and embeds it in the files
- **Batch Downloads**: Grabs 50 tracks at a time, resumes if something goes wrong
- **Decent Audio Quality**: High-quality opus files with album art embedded
- **Plays Nice**: Doesn't hammer YouTube's servers, rotates user agents sensibly

### Built-in Music Player

- **Learns Your Taste**: Tracks what you skip and what you replay, shuffles accordingly
- **Terminal Interface**: Clean TUI that doesn't get in your way, all keyboard driven
- **Fix Metadata**: Edit track info right in the player when things look wrong
- **Plays Everything**: MP3, FLAC, OGG, whatever you throw at it
- **Smart Deduplication**: Knows when you've moved files around, won't lose track
- **Everything Connected**: Downloads and player share the same database, no sync headaches

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

## Getting Started in 4 Commands

Jump right in with the essential workflow:

```bash
# 1. Set up virtual environment and install dependencies
python -m venv venv && source venv/bin/activate && pip install -r requirements.txt

# 2. Build some discovery batches from your seed tracks
python bang_tunes.py build

# 3. Download your first batch
python bang_tunes.py download batches/mix_001.csv

# 4. Set up the music player and start listening
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

### 1. Customize Your Musical DNA (seed.csv)

**🎯 IMPORTANT: This is the key to personalized music discovery!**

The `seed.csv` file is your musical DNA - it tells BangTunes what you like so it can find similar tracks. **You must edit this file with your own favorite songs** to get personalized playlists.

**Edit `seed.csv` with 10-20 of your favorite tracks:**

```csv
title,artist,notes
Rain,Sleep Token,alt rock
Paladin Strait,Twenty One Pilots,alt rock
Caramel,Sleep Token,alt rock
Sirens,Imagine Dragons,alt rock
Teardrops,Bring Me The Horizon,alt rock
Dark Signs,Sleep Token,alt rock
sTrAnGeRs,Bring Me The Horizon,alt rock
Middle of the Night,Loveless,alt rock
Without Me,Dayseeker,alt rock
Usurper,The Villians & NXCRE,alt metal
```

**Pro Tips for Better Discovery:**

- **Mix genres**: Include different styles you enjoy (rock, indie, electronic, etc.)
- **Use descriptive notes**: "heavy metal", "chill indie", "upbeat pop", "experimental"
- **Include variety**: Don't just use one artist - spread across your taste
- **Be specific**: "Radiohead,Creep,melancholy alt rock" works better than just "Radiohead,Creep"

**The `notes` field is crucial** - it helps BangTunes understand the vibe and find similar tracks with the right energy and style.

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

#### Player Controls

Once the player launches, use these keyboard commands:

**Playback Controls:**

- `Space` - Toggle play/pause
- `p` - Play current track
- `s` - Stop playback
- `n` or `→` - Next track
- `b` or `←` - Previous track

**Navigation:**

- `↑` / `↓` - Navigate through track list
- `Enter` - Select/play highlighted track
- `Backspace` - Go back/cancel

**Volume:**

- `+` or `=` - Volume up
- `-` - Volume down

**Playlist Features:**

- `z` - Toggle shuffle mode
- `r` - Toggle repeat mode

**Library:**

- `F5` - Refresh library
- `q` or `Esc` - Quit player

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
- Enhanced error messages for player integration
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

## Contributing

We welcome contributions to BangTunes! Whether you're fixing bugs, adding features, improving documentation, or enhancing the player integration, your help makes this project better for everyone.

### Development Setup

1. **Fork and Clone**

   ```bash
   git clone https://github.com/yourusername/BangTunes.git
   cd BangTunes
   ```

2. **Set Up Development Environment**

   ```bash
   # Run the setup script to install dependencies
   ./setup.sh
   
   # Activate the virtual environment
   source venv/bin/activate  # Linux/macOS
   # or
   venv\Scripts\activate     # Windows
   ```

3. **Install Rust (Optional - for player development)**

   ```bash
   # Only needed if working on the integrated player
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

### Development Workflow

1. **Create a Feature Branch**

   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b bugfix/issue-description
   ```

2. **Make Your Changes**
   - Follow the existing code style and patterns
   - Add comments for complex logic
   - Update docstrings for new functions
   - Keep commits focused and atomic

3. **Test Your Changes**

   ```bash
   # Test core functionality
   python bang_tunes.py install     # First-run setup
   python bang_tunes.py stats       # Library statistics
   python bang_tunes.py quickplay   # Basic playback
   
   # Test discovery workflow
   python bang_tunes.py build --seed seed.csv --limit 5
   python bang_tunes.py rescan
   
   # Test player integration (if applicable)
   python bang_tunes.py setup-player
   python bang_tunes.py sync
   python bang_tunes.py player-status
   ```

4. **Run Quality Checks**

   ```bash
   # Python linting (if available)
   python -m py_compile bang_tunes.py
   python -m py_compile player_integration.py
   
   # Rust checks (if working on player)
   cargo check
   cargo clippy
   ```

### Testing Guidelines

**Core Features to Test:**

- **Discovery**: Seed file parsing, YouTube Music API integration
- **Downloads**: yt-dlp integration, file organization, metadata extraction
- **Database**: SQLite operations, library management, statistics
- **Player Integration**: Config generation, library sync, binary detection
- **CLI Commands**: All subcommands work with proper error handling

**Test Scenarios:**

```bash
# Test with minimal seed file
echo "artist,title" > test_seed.csv
echo "Radiohead,Creep" >> test_seed.csv
python bang_tunes.py build --seed test_seed.csv --limit 3

# Test error handling
python bang_tunes.py build --seed nonexistent.csv  # Should fail gracefully
python bang_tunes.py quickplay  # Should handle no music gracefully

# Test cross-platform paths
python bang_tunes.py stats  # Should work on any OS
```

**Player Integration Testing:**

```bash
# Test player setup and sync
python bang_tunes.py setup-player
python bang_tunes.py sync
python bang_tunes.py player-status

# Test with missing Rust/cargo
# (temporarily rename cargo binary to test fallback behavior)
```

### Code Style Guidelines

**Python Code:**

- Use descriptive variable names: `youtube_id` not `yt_id`
- Add docstrings for all public functions
- Use f-strings for string formatting
- Handle exceptions gracefully with user-friendly messages
- Use `rich` console for all user output

**Comments and Documentation:**

- Explain *why* not just *what*
- Add context for complex algorithms or API interactions
- Document any external dependencies or assumptions
- Keep comments up-to-date with code changes

**Error Handling:**

- Always provide actionable error messages
- Include troubleshooting hints where possible
- Use appropriate exit codes
- Log errors for debugging while showing clean messages to users

### Contribution Areas

**🐛 Bug Fixes**

- YouTube Music API changes
- Cross-platform compatibility issues
- Edge cases in file handling or metadata parsing
- Player integration reliability

**✨ Feature Enhancements**

- New CLI commands or options
- Additional metadata sources
- Improved similarity algorithms
- Enhanced player integration features

**📚 Documentation**

- README improvements
- Code comments and docstrings
- Usage examples and tutorials
- Troubleshooting guides

**🎵 Player Development**

- Rust-based player improvements
- New TUI features
- Behavior tracking enhancements
- Performance optimizations

### Submitting Changes

1. **Commit Standards**

   ```bash
   # Use conventional commit format
   git commit -m "feat: add batch discovery report generation"
   git commit -m "fix: handle missing ffplay gracefully in quickplay"
   git commit -m "docs: improve setup instructions for Windows"
   ```

2. **Push and Create PR**

   ```bash
   git push origin feature/your-feature-name
   # Then create a Pull Request on GitHub
   ```

3. **PR Guidelines**
   - Describe what your change does and why
   - Include testing steps for reviewers
   - Reference any related issues
   - Keep PRs focused on a single feature/fix

### Getting Help

- **Issues**: Check existing GitHub issues or create a new one
- **Discussions**: Use GitHub Discussions for questions and ideas
- **Code Questions**: Comment on specific lines in PRs for targeted help

### Development Notes

**Architecture Overview:**

- `bang_tunes.py`: Main CLI application and core logic
- `player_integration.py`: Bridge between BangTunes and the integrated player
- `src/`: Rust-based intelligent music player (optional)
- `setup.sh`: Cross-platform development environment setup

**Key Dependencies:**

- `ytmusicapi`: YouTube Music API access
- `yt-dlp`: Audio download and conversion
- `mutagen`: Audio metadata handling
- `rich`: Terminal UI and formatting
- `rapidfuzz`: Fuzzy string matching for deduplication

Thanks for contributing to BangTunes! Every improvement helps build a better music discovery experience for everyone.

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
