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


def _deep_merge(dst: Dict[str, Any], src: Dict[str, Any]) -> Dict[str, Any]:
    """Merge src into dst recursively (src wins)."""
    for k, v in src.items():
        if isinstance(v, dict) and isinstance(dst.get(k), dict):
            _deep_merge(dst[k], v)  # type: ignore[index]
        else:
            dst[k] = v
    return dst


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
                    raw = tomllib.load(f) or {}
                    defaults = get_config_defaults()
                    return _deep_merge(defaults, raw)
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
                    return get_config_defaults()
    return get_config_defaults()


def get_config_defaults() -> Dict[str, Any]:
    """Get default configuration values."""
    # Import here to avoid circular imports
    from bangtunes.constants import DEFAULT_CONFIG
    return DEFAULT_CONFIG.copy()


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
