#!/data/data/com.termux/files/usr/bin/bash

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

echo "✓ Termux environment detected"

# Step 1: Update packages and install system dependencies
echo
echo "📦 Installing system dependencies..."
echo "   This may take a few minutes..."

# Update package lists
pkg update -y

# Install required system packages
pkg install -y python ffmpeg git rust

echo "✓ System packages installed"

# Step 2: Set up Python environment
echo
echo "🐍 Setting up Python environment..."

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    python -m venv venv
    echo "✓ Virtual environment created"
else
    echo "✓ Virtual environment already exists"
fi

# Activate virtual environment
source venv/bin/activate

# Upgrade pip to latest version
pip install --upgrade pip

# Install all Python dependencies at once
echo "📚 Installing Python packages..."
pip install -r requirements.txt

echo "✓ Python dependencies installed"

# Step 3: Create directory structure
echo
echo "📁 Creating project structure..."
mkdir -p batches downloads
echo "✓ Directory structure created"

# Step 4: Build Rust components (if possible)
echo
echo "🦀 Building Rust components..."
if command -v cargo >/dev/null 2>&1; then
    echo "   Building PanPipe player..."
    if cargo build --release --bin panpipe_interactive; then
        echo "✓ Rust player built successfully"
        RUST_AVAILABLE=true
    else
        echo "⚠️  Rust build failed - advanced player unavailable"
        echo "   Basic playback will still work with ffmpeg"
        RUST_AVAILABLE=false
    fi
else
    echo "⚠️  Rust not available - advanced player unavailable"
    echo "   Basic playback will still work with ffmpeg"
    RUST_AVAILABLE=false
fi

# Step 5: Test the installation
echo
echo "🧪 Testing installation..."

# Test basic functionality
if python bang_tunes.py --help >/dev/null 2>&1; then
    echo "✓ BangTunes CLI working"
else
    echo "❌ BangTunes CLI test failed"
    exit 1
fi

# Test YouTube Music connection
echo "   Testing YouTube Music connection..."
if python bang_tunes.py build --dry-run >/dev/null 2>&1; then
    echo "✓ YouTube Music API working"
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
