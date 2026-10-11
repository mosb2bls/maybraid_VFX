#!/usr/bin/env bash
# Capture one scripted Metal System Trace (real GPU time per render pass).
# Run from the repo root inside `nix develop`. macOS only.
#
# Bevy does not open a Tracy GPU context on Metal, so GPU zones come from
# Instruments: launch the game, wait for the measure-window scenario start,
# attach `xctrace`, then export and summarize per-pass GPU time.
set -euo pipefail

usage="usage: profiling/metal-flight.sh <scenario|script.jsonl> <phase> <repeat>"
target="${1:?$usage}"
phase="${2:?$usage}"
repeat="${3:?$usage}"

root="$(cd "$(dirname "$0")/.." && pwd)"
source "$root/profiling/resolve-script.sh"
resolve_script "$target"
cd "$root"

bin="${MAYBRAID_BIN:-$root/target/release/maybraid}"
if [[ ! -x "$bin" ]]; then
	echo "missing $bin — build with:" >&2
	echo "  cargo build -p maybraid --release --features bevy/trace_tracy" >&2
	exit 2
fi
if [[ ! -x /usr/bin/xctrace ]]; then
	echo "/usr/bin/xctrace missing — install Xcode and run xcode-select -s" >&2
	exit 2
fi

live="$(pgrep -fl 'target/release/maybraid|xctrace|tracy-capture|tracy-profiler' || true)"
if [[ -n "$live" ]]; then
	echo "refusing: this machine is one-at-a-time and these are running:" >&2
	echo "$live" >&2
	exit 3
fi

# Seconds of GPU to record after the measure scenario starts.
attach_limit="${MAYBRAID_GPU_TRACE_LIMIT:-28}"
ready_timeout="${MAYBRAID_FLIGHT_TIMEOUT:-180}"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
captures_root="${MAYBRAID_TRACY_CAPTURES:-$root/profiling/captures}"
out_dir="${captures_root}/gpu/${phase}/${scenario}/r${repeat}"
mkdir -p "$out_dir"
base="${out_dir}/${scenario}-${phase}-r${repeat}-${stamp}"

trace="${base}.trace"
events="${base}-events.jsonl"
game_log="${base}-game.log"
xc_log="${base}-xctrace.log"
discard_tracy="${base}-discard.tracy"

{
	echo "scenario=$scenario"
	echo "phase=$phase"
	echo "repeat=$repeat"
	echo "stamp=$stamp"
	echo "binary=$bin"
	echo "script=$script"
	echo "commit=$(git -C "$root" rev-parse HEAD 2>/dev/null || true)"
	echo "hw=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)"
	echo "model=$(sysctl -n hw.model 2>/dev/null || true)"
	echo "mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || true)"
	echo "xctrace=$(/usr/bin/xctrace version 2>/dev/null || true)"
	echo "attach_limit_s=$attach_limit"
	echo "vsync=off"
} >"${base}-meta.txt"

# A trace_tracy build buffers zones until a listener connects; give it one.
cap_pid=""
if command -v tracy-capture >/dev/null; then
	tracy-capture -o "$discard_tracy" -f >/dev/null 2>"${base}-tracy-capture.log" &
	cap_pid=$!
	sleep 1
fi

export MAYBRAID_VSYNC=off
"$bin" --start-at=0,0 \
	--agent-script "$script" \
	--agent-events "$events" \
	--agent-quit \
	>"$game_log" 2>&1 &
game_pid=$!

# macOS throttles occluded windows; keep the game frontmost while recording.
(
	while kill -0 "$game_pid" 2>/dev/null; do
		osascript -e 'tell application "System Events" to set frontmost of (first process whose name contains "maybraid") to true' >/dev/null 2>&1 || true
		sleep 2
	done
) &
front_pid=$!

cleanup() {
	kill "$front_pid" 2>/dev/null || true
	if kill -0 "$game_pid" 2>/dev/null; then
		sleep 3
		kill "$game_pid" 2>/dev/null || true
		sleep 2
		kill -9 "$game_pid" 2>/dev/null || true
	fi
	wait "$game_pid" 2>/dev/null || true
	if [[ -n "$cap_pid" ]]; then
		kill "$cap_pid" 2>/dev/null || true
		wait "$cap_pid" 2>/dev/null || true
	fi
	rm -f "$discard_tracy"
}
trap cleanup EXIT

echo "metal $scenario $phase r$repeat → $out_dir"
mark="\"name\":\"${scenario}\",\"mark\":\"start\""
waited=0
until grep -q "$mark" "$events" 2>/dev/null; do
	if ! kill -0 "$game_pid" 2>/dev/null; then
		echo "game exited before scenario $scenario started" >&2
		tail -20 "$game_log" >&2 || true
		exit 1
	fi
	if (( waited >= ready_timeout * 2 )); then
		echo "scenario $scenario did not start within ${ready_timeout}s" >&2
		exit 1
	fi
	sleep 0.5
	waited=$((waited + 1))
done

echo "scenario start seen; recording ${attach_limit}s (pid $game_pid)"
set +e
/usr/bin/xctrace record \
	--template 'Metal System Trace' \
	--time-limit "${attach_limit}s" \
	--output "$trace" \
	--no-prompt \
	--attach "$game_pid" \
	>"$xc_log" 2>&1
xc_status=$?
set -e
cleanup
trap - EXIT

if [[ "$xc_status" -ne 0 || ! -d "$trace" ]]; then
	echo "xctrace failed ($xc_status); see $xc_log" >&2
	exit 1
fi

# The exported tables are 10s–100s of MB; parse them and delete them.
/usr/bin/xctrace export --input "$trace" --toc >"${base}-toc.xml"
for table in metal-application-encoders-list metal-gpu-intervals gpu-performance-state-intervals; do
	/usr/bin/xctrace export --input "$trace" \
		--xpath "/trace-toc/run[@number=\"1\"]/data/table[@schema=\"$table\"]" \
		>"${base}-${table}.xml"
done
python3 "$root/profiling/metal-passes.py" \
	--stem "$base" \
	--events "$events" \
	--scenario "$scenario" >/dev/null
rm -f "${base}"-metal-*.xml "${base}-gpu-performance-state-intervals.xml"

# Upload form for Traces releases (a .trace is a directory bundle).
ditto -c -k --keepParent "$trace" "${trace}.zip"

echo "wrote ${base}-summary.md"
echo "wrote ${base}-passes.csv"
ls -lh "${trace}.zip" "${base}-summary.md"
