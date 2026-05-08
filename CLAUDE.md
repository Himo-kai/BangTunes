# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BangTunes is a two-component music discovery and playback system:

1. **Python CLI** (`bang_tunes.py` / `bangtunes/`) — discovers music via YouTube Music, downloads audio, manages a SQLite library
2. **Rust TUI Player** (`src/` / `panpipe_interactive` binary) — intelligent terminal music player with behavior tracking and smart shuffle

The Python side handles everything up to having audio files on disk. The Rust side handles playback. They share `library.db`.

## Commands

### Python (activate venv first)
```bash
source venv/bin/activate

# Run the CLI
python bang_tunes.py <command>

# Run tests
pytest tests/ -v
pytest tests/test_smoke.py -v         # smoke suite only
pytest -m "not slow" tests/           # skip slow tests

# Type checking
mypy bangtunes/

# Linting (ruff)
ruff check bangtunes/
ruff check --fix bangtunes/
```

### Rust
```bash
# Build debug
cargo build

# Build release (used in production)
cargo build --release

# Run the player directly
cargo run --bin panpipe_interactive -- [MUSIC_DIR]
cargo run --bin panpipe_interactive -- --dev   # enable debug logging

# Run checks without building binary
cargo check
cargo clippy
```

### Environment / Config
```bash
# Enable debug logging for Python side
BANGTUNES_DEBUG=1 python bang_tunes.py download mix_001.csv

# Config file locations (checked in order):
~/.config/bangtunes.toml
./bangtunes.toml          # project-local (see bangtunes.example.toml)
```

## Architecture

### Python Package (`bangtunes/`)

| File | Responsibility |
|------|---------------|
| `cli.py` | Argument parsing, dispatches to `commands.py` |
| `commands.py` | One function per CLI command (cmd_build, cmd_download, etc.) |
| `library.py` | Seed CSV → discovery pool → batch CSV pipeline |
| `downloads.py` | yt-dlp wrapper: downloads, metadata, cover art embedding |
| `db.py` | SQLite CRUD for `library.db` |
| `config.py` | TOML loading with `_deep_merge` into defaults |
| `constants.py` | Single source of truth for defaults and valid values |
| `env.py` | Resolves project root path |
| `player_integration.py` | Bridge to launch `panpipe_interactive`; falls back to `python_player.py` |
| `python_player.py` | Fallback player (mpv/vlc/ffplay) for when Rust binary is unavailable |

The `play` command tries the Rust binary first; if unavailable it falls back to `PythonMusicPlayer`.

### Rust Crate (`src/`)

```
src/
├── lib.rs                  # pub mod declarations + re-exports
├── audio/
│   ├── player.rs           # rodio-based audio playback, PlayerEvent channel
│   ├── scanner.rs          # walkdir library scan → Track objects
│   ├── metadata_parser.rs  # id3/mp4ameta tag reading
│   ├── playlist.rs         # PlaylistManager (CRUD, load-to-queue)
│   └── track.rs            # Track / TrackMetadata types
├── behavior/
│   ├── database.rs         # rusqlite schema for listening history
│   ├── tracker.rs          # BehaviorTracker: records events, updates weights
│   └── weighting.rs        # ShuffleWeighting: scores tracks for smart shuffle
├── config/mod.rs           # TOML config (shares bangtunes.toml format)
├── database/mod.rs         # BangTunesDatabase: reads Python-side library.db
└── ui/
    ├── events.rs           # EventHandler: crossterm input → app events
    └── mod.rs              # TerminalManager: ratatui setup/teardown
```

The main binary `src/bin/panpipe_interactive.rs` owns the full TUI event loop, all UI state, and calls into the library modules.

### Data Flow

```
seed.csv → build → batches/mix_NNN.csv → download → downloads/ + library.db
                                                              ↓
                                              panpipe_interactive (reads library.db)
                                              + behavior.db (Rust-managed, separate)
```

### Database

- `library.db` — Python-managed SQLite; `youtube_id` has a unique index for dedup
- Behavior data — Rust-managed SQLite (written by `BehaviorDatabase` in `src/behavior/database.rs`)

## Key Design Decisions

- **Constants are centralised** in `bangtunes/constants.py` — `VALID_FORMATS`, `VALID_SPEED_MODES`, `DEFAULT_CONFIG`. Don't hardcode these elsewhere.
- **`slow` speed mode is a deprecated alias** for `stealth` — `_normalize_speed_mode()` in commands.py handles the mapping.
- **Rust feature flags**: `tui`, `audio`, `behavior` are the three main features. `termux` is a headless preset (no rodio/ALSA). Check Cargo.toml before adding dependencies.
- **Track UUIDs are deterministic** (UUIDv5) so library rescans don't break behavior history.
- **Config search order**: `~/.config/bangtunes.toml` first, then `./bangtunes.toml`. `_deep_merge` lets local overrides win.

## Audio Format Support (rodio / Symphonia)

The Rust player uses **rodio** with the **Symphonia** backend. Supported formats:

| Format | Status |
|--------|--------|
| MP3 | ✅ Supported |
| AAC / M4A | ✅ Supported (Symphonia) |
| OGG Vorbis | ✅ Supported |
| FLAC | ✅ Supported |
| WAV | ✅ Supported |
| Opus | ✅ Supported |
| MP4 video | ❌ Not supported (audio-only M4A works) |
| WMA | ❌ Not supported |
| AIFF | ❌ Not supported |

When a file fails to decode, `panpipe_interactive` emits `PlayerEvent::Error` with "Unsupported audio format or corrupted file". The TUI auto-skips the track, marks it in the in-memory `failed_tracks` set (excluded from future shuffle/sequential playback for the session), and shows which file failed in the status bar.

## Testing

Tests live in `tests/` and are pytest smoke tests — they call the actual CLI via subprocess to validate the real pipeline, not mocked internals. Run fast (~0.4s). Integration tests are marked `@pytest.mark.integration`.
