#!/bin/bash
set -e

command -v cargo &>/dev/null || { echo "error: install rust from https://rustup.rs"; exit 1; }
command -v tmux &>/dev/null || { echo "error: install tmux"; exit 1; }
command -v git &>/dev/null || { echo "error: install git"; exit 1; }

cargo install --path .

# Create 'ws' shortcut
CARGO_BIN="$(dirname "$(which witshe)")"
ln -sf "$CARGO_BIN/witshe" "$CARGO_BIN/ws"

echo "installed: witshe (alias: ws)"
