"""
Python-based music player for when Rust compilation fails (especially on Termux).
Provides a simple but functional TUI for music playback.
"""

import random
import sqlite3
import subprocess
import threading
from pathlib import Path
from typing import Any, Dict, List, Optional

try:
    from rich.console import Console
    console: Optional[Console] = Console()
    RICH_AVAILABLE = True
except ImportError:
    console = None
    RICH_AVAILABLE = False


class PythonMusicPlayer:
    """Simple Python-based music player for Termux/platforms where Rust won't compile."""
    
    def __init__(self, root: Path, config: Dict[str, Any]):
        self.root = root
        self.config = config
        self.db_path = root / "library.db"
        self.current_track: Optional[Dict[str, Any]] = None
        self.playlist: List[Dict[str, Any]] = []
        self.playlist_index = 0
        self.is_playing = False
        self.player_process: Optional[subprocess.Popen] = None
        self.shuffle = False
        self.repeat = False
        
        # Detect available player
        self.player_cmd = self._detect_player()
        if not self.player_cmd:
            raise RuntimeError("No audio player available (mpv, vlc, or ffplay required)")
    
    def _detect_player(self) -> Optional[str]:
        """Detect available audio player."""
        import shutil
        players = ["mpv", "vlc", "ffplay", "mplayer"]
        for player in players:
            if shutil.which(player):
                return player
        return None
    
    def _get_player_command(self, file_path: str) -> List[str]:
        """Get the command to play audio with the detected player."""
        if self.player_cmd == "mpv":
            return ["mpv", "--no-video", "--really-quiet", file_path]
        elif self.player_cmd == "vlc":
            return ["vlc", "--intf", "dummy", "--play-and-exit", file_path]
        elif self.player_cmd == "ffplay":
            return ["ffplay", "-nodisp", "-autoexit", "-loglevel", "quiet", file_path]
        else:  # mplayer
            return ["mplayer", "-really-quiet", file_path]
    
    def load_tracks(self) -> None:
        """Load all tracks from the database."""
        try:
            conn = sqlite3.connect(self.db_path)
            cur = conn.cursor()
            cur.execute("""
                SELECT id, youtube_id, title, artist, album, file_path 
                FROM tracks 
                WHERE file_path IS NOT NULL
                ORDER BY artist, album, title
            """)
            rows = cur.fetchall()
            self.playlist = [
                {
                    "id": row[0],
                    "youtube_id": row[1],
                    "title": row[2] or "Unknown Title",
                    "artist": row[3] or "Unknown Artist",
                    "album": row[4] or "Unknown Album",
                    "file_path": row[5],
                }
                for row in rows
                if row[5] and Path(row[5]).exists()
            ]
            conn.close()
            
            if self.shuffle:
                random.shuffle(self.playlist)
                
        except Exception as e:
            if console:
                console.print(f"[red]Failed to load tracks: {e}[/red]")
            else:
                print(f"Failed to load tracks: {e}")
    
    def play_track(self, index: int) -> None:
        """Play a specific track by index."""
        if not self.playlist:
            return
            
        if index < 0 or index >= len(self.playlist):
            return
        
        # Stop current playback
        self.stop()
        
        self.playlist_index = index
        self.current_track = self.playlist[index]
        self.is_playing = True
        
        # Start playback in background
        file_path = self.current_track["file_path"]
        cmd = self._get_player_command(file_path)
        
        try:
            self.player_process = subprocess.Popen(
                cmd,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL
            )
            
            # Monitor playback in background thread
            threading.Thread(target=self._monitor_playback, daemon=True).start()
            
        except Exception as e:
            self.is_playing = False
            if console:
                console.print(f"[red]Playback failed: {e}[/red]")
            else:
                print(f"Playback failed: {e}")
    
    def _monitor_playback(self) -> None:
        """Monitor playback and auto-advance when track ends."""
        if not self.player_process:
            return
            
        self.player_process.wait()
        self.is_playing = False
        
        # Auto-advance to next track
        if self.repeat:
            self.play_track(self.playlist_index)
        elif self.playlist_index < len(self.playlist) - 1:
            self.next_track()
    
    def stop(self) -> None:
        """Stop current playback."""
        if self.player_process:
            try:
                self.player_process.terminate()
                self.player_process.wait(timeout=2)
            except Exception:
                try:
                    self.player_process.kill()
                except Exception:
                    pass
            self.player_process = None
        self.is_playing = False
    
    def next_track(self) -> None:
        """Play next track."""
        if not self.playlist:
            return
        next_index = (self.playlist_index + 1) % len(self.playlist)
        self.play_track(next_index)
    
    def prev_track(self) -> None:
        """Play previous track."""
        if not self.playlist:
            return
        prev_index = (self.playlist_index - 1) % len(self.playlist)
        self.play_track(prev_index)
    
    def toggle_shuffle(self) -> None:
        """Toggle shuffle mode."""
        self.shuffle = not self.shuffle
        if self.shuffle:
            # Re-shuffle playlist
            current = self.current_track
            random.shuffle(self.playlist)
            # Find current track in shuffled list
            if current:
                for i, track in enumerate(self.playlist):
                    if track["id"] == current["id"]:
                        self.playlist_index = i
                        break
    
    def toggle_repeat(self) -> None:
        """Toggle repeat mode."""
        self.repeat = not self.repeat
    
    def run_interactive(self) -> int:
        """Run interactive player with simple controls."""
        self.load_tracks()
        
        if not self.playlist:
            if console:
                console.print("[yellow]No tracks found in library[/yellow]")
                console.print("[dim]Run 'build' then 'download' to add music[/dim]")
            else:
                print("No tracks found in library")
                print("Run 'build' then 'download' to add music")
            return 1
        
        # Show player info
        if console:
            console.print("\n[bold green]🎵 BangTunes Python Player[/bold green]")
            console.print(f"[dim]Using {self.player_cmd} for playback[/dim]")
            console.print(f"[dim]Loaded {len(self.playlist)} tracks[/dim]\n")
            console.print("[bold]Controls:[/bold]")
            console.print("  [cyan]n[/cyan] - Next track")
            console.print("  [cyan]p[/cyan] - Previous track")
            console.print("  [cyan]s[/cyan] - Toggle shuffle")
            console.print("  [cyan]r[/cyan] - Toggle repeat")
            console.print("  [cyan]q[/cyan] - Quit\n")
        else:
            print("\n🎵 BangTunes Python Player")
            print(f"Using {self.player_cmd} for playback")
            print(f"Loaded {len(self.playlist)} tracks\n")
            print("Controls:")
            print("  n - Next track")
            print("  p - Previous track")
            print("  s - Toggle shuffle")
            print("  r - Toggle repeat")
            print("  q - Quit\n")
        
        # Start with first track
        self.play_track(0)
        
        # Simple input loop
        try:
            while True:
                # Show current track
                if self.current_track:
                    status = "Playing" if self.is_playing else "Paused"
                    shuffle_status = " [Shuffle]" if self.shuffle else ""
                    repeat_status = " [Repeat]" if self.repeat else ""
                    
                    if console:
                        console.print(f"\n{status}: {self.current_track['artist']} - {self.current_track['title']}{shuffle_status}{repeat_status}")
                    else:
                        print(f"\n{status}: {self.current_track['artist']} - {self.current_track['title']}{shuffle_status}{repeat_status}")
                
                # Get user input
                try:
                    cmd = input("> ").strip().lower()
                except EOFError:
                    break
                
                if cmd == 'q':
                    break
                elif cmd == 'n':
                    self.next_track()
                elif cmd == 'p':
                    self.prev_track()
                elif cmd == 's':
                    self.toggle_shuffle()
                    msg = "Shuffle ON" if self.shuffle else "Shuffle OFF"
                    if console:
                        console.print(f"[yellow]{msg}[/yellow]")
                    else:
                        print(msg)
                elif cmd == 'r':
                    self.toggle_repeat()
                    msg = "Repeat ON" if self.repeat else "Repeat OFF"
                    if console:
                        console.print(f"[yellow]{msg}[/yellow]")
                    else:
                        print(msg)
                        
        except KeyboardInterrupt:
            pass
        
        # Clean up
        self.stop()
        
        if console:
            console.print("\n[green]Thanks for using BangTunes![/green]")
        else:
            print("\nThanks for using BangTunes!")
        
        return 0


def run_python_player(root: Path, config: Dict[str, Any]) -> int:
    """Entry point for Python-based player."""
    try:
        player = PythonMusicPlayer(root, config)
        return player.run_interactive()
    except Exception as e:
        if console:
            console.print(f"[red]Player error: {e}[/red]")
        else:
            print(f"Player error: {e}")
        return 1
