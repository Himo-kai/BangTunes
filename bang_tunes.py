#!/usr/bin/env python3
"""
Bang Tunes - Music Discovery & Playback System

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

Started this in my truck during downtime between jobs.
Turned into something actually useful for finding new music.
Integrates with my terminal music player setup.

This is now a thin wrapper around the modular bangtunes package.
"""

import sys
import os


def check_dependencies() -> None:
    """Check if required dependencies are installed."""
    missing = []

    try:
        import rapidfuzz  # noqa: F401
    except ImportError:
        missing.append("rapidfuzz")

    try:
        import ytmusicapi  # noqa: F401
    except ImportError:
        missing.append("ytmusicapi")

    try:
        import mutagen  # noqa: F401
    except ImportError:
        missing.append("mutagen")

    try:
        import rich  # noqa: F401
    except ImportError:
        missing.append("rich")

    if missing:
        print("❌ Missing required dependencies:")
        for dep in missing:
            print(f"   - {dep}")
        print()

        # Check if we're in Termux
        if os.environ.get("TERMUX_VERSION") or "com.termux" in os.environ.get(
            "HOME", ""
        ):
            print("🤖 Termux detected! Run the automated setup:")
            print("   ./termux-setup.sh")
            print()
            print("Or install manually:")
            print("   pip install -r requirements.txt")
        else:
            print("💡 Install missing dependencies:")
            print("   pip install -r requirements.txt")
            print()
            print("Or run the setup script:")
            print("   ./setup.sh")

        sys.exit(1)


def main() -> int:
    """Main entry point - delegates to bangtunes.cli module."""
    try:
        # Import and delegate to the modular CLI
        from bangtunes.cli import main as cli_main

        return cli_main()

    except ImportError as e:
        # If bangtunes package not available, check dependencies
        if "bangtunes" in str(e):
            print("❌ BangTunes package not found")
            print("💡 Make sure you're in the BangTunes directory")
            print("   and all dependencies are installed")
            return 1
        else:
            # Missing dependency
            check_dependencies()
            return 1

    except KeyboardInterrupt:
        try:
            from rich.console import Console

            Console().print("\n[yellow]⚠️  Operation cancelled by user[/yellow]")
        except ImportError:
            print("\n⚠️  Operation cancelled by user")
        return 130

    except Exception as e:
        debug_mode = os.getenv("BANGTUNES_DEBUG", "false").lower() in (
            "true",
            "1",
            "yes",
        )

        if debug_mode:
            import traceback

            print("Full traceback:")
            traceback.print_exc()
        else:
            try:
                from rich.console import Console

                console = Console()
                console.print(f"[red]❌ Unexpected error: {e}[/red]")
                console.print(
                    "[dim]💡 Run with BANGTUNES_DEBUG=1 for detailed error info[/dim]"
                )
                console.print(
                    "[dim]🐛 If this persists, please report it as an issue[/dim]"
                )
            except ImportError:
                print(f"❌ Unexpected error: {e}")
                print("💡 Run with BANGTUNES_DEBUG=1 for detailed error info")
                print("🐛 If this persists, please report it as an issue")

        return 1


if __name__ == "__main__":
    sys.exit(main())
