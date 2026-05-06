#!/data/data/com.termux/files/usr/bin/bash

# BANGTUNES_LOCALE_GUARD
# Try to force a UTF-8 locale so ✓/✗ render correctly.
# You can override manually before launching if needed.
export LANG="${LANG:-en_US.UTF-8}"
export LC_ALL="${LC_ALL:-en_US.UTF-8}"


# BANGTUNES_SYMBOLS_HELPER
# Use Unicode symbols only when the terminal is UTF-8 (or when forced).
# Force ASCII fallback with: BANGTUNES_ASCII=1
_is_utf8() {
  # locale charmap is the most reliable; fall back to LANG/LC_ALL heuristics
  if command -v locale >/dev/null 2>&1; then
    locale charmap 2>/dev/null | grep -qi 'utf-8' && return 0
  fi
  echo "${LC_ALL:-${LANG:-}}" | grep -qi 'utf-8'
}

sym_ok()  { if [ "${BANGTUNES_ASCII:-0}" = "1" ]; then printf '%s' '[OK]';  elif _is_utf8; then printf '%s' '✓'; else printf '%s' '[OK]';  fi; }
sym_bad() { if [ "${BANGTUNES_ASCII:-0}" = "1" ]; then printf '%s' '[X]';   elif _is_utf8; then printf '%s' '✗'; else printf '%s' '[X]';   fi; }


# BangTunes Termux Setup Script
# Single command setup for Android/Termux users

set -e  # Exit on any error

echo "🎵 BangTunes Termux Setup"
echo "========================="
echo "Setting up complete music discovery environment for Android..."
echo

# Check if we're actually in Termux
if [[ ! "$PREFIX" == *"com.termux"* ]]; then
    echo "❌ This script is designed for Termux on Android"
    echo "   For other platforms, use: ./setup.sh"
    exit 1
fi

echo "$(sym_ok) Termux environment detected"

# Step 1: Update packages and install system dependencies
echo
echo "📦 Installing system dependencies..."
echo "   This may take a few minutes..."

# Update package lists
pkg update -y

# Prevent partial-upgrade Rust toolchain mismatches (common Termux failure mode)
echo "   Upgrading existing packages to prevent version mismatches..."
pkg upgrade -y || true

# Install required system packages
pkg install -y python ffmpeg git rust mpv

echo "$(sym_ok) System packages installed"

# Step 2: Set up Python environment
echo
echo "🐍 Setting up Python environment..."

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    python -m venv venv
    echo "$(sym_ok) Virtual environment created"
else
    echo "$(sym_ok) Virtual environment already exists"
fi

# Activate virtual environment
source venv/bin/activate

# Upgrade pip to latest version
pip install --upgrade pip

# Install all Python dependencies at once
echo "📚 Installing Python packages..."
pip install -r requirements.txt

echo "$(sym_ok) Python dependencies installed"

# Step 3: Create directory structure
echo
echo "📁 Creating project structure..."
mkdir -p batches downloads
echo "$(sym_ok) Directory structure created"

# Step 4: Build Rust components using specialized Termux setup
echo
echo "🦀 Building Rust components (headless mode - no audio backend)..."
if command -v cargo >/dev/null 2>&1; then
    echo "   Using Termux-optimized build (TUI only, audio via quickplay)..."
    # Clear inherited desktop env safely (unset, don't blank)
    unset RUSTFLAGS
    unset CARGO_BUILD_TARGET
    unset CARGO_TARGET_DIR
    
    # Build with termux feature (no rodio/ALSA dependency)
    if cargo build --release --bin panpipe_interactive \
        --no-default-features --features termux 2>&1 | tail -5; then
        echo "$(sym_ok) Rust TUI player built successfully (headless mode)"
        echo "   Library management, playlists, metadata editing available"
        echo "   Audio playback: use 'python bang_tunes.py quickplay'"
        RUST_AVAILABLE=true
    else
        echo "⚠️  Rust build failed - TUI player unavailable"
        echo "   All features still work via CLI commands"
        RUST_AVAILABLE=false
    fi
else
    echo "⚠️  Rust not available - TUI player unavailable"
    echo "   Basic playback will still work with mpv/termux-media-player"
    RUST_AVAILABLE=false
fi

# Step 5: Test the installation
echo
echo "🧪 Testing installation..."

# Test MPV installation
if command -v mpv >/dev/null 2>&1; then
    echo "$(sym_ok) MPV media player available"
else
    echo "⚠️  MPV not found - advanced player backend unavailable"
fi

# Test basic functionality
if python bang_tunes.py --help >/dev/null 2>&1; then
    echo "$(sym_ok) BangTunes CLI working"
else
    echo "❌ BangTunes CLI test failed"
    exit 1
fi

# Test YouTube Music connection
echo "   Testing YouTube Music connection..."
if python bang_tunes.py build --dry-run >/dev/null 2>&1; then
    echo "$(sym_ok) YouTube Music API working"
else
    echo "⚠️  YouTube Music API test failed - check internet connection"
fi

# Step 6: Setup complete
echo
echo "🎉 Setup Complete!"
echo "=================="
echo
echo "BangTunes is ready to use! Here's what you can do:"
echo
echo "📖 Quick Start Commands:"
echo "   python bang_tunes.py build              # Create discovery batches"
echo "   python bang_tunes.py download mix_001.csv  # Download first batch"
echo "   python bang_tunes.py stats              # View library statistics"
echo "   python bang_tunes.py quickplay          # Play music instantly"

if [ "$RUST_AVAILABLE" = true ]; then
    echo "   python bang_tunes.py play               # Advanced TUI player"
fi

echo
echo "📱 Termux-Specific Notes:"
echo "   • Music stored in: ~/BangTunes/downloads/"
echo "   • For shared storage access: termux-setup-storage"
echo "   • Files accessible via: ~/storage/shared/"
echo "   • Uses termux-media-player for audio playback"
echo
echo "🎵 Ready to discover new music!"
echo "   Edit seed.csv to customize your taste, then run 'python bang_tunes.py build'"
