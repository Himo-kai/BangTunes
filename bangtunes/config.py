# SPDX-License-Identifier: MIT
# Copyright (c) 2024 BangTunes Contributors

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
    Load configuration by merging both config files, local wins on conflicts.

    Load order: defaults → ~/.config/bangtunes.toml (global) → ./bangtunes.toml
    (local).  Both files are read when they exist; a key present in the local
    file always overrides the same key from the global file.

    Args:
        root: BangTunes root directory

    Returns:
        Merged configuration dictionary.
    """
    if tomllib is None:
        return get_config_defaults()

    # global first so local overrides it via _deep_merge
    candidates = [
        Path.home() / ".config" / "bangtunes.toml",
        root / "bangtunes.toml",
    ]

    debug_mode = os.getenv("BANGTUNES_DEBUG", "false").lower() in ("true", "1", "yes")

    config = get_config_defaults()

    for p in candidates:
        if not p.exists():
            continue
        with p.open("rb") as f:
            try:
                raw = tomllib.load(f) or {}
                _deep_merge(config, raw)
            except Exception as e:
                if debug_mode:
                    try:
                        from rich.console import Console

                        Console().print(
                            f"[yellow]Config parse failed for {p}: {e}[/yellow]"
                        )
                    except ImportError:
                        print(f"Config parse failed for {p}: {e}")
                # Skip the bad file but keep processing remaining candidates

    return config


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

    def _toml_value(v: Any) -> str:
        if isinstance(v, str):
            return f'"{v}"'
        if isinstance(v, bool):
            return "true" if v else "false"
        if isinstance(v, list):
            items = ", ".join(_toml_value(i) for i in v)
            return f"[{items}]"
        return str(v)

    for key, value in config.items():
        if isinstance(value, dict):
            lines.append(f"\n[{key}]")
            for subkey, subvalue in value.items():
                lines.append(f"{subkey} = {_toml_value(subvalue)}")
        else:
            lines.append(f"{key} = {_toml_value(value)}")

    config_path.write_text("\n".join(lines) + "\n")
