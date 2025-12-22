"""
Configuration handling for BangTunes.

Handles TOML parsing, defaults, and config file loading.
"""

import os
from pathlib import Path
from typing import Dict, Any

try:
    import tomllib  # py3.11+
except ImportError:
    try:
        import tomli as tomllib  # fallback for older Python
    except ImportError:
        tomllib = None


def load_config(root: Path) -> Dict[str, Any]:
    """
    Load configuration from TOML file if available.

    Args:
        root: BangTunes root directory

    Returns:
        Configuration dictionary (empty if no config found)
    """
    if tomllib is None:
        return {}

    candidates = [
        Path.home() / ".config" / "bangtunes.toml",
        root / "bangtunes.toml",
    ]

    debug_mode = os.getenv("BANGTUNES_DEBUG", "false").lower() in ("true", "1", "yes")

    for p in candidates:
        if p.exists():
            with p.open("rb") as f:
                try:
                    return tomllib.load(f) or {}
                except Exception as e:
                    if debug_mode:
                        # Import here to avoid circular dependency
                        try:
                            from rich.console import Console

                            console = Console()
                            console.print(
                                f"[yellow]Config parse failed for {p}: {e}[/yellow]"
                            )
                        except ImportError:
                            print(f"Config parse failed for {p}: {e}")
                    return {}
    return {}


def get_config_defaults() -> Dict[str, Any]:
    """Get default configuration values."""
    return {
        "root": "",  # Will be filled by caller
        "music_directories": [],
        "behavior": {
            "min_play_time_for_tracking": 30,
            "weight_decay_days": 30,
        },
        "download": {
            "quality": "best",
            "format": "mp3",
        },
        "player": {
            "volume": 0.7,
            "shuffle": False,
            "repeat": False,
        },
    }


def save_config(root: Path, config: Dict[str, Any]) -> None:
    """
    Save configuration to TOML file.

    Args:
        root: BangTunes root directory
        config: Configuration dictionary to save
    """
    config_path = root / "bangtunes.toml"

    # Create a simple TOML representation
    # Note: This is a basic implementation - for full TOML writing,
    # we'd need a proper TOML library like tomli-w
    lines = []

    for key, value in config.items():
        if isinstance(value, dict):
            lines.append(f"\n[{key}]")
            for subkey, subvalue in value.items():
                if isinstance(subvalue, str):
                    lines.append(f'{subkey} = "{subvalue}"')
                elif isinstance(subvalue, list):
                    lines.append(f"{subkey} = {subvalue}")
                else:
                    lines.append(f"{subkey} = {subvalue}")
        else:
            if isinstance(value, str):
                lines.append(f'{key} = "{value}"')
            elif isinstance(value, list):
                lines.append(f"{key} = {value}")
            else:
                lines.append(f"{key} = {value}")

    config_path.write_text("\n".join(lines) + "\n")
