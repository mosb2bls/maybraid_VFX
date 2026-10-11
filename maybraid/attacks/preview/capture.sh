#!/usr/bin/env bash
# Software-Vulkan capture of attacks-playground. Run under xvfb-run.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PREVIEW="$(cd "$(dirname "$0")" && pwd)"
STATE="${ATTACKS_PREVIEW_STATE:-/tmp/attacks-preview-state}"
BIN="${ATTACKS_PLAYGROUND_BIN:-$ROOT/target/debug/attacks-playground}"

export WGPU_BACKEND=vulkan
export VK_ICD_FILENAMES="${VK_ICD_FILENAMES:-/usr/share/vulkan/icd.d/lvp_icd.json}"
export VK_DRIVER_FILES="${VK_DRIVER_FILES:-$VK_ICD_FILENAMES}"
export WINIT_UNIX_BACKEND=x11
export WAYLAND_DISPLAY=
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/attacks-xdg-runtime}"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

rm -f "$STATE"
if [[ ! -x "$BIN" ]]; then
	echo "missing $BIN; build with: cargo build -p attacks-playground" >&2
	exit 1
fi

"$BIN" --capture &
PID=$!
cleanup() {
	kill "$PID" 2>/dev/null || true
	wait "$PID" 2>/dev/null || true
}
trap cleanup EXIT

wait_state() {
	local want="$1"
	local out="$2"
	for _ in $(seq 1 1200); do
		if [[ -f "$STATE" ]] && grep -q "^${want}" "$STATE"; then
			sleep 0.55
			scrot -o "$out"
			echo "captured $want -> $out"
			return 0
		fi
		if ! kill -0 "$PID" 2>/dev/null; then
			echo "playground exited before $want" >&2
			echo "last state: $(cat "$STATE" 2>/dev/null || echo none)" >&2
			return 1
		fi
		sleep 0.2
	done
	echo "timed out waiting for $want" >&2
	echo "last state: $(cat "$STATE" 2>/dev/null || echo none)" >&2
	return 1
}

wait_state WINDUP "$PREVIEW/playground-windup-telegraph.png"
wait_state EXECUTE "$PREVIEW/playground-execute-hazard.png"
cp "$PREVIEW/playground-execute-hazard.png" "$PREVIEW/playground-execute-side.png"
wait_state CANCELLED "$PREVIEW/playground-cancelled.png"

# Let the process write DONE / exit itself.
for _ in $(seq 1 50); do
	if ! kill -0 "$PID" 2>/dev/null; then
		trap - EXIT
		wait "$PID" || true
		exit 0
	fi
	sleep 0.2
done
echo "playground did not exit after CANCELLED" >&2
exit 1
