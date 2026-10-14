#!/usr/bin/env bash
# Headless Maybraid captures for PR #1151 (inventory D-pad categories).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/maybraid"
OUT="${1:-$ROOT/packaging/walkthrough/pr-1151-inventory-dpad}"
mkdir -p "$OUT"

export CXX="${CXX:-g++-13}"
export CC="${CC:-gcc-13}"
export LIBRARY_PATH="${LIBRARY_PATH:-/usr/lib/gcc/x86_64-linux-gnu/13}"

if [[ ! -x "$BIN" ]]; then
  echo "Building debug maybraid..."
  (cd "$ROOT" && cargo build -p maybraid)
fi

sudo modprobe uinput 2>/dev/null || true
if [[ ! -e /dev/uinput ]]; then
  echo "missing /dev/uinput — load the uinput module"
  exit 1
fi
sudo chmod 666 /dev/uinput 2>/dev/null || true

GAME_LOG=/tmp/maybraid-preview-1151.log
rm -f "$GAME_LOG"

xvfb-run -a -s "-screen 0 1280x720x24" \
  env -u WAYLAND_DISPLAY \
  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  WGPU_BACKEND=vk \
  MAYBRAID_START_AT=0,0 \
  OUT="$OUT" \
  BIN="$BIN" \
  GAME_LOG="$GAME_LOG" \
  bash -s <<'INNER'
set -euo pipefail

python3 <<'PY' &
import os
import signal
import time
from evdev import UInput, AbsInfo, ecodes

caps = {
    ecodes.EV_KEY: [ecodes.BTN_SOUTH, ecodes.BTN_START, ecodes.BTN_SELECT],
    ecodes.EV_ABS: [
        (ecodes.ABS_HAT0X, AbsInfo(0, -1, 1, 0, 0, 0)),
        (ecodes.ABS_HAT0Y, AbsInfo(0, -1, 1, 0, 0, 0)),
    ],
}
ui = UInput(caps, name="Maybraid Preview Gamepad", vendor=0x045e, product=0x028e)

def cleanup(*_):
    ui.close()
    raise SystemExit(0)

signal.signal(signal.SIGTERM, cleanup)
while True:
    time.sleep(3600)
PY
sleep 0.3

"$BIN" --start-at 0,0 >"$GAME_LOG" 2>&1 &
GAME_PID=$!
cleanup() { kill "$GAME_PID" 2>/dev/null || true; wait "$GAME_PID" 2>/dev/null || true; }
trap cleanup EXIT

echo "DISPLAY=$DISPLAY — waiting for Maybraid window..."
WID=""
for _ in $(seq 1 120); do
  WID=$(xdotool search --name 'Maybraid' 2>/dev/null | head -1 || true)
  if [[ -n "$WID" ]]; then
    break
  fi
  sleep 1
done
if [[ -z "$WID" ]]; then
  echo "No window; xwininfo -root -tree head:"
  xwininfo -root -tree 2>/dev/null | head -25 || true
  echo "log tail:"
  tail -40 "$GAME_LOG"
  exit 1
fi
xdotool windowactivate --sync "$WID"
sleep 1
xdotool key --window "$WID" Return
sleep 2
xdotool key --window "$WID" Return

shot() { import -window "$WID" "$OUT/$1.png"; }

# Fixed settle after title flow; category cycling no longer shows a HUD text cue.
sleep 8
rm -f "$OUT/_probe.png"

python3 - "$WID" "$OUT" <<'PY'
import subprocess
import sys
import time
from evdev import UInput, AbsInfo, ecodes

wid, out = sys.argv[1], sys.argv[2]
ui = UInput(
    {
        ecodes.EV_ABS: [
            (ecodes.ABS_HAT0X, AbsInfo(0, -1, 1, 0, 0, 0)),
            (ecodes.ABS_HAT0Y, AbsInfo(0, -1, 1, 0, 0, 0)),
        ],
    },
    name="Maybraid Preview Hat",
    vendor=0x045e,
    product=0x028e,
)

def shot(name):
    subprocess.run(["import", "-window", wid, f"{out}/{name}.png"], check=True)

def hat(x, y):
    ui.write(ecodes.EV_ABS, ecodes.ABS_HAT0X, x)
    ui.write(ecodes.EV_ABS, ecodes.ABS_HAT0Y, y)
    ui.syn()
    time.sleep(0.15)
    ui.write(ecodes.EV_ABS, ecodes.ABS_HAT0X, 0)
    ui.write(ecodes.EV_ABS, ecodes.ABS_HAT0Y, 0)
    ui.syn()
    time.sleep(0.8)

time.sleep(1)
shot("01-cycle-weapons")
hat(0, 1)
shot("02-cycle-skill-maps")
hat(-1, 0)
shot("03-cycle-lubikans")
subprocess.run(["xdotool", "key", "--window", wid, "m"], check=True)
time.sleep(1.5)
shot("04-map-open-m-key")
ui.close()
PY

echo "Wrote captures to $OUT"
md5sum "$OUT"/*.png
INNER
