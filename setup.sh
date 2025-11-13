#!/bin/bash
# Bang Tunes Setup Script
# Works for both Termux and Arch Linux

echo "🎵 Bang Tunes Setup 🎵"
echo "======================"

# Detect environment
if command -v termux-setup-storage &> /dev/null; then
    echo "Detected Termux environment"
    TERMUX=true
else
    echo "Detected Linux environment"
    TERMUX=false
fi

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    echo "Creating Python virtual environment..."
    python -m venv venv
fi

# Activate virtual environment and install dependencies
echo "Installing Python dependencies..."
./venv/bin/pip install -r requirements.txt

# Make bang_tunes.py executable
chmod +x bang_tunes.py

# Create directories
mkdir -p batches downloads

echo ""
echo "Setup complete!"
echo ""
echo "Quick Start:"
echo "  1. Edit seed.csv with your musical preferences"
echo "  2. Build batches: ./venv/bin/python bang_tunes.py build"
echo "  3. Download music: ./venv/bin/python bang_tunes.py download mix_001.csv"
echo "  4. View library: ./venv/bin/python bang_tunes.py view"
echo ""
echo "For detailed usage, see README.md"

if [ "$TERMUX" = true ]; then
    echo ""
    echo "Termux Notes:"
    echo "  - Run 'termux-setup-storage' for file access"
    echo "  - Install ffmpeg: pkg install ffmpeg"
    echo "  - Music files will be in ~/Builds/BangTunes/downloads/"
fi
