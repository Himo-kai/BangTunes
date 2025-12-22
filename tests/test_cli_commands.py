#!/usr/bin/env python3
"""
Tests for CLI commands to ensure they don't crash and handle edge cases properly.
"""

import subprocess
import tempfile
import csv
from pathlib import Path


def get_python_command() -> str:
    """Get the appropriate Python command (prefer venv if available)"""
    venv_python = Path(__file__).parent.parent / "venv" / "bin" / "python"
    return str(venv_python) if venv_python.exists() else "python"


def test_install_command() -> None:
    """Test the install/setup wizard command"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [get_python_command(), "bang_tunes.py", "install"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should not crash
        assert "Traceback" not in result.stderr
        # Should create basic structure (seed.csv is created in BANGTUNES_ROOT)
        assert Path(tmpdir, "seed.csv").exists() or result.returncode == 0


def test_stats_command_empty_library() -> None:
    """Test stats command with empty library"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [get_python_command(), "bang_tunes.py", "stats"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should handle empty library gracefully
        assert result.returncode in [0, 1]
        assert "Traceback" not in result.stderr
        assert (
            "No tracks yet" in result.stdout
            or "0 tracks" in result.stdout
            or "Library Statistics" in result.stdout
        )


def test_quickplay_command_no_files() -> None:
    """Test quickplay command with no audio files"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [get_python_command(), "bang_tunes.py", "quickplay"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should handle no files gracefully
        assert result.returncode in [0, 1]
        assert "Traceback" not in result.stderr
        assert (
            "No audio files found" in result.stdout
            or "Quick Play Mode" in result.stdout
        )


def test_list_batches_command() -> None:
    """Test list-batches command"""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create batches directory
        batches_dir = Path(tmpdir) / "batches"
        batches_dir.mkdir()

        # Create a test batch file
        batch_file = batches_dir / "test_mix_001.csv"
        with open(batch_file, "w", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(["title", "artist", "youtube_id"])
            writer.writerow(["Test Song", "Test Artist", "test123"])

        result = subprocess.run(
            [get_python_command(), "bang_tunes.py", "list-batches"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should list batch files (either test file or existing ones)
        assert result.returncode == 0
        assert "Traceback" not in result.stderr
        assert (
            "test_mix_001.csv" in result.stdout
            or "Batches" in result.stdout
            or "mix_" in result.stdout
        )


def test_rescan_command() -> None:
    """Test rescan command"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [get_python_command(), "bang_tunes.py", "rescan"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should handle empty library gracefully
        assert result.returncode in [0, 1]
        assert "Traceback" not in result.stderr


def test_rescan_command_with_fix() -> None:
    """Test rescan command with --fix flag"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [get_python_command(), "bang_tunes.py", "rescan", "--fix"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should handle empty library gracefully
        assert result.returncode in [0, 1]
        assert "Traceback" not in result.stderr


def test_build_command_with_invalid_seed() -> None:
    """Test build command with invalid seed file"""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create invalid seed file
        seed_file = Path(tmpdir) / "invalid_seed.csv"
        with open(seed_file, "w") as f:
            f.write("invalid,csv,content\n")

        result = subprocess.run(
            [get_python_command(), "bang_tunes.py", "build"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should handle invalid seed gracefully
        assert "Traceback" not in result.stderr


def test_download_command_nonexistent_batch() -> None:
    """Test download command with non-existent batch file"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [
                get_python_command(),
                "bang_tunes.py",
                "download",
                "nonexistent_batch.csv",
            ],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )

        # Should handle missing file gracefully
        assert result.returncode != 0  # Expected to fail
        assert "Traceback" not in result.stderr or "FileNotFoundError" in result.stderr


def test_edit_metadata_command_help() -> None:
    """Test that edit-metadata command is available in help"""
    result = subprocess.run(
        [get_python_command(), "bang_tunes.py", "-h"],
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent,
    )

    assert result.returncode == 0
    assert "edit-metadata" in result.stdout
    assert "Interactive metadata editor" in result.stdout


def test_invalid_command() -> None:
    """Test handling of invalid command"""
    result = subprocess.run(
        [get_python_command(), "bang_tunes.py", "invalid-command"],
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent,
    )

    # Should show error and help
    assert result.returncode != 0
    assert (
        "invalid choice" in result.stderr.lower()
        or "unrecognized arguments" in result.stderr.lower()
    )


if __name__ == "__main__":
    test_install_command()
    test_stats_command_empty_library()
    test_quickplay_command_no_files()
    test_list_batches_command()
    test_rescan_command()
    test_rescan_command_with_fix()
    test_build_command_with_invalid_seed()
    test_download_command_nonexistent_batch()
    test_edit_metadata_command_help()
    test_invalid_command()
    print("All CLI command tests passed!")
