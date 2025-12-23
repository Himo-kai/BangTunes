"""
Shared constants for BangTunes to prevent configuration drift.

This module serves as the single source of truth for:
- Valid formats
- Valid speed modes
- Default values
- Other shared configuration constants

All other modules should import from here to ensure consistency.
"""

from typing import Dict, Any, Tuple

# Audio format constants
VALID_FORMATS = ["opus", "mp3", "m4a"]
DEFAULT_FORMAT = "opus"

# Download speed mode constants
VALID_SPEED_MODES = ["stealth", "normal", "fast"]
DEFAULT_SPEED_MODE = "normal"

# Download mode configurations with timing parameters
DOWNLOAD_MODES: Dict[str, Dict[str, Tuple[float, float]]] = {
    "stealth": {
        "base_sleep": (2.0, 8.0),
        "max_sleep": (8.0, 15.0),
        "success_delay": (2.0, 6.0),
        "failure_delay": (10.0, 20.0),
    },
    "normal": {
        "base_sleep": (0.5, 2.0),
        "max_sleep": (2.0, 5.0),
        "success_delay": (1.0, 2.0),
        "failure_delay": (3.0, 8.0),
    },
    "fast": {
        "base_sleep": (0.1, 0.5),
        "max_sleep": (0.5, 1.5),
        "success_delay": (0.2, 0.5),
        "failure_delay": (1.0, 3.0),
    },
}

# Default configuration values
DEFAULT_CONFIG: Dict[str, Any] = {
    "root": "",  # Will be filled by caller
    "music_directories": [],
    "behavior": {
        "min_play_time_for_tracking": 30,
        "weight_decay_days": 30,
    },
    "download": {
        "quality": "best",
        "format": DEFAULT_FORMAT,
        "speed": DEFAULT_SPEED_MODE,
    },
    "player": {
        "volume": 0.7,
        "shuffle": False,
        "repeat": False,
    },
}

# Batch and discovery constants
DEFAULT_BATCH_SIZE = 50
DEFAULT_MIN_SCORE = 58
MIN_SCORE_RANGE = (0, 100)

# Database constants
DEFAULT_DB_NAME = "library.db"

# Directory structure constants
REQUIRED_DIRECTORIES = ["batches", "downloads", "downloads/temp"]
