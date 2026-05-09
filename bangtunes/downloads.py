# SPDX-License-Identifier: MIT
# Copyright (c) 2024 BangTunes Contributors

"""
Download pipeline for BangTunes.

Handles yt-dlp integration, batch downloading, file organization, and metadata embedding.
"""

import random
import signal
import shutil
import subprocess
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Dict, Generator, List, Optional, Union, Type, Callable

try:
    from rich.console import Console
    from rich.progress import (
        Progress,
        SpinnerColumn,
        TextColumn,
        BarColumn,
        TimeElapsedColumn,
        TimeRemainingColumn,
        MofNCompleteColumn,
    )

    console: Optional[Console] = Console()
    ProgressType: Optional[Type[Progress]] = Progress
except ImportError:
    console = None
    ProgressType = None

try:
    from ytmusicapi import YTMusic
except ImportError:
    YTMusic = None

try:
    from mutagen import File as MutagenFile
except ImportError:
    MutagenFile = None


# Import shared constants to prevent configuration drift
from bangtunes.constants import DOWNLOAD_MODES


@contextmanager
def graceful_sigint() -> Generator[Dict[str, bool], None, None]:
    """Handle Ctrl+C gracefully so we don't corrupt downloads"""
    stop = {"hit": False}

    def handler(signum: int, frame: Any) -> None:
        stop["hit"] = True

    old_handler = signal.signal(signal.SIGINT, handler)
    try:
        yield stop
    finally:
        signal.signal(signal.SIGINT, old_handler)


def yid_to_url(yid: str) -> str:
    """Convert YouTube video ID to full URL."""
    return f"https://www.youtube.com/watch?v={yid}"


def with_retries(
    fn: Callable[..., Any], *args: Any, attempts: int = 3, delay: int = 2, **kwargs: Any
) -> Optional[Path]:
    """Retry wrapper for flaky network operations."""
    for i in range(attempts):
        result = fn(*args, **kwargs)
        if result:
            return result  # type: ignore
        if i < attempts - 1:  # Don't sleep on the last attempt
            time.sleep(delay * (i + 1))
    return None


def sanitize_filename(name: str) -> str:
    """Sanitize filename by replacing invalid characters."""
    import re

    return re.sub(r"[^-\w.\s]", "_", name).strip()


def organize_target(
    base: Path, artist: str, album: Optional[str], title: str, ext: str
) -> Path:
    """Organize downloaded file into artist/album/title structure."""
    artist_dir = base / sanitize_filename(artist or "Unknown Artist")
    album_dir = artist_dir / sanitize_filename(album or "Unknown Album")
    album_dir.mkdir(parents=True, exist_ok=True)
    return album_dir / f"{sanitize_filename(title)}.{ext}"


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
    url: str,
    out_dir: Path,
    root: Path,
    audio_format: str = "opus",
    download_mode: str = "normal",
    debug_mode: bool = False,
) -> Optional[Path]:
    """Download audio from YouTube URL using yt-dlp."""
    # temp out: we'll move after tagging
    out_dir.mkdir(parents=True, exist_ok=True)
    tmpl = str(out_dir / "%(id)s.%(ext)s")

    # Get delay settings from mode configuration
    mode_config = DOWNLOAD_MODES.get(download_mode, DOWNLOAD_MODES["normal"])
    base_sleep = random.uniform(*mode_config["base_sleep"])
    max_sleep = random.uniform(*mode_config["max_sleep"])

    # Make yt-dlp path resilient
    ytdlp_path: Union[str, Path] = root / "venv" / "bin" / "yt-dlp"
    if isinstance(ytdlp_path, Path) and not ytdlp_path.exists():
        ytdlp_fallback = shutil.which("yt-dlp")
        if ytdlp_fallback:
            ytdlp_path = ytdlp_fallback
        else:
            ytdlp_path = "yt-dlp"  # Last resort

    # Use download archive to avoid re-downloading
    archive_path = root / "downloads" / "archive.txt"

    cmd = [
        str(ytdlp_path),
        "--quiet",
        "--no-warnings",
        "--force-ipv4",
        "--concurrent-fragments",
        "3",
        "--extract-audio",
        "--audio-format",
        audio_format,
        "--audio-quality",
        "0",
        "--embed-metadata",
        "--add-metadata",
        "--download-archive",
        str(archive_path),
        "--user-agent",
        get_random_user_agent(),
        "--sleep-interval",
        str(base_sleep),
        "--max-sleep-interval",
        str(max_sleep),
        "--retries",
        "8",
        "--fragment-retries",
        "8",
        "--retry-sleep",
        "exp=2:10",
        "-o",
        tmpl,
        url,
    ]

    try:
        # Record time just before download so we can filter out pre-existing files
        download_start = time.time()

        # Fire off yt-dlp but don't let it hang forever (5 min max)
        subprocess.run(cmd, check=True, timeout=300, capture_output=True, text=True)

        # Try to find the file it just downloaded — only consider files created
        # during this call (mtime >= download_start) to avoid picking up files
        # from previous tracks that are still in the shared temp directory.
        vid_match = url.split("v=")[-1].split("&")[0]  # grab the video ID from the URL
        files = [
            f for f in out_dir.glob(f"{vid_match}.*")
            if f.stat().st_mtime >= download_start
        ]

        if not files:
            # No file matched by video ID — try any file created during this download
            all_new = [
                f for f in out_dir.glob("*.*")
                if f.stat().st_mtime >= download_start
            ]
            if all_new:
                path = max(all_new, key=lambda p: p.stat().st_mtime)
                # Warn if the format doesn't match what was requested
                actual_ext = path.suffix.lstrip(".")
                if actual_ext != audio_format and console:
                    console.print(
                        f"[yellow]⚠️  Format mismatch: requested {audio_format}, "
                        f"got {actual_ext} for {url}[/yellow]"
                    )
                return path
            return None

        # Found it by video ID — grab the newest in case of duplicates
        path = max(files, key=lambda p: p.stat().st_mtime)

        # Warn if the format doesn't match (e.g. ffmpeg not installed, yt-dlp fell back)
        actual_ext = path.suffix.lstrip(".")
        if actual_ext != audio_format and console:
            console.print(
                f"[yellow]⚠️  Format mismatch: requested {audio_format}, "
                f"got {actual_ext} for {url}[/yellow]"
            )

        return path

    except subprocess.TimeoutExpired:
        if debug_mode and console:
            console.print(f"[yellow]DEBUG: yt-dlp timeout for {url}[/yellow]")
        return None
    except subprocess.CalledProcessError as e:
        if debug_mode and console:
            console.print(f"[yellow]DEBUG: yt-dlp failed for {url}: {e}[/yellow]")
            if e.stderr:
                console.print(f"[yellow]DEBUG: stderr: {e.stderr}[/yellow]")
        return None
    except FileNotFoundError:
        if console:
            console.print(f"[red]Error: yt-dlp not found at {ytdlp_path}[/red]")
            console.print("[dim]Install yt-dlp: pip install yt-dlp[/dim]")
        else:
            print(f"Error: yt-dlp not found at {ytdlp_path}")
            print("Install yt-dlp: pip install yt-dlp")
        return None
    except PermissionError:
        if console:
            console.print(f"[red]Permission denied writing to {out_dir}[/red]")
            console.print("[dim]Check directory permissions and try again[/dim]")
        else:
            print(f"Permission denied writing to {out_dir}")
            print("Check directory permissions and try again")
        return None


def embed_easy_tags(
    path: Path, title: str, artist: str, album: Optional[str], year: Optional[str]
) -> None:
    """Embed metadata tags into audio file with error handling."""
    if not MutagenFile:
        return

    try:
        # Use EasyID3 for mp3; mutagen handles opus/vorbis differently (vorbis comments)
        audio = MutagenFile(path, easy=True)
        if audio is None:
            if console:
                console.print(
                    f"[yellow]⚠️ Could not load audio file for tagging: {path}[/yellow]"
                )
            return

        # Set basic tags
        audio["title"] = title
        audio["artist"] = artist
        if album:
            audio["album"] = album
        if year:
            audio["date"] = year

        audio.save()

    except Exception as e:
        if console:
            console.print(f"[yellow]⚠️ Could not embed tags in {path}: {e}[/yellow]")


def download_batch(
    batch_path: Path,
    batch_dir: Path,
    dl_root: Path,
    root: Path,
    db_conn: Any,
    db_has_yid_func: Callable,
    db_add_track_func: Callable,
    read_batch_csv_func: Callable,
    graceful_sigint_func: Callable,
    audio_format: str = "opus",
    download_mode: str = "normal",
    debug_mode: bool = False,
) -> None:
    """Download a batch of tracks from CSV file."""
    items = read_batch_csv_func(batch_path, batch_dir)
    tag = batch_path.stem if batch_path.suffix == ".csv" else batch_path.name
    temp_dir = dl_root / f"tmp_{tag}"
    final_root = dl_root / tag
    temp_dir.mkdir(parents=True, exist_ok=True)
    final_root.mkdir(parents=True, exist_ok=True)

    if console:
        console.print("[bold]Starting downloads…[/bold]")
    else:
        print("Starting downloads...")

    if not YTMusic:
        if console:
            console.print("[red]ytmusicapi not available - cannot fetch metadata[/red]")
        else:
            print("ytmusicapi not available - cannot fetch metadata")
        return

    ytm = YTMusic()
    failures = 0
    failed_tracks: list[dict[str, str]] = []  # Track failed downloads for summary

    if ProgressType is not None and console:
        with (
            graceful_sigint_func() as flag,
            Progress(
                SpinnerColumn(),
                TextColumn("[bold blue]{task.description}"),
                BarColumn(),
                MofNCompleteColumn(),
                TimeElapsedColumn(),
                TimeRemainingColumn(),
                console=console,
            ) as progress,
        ):
            task = progress.add_task(f"Downloading {tag}", total=len(items))
            failures = _download_batch_items(
                items,
                flag,
                task,
                progress,
                ytm,
                db_conn,
                db_has_yid_func,
                db_add_track_func,
                temp_dir,
                final_root,
                root,
                audio_format,
                download_mode,
                debug_mode,
                failures,
                failed_tracks,
            )
    else:
        # Fallback without progress bar
        flag = {"hit": False}
        failures = _download_batch_items(
            items,
            flag,
            None,
            None,
            ytm,
            db_conn,
            db_has_yid_func,
            db_add_track_func,
            temp_dir,
            final_root,
            root,
            audio_format,
            download_mode,
            debug_mode,
            failures,
            failed_tracks,
        )

    # Show summary
    if console:
        console.print(
            f"\n[bold]Download complete![/bold] {len(items) - failures} successful, {failures} failed"
        )
        if failed_tracks:
            console.print(f"\n[red]Failed downloads ({len(failed_tracks)}):[/red]")
            for i, track in enumerate(failed_tracks[:10], 1):  # Show first 10 failures
                console.print(
                    f"[dim]  {i}. {track['title']} by {track['artist']} - {track['reason']}[/dim]"
                )
                console.print(f"[dim]     URL: {track['url']}[/dim]")
            if len(failed_tracks) > 10:
                console.print(
                    f"[dim]  ... and {len(failed_tracks) - 10} more failures[/dim]"
                )
            console.print(
                "[dim]💡 Common causes: YouTube rate limiting, region blocks, or deleted videos[/dim]"
            )
    else:
        print(
            f"\nDownload complete! {len(items) - failures} successful, {failures} failed"
        )


def _download_batch_items(
    items: List[Dict[str, Any]],
    flag: Dict[str, bool],
    task: Any,
    progress: Any,
    ytm: Any,
    db_conn: Any,
    db_has_yid_func: Callable,
    db_add_track_func: Callable,
    temp_dir: Path,
    final_root: Path,
    root: Path,
    audio_format: str,
    download_mode: str,
    debug_mode: bool,
    failures: int,
    failed_tracks: List[Dict[str, str]],
) -> int:
    """Internal helper for downloading batch items."""
    for row in items:
        if flag["hit"]:
            if console:
                console.print(
                    "\n[yellow]Download interrupted by user (Ctrl+C)[/yellow]"
                )
            else:
                print("\nDownload interrupted by user (Ctrl+C)")
            break

        yid = row["videoId"]
        url = yid_to_url(yid)

        # skip if already in DB
        if db_has_yid_func(db_conn, yid):
            if progress:
                progress.advance(task)
            continue

        # Update description to show current track
        if progress:
            title = row.get("title", "Unknown")[:35]
            artist = row.get("artist", "Unknown")[:20]
            progress.update(task, description=f"⬇ {artist} — {title}")

        tmp = with_retries(
            run_ytdlp_audio,
            url,
            temp_dir,
            root,
            audio_format,
            download_mode,
            debug_mode,
            attempts=3,
            delay=2,
        )

        if tmp is None or not tmp.exists():
            failures += 1
            failed_tracks.append(
                {
                    "title": row.get("title", "Unknown"),
                    "artist": row.get("artist", "Unknown"),
                    "url": url,
                    "reason": "Download failed",
                }
            )
            if progress:
                progress.advance(task)
            # Configurable delay on failure
            mode_config = DOWNLOAD_MODES.get(download_mode, DOWNLOAD_MODES["normal"])
            failure_delay = random.uniform(*mode_config["failure_delay"])
            time.sleep(failure_delay)
            continue

        # Process successful download
        _process_successful_download(
            tmp, row, ytm, final_root, db_conn, db_add_track_func, yid
        )

        if progress:
            progress.advance(task)

    return failures


def _process_successful_download(
    tmp: Path,
    row: Dict[str, Any],
    ytm: Any,
    final_root: Path,
    db_conn: Any,
    db_add_track_func: Any,
    yid: str,
) -> None:
    """Process a successful download - organize, tag, and add to database."""
    title = row.get("title", "Unknown Title")
    artist = row.get("artist", "Unknown Artist")

    # Try to get more metadata from YouTube Music
    album = None
    year = None
    try:
        # Search for the track to get additional metadata
        search_results = ytm.search(f"{title} {artist}", filter="songs", limit=1)
        if search_results:
            track_info = search_results[0]
            if track_info.get("album"):
                album = track_info["album"].get("name")
            if track_info.get("year"):
                year = str(track_info["year"])
    except Exception:
        # If metadata fetch fails, continue with what we have
        pass

    # Organize into final location
    ext = tmp.suffix.lstrip(".")
    final_path = organize_target(final_root, artist, album, title, ext)

    # Move file to final location
    final_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        tmp.rename(final_path)
    except OSError:
        # Cross-device or permission error — try copy+delete as fallback
        import shutil as _shutil
        _shutil.copy2(tmp, final_path)
        tmp.unlink(missing_ok=True)

    # Verify the file actually landed on disk before writing the DB entry
    if not final_path.exists():
        if console:
            console.print(
                f"[red]✗ File not found after move, skipping DB write: {final_path}[/red]"
            )
        else:
            print(f"✗ File not found after move, skipping DB write: {final_path}")
        return

    # Embed metadata tags
    embed_easy_tags(final_path, title, artist, album, year)

    # Add to database — only reached when the file exists on disk
    db_add_track_func(db_conn, yid, title, artist, album or "", str(final_path))
