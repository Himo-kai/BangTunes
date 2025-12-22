#!/bin/bash
# Bang Tunes Setup Script
# Works on Unix-like systems (limited testing) (tested on Termux and Arch Linux)
# Run this and it handles the rest
#
# Copyright (c) 2024 BangTunes Contributors
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.

set -e  # Exit on any error

echo "Bang Tunes Setup"
echo "================="

# Check if we're in Termux and recommend the specialized script
if [[ "$PREFIX" == *"com.termux"* ]]; then
    echo "🤖 Termux/Android detected!"
    echo
    echo "For the best Termux experience, use the specialized setup script:"
    echo "   ./termux-setup.sh"
    echo
    echo "This script will continue with generic setup, but termux-setup.sh"
    echo "handles Android-specific dependencies and optimizations better."
    echo
    read -p "Continue with generic setup? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Run: ./termux-setup.sh"
        exit 0
    fi
fi

echo "Setting up music discovery tools..."
echo ""

# Detect environment
if command -v termux-setup-storage &> /dev/null; then
    echo "✅ Detected Termux (Android)"
    ENV="termux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "✅ Detected macOS"
    ENV="macos"
elif [[ "$OSTYPE" == "linux-gnu"* ]] && grep -q Microsoft /proc/version 2>/dev/null; then
    echo "✅ Detected WSL (Windows Subsystem for Linux)"
    ENV="wsl"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "✅ Detected Linux"
    ENV="linux"
else
    echo "⚠️  Unknown environment: $OSTYPE"
    echo "Proceeding with generic Linux setup..."
    ENV="linux"
fi

echo ""

# Check Python version
echo "Checking Python installation..."
if ! command -v python3 &> /dev/null && ! command -v python &> /dev/null; then
    echo "ERROR: Python not found! Please install Python 3.8+ first."
    exit 1
fi

# Figure out which python command to use
if command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
else
    PYTHON_CMD="python"  # fallback for some systems
fi

# Check Python version
PYTHON_VERSION=$($PYTHON_CMD -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
echo "Found Python $PYTHON_VERSION"

# Proper version comparison using Python itself
VERSION_CHECK=$($PYTHON_CMD -c "import sys; print('OK' if sys.version_info >= (3, 8) else 'BAD')")
if [[ "$VERSION_CHECK" != "OK" ]]; then
    echo "ERROR: Python 3.8+ required, found $PYTHON_VERSION"
    exit 1
fi

# Set up a virtual environment to keep things clean
if [ ! -d "venv" ]; then
    echo "Creating Python virtual environment..."
    $PYTHON_CMD -m venv venv
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to create virtual environment"
        echo "Try: sudo apt install python3-venv (Ubuntu/Debian)"
        echo "Try: brew install python (macOS)"
        exit 1
    fi
else
    echo "Virtual environment already exists"
fi

# Activate virtual environment and install dependencies
echo "Installing Python dependencies..."
if [ -f "./venv/bin/pip" ]; then
    PIP_CMD="./venv/bin/pip"
else
    PIP_CMD="./venv/Scripts/pip"  # Windows/WSL compatibility
fi

$PIP_CMD install --upgrade pip
$PIP_CMD install -r requirements.txt

if [ $? -ne 0 ]; then
    echo "ERROR: Failed to install dependencies"
    echo "Check your internet connection and try again"
    exit 1
fi

# Make bang_tunes.py executable
chmod +x bang_tunes.py 2>/dev/null || true  # Ignore errors on Windows

# Create directories
echo "Creating project directories..."
mkdir -p batches downloads

# Check for MPV (required for advanced player backend)
echo "Checking for MPV media player..."
if command -v mpv &> /dev/null; then
    echo "MPV found - advanced player backend available!"
else
    echo "MPV not found - installing for advanced player features..."
    if [[ "$ENV" == "linux" ]]; then
        # Try to detect package manager and install MPV
        if command -v pacman &> /dev/null; then
            echo "Arch Linux detected - installing MPV..."
            sudo pacman -S --noconfirm mpv || echo "Failed to install MPV automatically. Run: sudo pacman -S mpv"
        elif command -v apt &> /dev/null; then
            echo "Debian/Ubuntu detected - installing MPV..."
            sudo apt update && sudo apt install -y mpv || echo "Failed to install MPV automatically. Run: sudo apt install mpv"
        elif command -v dnf &> /dev/null; then
            echo "Fedora detected - installing MPV..."
            sudo dnf install -y mpv || echo "Failed to install MPV automatically. Run: sudo dnf install mpv"
        else
            echo "Unknown Linux distribution. Please install MPV manually:"
            echo "  Arch: sudo pacman -S mpv"
            echo "  Ubuntu/Debian: sudo apt install mpv"
            echo "  Fedora: sudo dnf install mpv"
        fi
    elif [[ "$ENV" == "macos" ]]; then
        echo "macOS detected - installing MPV via Homebrew..."
        brew install mpv || echo "Failed to install MPV. Run: brew install mpv"
    else
        echo "Please install MPV manually for advanced player features"
    fi
fi

# Build integrated player (if Rust is available)
echo "Building integrated music player..."
if command -v cargo &> /dev/null; then
    echo "Rust detected - building player for extra features..."
    cargo build --release
    if [ $? -eq 0 ]; then
        echo "Player built successfully!"
    else
        echo "Warning: Player build failed. Extra player features will be unavailable."
        echo "Basic playback via 'quickplay' command will still work."
    fi
else
    echo "Rust not found - skipping player build."
    echo "Install Rust from https://rustup.rs/ for extra player features."
    echo "Basic playback via 'quickplay' command will still work."
fi

# Initialize seed file if it doesn't exist
if [ ! -f "seed.csv" ]; then
    echo "Creating example seed.csv..."
    cat > seed.csv << 'EOF'
title,artist
Parasite Eve,Bring Me The Horizon
Sleep Token,Take Me Back To Eden
Starset,Monster
Architects,Doomsday
Bad Omens,Just Pretend
EOF
fi

echo ""
echo "Setup complete!"
echo ""
echo "Quick start:"
echo "   1. ./venv/bin/python bang_tunes.py build"
echo "   2. ./venv/bin/python bang_tunes.py download batches/mix_001.csv"
echo "   3. ./venv/bin/python bang_tunes.py view"
echo ""
echo "Full documentation: Check the README.md file"
echo "Issues or questions: Open an issue on GitHub"

if [ "$ENV" = "termux" ]; then
    echo ""
    echo "Termux (Android) Notes:"
    echo "  - Run 'termux-setup-storage' for shared storage access"
    echo "  - Install audio tools: pkg install ffmpeg"
    echo "  - Music files will be in ~/BangTunes/downloads/"
    echo "  - Termux uses internal storage, not SD card"
    echo "  - Access shared storage via ~/storage/shared/"
fi
