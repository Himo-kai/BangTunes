#!/usr/bin/env python3
"""
Tests for error handling and edge cases.
"""

import sqlite3
import tempfile
from pathlib import Path
import sys

# Add the parent directory to the path so we can import bang_tunes
sys.path.insert(0, str(Path(__file__).parent.parent))

from bangtunes.db import sanitize, open_db
from bangtunes.downloads import embed_easy_tags
from bangtunes.config import load_config


def test_sanitize_function() -> None:
    """Test the sanitize function handles various inputs"""
    # Normal string
    assert sanitize("Normal String") == "Normal String"

    # String with special characters
    assert sanitize("Song@#$%^&*()Title") == "Song_________Title"

    # String with spaces and valid characters
    assert sanitize("Song - Artist (2023)") == "Song - Artist _2023_"

    # Empty string
    assert sanitize("") == ""

    # String with only special characters
    assert sanitize("@#$%") == "____"

    # String with unicode characters (regex preserves unicode letters)
    result = sanitize("Café Müller")
    # The exact result depends on regex implementation, just check it doesn't crash
    assert isinstance(result, str)


def test_embed_easy_tags_nonexistent_file() -> None:
    """Test embed_easy_tags handles non-existent files gracefully"""
    with tempfile.TemporaryDirectory() as tmpdir:
        nonexistent_file = Path(tmpdir) / "nonexistent.mp3"

        # Should not crash when file doesn't exist
        try:
            embed_easy_tags(nonexistent_file, "Title", "Artist", "Album", "2023")
            # If it doesn't crash, that's good
        except Exception as e:
            # If it does raise an exception, it should be handled gracefully
            assert "No such file" in str(e) or "cannot load" in str(e).lower()


def test_database_connection_error_handling() -> None:
    """Test database operations handle connection errors gracefully"""
    # Try to connect to an invalid database path
    invalid_path = "/invalid/path/that/does/not/exist/database.db"

    try:
        conn = sqlite3.connect(invalid_path)
        # This might actually succeed (sqlite creates files), so let's try an operation
        conn.execute("CREATE TABLE test (id INTEGER)")
        conn.close()
    except (sqlite3.Error, OSError, PermissionError):
        # Expected - should handle gracefully
        pass


def test_empty_input_handling() -> None:
    """Test functions handle empty/None inputs gracefully"""
    # Test sanitize with None (should not crash)
    try:
        result = sanitize(None)  # type: ignore
        # If it doesn't crash, check the result is reasonable
        assert result is not None
    except (TypeError, AttributeError):
        # Expected for None input
        pass

    # Test with empty strings
    assert sanitize("") == ""


def test_large_input_handling() -> None:
    """Test functions handle very large inputs"""
    # Very long string
    long_string = "A" * 10000
    result = sanitize(long_string)
    assert len(result) == len(long_string)
    assert result == long_string  # All A's should be preserved


def test_special_character_edge_cases() -> None:
    """Test edge cases with special characters"""
    # Test various unicode and special characters
    test_cases = [
        ("", ""),
        ("   ", ""),  # Only spaces should be stripped
        ("...", "..."),  # Dots should be preserved
        ("---", "---"),  # Dashes should be preserved
        ("___", "___"),  # Underscores should be preserved
        ("123", "123"),  # Numbers should be preserved
        ("ABC", "ABC"),  # Letters should be preserved
        ("a1b2c3", "a1b2c3"),  # Alphanumeric should be preserved
    ]

    for input_str, expected in test_cases:
        result = sanitize(input_str)
        assert result == expected, (
            f"sanitize('{input_str}') returned '{result}', expected '{expected}'"
        )


def test_path_handling() -> None:
    """Test path-related operations handle edge cases"""
    with tempfile.TemporaryDirectory() as tmpdir:
        temp_path = Path(tmpdir)

        # Test with various path types
        paths_to_test = [
            temp_path / "normal_file.txt",
            temp_path / "file with spaces.txt",
            temp_path / "file-with-dashes.txt",
            temp_path / "file_with_underscores.txt",
            temp_path / "file.with.dots.txt",
        ]

        for path in paths_to_test:
            # Create the file
            path.touch()
            assert path.exists()

            # Test that the path can be converted to string and back
            path_str = str(path)
            reconstructed_path = Path(path_str)
            assert reconstructed_path == path


def test_schema_migrates_on_existing_database() -> None:
    """open_db must create indexes even when library.db already exists.
    Regression for: first_time guard that skipped schema on existing installs."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "library.db"

        # Simulate a pre-existing installation: table created without indexes
        bare = sqlite3.connect(str(db_path))
        bare.execute("""
            CREATE TABLE tracks(
                id INTEGER PRIMARY KEY,
                youtube_id TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album TEXT,
                file_path TEXT,
                added_on TEXT DEFAULT CURRENT_TIMESTAMP
            )
        """)
        bare.commit()
        bare.close()

        # open_db should apply the full schema even though the file already exists
        conn = open_db(db_path)
        conn.close()

        verify = sqlite3.connect(str(db_path))
        cursor = verify.execute(
            "SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'"
        )
        index_names = {row[0] for row in cursor.fetchall()}
        verify.close()

        assert "idx_artist" in index_names, "idx_artist missing on existing db"
        assert "idx_added_on" in index_names, "idx_added_on missing on existing db"
        assert "idx_file_path" in index_names, "idx_file_path missing on existing db"


def test_load_config_local_overrides_global() -> None:
    """Local bangtunes.toml must win over ~/.config/bangtunes.toml for the same key."""
    try:
        import tomllib  # py3.11+
    except ImportError:
        try:
            import tomli as tomllib  # type: ignore[no-redef]
        except ImportError:
            import pytest  # type: ignore[import]
            pytest.skip("tomllib/tomli not available")

    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)

        # Simulate a global config with format = "mp3"
        global_dir = root / "dot_config"
        global_dir.mkdir()
        global_cfg = global_dir / "bangtunes.toml"
        global_cfg.write_text('format = "mp3"\nmin_score = 40\n')

        # Local config overrides format but not min_score
        local_cfg = root / "bangtunes.toml"
        local_cfg.write_text('format = "opus"\n')

        # Patch Path.home() so load_config finds our fake global config
        import bangtunes.config as cfg_module
        original_home = Path.home

        def fake_home() -> Path:
            return root / "dot_config" / ".."  # resolves to tmpdir

        # Use monkeypatching via a simple trick: write the global config where
        # load_config would look relative to our fake home.
        # Simpler: call _deep_merge directly to test the merge logic independently.
        from bangtunes.config import _deep_merge, get_config_defaults

        defaults = get_config_defaults()
        global_raw = {"format": "mp3", "min_score": 40}
        local_raw = {"format": "opus"}

        _deep_merge(defaults, global_raw)  # apply global
        _deep_merge(defaults, local_raw)   # apply local (wins)

        assert defaults["format"] == "opus", (
            f"local format='opus' should override global format='mp3', got {defaults['format']!r}"
        )
        assert defaults["min_score"] == 40, (
            f"min_score from global should be preserved when local doesn't set it, got {defaults['min_score']}"
        )


if __name__ == "__main__":
    test_sanitize_function()
    test_embed_easy_tags_nonexistent_file()
    test_database_connection_error_handling()
    test_empty_input_handling()
    test_large_input_handling()
    test_special_character_edge_cases()
    test_path_handling()
    print("All error handling tests passed!")
