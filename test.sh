#!/bin/bash
# Quick test: build and run daemon + popup locally.
# Your user must be in the 'input' group.
# Ctrl+C to stop both.
set -e

cd "$(dirname "$0")"
cargo build --release 2>&1

export ACCENTD_SOCK="/tmp/accentd-test.sock"
sudo rm -f "$ACCENTD_SOCK"

cleanup() {
    echo "Stopping..."
    kill "$POPUP_PID" 2>/dev/null || true
    sudo kill "$DAEMON_PID" 2>/dev/null || true
    sudo rm -f "$ACCENTD_SOCK"
}
trap cleanup EXIT INT TERM

# Kill any leftover test processes from previous runs
pkill -f "target/release/accentd-popup" 2>/dev/null || true
sudo pkill -f "target/release/accentd " 2>/dev/null || true
sleep 0.2

echo "Starting daemon..."
sudo -E target/release/accentd &
DAEMON_PID=$!
sleep 0.5

sudo chmod 666 "$ACCENTD_SOCK"

echo "Starting popup (as $USER)..."
RUST_LOG=accentd_popup=debug target/release/accentd-popup &
POPUP_PID=$!

echo "Running. Hold a vowel key for 300ms to test. Ctrl+C to stop."
wait
