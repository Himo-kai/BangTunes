# BangTunes Cross-Language Schema Reference

This document is the single source of truth for every contract between the Python CLI
(`bangtunes/`) and the Rust TUI player (`src/`). Both sides read from and write to shared
files. When either side changes a schema element listed here, this file **must** be updated
in the same commit.

> **Rule:** Before merging any PR that touches `bangtunes/db.py`, `src/behavior/database.rs`,
> `src/database/mod.rs`, `bangtunes/constants.py`, `src/config/mod.rs`, or
> `bangtunes/downloads.py` — open this file and check whether a contract changed.

---

## Table of Contents

1. [Database Schemas](#1-database-schemas)
   - [library.db — Python-managed](#librarydb--python-managed)
   - [behavior.db — Rust-managed](#behaviordb--rust-managed)
2. [Shared Identifiers](#2-shared-identifiers)
   - [Track UUIDs](#track-uuids)
   - [youtube_id](#youtube_id)
3. [Configuration Contract (bangtunes.toml)](#3-configuration-contract-bangtunestoml)
4. [File Path Conventions](#4-file-path-conventions)
5. [Behavior Tags and Weights](#5-behavior-tags-and-weights)
6. [Metadata Field States](#6-metadata-field-states)
7. [DO NOT CHANGE WITHOUT MIGRATION](#7-do-not-change-without-migration)

---

## 1. Database Schemas

### library.db — Python-managed

**Location:** `<BangTunes root>/library.db`

Python resolves the root via `bangtunes/env.py::get_root()` (checks `BANGTUNES_ROOT`
env var, CWD, script dir, then platform-specific search paths). The Rust player locates
it independently via `src/database/mod.rs::BangTunesDatabase::find_database()`, which
probes:

```
~/BangTunes/library.db
~/Builds/BangTunes/library.db
~/Downloads/BangTunes/library.db
~/Music/BangTunes/library.db
~/storage/shared/BangTunes/library.db   (Termux)
/data/data/com.termux/files/home/BangTunes/library.db  (Termux)
```

**Writer:** Python only (`bangtunes/db.py`)  
**Readers:** Python (`bangtunes/db.py`), Rust (`src/database/mod.rs`)  
**Schema managed:** Idempotent `CREATE TABLE IF NOT EXISTS` on every connection open. No `schema_version` column; all migrations are additive and backward-compatible.

#### Table: `tracks`

| Column       | Type    | Constraints           | Description |
|--------------|---------|-----------------------|-------------|
| `id`         | INTEGER | PRIMARY KEY           | Auto-increment row ID. Used by Rust `DatabaseTrack.id` for metadata updates. |
| `youtube_id` | TEXT    | UNIQUE NOT NULL       | YouTube video ID (typically 11-char base64url). Primary dedup key. |
| `title`      | TEXT    | NOT NULL              | Track title. May be "Unknown Title" if yt-dlp couldn't determine it. |
| `artist`     | TEXT    | NOT NULL              | Artist name. May be "Unknown Artist". |
| `album`      | TEXT    | NULL allowed          | Album name. NULL when not available from source. |
| `file_path`  | TEXT    | NULL allowed          | **Absolute path** to the audio file on disk. NULL before download completes. |
| `added_on`   | TEXT    | DEFAULT CURRENT_TIMESTAMP | ISO 8601 timestamp (SQLite CURRENT_TIMESTAMP format: `YYYY-MM-DD HH:MM:SS`). |

**Indexes:** `idx_artist`, `idx_album`, `idx_youtube_id`, `idx_file_path`, `idx_added_on`

**PRAGMAs set on every connection:**
- `PRAGMA foreign_keys = ON`
- `PRAGMA journal_mode = WAL`

---

### behavior.db — Rust-managed

**Location:** `~/.config/panpipe/panpipe.db` (default, from `src/config/mod.rs`)

The config path is `~/.config/panpipe/config.toml` and the database is in the same
directory. Override via `config.database_path` in `bangtunes.toml` (Rust reads this key
from `[panpipe]` section — currently no Python writes to behavior.db).

**Writer:** Rust only (`src/behavior/database.rs`)  
**Readers:** Rust only  
**Schema managed:** Idempotent `CREATE TABLE IF NOT EXISTS` on startup. No `schema_version` column.

#### Table: `track_behaviors`

| Column           | Type    | Constraints               | Description |
|------------------|---------|---------------------------|-------------|
| `track_id`       | TEXT    | PRIMARY KEY               | UUIDv5 string (see §2). Corresponds to `Track.id` in Rust. |
| `total_plays`    | INTEGER | NOT NULL DEFAULT 0        | Count of sessions that met `min_play_time_for_tracking`. |
| `total_skips`    | INTEGER | NOT NULL DEFAULT 0        | Count of sessions that ended with a skip reason. |
| `total_play_time`| INTEGER | NOT NULL DEFAULT 0        | Cumulative seconds of actual playback (excludes paused time). |
| `last_played`    | TEXT    | NULL allowed              | RFC 3339 timestamp of most recent session start. NULL = never played. |
| `skip_positions` | TEXT    | NOT NULL                  | JSON array of `u64` values — each is the playback position (seconds) at skip time. |
| `completion_rate`| REAL    | NOT NULL DEFAULT 0.0      | Running weighted average of completion percentages (0.0–100.0). |
| `weight`         | REAL    | NOT NULL DEFAULT 1.0      | Pre-computed shuffle weight. Recomputed on every session commit. |
| `tags`           | TEXT    | NULL allowed              | JSON array of tag strings. See §5 for valid values. |
| `created_at`     | TEXT    | NOT NULL DEFAULT CURRENT_TIMESTAMP | Row creation timestamp. |
| `updated_at`     | TEXT    | NOT NULL DEFAULT CURRENT_TIMESTAMP | Updated on every `save_track_behavior()` call. |

#### Table: `play_sessions`

| Column                | Type    | Constraints               | Description |
|-----------------------|---------|---------------------------|-------------|
| `session_id`          | TEXT    | PRIMARY KEY               | UUIDv4 (random, per-session). |
| `track_id`            | TEXT    | NOT NULL                  | Foreign key → `track_behaviors.track_id`. |
| `started_at`          | TEXT    | NOT NULL                  | RFC 3339 start timestamp. |
| `ended_at`            | TEXT    | NULL allowed              | RFC 3339 end timestamp. NULL if session still active. |
| `play_duration`       | INTEGER | NOT NULL DEFAULT 0        | Seconds of actual playback in this session. |
| `track_duration`      | INTEGER | NOT NULL DEFAULT 0        | Total track length in seconds (from file tag or learned). |
| `skip_reason`         | TEXT    | NULL allowed              | JSON-serialized `SkipReason` enum: `"UserSkip"`, `"NextTrack"`, `"PreviousTrack"`, `"PlaylistEnd"`, `"Error"`. NULL = track played to completion. |
| `completion_percentage` | REAL  | NOT NULL DEFAULT 0.0      | `play_duration / track_duration × 100`. Capped at 100.0. |
| `created_at`          | TEXT    | NOT NULL DEFAULT CURRENT_TIMESTAMP | Row creation timestamp. |

**Indexes:** `idx_sessions_track_id`, `idx_sessions_started_at`

#### Table: `track_metadata`

Stores per-track metadata learned at runtime (primarily durations discovered from actual playback).

| Column          | Type    | Constraints               | Description |
|-----------------|---------|---------------------------|-------------|
| `track_id`      | TEXT    | PRIMARY KEY               | UUIDv5 string. |
| `file_path`     | TEXT    | NULL allowed              | Absolute path (written by `save_track_metadata()`). |
| `title`         | TEXT    | NULL allowed              | Title override (written by metadata editor). |
| `artist`        | TEXT    | NULL allowed              | Artist override. |
| `album`         | TEXT    | NULL allowed              | Album override. |
| `duration`      | INTEGER | NULL allowed              | Track duration in **seconds**, learned from actual playback. |
| `file_size`     | INTEGER | NULL allowed              | File size in bytes. |
| `last_modified` | TEXT    | NULL allowed              | RFC 3339 timestamp, updated via `save_track_metadata()`. |
| `created_at`    | TEXT    | NOT NULL DEFAULT CURRENT_TIMESTAMP | Row creation timestamp. |

---

## 2. Shared Identifiers

### Track UUIDs

**Where generated:** Rust, in `src/audio/track.rs::Track::new()`

**Formula:**
```rust
Uuid::new_v5(&Uuid::NAMESPACE_OID, file_path.to_string_lossy().as_bytes())
```

This is **path-based** (not youtube_id-based). The UUID is derived deterministically
from the absolute file path using UUIDv5 with the OID namespace.

**Why path-based:**
- Zero I/O at scan time (no file read required)
- Stable across restarts as long as files don't move
- BangTunes stores files in a stable `downloads/` tree, so paths don't change after download

**What this means:**
- If a file is **moved or renamed**, its UUID changes and existing behavior history is orphaned
- The `rescan --fix` command handles this partially (removes DB entries for missing files)
- The `rescan --prune-behavior` flag removes orphaned behavior.db rows

> ⚠️ **CLAUDE.md note correction:** CLAUDE.md previously described the UUID as
> `UUIDv5(NAMESPACE_URL, youtube_id)`. The actual implementation uses `NAMESPACE_OID`
> and `file_path`. The NAMESPACE_OID + file_path approach is what the code does and what
> this document reflects.

**Python side:** Python does not compute or store track UUIDs. The `id` column in
`library.db` is an auto-increment INTEGER, not a UUID. The UUID is a Rust-only concept
used only in behavior.db.

### Known Limitation: Path-Based UUIDs

Track UUIDs are currently derived from absolute file paths. This means behavior records
(favorites, play counts, weights) are tied to file location. Moving or renaming a track
invalidates its behavior history.

A future migration is planned to switch to youtube_id-based UUIDs:
`UUIDv5(NAMESPACE_URL, youtube_id)`. This requires:
- Adding a `uuid` column to `library.db`
- Schema version bump on both `library.db` and `behavior.db`
- One-time migration of existing behavior records
- Coordinated change in Python (`db.py`) and Rust (`track.rs`, `scanner.rs`)

Until that migration: do not move or rename downloaded files. The stable `downloads/`
directory tree assumes paths are immutable.

### youtube_id

- **Format:** YouTube video ID string, typically 11 ASCII characters (base64url alphabet: `[A-Za-z0-9_-]`)
- **Constraints:** UNIQUE NOT NULL in `library.db`. Primary dedup key — a `youtube_id` can only exist once in the library.
- **Written by:** Python (`bangtunes/db.py::db_add_track()` with `INSERT OR IGNORE`)
- **Read by:** Python (dedup checks, display), Rust (metadata lookup via `BangTunesDatabase`)
- **Not used for:** UUID generation (see above)

---

## 3. Configuration Contract (bangtunes.toml)

**Config search order:** `~/.config/bangtunes.toml` → `<root>/bangtunes.toml`  
**Merge strategy:** Deep merge; later file wins on conflict (`_deep_merge` in `bangtunes/config.py`)  
**Default values source:** Python — `bangtunes/constants.py::DEFAULT_CONFIG`; Rust — `src/config/mod.rs::Config::default()`

### Top-level keys

| Key               | Type    | Default    | Who reads | Description |
|-------------------|---------|------------|-----------|-------------|
| `format`          | string  | `"opus"`   | Python    | Default audio format for downloads. Values: `"opus"`, `"mp3"`, `"m4a"`. |
| `min_score`       | integer | `58`       | Python    | Minimum fuzzy match score (0–100) for seed→batch matching. |
| `size`            | integer | `50`       | Python    | Tracks per batch CSV file. |
| `music_directories` | list of strings | `["~/Music"]` | **Rust** | Directories the TUI player scans for audio files. Python does not read this key. |

### `[download]` section

| Key       | Type   | Default    | Who reads | Description |
|-----------|--------|------------|-----------|-------------|
| `format`  | string | `"opus"`   | Python    | Audio format. Same values as top-level `format`. `[download].format` takes precedence over top-level. |
| `quality` | string | `"best"`   | Python    | yt-dlp quality selector. Values: `"best"`, `"medium"`, `"low"`. |
| `speed`   | string | `"normal"` | Python    | Download speed mode. Values: `"normal"`, `"fast"`, `"stealth"`. `"slow"` is a deprecated alias for `"stealth"` (normalized in `commands.py`). |

### `[behavior]` section

| Key                        | Type    | Default | Who reads      | Description |
|----------------------------|---------|---------|----------------|-------------|
| `min_play_time_for_tracking` | integer | `30`  | **Both**       | Seconds of playback before a session is recorded. Python uses this as a config value; Rust uses it as `BehaviorTracker.min_play_time`. |
| `weight_decay_days`        | integer | `30`    | **Rust**       | Days after which an unplayed track reaches full staleness boost. Python does not currently read this. |

### `[player]` section

| Key       | Type    | Default | Who reads | Description |
|-----------|---------|---------|-----------|-------------|
| `volume`  | float   | `0.7`   | Rust      | Initial playback volume (0.0–1.0). |
| `shuffle` | boolean | `false` | Rust      | Initial shuffle state. |
| `repeat`  | boolean | `false` | Rust      | Initial repeat state. |

---

## 4. File Path Conventions

### downloads/ directory structure

```
<root>/
  downloads/
    archive.txt                         # yt-dlp download archive (prevents re-downloads)
    tmp_<batch_tag>/                    # Temporary directory during active download of batch
    <batch_tag>/                        # Final destination after download completes
      <sanitized_artist>/
        <sanitized_album>/
          <sanitized_title>.<ext>
```

- `<batch_tag>` = the CSV filename stem (e.g., `mix_001` for `mix_001.csv`)
- `<ext>` = the audio format extension (`opus`, `mp3`, `m4a`)

### Filename sanitization

Both Python sides apply the same rule:

```python
# bangtunes/db.py::sanitize()  and  bangtunes/downloads.py::sanitize_filename()
re.sub(r"[^-\w.\s]", "_", name).strip()
```

Characters **kept:** ASCII letters, digits, `-`, `_`, `.`, whitespace  
Characters **replaced with `_`:** everything else (including `/`, `:`, `"`, `'`, `(`, `)`)  
Leading/trailing whitespace is stripped.

> ⚠️ The `sanitize()` function in `db.py` and `sanitize_filename()` in `downloads.py`
> use identical regexes but are defined separately. If one changes, the other must too.

### Path storage in library.db

- `file_path` is stored as an **absolute path string** (the result of Python's `Path.resolve()` or equivalent after `organize_target()`)
- Rust reads it via `find_track_by_path()` which does exact string comparison — no normalization
- Symlinks, relative paths, and trailing slashes will cause mismatches

### batches/ directory structure

```
<root>/batches/<prefix>_NNN.csv
```

Each CSV has columns: `youtube_id`, `title`, `artist`, `album` (exact column names matter — `commands.py::read_batch_csv` expects them).

---

## 5. Behavior Tags and Weights

Tags are stored as a JSON array of strings in `track_behaviors.tags`. There are two weight
systems, both applied to the same tags:

### Tag definitions

| Tag string           | Set by | Trigger condition | Note |
|----------------------|--------|-------------------|------|
| `"favorite"`         | User (explicit) | `f` key in TUI | Must survive all auto-tag recomputes. Set/cleared only by `BehaviorTracker::toggle_favorite()`. |
| `"high_completion"`  | Auto | `completion_rate > 90.0` | Recomputed on every session. Does not override `"favorite"`. |
| `"often_skipped"`    | Auto | `completion_rate < 30.0` | Recomputed on every session. |
| `"skip_early"`       | Auto | avg skip position < 25% AND `>3` skip positions recorded | Recomputed on every session. |
| `"skip_late"`        | Auto | avg skip position > 75% AND `>3` skip positions recorded | Recomputed on every session. |
| `"frequently_played"`| Auto | `total_plays > 10` | Recomputed on every session. |
| `"high_skip_rate"`   | Auto | `total_skips / total_plays > 0.7` | Recomputed on every session. |
| `"low_skip_rate"`    | Auto | `total_skips / total_plays < 0.2` | Recomputed on every session. |

### Weights (two separate calculators)

> ⚠️ There are **two weight calculation paths** that apply different multipliers for the same tags. This is a known inconsistency.

**Path 1 — `TrackBehavior::calculate_shuffle_weight()` in `src/behavior/mod.rs`**  
Called from `BehaviorTracker::record_session()` to store a pre-computed weight in `track_behaviors.weight`.

| Tag               | Multiplier |
|-------------------|-----------|
| `favorite`        | × 1.5     |
| `high_completion` | × 1.2     |
| `often_skipped`   | × 0.3     |

**Path 2 — `WeightCalculator::calculate_weight()` in `src/behavior/weighting.rs`**  
Called at shuffle-selection time by `ShuffleWeighting`. Uses the stored `weight` field as input, then applies additional tag multipliers in real time.

| Tag                | Multiplier |
|--------------------|-----------|
| `favorite`         | × 1.8     |
| `high_completion`  | × 1.3     |
| `often_skipped`    | × 0.2     |
| `skip_early`       | × 0.4     |
| `skip_late`        | (no effect — falls through `_ => {}`) |
| `frequently_played`| × 0.9     |
| `high_skip_rate`   | × 0.3     |
| `low_skip_rate`    | × 1.2     |

Both calculators clamp the final weight to `[0.05, 5.0]`.

### Settings help text vs actual weights

The Settings tab in the TUI displays: *"Favorites (f key) boost shuffle weight by 1.8x"* and *"Tracks played >90% auto-tagged 'high_completion' (+1.3x)"*. These reflect the `weighting.rs` values (Path 2), not the `mod.rs` stored weight (Path 1).

---

## 6. Metadata Field States

### In library.db (Python-written)

The `title` and `artist` columns are NOT NULL with no default, so they are always
present. However, yt-dlp may produce uninformative values:

| Value             | Meaning |
|-------------------|---------|
| Actual string     | Genuine metadata from YouTube or file tags |
| `"Unknown Title"` | yt-dlp returned no usable title; written as a fallback literal |
| `"Unknown Artist"`| yt-dlp returned no usable artist |

`album` is nullable. NULL means the track has no album metadata; the empty string `""`
should not be written (use NULL instead).

### In Rust Track display

| `display_title()` returns | Condition |
|--------------------------|-----------|
| `metadata.title`         | If `Some(_)` |
| filename stem (no ext)   | If `None` — fallback from file path |

| `display_artist()` returns | Condition |
|---------------------------|-----------|
| `metadata.artist`         | If `Some(_)` |
| `"Unknown Artist"`        | If `None` |

`display_artist()` never returns NULL or an empty string; it always falls back to `"Unknown Artist"`.

---

## 7. DO NOT CHANGE WITHOUT MIGRATION

These are the cross-language assumptions that will **silently break** something if changed
unilaterally on one side. Always update both sides together, and update this document.

### 1. Track UUID formula

**Current:** `UUIDv5(NAMESPACE_OID, absolute_file_path_bytes)` (Rust, `src/audio/track.rs:54`)

Changing the namespace, the input data, or switching to a different UUID version will
orphan all existing behavior history (all rows in `track_behaviors` and `play_sessions`
will become unreachable). Requires a migration that maps old UUIDs → new UUIDs in
behavior.db before the new formula ships.

### 2. `tags` JSON encoding

**Current:** `serde_json::to_string(&Vec<String>)` → standard JSON array, e.g. `["favorite","high_completion"]`

If the serialization format changes (e.g., switching to comma-delimited text), all existing
tags in behavior.db become unparseable and will silently default to empty `[]`. Requires a
migration that rewrites the `tags` column before the new format ships.

### 3. `skip_positions` JSON encoding

Same constraint as `tags`: a JSON array of `u64` integers. Format change = silent data loss.

### 4. `skip_reason` JSON encoding

`play_sessions.skip_reason` stores the serde-JSON representation of the `SkipReason` enum
(e.g., `"UserSkip"`, `"NextTrack"`). Renaming enum variants without a serde alias breaks
deserialization of historical rows.

### 5. `sanitize_filename` regex

**Current:** `[^-\w.\s]` → `_`

Python's `db.py::sanitize()` and `downloads.py::sanitize_filename()` use the same regex.
The Rust player looks up tracks in library.db by exact file path. If the sanitization
rule changes, files downloaded under the new rule won't match paths stored under the old
rule. Requires a DB migration that rewrites `file_path` for all affected rows.

### 6. `file_path` storage as absolute path

library.db stores absolute paths. Rust's `find_track_by_path()` does an exact SQL
`WHERE file_path = ?` match. Switching to relative paths on either side breaks the lookup.

### 7. Config key names in `[behavior]` section

Both Python and Rust read `behavior.min_play_time_for_tracking`. If this key is renamed on
either side, the other side silently falls back to its hardcoded default (30 seconds), and
the two sides become out of sync without any error.

### 8. Timestamp format

All timestamps in behavior.db use RFC 3339 (via chrono's `to_rfc3339()`). library.db uses
SQLite's `CURRENT_TIMESTAMP` (`YYYY-MM-DD HH:MM:SS` without timezone). These formats are
used by different sides and should not be unified without checking both parsers.

### 9. `youtube_id` uniqueness assumption

Python uses `INSERT OR IGNORE` keyed on `youtube_id` as the dedup guard. If the Rust side
ever inserts rows into library.db (it currently does not), it must respect the same
constraint.

---

*Last updated: 2026-05-10. Update this file in the same commit as any change to the items above.*
