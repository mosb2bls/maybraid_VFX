#!/usr/bin/env bash
# Capture one scripted Tracy flight. Run from the repo root inside `nix develop`.
set -euo pipefail

scenario="${1:?usage: profiling/tracy-flight.sh <scenario> <phase> <repeat>}"
phase="${2:?usage: profiling/tracy-flight.sh <scenario> <phase> <repeat>}"
repeat="${3:?usage: profiling/tracy-flight.sh <scenario> <phase> <repeat>}"

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

script="$root/maybraid/agent/scripts/${scenario}.jsonl"
if [[ ! -f "$script" ]]; then
	echo "missing script: $script" >&2
	exit 2
fi

bin="$root/target/release/maybraid"
if [[ ! -x "$bin" ]]; then
	echo "missing $bin — build with:" >&2
	echo "  cargo build -p maybraid --release --features bevy/trace_tracy" >&2
	exit 2
fi

if ! command -v tracy-capture >/dev/null || ! command -v tracy-csvexport >/dev/null; then
	echo "tracy-capture / tracy-csvexport must be on PATH (nix develop)" >&2
	exit 2
fi

# Only one Tracy listener can take the game. Use capture — never the GUI.
if pgrep -x tracy >/dev/null || pgrep -x tracy-profiler >/dev/null; then
	echo "Tracy GUI is running; quit it so tracy-capture is the only listener" >&2
	exit 2
fi
if pgrep -x tracy-capture >/dev/null; then
	echo "another tracy-capture is already running" >&2
	exit 2
fi
if pgrep -x maybraid >/dev/null; then
	echo "maybraid is already running; this machine is one-at-a-time" >&2
	exit 2
fi

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
out_dir="$root/profiling/captures/${phase}/${scenario}/r${repeat}"
mkdir -p "$out_dir"
base="${out_dir}/${scenario}-${phase}-r${repeat}-${stamp}"

tracy_file="${base}.tracy"
zones_csv="${base}-zones.csv"
messages_csv="${base}-messages.csv"
events_jsonl="${base}-events.jsonl"
meta_file="${base}-meta.txt"
summary_txt="${base}-summary.txt"

# Generous wall clocks: ready + warmup + measure. The game also --agent-quit.
case "$scenario" in
	origin-idle) timeout_secs=180 ;;
	origin-walk) timeout_secs=120 ;;
	locomotion-mix) timeout_secs=240 ;;
	biome-traverse) timeout_secs=420 ;;
	*) timeout_secs=300 ;;
esac

cleanup() {
	if [[ -n "${game_pid:-}" ]] && kill -0 "$game_pid" 2>/dev/null; then
		kill "$game_pid" 2>/dev/null || true
		wait "$game_pid" 2>/dev/null || true
	fi
	if [[ -n "${cap_pid:-}" ]] && kill -0 "$cap_pid" 2>/dev/null; then
		kill "$cap_pid" 2>/dev/null || true
		wait "$cap_pid" 2>/dev/null || true
	fi
}
trap cleanup EXIT

{
	echo "scenario=$scenario"
	echo "phase=$phase"
	echo "repeat=$repeat"
	echo "stamp=$stamp"
	echo "binary=$bin"
	echo "script=$script"
	echo "tracy=$(command -v tracy-capture)"
	tracy-capture --help 2>&1 | head -1 || true
	echo "hw=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)"
	echo "mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || true)"
	echo "vsync=off"
	echo "start_at=0,0"
} >"$meta_file"

echo "capture $scenario $phase r$repeat → $out_dir"

tracy-capture -o "$tracy_file" -f >/dev/null 2>"${base}-capture.log" &
cap_pid=$!
sleep 2
if ! kill -0 "$cap_pid" 2>/dev/null; then
	echo "tracy-capture exited before the game started" >&2
	cat "${base}-capture.log" >&2 || true
	exit 1
fi

export MAYBRAID_VSYNC=off
"$bin" --start-at=0,0 \
	--agent-script "$script" \
	--agent-events "$events_jsonl" \
	--agent-quit \
	>"${base}-game.log" 2>&1 &
game_pid=$!

elapsed=0
while kill -0 "$game_pid" 2>/dev/null; do
	if (( elapsed >= timeout_secs )); then
		echo "game exceeded ${timeout_secs}s — stopping" >&2
		kill "$game_pid" 2>/dev/null || true
		break
	fi
	sleep 2
	elapsed=$((elapsed + 2))
done
wait "$game_pid" 2>/dev/null || true
game_pid=""

# Give Tracy a moment to flush after the client disconnects.
wait "$cap_pid" 2>/dev/null || true
cap_pid=""
sleep 1

if [[ ! -s "$tracy_file" ]]; then
	echo "empty capture: $tracy_file" >&2
	cat "${base}-capture.log" >&2 || true
	exit 1
fi

unwrap_csv="${base}-unwrap.csv"
tracy-csvexport "$tracy_file" >"$zones_csv"
tracy-csvexport -m "$tracy_file" >"$messages_csv"
# Unwrapped events are large; keep them local (not committed) for windowed means.
tracy-csvexport -u "$tracy_file" >"$unwrap_csv"

python3 "$root/profiling/summarize-zones.py" \
	--zones "$zones_csv" \
	--unwrap "$unwrap_csv" \
	--events "$events_jsonl" \
	--scenario "$scenario" \
	--out "$summary_txt"

echo "wrote $zones_csv"
echo "wrote $summary_txt"
ls -lh "$tracy_file" "$zones_csv" "$events_jsonl"
