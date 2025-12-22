"""
Library management for BangTunes.

Handles seed parsing, pool building, batch generation, and related music discovery.
"""

import csv
import sys
import time
from pathlib import Path
from typing import Dict, List, Any

try:
    from rich.console import Console

    console = Console()
except ImportError:
    console = None

try:
    from rapidfuzz import fuzz
except ImportError:
    fuzz = None

try:
    from ytmusicapi import YTMusic
except ImportError:
    YTMusic = None


# Constants
MAX_CANDIDATES_PER_SEED = 50


def read_seed(seed_path: Path) -> List[Dict[str, str]]:
    """Load up the seed tracks - with error messages this time"""
    if not seed_path.exists():
        if console:
            console.print(f"[red]Missing seed file[/red]: {seed_path}")
            console.print(
                "[dim]Create a CSV file with headers: title,artist,notes[/dim]"
            )
            console.print(
                "[dim]Example: 'Bohemian Rhapsody,Queen,Classic rock anthem'[/dim]"
            )
        else:
            print(f"Missing seed file: {seed_path}")
            print("Create a CSV file with headers: title,artist,notes")
            print("Example: 'Bohemian Rhapsody,Queen,Classic rock anthem'")
        sys.exit(1)

    try:
        rows = []
        with open(seed_path, newline="", encoding="utf-8") as f:
            reader = csv.DictReader(f)

            # Make sure the CSV actually has the columns we need
            if not reader.fieldnames:
                if console:
                    console.print(f"[red]Empty or invalid CSV file[/red]: {seed_path}")
                else:
                    print(f"Empty or invalid CSV file: {seed_path}")
                sys.exit(1)

            # Check for the bare minimum: title and artist
            required_headers = {"title", "artist"}
            available_headers = set(reader.fieldnames)
            missing_headers = required_headers - available_headers

            if missing_headers:
                if console:
                    console.print(
                        f"[red]Missing required CSV headers[/red]: {', '.join(missing_headers)}"
                    )
                    console.print(
                        f"[dim]Found headers: {', '.join(reader.fieldnames)}[/dim]"
                    )
                    console.print(
                        "[dim]Required headers: title,artist (notes optional)[/dim]"
                    )
                else:
                    print(f"Missing required CSV headers: {', '.join(missing_headers)}")
                    print(f"Found headers: {', '.join(reader.fieldnames)}")
                    print("Required headers: title,artist (notes optional)")
                sys.exit(1)

            # Now check each row to make sure it's not garbage
            for line_num, row in enumerate(
                reader, start=2
            ):  # line 1 is headers, so start counting at 2
                title = row.get("title", "").strip()
                artist = row.get("artist", "").strip()

                if not title and not artist:
                    if console:
                        console.print(
                            f"[yellow]Warning: Empty row at line {line_num}, skipping[/yellow]"
                        )
                    else:
                        print(f"Warning: Empty row at line {line_num}, skipping")
                    continue

                if not title:
                    if console:
                        console.print(
                            f"[yellow]Warning: Missing title at line {line_num}, skipping[/yellow]"
                        )
                    else:
                        print(f"Warning: Missing title at line {line_num}, skipping")
                    continue

                if not artist:
                    if console:
                        console.print(
                            f"[yellow]Warning: Missing artist at line {line_num}, skipping[/yellow]"
                        )
                    else:
                        print(f"Warning: Missing artist at line {line_num}, skipping")
                    continue

                rows.append(row)

        if not rows:
            if console:
                console.print(
                    f"[red]No valid entries found in seed file[/red]: {seed_path}"
                )
                console.print(
                    "[dim]Each row needs both title and artist filled in[/dim]"
                )
            else:
                print(f"No valid entries found in seed file: {seed_path}")
                print("Each row needs both title and artist filled in")
            sys.exit(1)

        if console:
            console.print(f"[dim]Loaded {len(rows)} seed tracks from {seed_path}[/dim]")
        else:
            print(f"Loaded {len(rows)} seed tracks from {seed_path}")
        return rows

    except PermissionError:
        if console:
            console.print(f"[red]Permission denied reading[/red]: {seed_path}")
            console.print("[dim]Check file permissions and try again[/dim]")
        else:
            print(f"Permission denied reading: {seed_path}")
            print("Check file permissions and try again")
        sys.exit(1)
    except UnicodeDecodeError:
        if console:
            console.print(f"[red]Invalid file encoding[/red]: {seed_path}")
            console.print("[dim]Save the CSV file with UTF-8 encoding[/dim]")
        else:
            print(f"Invalid file encoding: {seed_path}")
            print("Save the CSV file with UTF-8 encoding")
        sys.exit(1)
    except csv.Error as e:
        if console:
            console.print(f"[red]CSV parsing error[/red]: {e}")
            console.print(f"[dim]Check that {seed_path} is a valid CSV file[/dim]")
        else:
            print(f"CSV parsing error: {e}")
            print(f"Check that {seed_path} is a valid CSV file")
        sys.exit(1)


def normalize_artists(artists: List[Dict[str, Any]]) -> str:
    """Convert YouTube Music artist list to a simple string."""
    names = []
    for a in artists or []:
        name = a.get("name", "").strip()
        if name:
            names.append(name)
    return ", ".join(names) if names else ""


def search_related(ytm: Any, title: str, artist: str) -> List[Dict[str, str]]:
    """Search for related tracks using YouTube Music API."""
    if not ytm:
        return []

    out = []

    # try the obvious approach first
    query = f"{title} {artist}"
    try:
        results = ytm.search(query, filter="songs", limit=20)
        for item in results:
            if item.get("category") != "Songs":
                continue

            # Extract track info
            track_title = item.get("title", "")
            track_artists = normalize_artists(item.get("artists", []))
            video_id = item.get("videoId", "")

            if track_title and track_artists and video_id:
                out.append(
                    {
                        "title": track_title,
                        "artist": track_artists,
                        "videoId": video_id,
                    }
                )
    except Exception as e:
        if console:
            console.print(f"[yellow]Search failed for '{query}': {e}[/yellow]")
        else:
            print(f"Search failed for '{query}': {e}")

    # Remove duplicate tracks by video ID
    seen, uniq = set(), []
    for track in out:
        if track["videoId"] in seen:
            continue
        seen.add(track["videoId"])
        uniq.append(track)

    return uniq[:MAX_CANDIDATES_PER_SEED]


def build_pool_from_seed(
    seed_rows: List[Dict[str, str]], min_score: int = 60, cc_only: bool = False
) -> List[Dict[str, str]]:
    """Build a pool of candidate tracks from seed data."""
    if not YTMusic:
        if console:
            console.print("[red]ytmusicapi not available - cannot build pool[/red]")
        else:
            print("ytmusicapi not available - cannot build pool")
        return []

    if not fuzz:
        if console:
            console.print(
                "[red]rapidfuzz not available - cannot calculate scores[/red]"
            )
        else:
            print("rapidfuzz not available - cannot calculate scores")
        return []

    ytm = YTMusic()  # Initialize YouTube Music API client
    pool = []  # Store discovered track candidates

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
            if isinstance(score, (int, float)) and score >= min_score:
                filtered_cands.append(c)

        pool.extend(filtered_cands)
        time.sleep(0.5)  # Rate limiting for API requests

    pool.sort(key=lambda x: x["score"], reverse=True)

    # Remove duplicate tracks by video ID
    seen, final = set(), []
    for c in pool:
        if c["videoId"] in seen:
            continue
        seen.add(c["videoId"])
        final.append(c)

    return final


def write_batches(
    pool: List[Dict[str, str]], batch_dir: Path, prefix: str, size: int
) -> List[Path]:
    """Write pool data to batch CSV files."""
    batch_dir.mkdir(parents=True, exist_ok=True)
    files = []

    for i in range(0, len(pool), size):
        chunk = pool[i : i + size]
        if not chunk:
            break
        name = f"{prefix}_{i // size + 1:03d}.csv"
        path = batch_dir / name

        with open(path, "w", newline="", encoding="utf-8") as f:
            w = csv.DictWriter(
                f, fieldnames=["title", "artist", "videoId", "score", "seed_key"]
            )
            w.writeheader()
            w.writerows(chunk)
        files.append(path)

    return files


def read_batch_csv(path: Path, batch_dir: Path) -> List[Dict[str, str]]:
    """Read a batch CSV file."""
    if not path.exists():
        path = batch_dir / path
    if not path.exists():
        if console:
            console.print(f"[red]Batch not found:[/red] {path}")
        else:
            print(f"Batch not found: {path}")
        sys.exit(1)

    out = []
    with open(path, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if row.get("videoId"):
                out.append(row)
    return out


def list_batches(batch_dir: Path) -> List[tuple[str, int]]:
    """List available batch files with their entry counts."""
    if not batch_dir.exists():
        return []

    rows = []
    for p in batch_dir.glob("*.csv"):
        try:
            with open(p, newline="", encoding="utf-8") as f:
                reader = csv.DictReader(f)
                count = sum(1 for _ in reader)
            rows.append((p.name, count))
        except Exception:
            # Skip files that can't be read
            continue

    return sorted(rows)
