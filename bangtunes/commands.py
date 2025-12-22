"""
Command implementations for BangTunes CLI.

Each command is implemented as a separate function that takes args, root, and config.
"""

from pathlib import Path
from typing import Any, Dict, Optional

try:
    from rich.console import Console

    console: Optional[Console] = Console()
except ImportError:
    console = None

# Import the modular functions we actually use
from bangtunes.library import list_batches, read_seed, build_pool_from_seed, write_batches, read_batch_csv
from bangtunes.downloads import download_batch, graceful_sigint
from bangtunes.db import get_db, db_get_all_tracks, db_has_yid, db_add_track


def print_banner() -> None:
    """Print the BangTunes ASCII banner."""
    if console:
        console.print(
            "──────────────────────────────────────────────────────────────── Bang Tunes ─────────────────────────────────────────────────────────────────"
        )
        console.print(str(Path.cwd()))
        console.print()
        console.print(
            "╔──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╗"
        )
        console.print(
            "│                                                                                                                              │"
        )
        console.print(
            "│   ▄▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄  ▄▄        ▄  ▄▄▄▄▄▄▄▄▄▄▄       ▄▄▄▄▄▄▄▄▄▄▄  ▄         ▄  ▄▄        ▄  ▄▄▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄▄▄   │"
        )
        console.print(
            "│  ▐░░░░░░░░░░▌ ▐░░░░░░░░░░░▌▐░░▌      ▐░▌▐░░░░░░░░░░░▌     ▐░░░░░░░░░░░▌▐░▌       ▐░▌▐░░▌      ▐░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌  │"
        )
        console.print(
            "│  ▐░█▀▀▀▀▀▀▀█░▌▐░█▀▀▀▀▀▀▀█░▌▐░▌░▌     ▐░▌▐░█▀▀▀▀▀▀▀▀▀       ▀▀▀▀█░█▀▀▀▀ ▐░▌       ▐░▌▐░▌░▌     ▐░▌▐░█▀▀▀▀▀▀▀▀▀ ▐░█▀▀▀▀▀▀▀▀▀   │"
        )
        console.print(
            "│  ▐░▌       ▐░▌▐░▌       ▐░▌▐░▌▐░▌    ▐░▌▐░▌                    ▐░▌     ▐░▌       ▐░▌▐░▌▐░▌    ▐░▌▐░▌          ▐░▌            │"
        )
        console.print(
            "│  ▐░█▄▄▄▄▄▄▄█░▌▐░█▄▄▄▄▄▄▄█░▌▐░▌ ▐░▌   ▐░▌▐░▌ ▄▄▄▄▄▄▄▄           ▐░▌     ▐░▌       ▐░▌▐░▌ ▐░▌   ▐░▌▐░█▄▄▄▄▄▄▄▄▄ ▐░█▄▄▄▄▄▄▄▄▄   │"
        )
        console.print(
            "│  ▐░░░░░░░░░░▌ ▐░░░░░░░░░░░▌▐░▌  ▐░▌  ▐░▌▐░▌▐░░░░░░░░▌          ▐░▌     ▐░▌       ▐░▌▐░▌  ▐░▌  ▐░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌  │"
        )
        console.print(
            "│  ▐░█▀▀▀▀▀▀▀█░▌▐░█▀▀▀▀▀▀▀█░▌▐░▌   ▐░▌ ▐░▌▐░▌ ▀▀▀▀▀▀█░▌          ▐░▌     ▐░▌       ▐░▌▐░▌   ▐░▌ ▐░▌▐░█▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀█░▌  │"
        )
        console.print(
            "│  ▐░▌       ▐░▌▐░▌       ▐░▌▐░▌    ▐░▌▐░▌▐░▌       ▐░▌          ▐░▌     ▐░▌       ▐░▌▐░▌    ▐░▌▐░▌▐░▌                    ▐░▌  │"
        )
        console.print(
            "│  ▐░█▄▄▄▄▄▄▄█░▌▐░▌       ▐░▌▐░▌     ▐░▐░▌▐░█▄▄▄▄▄▄▄█░▌          ▐░▌     ▐░█▄▄▄▄▄▄▄█░▌▐░▌     ▐░▐░▌▐░█▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄█░▌  │"
        )
        console.print(
            "│  ▐░░░░░░░░░░▌ ▐░▌       ▐░▌▐░▌      ▐░░▌▐░░░░░░░░░░░▌          ▐░▌     ▐░░░░░░░░░░░▌▐░▌      ▐░░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌  │"
        )
        console.print(
            "│   ▀▀▀▀▀▀▀▀▀▀   ▀         ▀  ▀        ▀▀  ▀▀▀▀▀▀▀▀▀▀▀            ▀       ▀▀▀▀▀▀▀▀▀▀▀  ▀        ▀▀  ▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀   │"
        )
        console.print(
            "│                                                                                                                              │"
        )
        console.print(
            "│                                                 created by Himokai                                                           │"
        )
        console.print(
            "╚──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╝"
        )
        console.print()
    else:
        print("Bang Tunes")
        print(str(Path.cwd()))


def cmd_build(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle build command."""
    try:
        # Read seed file
        seed_path = root / "seed.csv"
        if console:
            console.print(f"[dim]Reading seed file: {seed_path}[/dim]")
        else:
            print(f"Reading seed file: {seed_path}")
        
        seed_rows = read_seed(seed_path)
        
        # Build pool from seed
        if console:
            console.print("[dim]Building discovery pool from seed tracks...[/dim]")
        else:
            print("Building discovery pool from seed tracks...")
        
        pool = build_pool_from_seed(
            seed_rows, 
            min_score=getattr(args, 'min_score', 60),
            cc_only=getattr(args, 'cc_only', False)
        )
        
        if not pool:
            if console:
                console.print("[red]No tracks found in discovery pool[/red]")
            else:
                print("No tracks found in discovery pool")
            return 1
        
        # Write batches
        batch_dir = root / "batches"
        if console:
            console.print(f"[dim]Writing batches to: {batch_dir}[/dim]")
        else:
            print(f"Writing batches to: {batch_dir}")
        
        batch_files = write_batches(
            pool,
            batch_dir,
            prefix=getattr(args, 'prefix', 'mix'),
            size=getattr(args, 'size', 50)
        )
        
        # Show results
        if console:
            console.print(f"[green]✓ Created {len(batch_files)} batch files with {len(pool)} total tracks[/green]")
            for batch_file in batch_files:
                console.print(f"[dim]  {batch_file.name}[/dim]")
        else:
            print(f"✓ Created {len(batch_files)} batch files with {len(pool)} total tracks")
            for batch_file in batch_files:
                print(f"  {batch_file.name}")
        
        return 0
        
    except Exception as e:
        if console:
            console.print(f"[red]Build failed: {e}[/red]")
        else:
            print(f"Build failed: {e}")
        return 1


def cmd_download(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle download command."""
    try:
        # Get batch file path
        batch_csv = getattr(args, 'batch_csv', None)
        if not batch_csv:
            if console:
                console.print("[red]No batch CSV file specified[/red]")
            else:
                print("No batch CSV file specified")
            return 1
        
        batch_path = Path(batch_csv)
        if not batch_path.is_absolute():
            batch_path = root / "batches" / batch_csv
        
        if not batch_path.exists():
            if console:
                console.print(f"[red]Batch file not found: {batch_path}[/red]")
            else:
                print(f"Batch file not found: {batch_path}")
            return 1
        
        # Get database connection
        db_path = root / "library.db"
        db_conn = get_db(db_path)
        
        # Set up paths
        batch_dir = root / "batches"
        dl_root = root / "downloads"
        
        # Download the batch
        if console:
            console.print(f"[dim]Downloading batch: {batch_path.name}[/dim]")
        else:
            print(f"Downloading batch: {batch_path.name}")
        
        download_batch(
            batch_path=batch_path,
            batch_dir=batch_dir,
            dl_root=dl_root,
            root=root,
            db_conn=db_conn,
            db_has_yid_func=db_has_yid,
            db_add_track_func=db_add_track,
            read_batch_csv_func=read_batch_csv,
            graceful_sigint_func=graceful_sigint,
            audio_format=getattr(args, 'format', 'opus'),
            download_mode=getattr(args, 'speed', 'normal'),
            debug_mode=False
        )
        
        if console:
            console.print("[green]✓ Download batch completed[/green]")
        else:
            print("✓ Download batch completed")
        
        return 0
        
    except Exception as e:
        if console:
            console.print(f"[red]Download failed: {e}[/red]")
        else:
            print(f"Download failed: {e}")
        return 1


def cmd_view(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle view command."""
    try:
        # Get database connection
        db_path = root / "library.db"
        db_conn = get_db(db_path)
        
        # Get all tracks for statistics
        tracks = db_get_all_tracks(db_conn)
        
        if not tracks:
            if console:
                console.print("[yellow]No tracks found in library[/yellow]")
                console.print("[dim]Run 'python bang_tunes.py build' to discover music[/dim]")
            else:
                print("No tracks found in library")
                print("Run 'python bang_tunes.py build' to discover music")
            return 0
        
        # Count tracks by artist
        artist_counts = {}
        for track in tracks:
            artist = track.get("artist", "Unknown Artist")
            artist_counts[artist] = artist_counts.get(artist, 0) + 1
        
        # Sort by track count (descending)
        sorted_artists = sorted(artist_counts.items(), key=lambda x: x[1], reverse=True)
        
        if console:
            console.print("\n[bold]Bang Tunes — Top Artists[/bold]")
            
            # Create table
            from rich.table import Table
            table = Table()
            table.add_column("Artist", style="cyan")
            table.add_column("# Tracks", justify="right", style="magenta")
            
            for artist, count in sorted_artists:
                table.add_row(artist, str(count))
            
            console.print(table)
        else:
            print("\nBang Tunes — Top Artists")
            print("-" * 50)
            for artist, count in sorted_artists:
                print(f"{artist:<40} {count:>8}")
        
        return 0
        
    except Exception as e:
        if console:
            console.print(f"[red]Error viewing library: {e}[/red]")
        else:
            print(f"Error viewing library: {e}")
        return 1


def cmd_list_batches(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle list-batches command."""
    batch_dir = root / "batches"
    if not batch_dir.exists():
        if console:
            console.print("[yellow]No batches directory found[/yellow]")
            console.print("[dim]Run 'python bang_tunes.py build' to create batches[/dim]")
        else:
            print("No batches directory found")
            print("Run 'python bang_tunes.py build' to create batches")
        return 0
    
    batches = list_batches(batch_dir)
    if not batches:
        if console:
            console.print("[yellow]No batch files found[/yellow]")
            console.print("[dim]Run 'python bang_tunes.py build' to create batches[/dim]")
        else:
            print("No batch files found")
            print("Run 'python bang_tunes.py build' to create batches")
        return 0
    
    if console:
        console.print("\n[bold]Available Batch Files[/bold]")
        from rich.table import Table
        table = Table()
        table.add_column("Batch File", style="cyan")
        table.add_column("Tracks", justify="right", style="magenta")
        
        for name, count in batches:
            table.add_row(name, str(count))
        
        console.print(table)
    else:
        print("\nAvailable Batch Files")
        print("-" * 40)
        for name, count in batches:
            print(f"{name:<30} {count:>8}")
    
    return 0


def cmd_install(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle install command - first-run setup wizard."""
    try:
        if console:
            console.print("[bold]🎵 BangTunes Setup Wizard[/bold]")
            console.print()
        else:
            print("🎵 BangTunes Setup Wizard")
            print("=" * 30)
        
        # Check and create directories
        directories = [
            root / "batches",
            root / "downloads", 
            root / "downloads" / "temp"
        ]
        
        if console:
            console.print("[dim]Creating directory structure...[/dim]")
        else:
            print("Creating directory structure...")
        
        for directory in directories:
            directory.mkdir(parents=True, exist_ok=True)
            if console:
                console.print(f"[green]✓[/green] {directory}")
            else:
                print(f"✓ {directory}")
        
        # Initialize database
        if console:
            console.print("\n[dim]Initializing database...[/dim]")
        else:
            print("\nInitializing database...")
        
        db_path = root / "library.db"
        get_db(db_path)  # Initialize database
        
        if console:
            console.print(f"[green]✓[/green] Database initialized: {db_path}")
        else:
            print(f"✓ Database initialized: {db_path}")
        
        # Check for seed.csv
        seed_path = root / "seed.csv"
        if not seed_path.exists():
            if console:
                console.print("\n[yellow]⚠️  No seed.csv found[/yellow]")
                console.print("[dim]Creating example seed.csv...[/dim]")
            else:
                print("\n⚠️  No seed.csv found")
                print("Creating example seed.csv...")
            
            # Create example seed file
            example_seed = """title,artist,notes
Bohemian Rhapsody,Queen,Classic rock anthem
Hotel California,Eagles,Iconic guitar solo
Stairway to Heaven,Led Zeppelin,Epic rock ballad
Imagine,John Lennon,Peaceful message
Sweet Child O' Mine,Guns N' Roses,Great guitar work"""
            
            seed_path.write_text(example_seed)
            
            if console:
                console.print("[green]✓[/green] Example seed.csv created")
                console.print("[dim]Edit seed.csv to add your favorite tracks for discovery[/dim]")
            else:
                print("✓ Example seed.csv created")
                print("Edit seed.csv to add your favorite tracks for discovery")
        else:
            if console:
                console.print("\n[green]✓[/green] Found existing seed.csv")
            else:
                print("\n✓ Found existing seed.csv")
        
        # Check dependencies
        if console:
            console.print("\n[dim]Checking dependencies...[/dim]")
        else:
            print("\nChecking dependencies...")
        
        deps_ok = True
        required_deps = ["ytmusicapi", "rapidfuzz", "mutagen", "rich"]
        
        for dep in required_deps:
            try:
                __import__(dep)
                if console:
                    console.print(f"[green]✓[/green] {dep}")
                else:
                    print(f"✓ {dep}")
            except ImportError:
                if console:
                    console.print(f"[red]✗[/red] {dep} (missing)")
                else:
                    print(f"✗ {dep} (missing)")
                deps_ok = False
        
        # Check for yt-dlp
        import shutil
        if shutil.which("yt-dlp"):
            if console:
                console.print("[green]✓[/green] yt-dlp")
            else:
                print("✓ yt-dlp")
        else:
            if console:
                console.print("[red]✗[/red] yt-dlp (missing)")
            else:
                print("✗ yt-dlp (missing)")
            deps_ok = False
        
        # Summary
        if console:
            console.print()
            if deps_ok:
                console.print("[green]🎉 Setup complete! BangTunes is ready to use.[/green]")
                console.print()
                console.print("[bold]Next steps:[/bold]")
                console.print("1. Edit seed.csv with your favorite tracks")
                console.print("2. Run: [cyan]python bang_tunes.py build[/cyan]")
                console.print("3. Run: [cyan]python bang_tunes.py download mix_001.csv[/cyan]")
                console.print("4. Run: [cyan]python bang_tunes.py stats[/cyan]")
            else:
                console.print("[yellow]⚠️  Setup incomplete - missing dependencies[/yellow]")
                console.print("Run: [cyan]pip install -r requirements.txt[/cyan]")
        else:
            print()
            if deps_ok:
                print("🎉 Setup complete! BangTunes is ready to use.")
                print()
                print("Next steps:")
                print("1. Edit seed.csv with your favorite tracks")
                print("2. Run: python bang_tunes.py build")
                print("3. Run: python bang_tunes.py download mix_001.csv")
                print("4. Run: python bang_tunes.py stats")
            else:
                print("⚠️  Setup incomplete - missing dependencies")
                print("Run: pip install -r requirements.txt")
        
        return 0 if deps_ok else 1
        
    except Exception as e:
        if console:
            console.print(f"[red]Install failed: {e}[/red]")
        else:
            print(f"Install failed: {e}")
        return 1


def cmd_doctor(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle doctor command - comprehensive system health check."""
    try:
        if console:
            console.print("[bold]🩺 BangTunes System Health Check[/bold]")
            console.print()
        else:
            print("🩺 BangTunes System Health Check")
            print("=" * 35)
        
        issues = []
        
        # Check directory structure
        if console:
            console.print("[dim]Checking directory structure...[/dim]")
        else:
            print("Checking directory structure...")
        
        required_dirs = [
            root / "batches",
            root / "downloads",
            root / "downloads" / "temp"
        ]
        
        for directory in required_dirs:
            if directory.exists():
                if console:
                    console.print(f"[green]✓[/green] {directory}")
                else:
                    print(f"✓ {directory}")
            else:
                if console:
                    console.print(f"[red]✗[/red] {directory} (missing)")
                else:
                    print(f"✗ {directory} (missing)")
                issues.append(f"Missing directory: {directory}")
        
        # Check database
        if console:
            console.print("\n[dim]Checking database...[/dim]")
        else:
            print("\nChecking database...")
        
        db_path = root / "library.db"
        if db_path.exists():
            try:
                db_conn = get_db(db_path)
                tracks = db_get_all_tracks(db_conn)
                if console:
                    console.print(f"[green]✓[/green] Database accessible ({len(tracks)} tracks)")
                else:
                    print(f"✓ Database accessible ({len(tracks)} tracks)")
            except Exception as e:
                if console:
                    console.print(f"[red]✗[/red] Database error: {e}")
                else:
                    print(f"✗ Database error: {e}")
                issues.append(f"Database error: {e}")
        else:
            if console:
                console.print("[yellow]⚠️[/yellow] Database not found (run install first)")
            else:
                print("⚠️ Database not found (run install first)")
            issues.append("Database not initialized")
        
        # Check seed file
        if console:
            console.print("\n[dim]Checking seed file...[/dim]")
        else:
            print("\nChecking seed file...")
        
        seed_path = root / "seed.csv"
        if seed_path.exists():
            try:
                seed_rows = read_seed(seed_path)
                if console:
                    console.print(f"[green]✓[/green] seed.csv valid ({len(seed_rows)} tracks)")
                else:
                    print(f"✓ seed.csv valid ({len(seed_rows)} tracks)")
            except Exception as e:
                if console:
                    console.print(f"[red]✗[/red] seed.csv error: {e}")
                else:
                    print(f"✗ seed.csv error: {e}")
                issues.append(f"Seed file error: {e}")
        else:
            if console:
                console.print("[yellow]⚠️[/yellow] seed.csv not found")
            else:
                print("⚠️ seed.csv not found")
            issues.append("No seed file found")
        
        # Check dependencies
        if console:
            console.print("\n[dim]Checking Python dependencies...[/dim]")
        else:
            print("\nChecking Python dependencies...")
        
        required_deps = {
            "ytmusicapi": "YouTube Music API",
            "rapidfuzz": "Fuzzy string matching",
            "mutagen": "Audio metadata",
            "rich": "Terminal formatting"
        }
        
        for dep, desc in required_deps.items():
            try:
                __import__(dep)
                if console:
                    console.print(f"[green]✓[/green] {dep} ({desc})")
                else:
                    print(f"✓ {dep} ({desc})")
            except ImportError:
                if console:
                    console.print(f"[red]✗[/red] {dep} ({desc}) - missing")
                else:
                    print(f"✗ {dep} ({desc}) - missing")
                issues.append(f"Missing dependency: {dep}")
        
        # Check external tools
        if console:
            console.print("\n[dim]Checking external tools...[/dim]")
        else:
            print("\nChecking external tools...")
        
        import shutil
        external_tools = {
            "yt-dlp": "Audio downloader (required)",
            "ffplay": "Audio player (optional)",
            "mpv": "Audio player (optional)",
            "vlc": "Audio player (optional)"
        }
        
        for tool, desc in external_tools.items():
            if shutil.which(tool):
                if console:
                    console.print(f"[green]✓[/green] {tool} ({desc})")
                else:
                    print(f"✓ {tool} ({desc})")
            else:
                if "required" in desc:
                    if console:
                        console.print(f"[red]✗[/red] {tool} ({desc}) - missing")
                    else:
                        print(f"✗ {tool} ({desc}) - missing")
                    issues.append(f"Missing required tool: {tool}")
                else:
                    if console:
                        console.print(f"[yellow]⚠️[/yellow] {tool} ({desc}) - not found")
                    else:
                        print(f"⚠️ {tool} ({desc}) - not found")
        
        # Check batch files
        if console:
            console.print("\n[dim]Checking batch files...[/dim]")
        else:
            print("\nChecking batch files...")
        
        batch_dir = root / "batches"
        if batch_dir.exists():
            batches = list_batches(batch_dir)
            if batches:
                if console:
                    console.print(f"[green]✓[/green] Found {len(batches)} batch files")
                else:
                    print(f"✓ Found {len(batches)} batch files")
            else:
                if console:
                    console.print("[yellow]⚠️[/yellow] No batch files found")
                else:
                    print("⚠️ No batch files found")
        
        # Summary
        if console:
            console.print()
            if not issues:
                console.print("[green]🎉 All systems healthy! BangTunes is ready to rock.[/green]")
            else:
                console.print(f"[yellow]⚠️  Found {len(issues)} issues:[/yellow]")
                for i, issue in enumerate(issues, 1):
                    console.print(f"[red]{i}.[/red] {issue}")
                console.print()
                console.print("[dim]💡 Run 'python bang_tunes.py install' to fix setup issues[/dim]")
        else:
            print()
            if not issues:
                print("🎉 All systems healthy! BangTunes is ready to rock.")
            else:
                print(f"⚠️  Found {len(issues)} issues:")
                for i, issue in enumerate(issues, 1):
                    print(f"{i}. {issue}")
                print()
                print("💡 Run 'python bang_tunes.py install' to fix setup issues")
        
        return 0 if not issues else 1
        
    except Exception as e:
        if console:
            console.print(f"[red]Doctor check failed: {e}[/red]")
        else:
            print(f"Doctor check failed: {e}")
        return 1


def cmd_stats(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle stats command."""
    try:
        # Get database connection
        db_path = root / "library.db"
        db_conn = get_db(db_path)
        
        # Get all tracks for statistics
        tracks = db_get_all_tracks(db_conn)
        
        if not tracks:
            if console:
                console.print("[yellow]No tracks found in library[/yellow]")
                console.print("[dim]Run 'python bang_tunes.py build' then 'download' to add music[/dim]")
            else:
                print("No tracks found in library")
                print("Run 'python bang_tunes.py build' then 'download' to add music")
            return 0
        
        # Calculate statistics
        total_tracks = len(tracks)
        artists = set(track.get("artist", "Unknown Artist") for track in tracks)
        albums = set(track.get("album", "Unknown Album") for track in tracks if track.get("album"))
        
        # Count tracks by artist
        artist_counts = {}
        for track in tracks:
            artist = track.get("artist", "Unknown Artist")
            artist_counts[artist] = artist_counts.get(artist, 0) + 1
        
        # Get top artists
        top_artists = sorted(artist_counts.items(), key=lambda x: x[1], reverse=True)[:10]
        
        if console:
            console.print("\n[bold]🎵 BangTunes Library Statistics[/bold]")
            console.print()
            
            # Overview stats
            from rich.table import Table
            stats_table = Table(title="Library Overview")
            stats_table.add_column("Metric", style="cyan")
            stats_table.add_column("Count", justify="right", style="magenta")
            
            stats_table.add_row("Total Tracks", str(total_tracks))
            stats_table.add_row("Unique Artists", str(len(artists)))
            stats_table.add_row("Albums", str(len(albums)))
            
            console.print(stats_table)
            console.print()
            
            # Top artists
            if top_artists:
                artist_table = Table(title="Top Artists")
                artist_table.add_column("Artist", style="cyan")
                artist_table.add_column("Tracks", justify="right", style="magenta")
                
                for artist, count in top_artists:
                    artist_table.add_row(artist, str(count))
                
                console.print(artist_table)
        else:
            print("\n🎵 BangTunes Library Statistics")
            print("=" * 40)
            print(f"Total Tracks: {total_tracks}")
            print(f"Unique Artists: {len(artists)}")
            print(f"Albums: {len(albums)}")
            print()
            print("Top Artists:")
            print("-" * 30)
            for artist, count in top_artists:
                print(f"{artist:<25} {count:>3}")
        
        return 0
        
    except Exception as e:
        if console:
            console.print(f"[red]Stats failed: {e}[/red]")
        else:
            print(f"Stats failed: {e}")
        return 1


def cmd_quickplay(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle quickplay command."""
    try:
        # Get database connection
        db_path = root / "library.db"
        db_conn = get_db(db_path)
        
        # Get all tracks
        tracks = db_get_all_tracks(db_conn)
        
        if not tracks:
            if console:
                console.print("[yellow]No tracks found in library[/yellow]")
                console.print("[dim]Run 'python bang_tunes.py build' then 'download' to add music[/dim]")
            else:
                print("No tracks found in library")
                print("Run 'python bang_tunes.py build' then 'download' to add music")
            return 0
        
        # Pick a random track
        import random
        track = random.choice(tracks)
        file_path = track.get("file_path")
        
        if not file_path or not Path(file_path).exists():
            if console:
                console.print("[red]Selected track file not found[/red]")
            else:
                print("Selected track file not found")
            return 1
        
        # Show what we're playing
        title = track.get("title", "Unknown Title")
        artist = track.get("artist", "Unknown Artist")
        
        if console:
            console.print(f"[green]🎵 Now playing:[/green] {title} by {artist}")
        else:
            print(f"🎵 Now playing: {title} by {artist}")
        
        # Try to play with available players
        import subprocess
        import shutil
        
        # Check for available players
        players = ["ffplay", "mpv", "vlc", "mplayer"]
        available_player = None
        
        for player in players:
            if shutil.which(player):
                available_player = player
                break
        
        if not available_player:
            if console:
                console.print("[red]No audio player found[/red]")
                console.print("[dim]Install ffplay, mpv, vlc, or mplayer to use quickplay[/dim]")
            else:
                print("No audio player found")
                print("Install ffplay, mpv, vlc, or mplayer to use quickplay")
            return 1
        
        # Play the track
        if available_player == "ffplay":
            cmd = ["ffplay", "-nodisp", "-autoexit", file_path]
        elif available_player == "mpv":
            cmd = ["mpv", "--no-video", file_path]
        elif available_player == "vlc":
            cmd = ["vlc", "--intf", "dummy", "--play-and-exit", file_path]
        else:  # mplayer
            cmd = ["mplayer", "-really-quiet", file_path]
        
        if console:
            console.print(f"[dim]Using {available_player} to play audio[/dim]")
        else:
            print(f"Using {available_player} to play audio")
        
        # Run the player
        subprocess.run(cmd, check=False)
        
        return 0
        
    except Exception as e:
        if console:
            console.print(f"[red]Quickplay failed: {e}[/red]")
        else:
            print(f"Quickplay failed: {e}")
        return 1


def cmd_edit_metadata(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle edit-metadata command - interactive metadata editor."""
    try:
        if console:
            console.print("[bold]✏️  BangTunes Metadata Editor[/bold]")
            console.print()
        else:
            print("✏️  BangTunes Metadata Editor")
            print("=" * 30)
        
        # Get database connection
        db_path = root / "library.db"
        db_conn = get_db(db_path)
        
        # Get all tracks
        tracks = db_get_all_tracks(db_conn)
        
        if not tracks:
            if console:
                console.print("[yellow]No tracks found in library[/yellow]")
            else:
                print("No tracks found in library")
            return 0
        
        if console:
            console.print(f"[dim]Found {len(tracks)} tracks in library[/dim]")
            console.print("[dim]Showing tracks with 'Unknown' metadata that need editing...[/dim]")
            console.print()
        else:
            print(f"Found {len(tracks)} tracks in library")
            print("Showing tracks with 'Unknown' metadata that need editing...")
            print()
        
        # Find tracks that need metadata editing
        needs_editing = []
        for track in tracks:
            title = track.get("title", "")
            artist = track.get("artist", "")
            
            if ("Unknown" in title or "Unknown" in artist or 
                not title or not artist or
                title.startswith("yt") or artist.startswith("yt")):
                needs_editing.append(track)
        
        if not needs_editing:
            if console:
                console.print("[green]✓ All tracks have good metadata![/green]")
            else:
                print("✓ All tracks have good metadata!")
            return 0
        
        if console:
            console.print(f"[yellow]Found {len(needs_editing)} tracks that need metadata editing:[/yellow]")
            console.print()
            
            from rich.table import Table
            table = Table()
            table.add_column("ID", style="dim")
            table.add_column("Current Title", style="red")
            table.add_column("Current Artist", style="red")
            table.add_column("File", style="dim")
            
            for track in needs_editing[:20]:  # Show first 20
                table.add_row(
                    str(track.get("id", "")),
                    track.get("title", "Unknown")[:40],
                    track.get("artist", "Unknown")[:30],
                    Path(track.get("file_path", "")).name[:30] if track.get("file_path") else ""
                )
            
            console.print(table)
            
            if len(needs_editing) > 20:
                console.print(f"[dim]... and {len(needs_editing) - 20} more tracks[/dim]")
            
            console.print()
            console.print("[bold]💡 Metadata Editing Options:[/bold]")
            console.print("1. Use the full PanPipe player: [cyan]python bang_tunes.py play[/cyan]")
            console.print("   (Interactive TUI with metadata editor)")
            console.print("2. Manual database editing with SQL tools")
            console.print("3. Re-download with better metadata sources")
            
        else:
            print(f"Found {len(needs_editing)} tracks that need metadata editing:")
            print()
            
            for i, track in enumerate(needs_editing[:20], 1):
                title = track.get("title", "Unknown")[:40]
                artist = track.get("artist", "Unknown")[:30]
                file_name = Path(track.get("file_path", "")).name[:30] if track.get("file_path") else ""
                print(f"{i:2d}. {title} | {artist} | {file_name}")
            
            if len(needs_editing) > 20:
                print(f"    ... and {len(needs_editing) - 20} more tracks")
            
            print()
            print("💡 Metadata Editing Options:")
            print("1. Use the full PanPipe player: python bang_tunes.py play")
            print("   (Interactive TUI with metadata editor)")
            print("2. Manual database editing with SQL tools")
            print("3. Re-download with better metadata sources")
        
        return 0
        
    except Exception as e:
        if console:
            console.print(f"[red]Edit-metadata failed: {e}[/red]")
        else:
            print(f"Edit-metadata failed: {e}")
        return 1


def cmd_rescan(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle rescan command - compare DB with disk and report mismatches."""
    try:
        if console:
            console.print("[bold]🔍 BangTunes Library Rescan[/bold]")
            console.print()
        else:
            print("🔍 BangTunes Library Rescan")
            print("=" * 30)
        
        # Get database connection
        db_path = root / "library.db"
        db_conn = get_db(db_path)
        
        # Get all tracks from database
        tracks = db_get_all_tracks(db_conn)
        
        if not tracks:
            if console:
                console.print("[yellow]No tracks found in database[/yellow]")
            else:
                print("No tracks found in database")
            return 0
        
        if console:
            console.print(f"[dim]Checking {len(tracks)} tracks in database...[/dim]")
        else:
            print(f"Checking {len(tracks)} tracks in database...")
        
        missing_files = []
        invalid_paths = []
        
        for track in tracks:
            file_path = track.get("file_path")
            if not file_path:
                invalid_paths.append(track)
                continue
            
            path_obj = Path(file_path)
            if not path_obj.exists():
                missing_files.append(track)
        
        # Report findings
        if console:
            console.print()
            if not missing_files and not invalid_paths:
                console.print("[green]✓ All tracks found on disk - library is in sync[/green]")
            else:
                if missing_files:
                    console.print(f"[red]✗ {len(missing_files)} tracks missing from disk:[/red]")
                    for track in missing_files[:10]:  # Show first 10
                        title = track.get("title", "Unknown")
                        artist = track.get("artist", "Unknown")
                        file_path = track.get("file_path", "")
                        console.print(f"  [dim]• {title} by {artist}[/dim]")
                        console.print(f"    [red]{file_path}[/red]")
                    
                    if len(missing_files) > 10:
                        console.print(f"    [dim]... and {len(missing_files) - 10} more[/dim]")
                
                if invalid_paths:
                    console.print(f"[yellow]⚠️  {len(invalid_paths)} tracks with invalid paths[/yellow]")
                
                # Offer to fix issues
                fix_requested = getattr(args, 'fix', False)
                if fix_requested:
                    if console:
                        console.print("\n[dim]Removing invalid entries from database...[/dim]")
                    else:
                        print("\nRemoving invalid entries from database...")
                    
                    # Remove missing files from database
                    cur = db_conn.cursor()
                    removed_count = 0
                    
                    for track in missing_files + invalid_paths:
                        track_id = track.get("id")
                        if track_id:
                            cur.execute("DELETE FROM tracks WHERE id = ?", (track_id,))
                            removed_count += 1
                    
                    db_conn.commit()
                    
                    if console:
                        console.print(f"[green]✓ Removed {removed_count} invalid entries[/green]")
                    else:
                        print(f"✓ Removed {removed_count} invalid entries")
                else:
                    if console:
                        console.print("\n[dim]💡 Run with --fix to remove invalid entries from database[/dim]")
                    else:
                        print("\n💡 Run with --fix to remove invalid entries from database")
        else:
            print()
            if not missing_files and not invalid_paths:
                print("✓ All tracks found on disk - library is in sync")
            else:
                if missing_files:
                    print(f"✗ {len(missing_files)} tracks missing from disk:")
                    for track in missing_files[:10]:
                        title = track.get("title", "Unknown")
                        artist = track.get("artist", "Unknown")
                        file_path = track.get("file_path", "")
                        print(f"  • {title} by {artist}")
                        print(f"    {file_path}")
                    
                    if len(missing_files) > 10:
                        print(f"    ... and {len(missing_files) - 10} more")
                
                if invalid_paths:
                    print(f"⚠️  {len(invalid_paths)} tracks with invalid paths")
                
                fix_requested = getattr(args, 'fix', False)
                if fix_requested:
                    print("\nRemoving invalid entries from database...")
                    
                    cur = db_conn.cursor()
                    removed_count = 0
                    
                    for track in missing_files + invalid_paths:
                        track_id = track.get("id")
                        if track_id:
                            cur.execute("DELETE FROM tracks WHERE id = ?", (track_id,))
                            removed_count += 1
                    
                    db_conn.commit()
                    print(f"✓ Removed {removed_count} invalid entries")
                else:
                    print("\n💡 Run with --fix to remove invalid entries from database")
        
        return 0 if not (missing_files or invalid_paths) else 1
        
    except Exception as e:
        if console:
            console.print(f"[red]Rescan failed: {e}[/red]")
        else:
            print(f"Rescan failed: {e}")
        return 1


def cmd_player(args: Any, root: Path, config: Dict[str, Any]) -> int:
    """Handle player commands (play, setup-player, sync, player-status)."""
    try:
        cmd = getattr(args, 'cmd', 'play')
        
        if cmd == 'play':
            if console:
                console.print("[bold]🎵 BangTunes Player[/bold]")
                console.print()
                console.print("[dim]Launching PanPipe interactive player...[/dim]")
            else:
                print("🎵 BangTunes Player")
                print("Launching PanPipe interactive player...")
            
            # Try to launch the integrated PanPipe player
            import subprocess
            import shutil
            
            # Look for the PanPipe binary
            panpipe_paths = [
                root / "target" / "release" / "panpipe_interactive",
                root / "panpipe_interactive",
                "panpipe_interactive"
            ]
            
            panpipe_binary = None
            for path in panpipe_paths:
                if isinstance(path, Path) and path.exists():
                    panpipe_binary = str(path)
                    break
                elif isinstance(path, str) and shutil.which(path):
                    panpipe_binary = path
                    break
            
            if panpipe_binary:
                try:
                    # Launch PanPipe with the music directory
                    music_dir = root / "downloads"
                    subprocess.run([panpipe_binary, str(music_dir)], check=False)
                    return 0
                except Exception as e:
                    if console:
                        console.print(f"[red]Failed to launch PanPipe: {e}[/red]")
                    else:
                        print(f"Failed to launch PanPipe: {e}")
            else:
                if console:
                    console.print("[yellow]PanPipe player not found[/yellow]")
                    console.print("[dim]Run setup to build the integrated player, or use quickplay for basic playback[/dim]")
                else:
                    print("PanPipe player not found")
                    print("Run setup to build the integrated player, or use quickplay for basic playback")
                return 1
        
        elif cmd == 'setup-player':
            if console:
                console.print("[bold]🔧 Player Setup[/bold]")
                console.print()
                console.print("[dim]Setting up PanPipe integration...[/dim]")
            else:
                print("🔧 Player Setup")
                print("Setting up PanPipe integration...")
            
            # Check if Rust is available
            import shutil
            if not shutil.which("cargo"):
                if console:
                    console.print("[red]✗ Rust/Cargo not found[/red]")
                    console.print("[dim]Install Rust from https://rustup.rs/[/dim]")
                else:
                    print("✗ Rust/Cargo not found")
                    print("Install Rust from https://rustup.rs/")
                return 1
            
            # Try to build PanPipe
            try:
                if console:
                    console.print("[dim]Building PanPipe player...[/dim]")
                else:
                    print("Building PanPipe player...")
                
                result = subprocess.run(
                    ["cargo", "build", "--release", "--bin", "panpipe_interactive"],
                    cwd=root,
                    capture_output=True,
                    text=True
                )
                
                if result.returncode == 0:
                    if console:
                        console.print("[green]✓ PanPipe player built successfully[/green]")
                    else:
                        print("✓ PanPipe player built successfully")
                    return 0
                else:
                    if console:
                        console.print(f"[red]✗ Build failed: {result.stderr}[/red]")
                    else:
                        print(f"✗ Build failed: {result.stderr}")
                    return 1
            
            except Exception as e:
                if console:
                    console.print(f"[red]Build error: {e}[/red]")
                else:
                    print(f"Build error: {e}")
                return 1
        
        elif cmd in ['sync', 'player-status']:
            if console:
                console.print(f"[yellow]{cmd.title()} command not yet implemented[/yellow]")
                console.print("[dim]This will integrate with the PanPipe player database[/dim]")
            else:
                print(f"{cmd.title()} command not yet implemented")
                print("This will integrate with the PanPipe player database")
            return 1
        
        else:
            if console:
                console.print(f"[red]Unknown player command: {cmd}[/red]")
            else:
                print(f"Unknown player command: {cmd}")
            return 1
        
    except Exception as e:
        if console:
            console.print(f"[red]Player command failed: {e}[/red]")
        else:
            print(f"Player command failed: {e}")
        return 1
