#!/usr/bin/env python3
"""
PanPipe Integration for Bang Tunes

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

Integration layer for PanPipe terminal music player.
Basically bridges the gap between downloaded music and the player.
"""

import os
import subprocess
import sqlite3
import toml
from pathlib import Path
from typing import Dict, Optional

from rich.console import Console

console = Console()

class PanPipeIntegration:
    """Bridges BangTunes discovery with PanPipe's smart playback
    
    This is where the magic happens - your downloaded music gets fed into
    a player that actually learns what you like. No more random shuffle!
    """
    
    def __init__(self, bangtunes_root: Path, config: Optional[Dict] = None):
        self.bangtunes_root = bangtunes_root
        self.downloads_dir = bangtunes_root / "downloads"
        self.bangtunes_db = bangtunes_root / "library.db"
        
        # Load PanPipe configuration with sensible defaults
        panpipe_config = (config or {}).get("panpipe", {})
        
        # PanPipe paths from config or auto-detect
        self.panpipe_root = self._resolve_panpipe_root(panpipe_config.get("root"))
        self.panpipe_config_dir = Path(panpipe_config.get("config_dir", Path.home() / ".config" / "bangtunes"))
        self.panpipe_config_file = self.panpipe_config_dir / "panpipe.toml"
        self.panpipe_db = self.panpipe_config_dir / "panpipe.db"
        
        # Ensure config directory exists
        self.panpipe_config_dir.mkdir(parents=True, exist_ok=True)
    
    def _resolve_panpipe_root(self, configured_root: Optional[str]) -> Path:
        """Resolve PanPipe root directory from config or auto-detect"""
        if configured_root:
            return Path(configured_root).expanduser()
        
        candidates = [
            # Preferred: inside this repo (single-repo layout)
            self.bangtunes_root,
            self.bangtunes_root / "PanPipe",
            # Then CWD-based (for development)
            Path.cwd(),
            Path.cwd() / "PanPipe",
            # Legacy fallbacks for existing setups
            Path.home() / "Builds" / "PanPipe",
            Path.home() / "PanPipe",
        ]
        
        for candidate in candidates:
            if (candidate / "Cargo.toml").exists():
                return candidate
        
        raise RuntimeError(
            "Could not locate PanPipe project. Set [panpipe].root in bangtunes.toml."
        )
    
    def setup_panpipe_config(self) -> None:
        """Create PanPipe configuration pointing to BangTunes downloads"""
        config = {
            "music_directories": [str(self.downloads_dir)],
            "database_path": str(self.panpipe_db),
            "spotify": {
                "client_id": None,
                "redirect_uri": "http://localhost:8888/callback"
            },
            "behavior": {
                "skip_threshold_seconds": 30,
                "weight_decay_days": 30,
                "min_play_time_for_tracking": 10
            },
            "ui": {
                "show_notifications": True,
                "notification_duration_ms": 3000,
                "theme": "default"
            }
        }
        
        with open(self.panpipe_config_file, 'w') as f:
            toml.dump(config, f)
        
        console.print(f"[green]✅ PanPipe config created at {self.panpipe_config_file}[/green]")
    
    def sync_libraries(self) -> None:
        """Sync BangTunes library data with PanPipe database"""
        if not self.bangtunes_db.exists():
            console.print("[yellow]⚠️  BangTunes library.db not found[/yellow]")
            return
        
        # Read BangTunes tracks
        bangtunes_conn = sqlite3.connect(self.bangtunes_db)
        cursor = bangtunes_conn.cursor()
        cursor.execute("""
            SELECT youtube_id, title, artist, album, file_path 
            FROM tracks 
            WHERE file_path IS NOT NULL
        """)
        tracks = cursor.fetchall()
        bangtunes_conn.close()
        
        if not tracks:
            console.print("[yellow]⚠️  No tracks found in BangTunes library[/yellow]")
            return
        
        console.print(f"[cyan]📚 Syncing {len(tracks)} tracks to PanPipe...[/cyan]")
        
        # Initialize PanPipe database if needed
        self._init_panpipe_db()
        
        # Insert tracks into PanPipe database
        panpipe_conn = sqlite3.connect(self.panpipe_db)
        panpipe_cursor = panpipe_conn.cursor()
        
        synced_count = 0
        for youtube_id, title, artist, album, file_path in tracks:
            if Path(file_path).exists():
                try:
                    # Insert track with BangTunes metadata
                    panpipe_cursor.execute("""
                        INSERT OR REPLACE INTO tracks 
                        (file_path, title, artist, album, youtube_id, duration_ms, content_hash)
                        VALUES (?, ?, ?, ?, ?, NULL, NULL)
                    """, (file_path, title, artist, album, youtube_id))
                    synced_count += 1
                except sqlite3.Error as e:
                    console.print(f"[red]❌ Error syncing {file_path}: {e}[/red]")
        
        panpipe_conn.commit()
        panpipe_conn.close()
        
        console.print(f"[green]✅ Synced {synced_count} tracks to PanPipe library[/green]")
    
    def _init_panpipe_db(self) -> None:
        """Initialize PanPipe database schema"""
        conn = sqlite3.connect(self.panpipe_db)
        cursor = conn.cursor()
        
        # Create tracks table compatible with PanPipe
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT UNIQUE NOT NULL,
                title TEXT,
                artist TEXT,
                album TEXT,
                duration_ms INTEGER,
                content_hash TEXT,
                youtube_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        """)
        
        # Create behavior tracking table
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS behavior_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                play_duration_ms INTEGER,
                skip_position_ms INTEGER,
                FOREIGN KEY (track_id) REFERENCES tracks (id)
            )
        """)
        
        # Create indexes
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_tracks_path ON tracks(file_path)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_behavior_track ON behavior_events(track_id)")
        
        conn.commit()
        conn.close()
    
    def launch_player(self, track_path: Optional[str] = None) -> bool:
        """Launch PanPipe player"""
        if not self.panpipe_root.exists():
            console.print(f"[red]❌ PanPipe not found at {self.panpipe_root}[/red]")
            console.print("[yellow]💡 Please ensure PanPipe is cloned and available, or update the path in bangtunes.toml[/yellow]")
            return False
        
        # Ensure PanPipe is built
        if not self._ensure_panpipe_built():
            return False
        
        # Set environment variable for config
        env = os.environ.copy()
        env['PANPIPE_CONFIG'] = str(self.panpipe_config_file)
        
        try:
            # Launch PanPipe interactive player
            binary_path = self.panpipe_root / "target" / "release" / "panpipe_interactive"
            if not binary_path.exists():
                binary_path = self.panpipe_root / "target" / "debug" / "panpipe_interactive"
            
            if not binary_path.exists():
                console.print("[red]PanPipe binary not found. Building...[/red]")
                if not self._build_panpipe():
                    console.print("[red]Failed to build PanPipe. Please check:[/red]")
                    console.print("[yellow]   1. Rust is installed: https://rustup.rs/[/yellow]")
                    console.print("[yellow]   2. PanPipe source is available at the configured path[/yellow]")
                    console.print("[yellow]   3. Run 'cargo build --release' manually in PanPipe directory[/yellow]")
                    return False
                binary_path = self.panpipe_root / "target" / "release" / "panpipe_interactive"
            
            console.print("[green]Launching PanPipe player...[/green]")
            
            # Launch in the background or foreground based on track_path
            if track_path:
                # TODO: Add support for launching with specific track
                subprocess.run([str(binary_path)], cwd=self.panpipe_root, env=env)
            else:
                subprocess.run([str(binary_path)], cwd=self.panpipe_root, env=env)
            
            return True
            
        except subprocess.CalledProcessError as e:
            console.print(f"[red]❌ Failed to launch PanPipe: {e}[/red]")
            console.print("[yellow]💡 Try running the player manually:[/yellow]")
            console.print(f"[dim]   cd {self.panpipe_root}[/dim]")
            console.print(f"[dim]   PANPIPE_CONFIG={self.panpipe_config_file} ./target/release/panpipe_interactive[/dim]")
            return False
        except FileNotFoundError:
            console.print("[red]❌ PanPipe binary not found after build attempt[/red]")
            console.print("[yellow]💡 Please build PanPipe manually:[/yellow]")
            console.print(f"[dim]   cd {self.panpipe_root}[/dim]")
            console.print("[dim]   cargo build --release[/dim]")
            return False
        except KeyboardInterrupt:
            console.print("\n[yellow]🎵 PanPipe player closed[/yellow]")
            return True
    
    def _ensure_panpipe_built(self) -> bool:
        """Ensure PanPipe is built and ready"""
        cargo_toml = self.panpipe_root / "Cargo.toml"
        if not cargo_toml.exists():
            console.print(f"[red]❌ PanPipe Cargo.toml not found at {cargo_toml}[/red]")
            return False
        
        # Check if binary exists
        release_binary = self.panpipe_root / "target" / "release" / "panpipe_interactive"
        debug_binary = self.panpipe_root / "target" / "debug" / "panpipe_interactive"
        
        if release_binary.exists() or debug_binary.exists():
            return True
        
        return self._build_panpipe()
    
    def _build_panpipe(self) -> bool:
        """Build PanPipe using cargo"""
        console.print("[cyan]🔨 Building PanPipe (this may take a few minutes)...[/cyan]")
        
        try:
            # Build in release mode for better performance
            result = subprocess.run(
                ["cargo", "build", "--release", "--bin", "panpipe_interactive"],
                cwd=self.panpipe_root,
                capture_output=True,
                text=True,
                timeout=300  # 5 minute timeout
            )
            
            if result.returncode == 0:
                console.print("[green]✅ PanPipe built successfully[/green]")
                return True
            else:
                console.print("[red]❌ PanPipe build failed:[/red]")
                console.print(f"[red]{result.stderr}[/red]")
                return False
                
        except subprocess.TimeoutExpired:
            console.print("[red]❌ PanPipe build timed out[/red]")
            return False
        except FileNotFoundError:
            console.print("[red]❌ Cargo not found. Please install Rust: https://rustup.rs/[/red]")
            return False
        except Exception as e:
            console.print(f"[red]❌ Build error: {e}[/red]")
            return False
    
    def get_library_stats(self) -> Dict[str, int]:
        """Get combined library statistics"""
        stats = {"bangtunes_tracks": 0, "panpipe_tracks": 0, "synced_tracks": 0}
        
        # BangTunes stats
        if self.bangtunes_db.exists():
            conn = sqlite3.connect(self.bangtunes_db)
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM tracks")
            stats["bangtunes_tracks"] = cursor.fetchone()[0]
            conn.close()
        
        # PanPipe stats
        if self.panpipe_db.exists():
            conn = sqlite3.connect(self.panpipe_db)
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM tracks")
            stats["panpipe_tracks"] = cursor.fetchone()[0]
            
            # Count synced tracks (those with youtube_id)
            cursor.execute("SELECT COUNT(*) FROM tracks WHERE youtube_id IS NOT NULL")
            stats["synced_tracks"] = cursor.fetchone()[0]
            conn.close()
        
        return stats
    
    def status(self) -> None:
        """Display integration status"""
        console.print("[bold]🎵 BangTunes ↔ PanPipe Integration Status[/bold]")
        console.print("=" * 50)
        
        # Check PanPipe availability
        if self.panpipe_root.exists():
            console.print(f"[green]✅ PanPipe found at {self.panpipe_root}[/green]")
        else:
            console.print(f"[red]❌ PanPipe not found at {self.panpipe_root}[/red]")
            return
        
        # Check configuration
        if self.panpipe_config_file.exists():
            console.print(f"[green]✅ Config file: {self.panpipe_config_file}[/green]")
        else:
            console.print(f"[yellow]⚠️  Config file missing: {self.panpipe_config_file}[/yellow]")
        
        # Library stats
        stats = self.get_library_stats()
        console.print(f"[cyan]📚 BangTunes tracks: {stats['bangtunes_tracks']}[/cyan]")
        console.print(f"[cyan]🎵 PanPipe tracks: {stats['panpipe_tracks']}[/cyan]")
        console.print(f"[cyan]🔗 Synced tracks: {stats['synced_tracks']}[/cyan]")
        
        # Check if binary is built
        release_binary = self.panpipe_root / "target" / "release" / "panpipe_interactive"
        debug_binary = self.panpipe_root / "target" / "debug" / "panpipe_interactive"
        
        if release_binary.exists():
            console.print("[green]✅ PanPipe binary ready (release)[/green]")
        elif debug_binary.exists():
            console.print("[yellow]⚠️  PanPipe binary ready (debug)[/yellow]")
        else:
            console.print("[red]❌ PanPipe binary not built[/red]")


def create_integration(bangtunes_root: Path, config: Optional[Dict] = None) -> PanPipeIntegration:
    """Factory function to create PanPipe integration"""
    return PanPipeIntegration(bangtunes_root, config)
