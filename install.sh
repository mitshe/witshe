#!/bin/bash
set -e

echo "Installing witshe..."

# Check dependencies
if ! command -v cargo &> /dev/null; then
    echo "error: rust/cargo not found. Install from https://rustup.rs"
    exit 1
fi

if ! command -v tmux &> /dev/null; then
    echo "error: tmux not found. Install with: brew install tmux (macOS) or apt install tmux (Linux)"
    exit 1
fi

if ! command -v git &> /dev/null; then
    echo "error: git not found."
    exit 1
fi

# Build and install
cargo install --path .

echo ""
echo "witshe installed! Run 'witshe' to get started."
