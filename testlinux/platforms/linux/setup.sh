#!/bin/bash
# Setup script for Linux development

set -e

echo "Setting up Linux development environment..."

# Check for Python 3
if ! command -v python3 &> /dev/null; then
    echo "Python 3 is required but not installed."
    echo "Install with: sudo apt install python3 python3-pip"
    exit 1
fi

# Install GTK 4 and dependencies
if ! pkg-config --exists gtk4; then
    echo "Installing GTK 4..."
    sudo apt install -y libgtk-4-dev libadwaita-1-dev python3-gi python3-gi-cairo gir1.2-gtk-4.0 gir1.2-adw-1
fi

# Install Python dependencies
echo "Installing Python dependencies..."
pip3 install --user -r requirements.txt

echo "Setup complete!"
