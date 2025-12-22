#!/usr/bin/env python3
"""
Tests for the metadata editor functionality.
"""

import sqlite3
import tempfile
from pathlib import Path
import sys
import os

# Add the parent directory to the path so we can import bang_tunes
sys.path.insert(0, str(Path(__file__).parent.parent))

from bang_tunes import db_add_track, db_get_all_tracks, db_update_track_metadata


def test_db_get_all_tracks():
    """Test retrieving all tracks from database"""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Set up test database
        test_db = Path(tmpdir) / "test.db"
        os.environ["BANGTUNES_ROOT"] = tmpdir

        # Initialize database and add test data
        conn = sqlite3.connect(test_db)
        conn.execute("PRAGMA foreign_keys = ON")
        conn.execute("""
        CREATE TABLE IF NOT EXISTS tracks(
            id INTEGER PRIMARY KEY,
            youtube_id TEXT UNIQUE NOT NULL,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT,
            file_path TEXT,
            added_on TEXT DEFAULT CURRENT_TIMESTAMP
        );
        """)

        # Add test tracks
        db_add_track(
            conn,
            "test1",
            "Test Song 1",
            "Test Artist 1",
            "Test Album 1",
            "/path/to/song1.mp3",
        )
        db_add_track(
            conn,
            "test2",
            "Test Song 2",
            "Test Artist 2",
            "Test Album 2",
            "/path/to/song2.mp3",
        )

        # Test retrieval
        tracks = db_get_all_tracks(conn)

        assert len(tracks) == 2
        assert tracks[0]["title"] == "Test Song 1"
        assert tracks[0]["artist"] == "Test Artist 1"
        assert tracks[1]["title"] == "Test Song 2"
        assert tracks[1]["artist"] == "Test Artist 2"

        conn.close()


def test_db_update_track_metadata():
    """Test updating track metadata in database"""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Set up test database
        test_db = Path(tmpdir) / "test.db"
        os.environ["BANGTUNES_ROOT"] = tmpdir

        # Initialize database and add test data
        conn = sqlite3.connect(test_db)
        conn.execute("PRAGMA foreign_keys = ON")
        conn.execute("""
        CREATE TABLE IF NOT EXISTS tracks(
            id INTEGER PRIMARY KEY,
            youtube_id TEXT UNIQUE NOT NULL,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT,
            file_path TEXT,
            added_on TEXT DEFAULT CURRENT_TIMESTAMP
        );
        """)

        # Add test track
        db_add_track(
            conn, "test1", "Old Title", "Old Artist", "Old Album", "/path/to/song1.mp3"
        )

        # Get the track ID
        tracks = db_get_all_tracks(conn)
        track_id = tracks[0]["id"]

        # Update metadata
        success = db_update_track_metadata(
            conn, track_id, "New Title", "New Artist", "New Album"
        )
        assert success

        # Verify update
        updated_tracks = db_get_all_tracks(conn)
        assert len(updated_tracks) == 1
        assert updated_tracks[0]["title"] == "New Title"
        assert updated_tracks[0]["artist"] == "New Artist"
        assert updated_tracks[0]["album"] == "New Album"

        conn.close()


def test_db_update_nonexistent_track():
    """Test updating metadata for non-existent track"""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Set up test database
        test_db = Path(tmpdir) / "test.db"
        os.environ["BANGTUNES_ROOT"] = tmpdir

        # Initialize empty database
        conn = sqlite3.connect(test_db)
        conn.execute("PRAGMA foreign_keys = ON")
        conn.execute("""
        CREATE TABLE IF NOT EXISTS tracks(
            id INTEGER PRIMARY KEY,
            youtube_id TEXT UNIQUE NOT NULL,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT,
            file_path TEXT,
            added_on TEXT DEFAULT CURRENT_TIMESTAMP
        );
        """)

        # Try to update non-existent track
        success = db_update_track_metadata(
            conn, 999, "New Title", "New Artist", "New Album"
        )
        assert not success

        conn.close()


def test_metadata_editor_search_functionality():
    """Test the search functionality used in metadata editor"""
    tracks = [
        {
            "id": 1,
            "title": "Bohemian Rhapsody",
            "artist": "Queen",
            "album": "A Night at the Opera",
        },
        {
            "id": 2,
            "title": "Stairway to Heaven",
            "artist": "Led Zeppelin",
            "album": "Led Zeppelin IV",
        },
        {
            "id": 3,
            "title": "Hotel California",
            "artist": "Eagles",
            "album": "Hotel California",
        },
        {
            "id": 4,
            "title": "Another Brick in the Wall",
            "artist": "Pink Floyd",
            "album": "The Wall",
        },
    ]

    # Test artist search
    query = "queen"
    matches = []
    for track in tracks:
        if (
            query in (track["artist"] or "").lower()
            or query in (track["title"] or "").lower()
            or query in (track["album"] or "").lower()
        ):
            matches.append(track)

    assert len(matches) == 1
    assert matches[0]["artist"] == "Queen"

    # Test title search
    query = "brick"
    matches = []
    for track in tracks:
        if (
            query in (track["artist"] or "").lower()
            or query in (track["title"] or "").lower()
            or query in (track["album"] or "").lower()
        ):
            matches.append(track)

    assert len(matches) == 1
    assert matches[0]["title"] == "Another Brick in the Wall"

    # Test album search
    query = "california"
    matches = []
    for track in tracks:
        if (
            query in (track["artist"] or "").lower()
            or query in (track["title"] or "").lower()
            or query in (track["album"] or "").lower()
        ):
            matches.append(track)

    assert len(matches) == 1
    assert matches[0]["album"] == "Hotel California"


if __name__ == "__main__":
    test_db_get_all_tracks()
    test_db_update_track_metadata()
    test_db_update_nonexistent_track()
    test_metadata_editor_search_functionality()
    print("All metadata editor tests passed!")
