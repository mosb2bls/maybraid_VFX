#!/usr/bin/env bash
# Headless Lubikan capture script: drive debug commands and capture 7 required states.
# Usage: xvfb-run -a -s "-screen 0 1280x720x24" VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vk ./capture-lubikan-states.sh

set -euo pipefail

WORLD_BIN="target/release/maybraid-world"
CAPTURE_DIR="lubikan-captures"
WINDOW_ID=""

if [ ! -f "$WORLD_BIN" ]; then
    echo "Error: $WORLD_BIN not found. Build with: cargo build --release -p maybraid-world --features debug-commands"
    exit 1
fi

mkdir -p "$CAPTURE_DIR"

# Helper: capture screenshot
capture() {
    local name="$1"
    local png="$CAPTURE_DIR/${name}.png"
    sleep 0.5  # Let render settle
    DISPLAY="${DISPLAY:-:99}" scrot "$png"
    echo "Captured: $png"
}

# Helper: send command via xdotool
send_command() {
    local cmd="$1"
    if [ -z "$WINDOW_ID" ]; then
        # Find the world window
        sleep 1
        WINDOW_ID=$(DISPLAY="${DISPLAY:-:99}" xdotool search --name "maybraid" | head -1)
        if [ -z "$WINDOW_ID" ]; then
            echo "Error: Could not find maybraid window"
            exit 1
        fi
        echo "Found window: $WINDOW_ID"
    fi
    DISPLAY="${DISPLAY:-:99}" xdotool windowactivate --sync "$WINDOW_ID"
    sleep 0.2
    # Open command console (F1)
    DISPLAY="${DISPLAY:-:99}" xdotool key F1
    sleep 0.3
    # Type command
    DISPLAY="${DISPLAY:-:99}" xdotool type "$cmd"
    sleep 0.2
    # Execute (Enter)
    DISPLAY="${DISPLAY:-:99}" xdotool key Return
    sleep 0.5
    # Close console (F1)
    DISPLAY="${DISPLAY:-:99}" xdotool key F1
    sleep 0.3
}

# Start world app in background
echo "Starting maybraid-world..."
"$WORLD_BIN" --debug-chrome &
WORLD_PID=$!
echo "World PID: $WORLD_PID"

# Wait for window to appear
echo "Waiting for window..."
sleep 8

# State 1: Partial charge above health
echo "State 1: Partial charge above health"
send_command "debug grant-points 15"
capture "01-partial-charge"

# State 2: Point flyer (grant more points to trigger flyer)
echo "State 2: Point flyer"
send_command "debug grant-points 5"
sleep 0.3
capture "02-point-flyer"

# State 3: Ready state (full charge)
echo "State 3: Ready (enough charge to activate)"
send_command "debug grant-points 10"
capture "03-ready"

# State 4: Take damage to enable healing cap
echo "Taking damage for heal cap..."
send_command "debug take-damage 40"
sleep 0.5

# Activate Self (X + fire, or via intent - we'll use command if available)
# For now, assume user manually activates or we need to trigger via gamepad
echo "State 4: Mid-heal with pulse and preview"
echo "TODO: Activate Self Lubikan via PowerUseItem intent"
# Placeholder: wait for manual activation or implement activation command
sleep 2
capture "04-mid-heal"

# State 5: Under 70% cap marker (should be healing toward 70%)
echo "State 5: Under 70% cap marker"
sleep 3
capture "05-under-cap"

# State 6: Completion flash (wait for heal to finish)
echo "State 6: Completion flash"
sleep 15
capture "06-completion-flash"

# State 7: Paused
echo "State 7: Paused"
DISPLAY="${DISPLAY:-:99}" xdotool key Escape
sleep 0.5
capture "07-paused"

# Cleanup
echo "Stopping world app..."
kill "$WORLD_PID" || true
wait "$WORLD_PID" 2>/dev/null || true

echo "Done! Captures in $CAPTURE_DIR/"
ls -lh "$CAPTURE_DIR"

# Verify distinct md5s
echo ""
echo "MD5 checksums:"
cd "$CAPTURE_DIR" && md5sum *.png

