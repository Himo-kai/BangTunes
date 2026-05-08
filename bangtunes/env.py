# SPDX-License-Identifier: MIT
# Copyright (c) 2024 BangTunes Contributors

"""
Environment and platform detection for BangTunes.

Handles ROOT resolution, platform detection, and common directory paths.
"""

import os
from pathlib import Path


def is_termux() -> bool:
    """Detect if running in Termux (Android)."""
    # TERMUX_VERSION is reliable in Termux
    if os.environ.get("TERMUX_VERSION"):
        return True
    # fallback heuristics
    home = os.environ.get("HOME", "")
    return "com.termux" in home or os.path.exists("/data/data/com.termux")


def get_root() -> Path:
    """
    Resolve BangTunes root directory.

    Priority:
    1. BANGTUNES_ROOT environment variable
    2. Current working directory (if bang_tunes.py exists)
    3. Script directory (if bang_tunes.py exists there)
    4. Platform-specific search paths
    """
    # Check environment override first
    if "BANGTUNES_ROOT" in os.environ:
        root = Path(os.environ["BANGTUNES_ROOT"])
        if root.exists():
            return root

    # Check current working directory
    cwd = Path.cwd()
    if (cwd / "bang_tunes.py").exists():
        return cwd

    # Maybe you're running it from somewhere else? Check where the script actually is
    script_dir = Path(__file__).parent.parent.absolute()
    if (script_dir / "bang_tunes.py").exists():
        return script_dir

    # Check if we're running in Termux (Android)
    termux = is_termux()

    # Check the usual suspects where people might put this
    home = Path.home()
    candidates = []

    if termux:
        # Termux-specific paths - uses internal storage, not SD card
        prefix = os.environ.get("PREFIX", "")
        termux_home = Path(prefix).parent / "home" if prefix else home
        candidates = [
            home / "BangTunes",
            termux_home / "BangTunes",
            home / "storage" / "shared" / "BangTunes",
            home / "storage" / "downloads" / "BangTunes",
        ]
    else:
        # Linux/macOS paths
        candidates = [
            home / "BangTunes",
            home / "Music" / "BangTunes",
            home / "Documents" / "BangTunes",
            Path("/opt/BangTunes"),
            Path("/usr/local/BangTunes"),
        ]

    for candidate in candidates:
        if candidate.exists() and (candidate / "bang_tunes.py").exists():
            return candidate

    # If nothing found, default to current directory
    return cwd


def get_config_path(root: Path) -> Path:
    """Get path to bangtunes.toml config file."""
    return root / "bangtunes.toml"


def get_db_path(root: Path) -> Path:
    """Get path to main library database."""
    return root / "library.db"


def get_batches_dir(root: Path) -> Path:
    """Get path to batches directory."""
    return root / "batches"


def get_downloads_dir(root: Path) -> Path:
    """Get path to downloads directory."""
    return root / "downloads"


def get_cache_dir() -> Path:
    """Get platform-appropriate cache directory."""
    if os.name == "nt":  # Windows
        return Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local"))
    elif "XDG_CACHE_HOME" in os.environ:  # Linux with XDG
        return Path(os.environ["XDG_CACHE_HOME"])
    else:  # macOS and fallback
        return Path.home() / ".cache"
