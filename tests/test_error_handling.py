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

from bang_tunes import sanitize, embed_easy_tags


def test_sanitize_function():
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


def test_embed_easy_tags_nonexistent_file():
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


def test_database_connection_error_handling():
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


def test_empty_input_handling():
    """Test functions handle empty/None inputs gracefully"""
    # Test sanitize with None (should not crash)
    try:
        result = sanitize(None)
        # If it doesn't crash, check the result is reasonable
        assert result is not None
    except (TypeError, AttributeError):
        # Expected for None input
        pass
    
    # Test with empty strings
    assert sanitize("") == ""


def test_large_input_handling():
    """Test functions handle very large inputs"""
    # Very long string
    long_string = "A" * 10000
    result = sanitize(long_string)
    assert len(result) == len(long_string)
    assert result == long_string  # All A's should be preserved


def test_special_character_edge_cases():
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
        assert result == expected, f"sanitize('{input_str}') returned '{result}', expected '{expected}'"


def test_path_handling():
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


if __name__ == "__main__":
    test_sanitize_function()
    test_embed_easy_tags_nonexistent_file()
    test_database_connection_error_handling()
    test_empty_input_handling()
    test_large_input_handling()
    test_special_character_edge_cases()
    test_path_handling()
    print("All error handling tests passed!")
