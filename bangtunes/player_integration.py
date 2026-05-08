#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2024 BangTunes Contributors

"""
BangTunes Player Integration

Copyright (c) 2024 BangTunes Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

import os
import subprocess
from pathlib import Path
from typing import Dict, Optional

from rich.console import Console


def get_user_config_dir() -> Path:
    """Get platform-appropriate config directory"""
    if os.name == "nt":  # Windows
        return Path(os.environ.get("APPDATA", Path.home() / "AppData" / "Roaming"))
    elif "XDG_CONFIG_HOME" in os.environ:  # Linux with XDG
        return Path(os.environ["XDG_CONFIG_HOME"])
    else:  # macOS and fallback
        return Path.home() / ".config"


console = Console()


class BangTunesPlayer:
    """Small player backend that reuses the BangTunes library.

    Feeds downloaded tracks into a local player DB so you can track behavior
    """

    def __init__(self, bangtunes_root: Path, config: Optional[Dict] = None):
        self.bangtunes_root = bangtunes_root
        self.downloads_dir = bangtunes_root / "downloads"
        self.bangtunes_db = bangtunes_root / "library.db"

        # Load player configuration with sensible defaults
        player_config = (config or {}).get("player", {})

        # Player paths from config or auto-detect
        self.player_root = self._resolve_player_root(player_config.get("root"))
        self.player_config_dir = Path(
            player_config.get("config_dir", get_user_config_dir() / "bangtunes")
        )
        self.player_config_file = self.player_config_dir / "player.toml"

        # Make sure config dir. exists
        self.player_config_dir.mkdir(parents=True, exist_ok=True)

    def _resolve_player_root(self, configured_root: Optional[str]) -> Path:
        """Resolve player root directory from config or auto-detect"""
        if configured_root:
            return Path(configured_root).expanduser()

        candidates = [
            # Preferred: inside this repo (single-repo layout)
            self.bangtunes_root,
            self.bangtunes_root / "player",
            # Then CWD-based (for development)
            Path.cwd(),
            Path.cwd() / "player",
            # Legacy fallbacks for existing setups
            Path.home() / "Builds" / "player",
            Path.home() / "player",
        ]

        for candidate in candidates:
            if (candidate / "Cargo.toml").exists():
                return candidate

        raise RuntimeError(
            "Could not locate player project. Set [player].root in bangtunes.toml."
        )

    def setup_player_config(self) -> None:
        """Create player configuration pointing to BangTunes downloads"""
        toml_content = """[audio]
volume = 0.7
buffer_size_kb = 65

[behavior]
skip_threshold_seconds = 30
weight_decay_days = 30
min_play_time_for_tracking = 10

[ui]
show_notifications = true
notification_duration_ms = 3000
theme = "default"
"""

        with open(self.player_config_file, "w") as f:
            f.write(toml_content)

        console.print(
            f"[green]✅ Player config created at {self.player_config_file}[/green]"
        )

    # Note: Rust player reads library.db directly - no sync needed

    def launch_player(self, track_path: Optional[str] = None) -> bool:
        """Launch BangTunes player"""
        if not self.player_root.exists():
            console.print(f"[red]❌ Player not found at {self.player_root}[/red]")
            console.print(
                "[yellow]💡 Please check that player source is available, or update the path in bangtunes.toml[/yellow]"
            )
            return False

        # Check if player is built
        if not self._check_player_built():
            return False

        # Set environment variable for config
        env = os.environ.copy()
        env["BANGTUNES_PLAYER_CONFIG"] = str(self.player_config_file)

        try:
            # Launch BangTunes interactive player
            binary_path = (
                self.player_root / "target" / "release" / "panpipe_interactive"
            )
            if not binary_path.exists():
                binary_path = (
                    self.player_root / "target" / "debug" / "panpipe_interactive"
                )

            if not binary_path.exists():
                console.print("[red]Player binary not found. Building...[/red]")
                if not self._build_player():
                    console.print("[red]Failed to build player. Please check:[/red]")
                    console.print(
                        "[yellow]   1. Rust is installed: https://rustup.rs/[/yellow]"
                    )
                    console.print(
                        "[yellow]   2. Player source is available at the configured path[/yellow]"
                    )
                    console.print(
                        "[yellow]   3. Run 'cargo build --release' manually in player directory[/yellow]"
                    )
                    return False
                binary_path = (
                    self.player_root / "target" / "release" / "panpipe_interactive"
                )

            console.print("[green]Launching BangTunes player...[/green]")

            # Launch in the background or foreground based on track_path
            cmd = [str(binary_path)]
            if track_path:
                # Launch with specific track using --play-track argument
                cmd += ["--play-track", str(track_path)]

            # Make failures visible: CalledProcessError will now be raised and caught below
            subprocess.run(
                cmd,
                cwd=self.player_root,
                env=env,
                check=True,
            )

            return True

        except subprocess.CalledProcessError as e:
            console.print(f"[red]❌ Failed to launch player: {e}[/red]")
            console.print("[yellow]💡 Try running the player manually:[/yellow]")
            console.print(f"[dim]   cd {self.player_root}[/dim]")
            console.print(
                f"[dim]   BANGTUNES_PLAYER_CONFIG={self.player_config_file} ./target/release/panpipe_interactive[/dim]"
            )
            return False
        except FileNotFoundError:
            console.print("[red]❌ Player binary not found after build attempt[/red]")
            console.print("[yellow]💡 Please build player manually:[/yellow]")
            console.print(f"[dim]   cd {self.player_root}[/dim]")
            console.print("[dim]   cargo build --release[/dim]")
            return False
        except KeyboardInterrupt:
            console.print("\n[yellow]🎵 BangTunes player closed[/yellow]")
            return True

    def _check_player_built(self) -> bool:
        """Check if player is built and ready"""
        cargo_toml = self.player_root / "Cargo.toml"
        if not cargo_toml.exists():
            console.print(f"[red]❌ Player Cargo.toml not found at {cargo_toml}[/red]")
            return False

        # Check if binary exists
        release_binary = self.player_root / "target" / "release" / "panpipe_interactive"
        debug_binary = self.player_root / "target" / "debug" / "panpipe_interactive"

        if release_binary.exists() or debug_binary.exists():
            return True

        return self._build_player()

    def _build_player(self) -> bool:
        """Build player using cargo"""
        console.print(
            "[cyan]🔨 Building player (this may take a few minutes)...[/cyan]"
        )

        try:
            # Build in release mode for better performance
            result = subprocess.run(
                ["cargo", "build", "--release", "--bin", "panpipe_interactive"],
                cwd=self.player_root,
                capture_output=True,
                text=True,
                timeout=300,  # 5 minute timeout
            )

            if result.returncode == 0:
                console.print("[green]✅ Player built successfully[/green]")
                return True
            else:
                console.print("[red]❌ Player build failed:[/red]")
                console.print(f"[red]{result.stderr}[/red]")
                return False

        except subprocess.TimeoutExpired:
            console.print("[red]❌ Player build timed out[/red]")
            return False
        except FileNotFoundError:
            console.print(
                "[red]❌ Cargo not found. Please install Rust: https://rustup.rs/[/red]"
            )
            return False
        except Exception as e:
            console.print(f"[red]❌ Build error: {e}[/red]")
            return False

    # Note: Old sync/status methods removed - commands.py handles these directly


def create_integration(
    bangtunes_root: Path, config: Optional[Dict] = None
) -> BangTunesPlayer:
    """Factory function to create BangTunes player instance"""
    return BangTunesPlayer(bangtunes_root, config)
