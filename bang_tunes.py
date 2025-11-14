#!/usr/bin/env python3
"""
Bang Tunes - Music Discovery Tool

Copyright (c) 2024 BangTunes Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

Started this in my truck during downtime between jobs. 
Turned into something actually useful for finding new music.
Integrates with my terminal music player setup.
"""
# Bang Tunes — CLI music discovery & playback
# Basic idea: seed tracks -> find similar -> batch download -> profit
import argparse
import csv
import os
import random
import re
import shutil
import signal
import sqlite3
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path
from textwrap import dedent
from typing import Dict, List, Optional, Callable, Any, Generator

try:
    import tomllib  # py3.11+
except ImportError:
    try:
        import tomli as tomllib  # fallback for older Python
    except ImportError:
        tomllib = None

from rapidfuzz import fuzz
from ytmusicapi import YTMusic
from mutagen import File as MutagenFile
from mutagen.id3 import ID3, APIC, ID3NoHeaderError
from rich.console import Console
from rich.table import Table
from rich.progress import (
    Progress,
    SpinnerColumn,
    BarColumn,
    TextColumn,
    TimeRemainingColumn,
)

# PanPipe Integration
try:
    from panpipe_integration import create_integration
    PANPIPE_AVAILABLE = True
except ImportError:
    PANPIPE_AVAILABLE = False


# --- Configurable ROOT -------------------------------------------------------
# Prefer ~/BangTunes if it exists, otherwise fall back to ~/Builds/BangTunes.

# Debug mode support
DEBUG_MODE = os.getenv("BANGTUNES_DEBUG", "false").lower() in ("true", "1", "yes")
def detect_root() -> Path:
    home = Path.home()
    preferred = home / "BangTunes"
    builds = home / "Builds" / "BangTunes"
    if preferred.exists():
        return preferred
    return builds


def load_config() -> dict:
    """Load configuration from TOML file if available."""
    if tomllib is None:
        return {}

    candidates = [
        Path.home() / ".config" / "bangtunes.toml",
        detect_root() / "bangtunes.toml",
    ]
    for p in candidates:
        if p.exists():
            with p.open("rb") as f:
                try:
                    return tomllib.load(f) or {}
                except Exception:
                    return {}
    return {}


@contextmanager
def graceful_sigint() -> Generator[Dict[str, bool], None, None]:
    """Handle Ctrl+C gracefully so we don't corrupt downloads"""
    stop = {"hit": False}

    def handler(signum: int, frame: Any) -> None:
        stop["hit"] = True

    old = signal.signal(signal.SIGINT, handler)
    try:
        yield stop
    finally:
        signal.signal(signal.SIGINT, old)


# --- Paths & constants ---
CFG = load_config()
ROOT = Path(CFG.get("root", str(detect_root())))
SEED = ROOT / "seed.csv"
BATCH_DIR = ROOT / "batches"
DL_ROOT = ROOT / "downloads"
DB = ROOT / "library.db"
ARCHIVE = ROOT / "download_archive.txt"

BATCH_SIZE = int(CFG.get("size", 50))
MAX_CANDIDATES_PER_SEED = 80
MIN_SCORE = int(CFG.get("min_score", 50))
DEFAULT_FORMAT = CFG.get("format", "opus")

console = Console()

# --- Banner ------------------------------------------------------------------
BANNER = dedent(r"""
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
""").strip("\n")


def print_banner() -> None:
    if os.environ.get("BANGTUNES_NO_BANNER") == "1":
        return
    console.rule("[bold magenta]Bang Tunes[/bold magenta]")
    console.print(f"[dim]{ROOT}[/dim]")
    console.print(f"[white]\n{BANNER}\n[/white]")
    console.rule()


# --- DB layer ---
def db_init() -> sqlite3.Connection:
    """Initialize database with schema - returns connection for immediate use."""
    conn = sqlite3.connect(DB)
    cur = conn.cursor()
    cur.execute("""
    CREATE TABLE IF NOT EXISTS tracks(
        id INTEGER PRIMARY KEY,
        youtube_id TEXT UNIQUE,
        title TEXT,
        artist TEXT,
        album TEXT,
        file_path TEXT,
        added_on TEXT DEFAULT CURRENT_TIMESTAMP
    );
    """)
    cur.execute("CREATE INDEX IF NOT EXISTS idx_artist ON tracks(artist);")
    cur.execute("CREATE INDEX IF NOT EXISTS idx_album ON tracks(album);")
    conn.commit()
    return conn


def get_db():
    """Get database connection with context manager for consistent resource handling.
    
    Automatically initializes schema if database doesn't exist.
    """
    db_exists = Path(DB).exists()
    conn = sqlite3.connect(DB)
    
    # Set up the database schema if it's a fresh install
    if not db_exists:
        cur = conn.cursor()
        cur.execute("""
        CREATE TABLE IF NOT EXISTS tracks(
            id INTEGER PRIMARY KEY,
            youtube_id TEXT UNIQUE,
            title TEXT,
            artist TEXT,
            album TEXT,
            file_path TEXT,
            added_on TEXT DEFAULT CURRENT_TIMESTAMP
        );
        """)
        # These indexes help with the stats queries
        cur.execute("CREATE INDEX IF NOT EXISTS idx_artist ON tracks(artist);")
        cur.execute("CREATE INDEX IF NOT EXISTS idx_album ON tracks(album);")
        conn.commit()
    
    return conn


def db_has_yid(conn: sqlite3.Connection, yid: str) -> bool:
    cur = conn.cursor()
    cur.execute("SELECT 1 FROM tracks WHERE youtube_id = ? LIMIT 1", (yid,))
    return cur.fetchone() is not None


def db_add_track(
    conn: sqlite3.Connection,
    yid: str,
    title: str,
    artist: str,
    album: str,
    file_path: str,
) -> None:
    cur = conn.cursor()
    cur.execute(
        """
    INSERT OR IGNORE INTO tracks(youtube_id,title,artist,album,file_path)
    VALUES(?,?,?,?,?)
    """,
        (yid, title, artist, album, file_path),
    )
    conn.commit()


def db_summary(conn: sqlite3.Connection) -> tuple[int, int, int]:
    cur = conn.cursor()
    cur.execute(
        "SELECT COUNT(*), COUNT(DISTINCT artist), COUNT(DISTINCT album) FROM tracks;"
    )
    result = cur.fetchone()
    return result if result else (0, 0, 0)  # (tracks, artists, albums)


def sanitize(name: str) -> str:
    return re.sub(r"[^-\w.\s]", "_", name).strip()


# --- Metadata helpers ---
def embed_easy_tags(
    path: Path, title: str, artist: str, album: Optional[str], year: Optional[str]
) -> None:
    # Use EasyID3 for mp3; mutagen handles opus/vorbis differently (vorbis comments)
    audio = MutagenFile(path, easy=True)
    if audio is None:
        return
    audio["title"] = [title] if title else []
    if artist:
        audio["artist"] = [artist]
    if album:
        audio["album"] = [album]
    if year:
        audio["date"] = [str(year)]
    audio.save()


def embed_cover_mp3(path: Path, image_bytes: bytes) -> None:
    try:
        tags = ID3(path)
    except ID3NoHeaderError:
        tags = ID3()
    tags["APIC"] = APIC(
        encoding=3, mime="image/jpeg", type=3, desc="Cover", data=image_bytes
    )
    tags.save(path)


def fetch_ytm_details(ytm: "YTMusic", title: str, artist: str) -> Optional[Dict]:
    # Try YT Music search for richer metadata and a cover
    q = f"{title} {artist}".strip()
    try:
        hits = ytm.search(q, filter="songs")[:1]
        if not hits:
            if DEBUG_MODE:
                console.print(f"[yellow]DEBUG: No YTM hits for '{q}'[/yellow]")
            return None
        h = hits[0]
        info = {
            "title": h.get("title") or title,
            "artist": ", ".join(
                [a.get("name") for a in (h.get("artists") or []) if a.get("name")]
            ),
            "album": (h.get("album") or {}).get("name"),
            "year": h.get("year"),
        }
        # Try to fetch thumbnail bytes (highest res available)
        thumbs = h.get("thumbnails") or []
        if thumbs:
            # thumbnails are dicts with url; but ytmusicapi often returns http(s) links
            import urllib.request

            thumbs_sorted = sorted(
                thumbs, key=lambda t: t.get("width", 0), reverse=True
            )
            url = thumbs_sorted[0].get("url")
            if url:
                try:
                    with urllib.request.urlopen(url, timeout=10) as resp:
                        info["cover_bytes"] = resp.read()
                except Exception:
                    info["cover_bytes"] = None
        return info
    except Exception as e:
        if DEBUG_MODE:
            console.print(f"[yellow]DEBUG: YTM fetch error for '{q}': {e}[/yellow]")
        return None


# --- YT Music related discovery ---
def normalize_artists(artists: Optional[List[Dict]]) -> str:
    names = []
    for a in artists or []:
        nm = a.get("name")
        if nm:
            names.append(nm)
    return ", ".join(names) if names else ""


def search_related(ytm: "YTMusic", title: str, artist: str) -> List[Dict[str, str]]:
    out = []

    # direct song search
    q = f"{title} {artist}".strip() if (title or artist) else title
    try:
        for r in ytm.search(q, filter="songs")[:40]:
            vid = r.get("videoId")
            if not vid:
                continue
            out.append(
                {
                    "title": r.get("title"),
                    "artist": normalize_artists(r.get("artists")),
                    "videoId": vid,
                }
            )
    except Exception as e:
        if DEBUG_MODE:
            console.print(f"[yellow]DEBUG: YTM search error for '{q}': {e}[/yellow]")
        pass

    # artist top tracks as proxy for "related"
    if artist:
        try:
            a_hit = ytm.search(artist, filter="artists")[:1]
            if a_hit:
                aid = a_hit[0].get("browseId")
                if aid:
                    apage = ytm.get_artist(aid)
                    for sec in apage.get("songs", {}).get("results", [])[:60]:
                        vid = sec.get("videoId")
                        if not vid:
                            continue
                        out.append(
                            {
                                "title": sec.get("title"),
                                "artist": normalize_artists(sec.get("artists")),
                                "videoId": vid,
                            }
                        )
        except Exception as e:
            if DEBUG_MODE:
                console.print(f"[yellow]DEBUG: YTM artist search error: {e}[/yellow]")
            pass

    # de-dupe
    seen, uniq = set(), []
    for x in out:
        if x["videoId"] in seen:
            continue
        seen.add(x["videoId"])
        uniq.append(x)
    return uniq[:MAX_CANDIDATES_PER_SEED]


def build_pool_from_seed(
    seed_rows: List[Dict[str, str]], cc_only: bool = False
) -> List[Dict[str, str]]:
    ytm = YTMusic()  # using public mode for now, might add login later
    pool = []  # collect all candidates here
    for row in seed_rows:
        base_key = " ".join(
            [row.get("title", ""), row.get("artist", ""), row.get("notes", "")]
        ).strip()
        key = f"{base_key} creative commons" if cc_only else base_key
        cands = search_related(ytm, row.get("title", ""), row.get("artist", ""))
        for c in cands:
            text = f"{c['title']} {c['artist']}"
            c["score"] = fuzz.token_set_ratio(key, text)
            c["seed_key"] = key
        filtered_cands = []
        for c in cands:
            score = c.get("score", 0)
            if isinstance(score, (int, float)) and score >= MIN_SCORE:
                filtered_cands.append(c)
        pool.extend(filtered_cands)
        time.sleep(0.5)  # be nice to YouTube's servers
    pool.sort(key=lambda x: x["score"], reverse=True)
    # remove duplicates from the pool
    seen, final = set(), []
    for c in pool:
        if c["videoId"] in seen:
            continue
        seen.add(c["videoId"])
        final.append(c)
    return final


# --- Batch I/O ---
def read_seed() -> List[Dict[str, str]]:
    if not SEED.exists():
        console.print(f"[red]Missing seed file[/red]: {SEED}")
        sys.exit(1)
    rows = []
    with open(SEED, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if row.get("title") or row.get("artist"):
                rows.append(row)
    if not rows:
        console.print(
            "[red]seed.csv is empty or malformed. Need headers: title,artist,notes[/red]"
        )
        sys.exit(1)
    return rows


def write_batches(pool: List[Dict[str, str]], prefix: str, size: int) -> List[Path]:
    BATCH_DIR.mkdir(parents=True, exist_ok=True)
    files = []
    for i in range(0, len(pool), size):
        chunk = pool[i : i + size]
        if not chunk:
            break
        name = f"{prefix}_{i // size + 1:03d}.csv"
        path = BATCH_DIR / name
        with open(path, "w", newline="", encoding="utf-8") as f:
            w = csv.DictWriter(
                f, fieldnames=["title", "artist", "videoId", "score", "seed_key"]
            )
            w.writeheader()
            w.writerows(chunk)
        files.append(path)
    return files


def read_batch_csv(path: Path) -> List[Dict[str, str]]:
    if not path.exists():
        path = BATCH_DIR / path
    if not path.exists():
        console.print(f"[red]Batch not found:[/red] {path}")
        sys.exit(1)
    out = []
    with open(path, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if row.get("videoId"):
                out.append(row)
    if not out:
        console.print("[yellow]No entries in batch.[/yellow]")
        sys.exit(1)
    return out


def yid_to_url(yid: str) -> str:
    return f"https://www.youtube.com/watch?v={yid}"


def with_retries(
    fn: Callable[..., Any], attempts: int = 3, delay: int = 2, *args: Any, **kwargs: Any
) -> Optional[Path]:
    """Retry wrapper for flaky network operations."""
    for i in range(attempts):
        result = fn(*args, **kwargs)
        if result:
            return result  # type: ignore
        if i < attempts - 1:  # Don't sleep on the last attempt
            time.sleep(delay * (i + 1))
    return None


def organize_target(
    base: Path, artist: str, album: Optional[str], title: str, ext: str
) -> Path:
    artist_dir = base / sanitize(artist or "Unknown Artist")
    album_dir = artist_dir / sanitize(album or "Unknown Album")
    album_dir.mkdir(parents=True, exist_ok=True)
    return album_dir / f"{sanitize(title)}.{ext}"


def get_random_user_agent() -> str:
    """Get a random user agent to avoid detection patterns."""
    user_agents = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:120.0) Gecko/20100101 Firefox/120.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/120.0.0.0 Safari/537.36",
    ]
    return random.choice(user_agents)


def run_ytdlp_audio(
    url: str, out_dir: Path, audio_format: str = "opus"
) -> Optional[Path]:
    # temp out: we'll move after tagging
    out_dir.mkdir(parents=True, exist_ok=True)
    tmpl = str(out_dir / "%(id)s.%(ext)s")
    
    # Add random jitter to sleep intervals (2-8 seconds base, 5-15 max)
    base_sleep = random.uniform(2.0, 8.0)
    max_sleep = random.uniform(8.0, 15.0)
    
    # Make yt-dlp path resilient
    ytdlp_path = ROOT / "venv" / "bin" / "yt-dlp"
    if not ytdlp_path.exists():
        ytdlp_fallback = shutil.which("yt-dlp")
        if ytdlp_fallback:
            ytdlp_path = ytdlp_fallback
        else:
            ytdlp_path = "yt-dlp"  # Last resort
    
    # Use download archive to avoid re-downloading
    archive_file = ROOT / "download_archive.txt"
    
    cmd = [
        str(ytdlp_path),
        "--quiet",
        "--no-warnings",
        "--force-ipv4",
        "--concurrent-fragments", "3",
        "--extract-audio",
        "--audio-format", audio_format,
        "--audio-quality", "0",
        "--embed-metadata",
        "--add-metadata",
        "--download-archive", str(archive_file),
        "--user-agent", get_random_user_agent(),
        "--sleep-interval", str(base_sleep),
        "--max-sleep-interval", str(max_sleep),
        "--retries", "8",
        "--fragment-retries", "8",
        "--retry-sleep", "exp=2:10",
        "-o", tmpl,
        url,
    ]
    try:
        subprocess.run(cmd, check=True)
        # Safer file pick: match by video ID from URL
        vid = url.rsplit("=", 1)[-1]
        files = list(out_dir.glob(f"{vid}.*"))
        if not files:
            return None
        # pick newest matching file
        path = max(files, key=lambda p: p.stat().st_mtime)
        return path
    except subprocess.CalledProcessError as e:
        if DEBUG_MODE:
            console.print(f"[yellow]DEBUG: yt-dlp failed for {url}: {e}[/yellow]")
        return None


def download_batch(batch_path: Path, audio_format: str = "opus") -> None:
    conn = get_db()
    items = read_batch_csv(batch_path)
    tag = batch_path.stem if batch_path.suffix == ".csv" else batch_path.name
    temp_dir = DL_ROOT / f"tmp_{tag}"
    final_root = DL_ROOT / tag
    temp_dir.mkdir(parents=True, exist_ok=True)
    final_root.mkdir(parents=True, exist_ok=True)

    console.print("[bold]Starting downloads…[/bold]")
    ytm = YTMusic()
    failures = 0
    failed_tracks = []  # Track failed downloads for summary

    with (
        graceful_sigint() as flag,
        Progress(
            SpinnerColumn(),
            TextColumn("[bold blue]{task.description}"),
            BarColumn(),
            TextColumn("{task.completed}/{task.total}"),
            TimeRemainingColumn(),
            console=console,
        ) as progress,
    ):
        task = progress.add_task(f"Downloading {tag}", total=len(items))
        for row in items:
            if flag["hit"]:
                console.print(
                    "\n[yellow]Download interrupted by user (Ctrl+C)[/yellow]"
                )
                break
            yid = row["videoId"]
            url = yid_to_url(yid)

            # skip if already in DB
            if db_has_yid(conn, yid):
                progress.advance(task)
                continue

            tmp = with_retries(run_ytdlp_audio, 3, 2, url, temp_dir, audio_format)
            if tmp is None or not tmp.exists():
                failures += 1
                failed_tracks.append({
                    "title": row.get("title", "Unknown"),
                    "artist": row.get("artist", "Unknown"),
                    "url": url,
                    "reason": "Download failed"
                })
                progress.advance(task)
                # Random delay on failure to avoid predictable patterns (3-8 seconds)
                failure_delay = random.uniform(3.0, 8.0)
                time.sleep(failure_delay)
                continue

            # try to enrich metadata & relocate
            title = row.get("title") or "Unknown Title"
            artist = row.get("artist") or "Unknown Artist"
            album = None
            year = None

            meta = fetch_ytm_details(ytm, title, artist)
            if meta:
                title = meta.get("title") or title
                artist = meta.get("artist") or artist
                album = meta.get("album")
                year = meta.get("year")

            # Embed metadata tags
            embed_easy_tags(tmp, title, artist, album, str(year) if year else None)

            # Cover art embedding for MP3 format
            if audio_format == "mp3" and meta and meta.get("cover_bytes"):
                embed_cover_mp3(tmp, meta["cover_bytes"])

            target = organize_target(
                final_root, artist, album, title, tmp.suffix.lstrip(".")
            )
            try:
                if target.exists():
                    target.unlink()
                shutil.move(str(tmp), str(target))
                db_add_track(conn, yid, title, artist, album or "", str(target))
                # Random delay after successful download (2-6 seconds)
                success_delay = random.uniform(2.0, 6.0)
                time.sleep(success_delay)
            except Exception:
                failures += 1

            progress.advance(task)

    # cleanup temps
    try:
        shutil.rmtree(temp_dir, ignore_errors=True)
    except Exception:
        pass

    t, a, al = db_summary(conn)
    console.print(
        f"[bold green]Batch done[/bold green] — total in library: [cyan]{t}[/cyan] tracks, [cyan]{a}[/cyan] artists, [cyan]{al}[/cyan] albums. Failures: {failures}"
    )
    
    # Enhanced debug logging for failures
    if failed_tracks and DEBUG_MODE:
        console.print("\n[yellow]🔍 Debug: Failed Downloads Summary[/yellow]")
        for i, track in enumerate(failed_tracks[:10], 1):  # Show first 10 failures
            console.print(f"[dim]  {i}. {track['title']} by {track['artist']} - {track['reason']}[/dim]")
            console.print(f"[dim]     URL: {track['url']}[/dim]")
        if len(failed_tracks) > 10:
            console.print(f"[dim]  ... and {len(failed_tracks) - 10} more failures[/dim]")
        console.print("[dim]💡 Common causes: YouTube rate limiting, region blocks, or deleted videos[/dim]")


# --- Library views ---
def show_library() -> None:
    with get_db() as conn:
        cur = conn.cursor()
        cur.execute(
            "SELECT artist, COUNT(*) as n FROM tracks GROUP BY artist ORDER BY n DESC, artist ASC LIMIT 50;"
        )
        rows = cur.fetchall()
        
    table = Table(title="Bang Tunes — Top Artists")
    table.add_column("Artist", style="magenta", overflow="fold")
    table.add_column("# Tracks", style="cyan", justify="right")
    for artist, n in rows:
        table.add_row(artist or "Unknown", str(n))
    console.print(table)


def list_batches() -> None:
    """List available batch CSV files with entry counts."""
    BATCH_DIR.mkdir(parents=True, exist_ok=True)
    rows = []
    for p in sorted(BATCH_DIR.glob("*.csv")):
        try:
            with p.open(encoding="utf-8") as f:
                n = sum(1 for _ in csv.DictReader(f))
        except Exception:
            n = 0
        rows.append((p.name, n))

    table = Table(title="Bang Tunes — Batches")
    table.add_column("File", style="magenta")
    table.add_column("# Entries", style="cyan", justify="right")
    for name, n in rows:
        table.add_row(name, str(n))
    console.print(table)


def rescan_library(fix_issues: bool = False) -> None:
    """Compare DB with disk and report mismatches. Optionally fix orphan files."""
    with get_db() as conn:
        cur = conn.cursor()
        cur.execute("SELECT youtube_id, file_path, title, artist FROM tracks;")
        db_rows = cur.fetchall()

        # Check for missing files referenced in DB
        missing = []
        for yid, fpath, title, artist in db_rows:
            if not fpath or not Path(fpath).exists():
                missing.append((yid, fpath, title, artist))

        # Find all audio files on disk
        on_disk = []
        db_file_paths = {row[1] for row in db_rows if row[1]}  # Set of known file paths
        
        if DL_ROOT.exists():
            for p in DL_ROOT.rglob("*.*"):
                if p.is_file() and p.suffix.lower() in [".opus", ".mp3", ".flac", ".m4a"]:
                    on_disk.append(p)

        # Find orphan files (on disk but not in DB)
        orphans = []
        for file_path in on_disk:
            if str(file_path) not in db_file_paths:
                orphans.append(file_path)

        # Calculate file_path breakdown
        entries_with_files = sum(1 for _, fpath, _, _ in db_rows if fpath)
        entries_metadata_only = len(db_rows) - entries_with_files
        
        # Report findings with clear breakdown
        if entries_metadata_only > 0:
            console.print(
                f"[bold]Rescan Results[/bold]: {len(db_rows)} DB entries ({entries_with_files} with files, {entries_metadata_only} metadata-only), {len(on_disk)} files on disk, {len(orphans)} orphans"
            )
        else:
            console.print(
                f"[bold]Rescan Results[/bold]: {len(db_rows)} DB entries, {len(on_disk)} files on disk, {len(orphans)} orphans"
            )

        # Report missing files
        if missing:
            console.print(
                f"[yellow]⚠️  Missing files referenced in DB ({len(missing)}):[/yellow]"
            )
            for yid, fpath, title, artist in missing[:10]:
                console.print(f" - {title} by {artist} ({yid})")
                if DEBUG_MODE:
                    console.print(f"   Path: {fpath}")
            if len(missing) > 10:
                console.print(f" ... and {len(missing) - 10} more")
            
            if fix_issues:
                # Confirmation prompt for missing DB entries
                console.print(f"[yellow]⚠️  About to remove {len(missing)} missing entries from database[/yellow]")
                try:
                    confirm = input("Continue? [y/N]: ").strip().lower()
                    if confirm not in ['y', 'yes']:
                        console.print("[dim]Skipped removing missing DB entries[/dim]")
                    else:
                        console.print("[red]🔧 Removing missing file entries from DB...[/red]")
                        missing_ids = [row[0] for row in missing]
                        placeholders = ','.join('?' * len(missing_ids))
                        cur.execute(f"DELETE FROM tracks WHERE youtube_id IN ({placeholders})", missing_ids)
                        conn.commit()
                        console.print(f"[green]✅ Removed {len(missing)} missing entries[/green]")
                except (KeyboardInterrupt, EOFError):
                    console.print("\n[dim]Operation cancelled[/dim]")
        else:
            console.print("[green]✅ All DB entries have valid file paths[/green]")

        # Report orphan files
        if orphans:
            console.print(
                f"[cyan]📁 Orphan files found ({len(orphans)}) - on disk but not in DB:[/cyan]"
            )
            for orphan in orphans[:10]:
                rel_path = orphan.relative_to(DL_ROOT) if orphan.is_relative_to(DL_ROOT) else orphan
                console.print(f" - {rel_path}")
            if len(orphans) > 10:
                console.print(f" ... and {len(orphans) - 10} more")
            
            if not fix_issues:
                console.print("[dim]💡 Use --fix to attempt automatic cleanup of orphan files[/dim]")
            else:
                # Confirmation prompt for orphan file cleanup
                console.print(f"[yellow]⚠️  About to delete {len(orphans)} orphan files from disk[/yellow]")
                try:
                    confirm = input("Continue? [y/N]: ").strip().lower()
                    if confirm not in ['y', 'yes']:
                        console.print("[dim]Skipped orphan file cleanup[/dim]")
                    else:
                        console.print("[yellow]🔧 Cleaning up orphan files...[/yellow]")
                        removed_count = 0
                        for orphan in orphans:
                            try:
                                orphan.unlink()
                                removed_count += 1
                                if DEBUG_MODE:
                                    console.print(f"[dim]   Removed: {orphan}[/dim]")
                            except Exception as e:
                                if DEBUG_MODE:
                                    console.print(f"[red]   Failed to remove {orphan}: {e}[/red]")
                        console.print(f"[green]✅ Removed {removed_count} orphan files[/green]")
                except (KeyboardInterrupt, EOFError):
                    console.print("\n[dim]Operation cancelled[/dim]")
        else:
            console.print("[green]✅ No orphan files found[/green]")

        # Summary
        if not missing and not orphans:
            console.print("[bold green]🎉 Library is perfectly synchronized![/bold green]")
        elif fix_issues:
            console.print("[bold blue]🔧 Library cleanup completed[/bold blue]")


def first_run_wizard() -> None:
    """First-run setup - checks deps and gets you started"""
    console.print("[bold green]Bang Tunes First-Run Setup Wizard[/bold green]")
    console.print("[dim]Creating the perfect music discovery environment...[/dim]")
    console.print()
    
    # Check system dependencies
    console.print("[bold]1. Checking system dependencies...[/bold]")
    
    # Check Python
    python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
    if sys.version_info >= (3, 8):
        console.print(f"   ✓ Python {python_version} (compatible)")
    else:
        console.print(f"   ✗ Python {python_version} (need 3.8+)")
        console.print("[red]Please upgrade Python and try again[/red]")
        return
    
    # Check ffmpeg
    try:
        subprocess.run(["ffmpeg", "-version"], capture_output=True, check=True)
        console.print("   ✓ ffmpeg (found)")
    except (subprocess.CalledProcessError, FileNotFoundError):
        console.print("   ⚠ ffmpeg (not found - audio conversion may fail)")
        console.print("   [dim]Install: sudo apt install ffmpeg (Ubuntu) or brew install ffmpeg (macOS)[/dim]")
    
    # Check yt-dlp
    try:
        import yt_dlp
        console.print("   ✓ yt-dlp (installed)")
    except ImportError:
        console.print("   ✗ yt-dlp (missing)")
        console.print("[red]Run: pip install yt-dlp[/red]")
        return
    
    console.print()
    
    # Create directories
    console.print("[bold]2. Creating project structure...[/bold]")
    for directory in ["batches", "downloads"]:
        dir_path = ROOT / directory
        dir_path.mkdir(exist_ok=True)
        console.print(f"   ✓ {directory}/")
    
    console.print()
    
    # Create example seed if missing
    console.print("[bold]3. Setting up music preferences...[/bold]")
    seed_file = ROOT / "seed.csv"
    if not seed_file.exists():
        example_seeds = [
            "title,artist,notes",
            "Parasite Eve,Bring Me The Horizon,metalcore",
            "Take Me Back To Eden,Sleep Token,progressive metal",
            "Monster,Starset,electronic rock",
            "Doomsday,Architects,metalcore",
            "Just Pretend,Bad Omens,alternative metal"
        ]
        seed_file.write_text("\n".join(example_seeds) + "\n")
        console.print("   ✓ Created example seed.csv with popular tracks")
    else:
        console.print("   ✓ seed.csv already exists")
    
    console.print()
    
    # Test YouTube Music API
    console.print("[bold]4. Testing YouTube Music connection...[/bold]")
    try:
        ytmusic = YTMusic()
        test_results = ytmusic.search("test", filter="songs", limit=1)
        if test_results:
            console.print("   ✓ YouTube Music API working")
        else:
            console.print("   ⚠ YouTube Music API connected but no results")
    except Exception as e:
        console.print(f"   ⚠ YouTube Music API issue: {e}")
        console.print("   [dim]This may work anyway - try building a batch[/dim]")
    
    console.print()
    
    # Success message with next steps
    console.print("[bold green]Setup Complete![/bold green]")
    console.print()
    console.print("[bold]Ready to discover music! Try these commands:[/bold]")
    console.print("   [cyan]python bang_tunes.py build[/cyan]           # Build discovery batches")
    console.print("   [cyan]python bang_tunes.py download mix_001.csv[/cyan]  # Download first batch")
    console.print("   [cyan]python bang_tunes.py stats[/cyan]            # View library stats")
    console.print("   [cyan]python bang_tunes.py quickplay[/cyan]        # Play music instantly")
    console.print()
    console.print("[dim]Edit seed.csv to customize your music taste, then run build again![/dim]")


def show_library_stats() -> None:
    """Show some stats about your music collection"""
    console.print("[bold]Bang Tunes Library Statistics[/bold]")
    console.print()
    
    with get_db() as conn:
        cur = conn.cursor()
        
        # Basic counts
        cur.execute("SELECT COUNT(*) FROM tracks")
        total_tracks = cur.fetchone()[0]
        
        cur.execute("SELECT COUNT(DISTINCT artist) FROM tracks WHERE artist IS NOT NULL")
        total_artists = cur.fetchone()[0]
        
        cur.execute("SELECT COUNT(*) FROM tracks WHERE file_path IS NOT NULL")
        downloaded_tracks = cur.fetchone()[0]
        
        # File size calculation
        total_size = 0
        if DL_ROOT.exists():
            for file_path in DL_ROOT.rglob("*.*"):
                if file_path.is_file() and file_path.suffix.lower() in [".opus", ".mp3", ".flac", ".m4a"]:
                    total_size += file_path.stat().st_size
        
        size_mb = total_size / (1024 * 1024)
        
        # Create stats table
        from rich.table import Table
        stats_table = Table(title="Library Overview", show_header=True)
        stats_table.add_column("Metric", style="cyan")
        stats_table.add_column("Value", style="green")
        
        stats_table.add_row("Total Tracks", str(total_tracks))
        stats_table.add_row("Downloaded Tracks", str(downloaded_tracks))
        stats_table.add_row("Unique Artists", str(total_artists))
        stats_table.add_row("Disk Usage", f"{size_mb:.1f} MB")
        
        console.print(stats_table)
        console.print()
        
        # Top artists
        if total_tracks > 0:
            cur.execute("""
                SELECT artist, COUNT(*) as count 
                FROM tracks 
                WHERE artist IS NOT NULL 
                GROUP BY artist 
                ORDER BY count DESC 
                LIMIT 10
            """)
            top_artists = cur.fetchall()
            
            if top_artists:
                artists_table = Table(title="Top Artists", show_header=True)
                artists_table.add_column("Artist", style="yellow")
                artists_table.add_column("Tracks", style="green")
                
                for artist, count in top_artists:
                    artists_table.add_row(artist, str(count))
                
                console.print(artists_table)
                console.print()
        
        # Fun stats
        cur.execute("SELECT title FROM tracks ORDER BY LENGTH(title) DESC LIMIT 1")
        longest_title = cur.fetchone()
        
        if longest_title:
            console.print(f"[bold]Longest Track Title:[/bold] {longest_title[0]}")
            console.print()
        
        # Batch success rate
        batch_files = list((ROOT / "batches").glob("*.csv")) if (ROOT / "batches").exists() else []
        console.print(f"[bold]Discovery Batches:[/bold] {len(batch_files)} created")
        
        if total_tracks == 0:
            console.print()
            console.print("[dim]No tracks yet! Run 'python bang_tunes.py build' to start discovering music.[/dim]")


def quick_play_mode() -> None:
    """Quick and dirty music player - just plays stuff without setup"""
    console.print("[bold]Quick Play Mode[/bold]")
    console.print("[dim]Instant music playback without setup[/dim]")
    console.print()
    
    # Find downloaded audio files
    audio_files = []
    if DL_ROOT.exists():
        for file_path in DL_ROOT.rglob("*.*"):
            if file_path.is_file() and file_path.suffix.lower() in [".opus", ".mp3", ".flac", ".m4a"]:
                audio_files.append(file_path)
    
    if not audio_files:
        console.print("[red]No audio files found![/red]")
        console.print("[dim]Download some music first:[/dim]")
        console.print("   [cyan]python bang_tunes.py build[/cyan]")
        console.print("   [cyan]python bang_tunes.py download mix_001.csv[/cyan]")
        return
    
    # Shuffle the playlist
    import random
    random.shuffle(audio_files)
    
    console.print(f"[green]Found {len(audio_files)} tracks![/green]")
    console.print()
    
    # Check for ffplay
    ffplay_available = False
    try:
        subprocess.run(["ffplay", "-version"], capture_output=True, check=True)
        ffplay_available = True
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    
    # Check for termux media player
    termux_available = False
    try:
        subprocess.run(["termux-media-player", "info"], capture_output=True, check=True)
        termux_available = True
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    
    if not ffplay_available and not termux_available:
        console.print("[red]No audio player found![/red]")
        console.print("[dim]Install one of these:[/dim]")
        console.print("   • ffplay (part of ffmpeg): sudo apt install ffmpeg")
        console.print("   • termux-media-player (Termux): pkg install termux-api")
        console.print()
        console.print("[dim]Or use the full PanPipe player:[/dim]")
        console.print("   [cyan]python bang_tunes.py setup-player && python bang_tunes.py play[/cyan]")
        return
    
    # Play music
    console.print("[bold green]Starting playback...[/bold green]")
    console.print("[dim]Press Ctrl+C to stop[/dim]")
    console.print()
    
    try:
        for i, audio_file in enumerate(audio_files[:10]):  # Play first 10 tracks
            # Get track info from database
            with get_db() as conn:
                cur = conn.cursor()
                cur.execute("SELECT title, artist FROM tracks WHERE file_path = ?", (str(audio_file),))
                track_info = cur.fetchone()
            
            if track_info:
                title, artist = track_info
                console.print(f"[bold]Now Playing ({i+1}/10):[/bold] {artist} - {title}")
            else:
                console.print(f"[bold]Now Playing ({i+1}/10):[/bold] {audio_file.name}")
            
            # Play the file
            if ffplay_available:
                # Use ffplay with minimal UI
                subprocess.run([
                    "ffplay", "-nodisp", "-autoexit", "-loglevel", "quiet", str(audio_file)
                ], check=False)
            elif termux_available:
                # Use termux media player
                subprocess.run(["termux-media-player", "play", str(audio_file)], check=False)
                # Wait for completion (termux doesn't block)
                import time
                time.sleep(30)  # Assume 30 second tracks for demo
            
            console.print()
    
    except KeyboardInterrupt:
        console.print("\n[yellow]Playback stopped by user[/yellow]")
    
    console.print("[dim]For a better music experience, try the full PanPipe player:[/dim]")
    console.print("   [cyan]python bang_tunes.py setup-player && python bang_tunes.py play[/cyan]")


# --- CLI ---
def main() -> None:
    global MIN_SCORE, BATCH_SIZE

    ROOT.mkdir(parents=True, exist_ok=True)
    BATCH_DIR.mkdir(parents=True, exist_ok=True)
    DL_ROOT.mkdir(parents=True, exist_ok=True)
    db_init()

    ap = argparse.ArgumentParser(
        prog="Bang Tunes", description="Unified Music Discovery & Playback System (Termux/Linux)"
    )
    ap.add_argument("--no-banner", action="store_true", help="Hide ASCII banner")
    sub = ap.add_subparsers(dest="cmd", required=True)

    b = sub.add_parser("build", help="Build batches from seed.csv")
    b.add_argument("--prefix", default="mix", help="Batch prefix (default: mix)")
    b.add_argument(
        "--min-score", type=int, default=MIN_SCORE, help="Fuzzy floor (default: 50)"
    )
    b.add_argument(
        "--size", type=int, default=BATCH_SIZE, help="Tracks per batch (default: 50)"
    )
    b.add_argument(
        "--cc-only",
        action="store_true",
        help="Bias discovery toward Creative Commons content",
    )

    d = sub.add_parser("download", help="Download a batch CSV to audio & add to DB")
    d.add_argument("batch_csv", help="Path under batches/ or full path")
    d.add_argument(
        "--format",
        choices=["opus", "mp3", "flac"],
        default=DEFAULT_FORMAT,
        help="Audio format (default: opus)",
    )

    sub.add_parser("view", help="Show quick library summary")
    sub.add_parser("list-batches", help="Show available batch CSVs and sizes")
    sub.add_parser("install", help="First-run setup wizard - creates directories, checks deps, tests API")
    sub.add_parser("stats", help="Show library statistics and gallery of results")
    sub.add_parser("quickplay", help="Instant music playback (ffplay/termux) - no setup required")
    
    r = sub.add_parser("rescan", help="Compare DB with disk and report mismatches")
    r.add_argument(
        "--fix", action="store_true", help="Automatically fix orphan files and missing entries"
    )
    
    # Integrated Player Commands
    if PANPIPE_AVAILABLE:
        sub.add_parser("play", help="Launch full PanPipe player (TUI with smart shuffle, behavior tracking)")
        sub.add_parser("setup-player", help="Setup integrated PanPipe music player")
        sub.add_parser("sync", help="Sync library with PanPipe player database")
        sub.add_parser("player-status", help="Show PanPipe player integration status")

    args = ap.parse_args()

    if not args.no_banner:
        print_banner()

    if args.cmd == "build":
        MIN_SCORE = args.min_score
        BATCH_SIZE = args.size
        seeds = read_seed()
        console.print(
            f"[bold]Building pool[/bold] from [cyan]{len(seeds)}[/cyan] seed rows…"
        )
        pool = build_pool_from_seed(seeds, cc_only=getattr(args, "cc_only", False))
        files = write_batches(pool, args.prefix, BATCH_SIZE)
        if files:
            console.print(f"[green]Built {len(files)} batch file(s):[/green]")
            for p in files:
                console.print(f" - {p.name}")
        else:
            console.print("[yellow]No candidates met the score threshold.[/yellow]")
        return

    if args.cmd == "download":
        p = Path(args.batch_csv)
        download_batch(p, args.format)
        return

    if args.cmd == "view":
        show_library()
        return

    if args.cmd == "list-batches":
        list_batches()
        return

    if args.cmd == "rescan":
        rescan_library(fix_issues=getattr(args, "fix", False))
        return
    
    # PanPipe integration commands
    if PANPIPE_AVAILABLE:
        config = load_config()
        integration = create_integration(ROOT, config)
        
        if args.cmd == "play":
            integration.launch_player()
            return
        
        if args.cmd == "setup-player":
            console.print("[bold]🎵 Setting up PanPipe integration...[/bold]")
            integration.setup_panpipe_config()
            integration.sync_libraries()
            console.print("[green]✅ PanPipe integration setup complete![/green]")
            console.print("[cyan]💡 Use 'python bang_tunes.py play' to launch the player[/cyan]")
            return
        
        if args.cmd == "sync":
            console.print("[bold]🔄 Syncing libraries...[/bold]")
            integration.sync_libraries()
            return
        
        if args.cmd == "player-status":
            integration.status()
            return
    
    # New Reddit-ready commands for instant gratification
    if args.cmd == "install":
        first_run_wizard()
        return
    
    if args.cmd == "stats":
        show_library_stats()
        return
    
    if args.cmd == "quickplay":
        quick_play_mode()
        return


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        console.print("\n[yellow]⚠️  Operation cancelled by user[/yellow]")
        sys.exit(1)
    except ImportError as e:
        console.print(f"[red]❌ Missing dependency: {e}[/red]")
        console.print("[cyan]💡 Run: ./setup.sh to install all dependencies[/cyan]")
        sys.exit(1)
    except FileNotFoundError as e:
        console.print(f"[red]❌ File not found: {e}[/red]")
        console.print("[cyan]💡 Make sure you're in the BangTunes directory[/cyan]")
        sys.exit(1)
    except PermissionError as e:
        console.print(f"[red]❌ Permission denied: {e}[/red]")
        console.print("[cyan]💡 Check file permissions or run with appropriate privileges[/cyan]")
        sys.exit(1)
    except Exception as e:
        console.print(f"[red]❌ Unexpected error: {e}[/red]")
        if DEBUG_MODE:
            import traceback
            console.print("[dim]Full traceback:[/dim]")
            traceback.print_exc()
        else:
            console.print("[cyan]💡 Run with BANGTUNES_DEBUG=1 for detailed error info[/cyan]")
        console.print("[cyan]🐛 If this persists, please report it as an issue[/cyan]")
        sys.exit(1)
