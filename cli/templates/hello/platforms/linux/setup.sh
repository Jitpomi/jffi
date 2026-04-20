#!/bin/bash
# Setup script for Linux development

set -e

echo "Setting up Linux development environment..."

# Install build essentials (required for Rust compilation)
if ! command -v gcc &> /dev/null; then
    echo "Installing build essentials..."
    sudo apt update
    sudo apt install -y build-essential pkg-config
fi

# Check for Python 3
if ! command -v python3 &> /dev/null; then
    echo "Installing Python 3..."
    sudo apt install -y python3 python3-pip
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
