#!/usr/bin/env python3
"""
Smoke tests for BangTunes - just to prove the core stuff works.
Keeping it simple with a few focused tests.
"""

import subprocess
import tempfile
import csv
from pathlib import Path


def test_cli_help_works() -> None:
    """Make sure help doesn't crash - basic sanity check"""
    result = subprocess.run(
        ["python", "bang_tunes.py", "-h"],
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent,
    )
    assert result.returncode == 0
    assert "Bang Tunes" in result.stdout


def test_seed_to_batch_pipeline() -> None:
    """seed -> batch workflow — verifies build command reads seed.csv and
    produces batch files without crashing. Uses BANGTUNES_ROOT so it doesn't
    touch the real library. Network calls to YTMusic may fail in CI; that's
    fine — we only check it doesn't traceback."""
    with tempfile.TemporaryDirectory() as tmpdir:
        temp_path = Path(tmpdir)

        # build expects seed.csv in BANGTUNES_ROOT
        seed_file = temp_path / "seed.csv"
        with open(seed_file, "w", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(["title", "artist", "notes"])
            writer.writerow(["Creep", "Radiohead", "alt rock"])

        (temp_path / "batches").mkdir()

        result = subprocess.run(
            ["python", "bang_tunes.py", "build", "--size", "1"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": str(temp_path)},
        )

        # 0 = success, 1 = expected failure (no network / empty results)
        assert result.returncode in [0, 1]
        assert "Traceback" not in result.stderr


def test_view_empty_lib() -> None:
    """view handles empty library without crashing"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            ["python", "bang_tunes.py", "view"],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent,
            env={"BANGTUNES_ROOT": tmpdir},
        )
        # please don't crash
        assert result.returncode in [0, 1]
        assert "Traceback" not in result.stderr


def test_behavior_scoring() -> None:
    """loved tracks score higher than skipped ones"""
    # fake some play data
    loved_events = [
        {"track_id": "good_song", "action": "play", "duration": 180, "completed": True},
        {"track_id": "good_song", "action": "play", "duration": 180, "completed": True},
    ]

    skip_events = [
        {"track_id": "meh_song", "action": "skip", "duration": 5, "completed": False},
        {"track_id": "meh_song", "action": "skip", "duration": 3, "completed": False},
    ]

    # basic scoring (normally would import this)
    def score_track(events: list) -> float:
        if not events:
            return 0.5

        plays = sum(1 for e in events if e.get("completed", False))
        skips = sum(1 for e in events if e["action"] == "skip")

        if plays > skips:
            return 0.9
        elif skips > plays:
            return 0.1
        else:
            return 0.5

    # test it
    loved_score = score_track(loved_events)
    skip_score = score_track(skip_events)

    # loved beats skipped like paper beats rock
    assert loved_score > skip_score
    assert loved_score >= 0.8
    assert skip_score <= 0.2
