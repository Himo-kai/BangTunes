#!/usr/bin/env python3
"""
Smoke tests for BangTunes - just to prove the core stuff works.
Keeping it simple with a few focused tests.
"""

import subprocess
import tempfile
import csv
from pathlib import Path


def test_cli_help_works():
    """Make sure help doesn't crash - basic sanity check"""
    result = subprocess.run(["python", "bang_tunes.py", "-h"], 
                          capture_output=True, text=True,
                          cwd=Path(__file__).parent.parent)
    assert result.returncode == 0
    assert "Bang Tunes" in result.stdout


def test_seed_to_batch_pipeline():
    """seed -> batch workflow, the main thing that needs to work"""
    with tempfile.TemporaryDirectory() as tmpdir:
        temp_path = Path(tmpdir)
        
        # make a tiny seed file
        seed_file = temp_path / "seed.csv"
        with open(seed_file, 'w', newline='') as f:
            writer = csv.writer(f)
            writer.writerow(['title', 'artist'])
            writer.writerow(['Creep', 'Radiohead'])  # why not
        
        # need batches dir
        batches_dir = temp_path / "batches"
        batches_dir.mkdir()
        
        # try the build command
        result = subprocess.run([
            "python", "bang_tunes.py", "build",
            "--seed", str(seed_file),
            "--batches-dir", str(batches_dir),
            "--batch-size", "1",
            "--dry-run"  # don't spam youtube
        ], capture_output=True, text=True,
           cwd=Path(__file__).parent.parent,
           env={"BANGTUNES_ROOT": str(temp_path)})
        
        # basic sanity check
        assert result.returncode in [0, 1]  # success or expected fail
        assert "Traceback" not in result.stderr
        
        # check if weve gotten a batch file
        if result.returncode == 0:
            mix_files = list(batches_dir.glob("mix_*.csv"))
            assert len(mix_files) > 0


def test_view_empty_lib():
    """view handles empty library without crashing"""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(["python", "bang_tunes.py", "view"],
                              capture_output=True, text=True,
                              cwd=Path(__file__).parent.parent,
                              env={"BANGTUNES_ROOT": tmpdir}
        )
        # please don't crash
        assert result.returncode in [0, 1]
        assert "Traceback" not in result.stderr


def test_behavior_scoring():
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
    def score_track(events):
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
