"""
Command-line interface for BangTunes.

Main CLI entry point and argument parsing.
"""

import argparse
import os
import sys
from typing import List, Optional

try:
    from rich.console import Console

    console: Optional[Console] = Console()
except ImportError:
    console = None

# Import all the modules we need
from bangtunes.env import get_root
from bangtunes.config import load_config
from bangtunes.db import db_init

# Try to import player integration
try:
    from bangtunes.player_integration import create_integration
    PLAYER_AVAILABLE = True
except ImportError:
    create_integration = None
    PLAYER_AVAILABLE = False

# Import CLI command implementations
from . import commands


def build_parser() -> argparse.ArgumentParser:
    """Build the argument parser for BangTunes CLI."""
    ap = argparse.ArgumentParser(
        prog="Bang Tunes",
        description="Music Discovery & Playback System (Termux/Linux)",
        epilog="""
Common workflows:
  build -> download -> stats/quickplay    Discover and download new music
  edit-metadata                          Fix track information
  rescan --fix                          Clean up library inconsistencies
  
Examples:
  python bang_tunes.py build --min-score 60
  python bang_tunes.py download mix_001.csv --speed fast
  python bang_tunes.py stats
        """,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    ap.add_argument("--no-banner", action="store_true", help="Hide ASCII banner")
    sub = ap.add_subparsers(dest="cmd", required=True)

    # Build command
    b = sub.add_parser("build", help="Build batches from seed.csv")
    b.add_argument("--prefix", default="mix", help="Batch prefix (default: mix)")
    b.add_argument(
        "--min-score",
        type=int,
        default=60,
        help="Minimum fuzzy match score (default: 60)",
    )
    b.add_argument(
        "--size", type=int, default=50, help="Tracks per batch file (default: 50)"
    )
    b.add_argument(
        "--cc-only", action="store_true", help="Search for Creative Commons tracks only"
    )

    # Download command
    d = sub.add_parser("download", help="Download a batch CSV to audio & add to DB")
    d.add_argument("batch_csv", help="Batch CSV file to download")
    d.add_argument(
        "--format",
        default="opus",
        choices=["opus", "mp3", "m4a"],
        help="Audio format (default: opus)",
    )
    d.add_argument(
        "--speed",
        default="normal",
        choices=["slow", "normal", "fast"],
        help="Download speed mode (default: normal)",
    )

    # Other commands
    sub.add_parser("view", help="Show quick library summary")
    sub.add_parser("list-batches", help="Show available batch CSVs and sizes")
    sub.add_parser(
        "install",
        help="First-run setup wizard - creates directories, checks deps, tests API",
    )
    sub.add_parser(
        "doctor",
        help="Comprehensive system health check - validates all dependencies and configuration",
    )
    sub.add_parser("stats", help="Show library statistics and gallery of results")
    sub.add_parser(
        "quickplay", help="Instant music playback (ffplay/termux) - no setup required"
    )
    sub.add_parser(
        "edit-metadata",
        help="Interactive metadata editor - search, view, and edit track information",
    )

    r = sub.add_parser("rescan", help="Compare DB with disk and report mismatches")
    r.add_argument(
        "--fix",
        action="store_true",
        help="Automatically fix orphan files and missing entries",
    )
    r.add_argument(
        "--yes",
        action="store_true",
        help="Skip confirmation prompts (non-interactive mode)",
    )

    # Integrated Player Commands (always visible; may be unavailable at runtime)
    sub.add_parser(
        "play",
        help="Launch BangTunes intelligent music player (TUI with smart shuffle, behavior tracking)",
    )
    sub.add_parser("setup-player", help="Setup BangTunes player integration")
    sub.add_parser("sync", help="Sync library with player database")
    sub.add_parser("player-status", help="Show player integration status")

    return ap


def main(argv: Optional[List[str]] = None) -> int:
    """Main CLI entry point."""
    # Get configuration
    root = get_root()
    config = load_config(root)

    # Set up paths
    batch_dir = root / "batches"
    dl_root = root / "downloads"
    db_path = root / "library.db"

    # Ensure directories exist
    root.mkdir(parents=True, exist_ok=True)
    batch_dir.mkdir(parents=True, exist_ok=True)
    dl_root.mkdir(parents=True, exist_ok=True)

    # Initialize database
    db_init(db_path)

    # Parse arguments
    parser = build_parser()
    args = parser.parse_args(argv)

    # Print banner unless suppressed
    if not args.no_banner:
        commands.print_banner()

    # Dispatch to command handlers
    try:
        if args.cmd == "build":
            return commands.cmd_build(args, root, config)
        elif args.cmd == "download":
            return commands.cmd_download(args, root, config)
        elif args.cmd == "view":
            return commands.cmd_view(args, root, config)
        elif args.cmd == "list-batches":
            return commands.cmd_list_batches(args, root, config)
        elif args.cmd == "install":
            return commands.cmd_install(args, root, config)
        elif args.cmd == "doctor":
            return commands.cmd_doctor(args, root, config)
        elif args.cmd == "stats":
            return commands.cmd_stats(args, root, config)
        elif args.cmd == "quickplay":
            return commands.cmd_quickplay(args, root, config)
        elif args.cmd == "edit-metadata":
            return commands.cmd_edit_metadata(args, root, config)
        elif args.cmd == "rescan":
            return commands.cmd_rescan(args, root, config)
        elif args.cmd in {"play", "setup-player", "sync", "player-status"}:
            return commands.cmd_player(args, root, config)
        else:
            if console:
                console.print(f"[red]Unknown command: {args.cmd}[/red]")
            else:
                print(f"Unknown command: {args.cmd}")
            return 1

    except KeyboardInterrupt:
        if console:
            console.print("\n[yellow]Interrupted by user[/yellow]")
        else:
            print("\nInterrupted by user")
        return 130
    except Exception as e:
        debug_mode = os.getenv("BANGTUNES_DEBUG", "false").lower() in (
            "true",
            "1",
            "yes",
        )
        if debug_mode:
            raise
        if console:
            console.print(f"[red]❌ Unexpected error: {e}[/red]")
            console.print(
                "[dim]💡 Run with BANGTUNES_DEBUG=1 for detailed error info[/dim]"
            )
            console.print(
                "[dim]🐛 If this persists, please report it as an issue[/dim]"
            )
        else:
            print(f"❌ Unexpected error: {e}")
            print("💡 Run with BANGTUNES_DEBUG=1 for detailed error info")
            print("🐛 If this persists, please report it as an issue")
        return 1


if __name__ == "__main__":
    sys.exit(main())
