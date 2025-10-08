#!/bin/bash

set -e

echo "❄️  ICY Installer"
echo "================="
echo ""

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "Error: This script must be run as root (use sudo)"
   exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "✓ Rust toolchain found"

# Check for required system tools
echo ""
echo "Checking system dependencies..."

MISSING_DEPS=0

if ! command -v btrfs &> /dev/null && ! command -v lvm &> /dev/null; then
    echo "⚠️  Warning: Neither btrfs-progs nor lvm2 found"
    echo "   Install at least one:"
    echo "   - Debian/Ubuntu: apt install btrfs-progs lvm2"
    echo "   - Fedora: dnf install btrfs-progs lvm2"
    echo "   - Arch: pacman -S btrfs-progs lvm2"
    MISSING_DEPS=1
fi

if [[ $MISSING_DEPS -eq 1 ]]; then
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Build the project
echo ""
echo "Building ICY (release mode)..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi

echo "✓ Build successful"

# Install binary
echo ""
echo "Installing binary to /usr/local/bin/icy..."
cp target/release/icy /usr/local/bin/icy
chmod +x /usr/local/bin/icy

echo "✓ Binary installed"

# Create directories
echo ""
echo "Creating configuration directories..."
mkdir -p /etc/icy/configs
mkdir -p /var/log

echo "✓ Directories created"

# Create default configuration
if [ ! -f /etc/icy/configs/root.yaml ]; then
    echo ""
    read -p "Create default 'root' configuration? (Y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        cat > /etc/icy/configs/root.yaml << EOF
name: root
path: /
snapshot_dir: /.icy-snapshots/root
retention:
  hourly: 0
  daily: 7
  weekly: 4
  monthly: 3
fs_type: auto
EOF
        mkdir -p /.icy-snapshots/root
        echo "✓ Default configuration created"
    fi
fi

# Success message
echo ""
echo "═══════════════════════════════════════"
echo "✓ ICY installed successfully!"
echo "═══════════════════════════════════════"
echo ""
echo "Quick start:"
echo "  1. Run TUI:  sudo icy"
echo "  2. Create config:  sudo icy init <name> <path>"
echo "  3. Create snapshot:  sudo icy create --config <name>"
echo ""
echo "For help:  icy --help"
echo "Documentation:  cat README.md"
echo ""