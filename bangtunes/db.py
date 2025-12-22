"""
Database layer for BangTunes.

Handles SQLite connections, schema management, and query helpers.
"""

import re
import sqlite3
from pathlib import Path
from typing import Dict, Any, List, Optional, Tuple

try:
    from rich.console import Console

    console: Optional[Console] = Console()
except ImportError:
    console = None


def open_db(db_path: Path) -> sqlite3.Connection:
    """
    Open database connection with consistent schema and PRAGMAs.

    Single source of truth for database initialization and connection.

    Args:
        db_path: Path to the SQLite database file

    Returns:
        SQLite connection with proper configuration
    """
    try:
        # Ensure database directory exists
        db_path.parent.mkdir(parents=True, exist_ok=True)

        first_time = not db_path.exists()
        conn = sqlite3.connect(db_path)

        # Always set PRAGMAs for every connection
        conn.execute("PRAGMA foreign_keys = ON")  # keep data integrity
        conn.execute("PRAGMA journal_mode = WAL")  # faster for concurrent access

        # Initialize schema on first run
        if first_time:
            conn.executescript("""
            CREATE TABLE IF NOT EXISTS tracks(
                id INTEGER PRIMARY KEY,
                youtube_id TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album TEXT,
                file_path TEXT,
                added_on TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_album ON tracks(album);
            CREATE INDEX IF NOT EXISTS idx_youtube_id ON tracks(youtube_id);
            CREATE INDEX IF NOT EXISTS idx_file_path ON tracks(file_path);
            CREATE INDEX IF NOT EXISTS idx_added_on ON tracks(added_on);
            """)
            conn.commit()

        return conn

    except sqlite3.Error as e:
        if console:
            console.print(f"[red]Database error[/red]: {e}")
            console.print(f"[dim]Database path: {db_path}[/dim]")
        else:
            print(f"Database error: {e}")
            print(f"Database path: {db_path}")
        raise
    except PermissionError:
        if console:
            console.print(f"[red]Permission denied accessing database[/red]: {db_path}")
            console.print("[dim]Check directory permissions and try again[/dim]")
        else:
            print(f"Permission denied accessing database: {db_path}")
            print("Check directory permissions and try again")
        raise


def db_init(db_path: Path) -> sqlite3.Connection:
    """Set up the database (legacy wrapper)."""
    return open_db(db_path)


def get_db(db_path: Path) -> sqlite3.Connection:
    """Get db connection with context manager for consistent resource handling."""
    return open_db(db_path)


def db_has_yid(conn: sqlite3.Connection, yid: str) -> bool:
    """Check if a YouTube ID already exists in the database."""
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
    """Add a track to the database."""
    cur = conn.cursor()
    cur.execute(
        """
    INSERT OR IGNORE INTO tracks(youtube_id,title,artist,album,file_path)
    VALUES(?,?,?,?,?)
    """,
        (yid, title, artist, album, file_path),
    )
    conn.commit()


def db_summary(conn: sqlite3.Connection) -> Tuple[int, int, int]:
    """Get database summary statistics."""
    cur = conn.cursor()
    cur.execute(
        "SELECT COUNT(*), COUNT(DISTINCT artist), COUNT(DISTINCT album) FROM tracks;"
    )
    result = cur.fetchone()
    return result if result else (0, 0, 0)  # (tracks, artists, albums)


def sanitize(name: str) -> str:
    """Sanitize filename by replacing invalid characters."""
    return re.sub(r"[^-\w.\s]", "_", name).strip()


def db_get_all_tracks(
    conn: sqlite3.Connection, limit: Optional[int] = None, offset: int = 0
) -> List[Dict[str, Any]]:
    """Get tracks from database with metadata, with optional pagination for performance."""
    cur = conn.cursor()

    # Use pagination for large libraries to improve performance
    query = """
        SELECT id, youtube_id, title, artist, album, file_path, added_on 
        FROM tracks 
        ORDER BY artist, album, title
    """

    if limit is not None:
        query += f" LIMIT {limit} OFFSET {offset}"

    cur.execute(query)
    rows = cur.fetchall()
    return [
        {
            "id": row[0],
            "youtube_id": row[1],
            "title": row[2],
            "artist": row[3],
            "album": row[4],
            "file_path": row[5],
            "added_on": row[6],
        }
        for row in rows
    ]


def db_get_track_count(conn: sqlite3.Connection) -> int:
    """Get total number of tracks for pagination."""
    cur = conn.cursor()
    cur.execute("SELECT COUNT(*) FROM tracks")
    result = cur.fetchone()
    return int(result[0]) if result else 0


def db_update_track_metadata(
    conn: sqlite3.Connection, track_id: int, title: str, artist: str, album: str
) -> bool:
    """Update track metadata in database and file tags."""
    try:
        cur = conn.cursor()

        # Get current track info
        cur.execute("SELECT file_path FROM tracks WHERE id = ?", (track_id,))
        result = cur.fetchone()
        if not result:
            return False

        file_path = result[0]

        # Update database
        cur.execute(
            """
            UPDATE tracks 
            SET title = ?, artist = ?, album = ? 
            WHERE id = ?
        """,
            (title, artist, album, track_id),
        )

        conn.commit()

        # Update file tags if file exists
        if file_path and Path(file_path).exists():
            try:
                from mutagen import File

                audio_file = File(file_path)
                if audio_file is not None:
                    # Update common tags
                    audio_file["TITLE"] = title
                    audio_file["ARTIST"] = artist
                    audio_file["ALBUM"] = album
                    audio_file.save()
            except Exception as e:
                if console:
                    console.print(
                        f"[yellow]Warning: Could not update file tags: {e}[/yellow]"
                    )
                else:
                    print(f"Warning: Could not update file tags: {e}")

        return True

    except Exception as e:
        if console:
            console.print(f"[red]Error updating track metadata: {e}[/red]")
        else:
            print(f"Error updating track metadata: {e}")
        return False
