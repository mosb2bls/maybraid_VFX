#!/usr/bin/env bash
# Run one checked-in smoke-*.jsonl (or any agent script) against the real
# maybraid binary on a throwaway save. Assert scripted events, never FPS.
# Usage: bin/script-smoke.sh [--timeout SECS] <script.jsonl|smoke-name>
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
scripts_dir="$root/maybraid/agent/scripts"
bin="${MAYBRAID_BIN:-}"
timeout_secs=""
script=""

usage="usage: bin/script-smoke.sh [--timeout SECS] <script.jsonl|smoke-name>"

while [[ $# -gt 0 ]]; do
	case "$1" in
		--timeout)
			timeout_secs="${2:?$usage}"
			shift 2
			;;
		-h | --help)
			echo "$usage"
			exit 0
			;;
		*)
			if [[ -n "$script" ]]; then
				echo "$usage" >&2
				exit 2
			fi
			script="$1"
			shift
			;;
	esac
done

if [[ -z "$script" ]]; then
	echo "$usage" >&2
	exit 2
fi

if [[ ! -f "$script" ]]; then
	if [[ -f "$scripts_dir/$script" ]]; then
		script="$scripts_dir/$script"
	elif [[ -f "$scripts_dir/${script}.jsonl" ]]; then
		script="$scripts_dir/${script}.jsonl"
	else
		echo "missing script $script" >&2
		exit 2
	fi
fi

name="$(basename "$script" .jsonl)"

if [[ -z "$bin" ]]; then
	if [[ -x "$root/target/release/maybraid" ]]; then
		bin="$root/target/release/maybraid"
	elif [[ -x "$root/target/debug/maybraid" ]]; then
		bin="$root/target/debug/maybraid"
	else
		echo "missing maybraid binary — build with cargo build -p maybraid --release" >&2
		exit 2
	fi
fi

if [[ ! -x "$bin" ]]; then
	echo "not executable: $bin" >&2
	exit 2
fi

default_timeout() {
	case "$name" in
		smoke-menu-* | menu-create-back-mid-spin | menu-mash-home-*) echo 120 ;;
		*) echo 240 ;;
	esac
}

if [[ -z "$timeout_secs" ]]; then
	timeout_secs="$(default_timeout)"
fi

uses_start_at() {
	case "$name" in
		smoke-menu-* | menu-create-back-mid-spin | menu-mash-home-*) return 1 ;;
		*) return 0 ;;
	esac
}

work="$(mktemp -d "${TMPDIR:-/tmp}/maybraid-script-smoke.XXXXXX")"
keep="${MAYBRAID_SMOKE_KEEP:-}"
if [[ -z "$keep" ]]; then
	trap 'rm -rf "$work"' EXIT
else
	echo "smoke work dir $work"
fi

mkdir -p "$work/saves/characters" "$work/logs" "$work/shots"
events="$work/events.jsonl"
log="$work/game.log"
assets="${MAYBRAID_ASSETS:-$root/maybraid/assets}"

args=(--agent-script "$script" --agent-events "$events" --agent-quit)
if uses_start_at; then
	args+=(--start-at=0,0)
fi

set +e
if command -v timeout >/dev/null; then
	env \
		MAYBRAID_SAVES="$work/saves" \
		MAYBRAID_LOGS="$work/logs" \
		MAYBRAID_ASSETS="$assets" \
		MAYBRAID_RESOLUTION="${MAYBRAID_RESOLUTION:-960x540}" \
		MAYBRAID_VSYNC="${MAYBRAID_VSYNC:-off}" \
		MAYBRAID_FULLSCREEN="${MAYBRAID_FULLSCREEN:-0}" \
		MAYBRAID_VISUAL_STYLE="${MAYBRAID_VISUAL_STYLE:-sharp}" \
		MAYBRAID_RENDER_SCALE="${MAYBRAID_RENDER_SCALE:-0.5}" \
		timeout --signal=KILL "${timeout_secs}s" "$bin" "${args[@]}" >"$log" 2>&1
else
	env \
		MAYBRAID_SAVES="$work/saves" \
		MAYBRAID_LOGS="$work/logs" \
		MAYBRAID_ASSETS="$assets" \
		MAYBRAID_RESOLUTION="${MAYBRAID_RESOLUTION:-960x540}" \
		MAYBRAID_VSYNC="${MAYBRAID_VSYNC:-off}" \
		MAYBRAID_FULLSCREEN="${MAYBRAID_FULLSCREEN:-0}" \
		MAYBRAID_VISUAL_STYLE="${MAYBRAID_VISUAL_STYLE:-sharp}" \
		MAYBRAID_RENDER_SCALE="${MAYBRAID_RENDER_SCALE:-0.5}" \
		"$bin" "${args[@]}" >"$log" 2>&1
fi
rc=$?
set -e

reason=""
if [[ $rc -eq 124 || $rc -eq 137 ]]; then
	reason="timeout after ${timeout_secs}s"
elif [[ $rc -ne 0 ]]; then
	reason="exit $rc"
fi
if [[ -s "$work/logs/panic.log" ]]; then
	reason="${reason:+$reason; }panic.log"
fi

set +e
analysis="$(
	python3 - "$events" "$name" <<'PY'
import json, sys
path, name = sys.argv[1], sys.argv[2]
quit_seen = False
failed = []
invariants = []
ready = None
waypoint_reached = False
picker_open = False
max_bodies = 0
screen_home = False
focus_characters = False
try:
	lines = open(path).read().splitlines()
except FileNotFoundError:
	print("missing events file")
	sys.exit(3)
for raw in lines:
	raw = raw.strip()
	if not raw:
		continue
	try:
		event = json.loads(raw)
	except json.JSONDecodeError:
		continue
	kind = event.get("kind")
	if kind == "quit":
		quit_seen = True
	elif kind == "ready":
		ready = event.get("detail")
	elif kind == "command" and event.get("status") == "failed":
		failed.append(f"#{event.get('id')} {event.get('cmd')}: {event.get('detail')}")
	elif kind == "invariant":
		invariants.append(event.get("detail") or "invariant")
	elif kind == "waypoint" and event.get("reached"):
		waypoint_reached = True
	elif kind == "picker" and event.get("open"):
		picker_open = True
	elif kind == "actors":
		bodies = event.get("bodies") or 0
		if bodies > max_bodies:
			max_bodies = bodies
	elif kind == "screen" and event.get("name") == "home":
		screen_home = True
	elif kind == "focus" and event.get("label") == "Characters":
		focus_characters = True
problems = []
if not quit_seen:
	problems.append("no quit event")
problems.extend(f"failed {item}" for item in failed)
problems.extend(f"invariant {item}" for item in invariants)
if name.startswith("smoke-origin-walk") or name == "origin-walk":
	if not waypoint_reached:
		problems.append("no waypoint reached")
if name.startswith("smoke-player-down") or name == "death-picker":
	if not picker_open:
		problems.append("no picker open (died)")
if name.startswith("smoke-menu-") or name.startswith("menu-"):
	if ready != "menu" and not screen_home:
		problems.append(f"menu did not open (ready={ready or '?'})")
	if name == "smoke-menu-home" and not focus_characters:
		problems.append("no focus on Characters")
if name.startswith("smoke-mobs-"):
	if max_bodies < 1:
		problems.append("no actor bodies")
if problems:
	print("; ".join(problems))
	sys.exit(1)
print(f"ok ready={ready or '?'} bodies={max_bodies}")
PY
)"
analyze_rc=$?
set -e
if [[ $analyze_rc -ne 0 ]]; then
	reason="${reason:+$reason; }$analysis"
fi

if [[ -n "$reason" ]]; then
	echo "FAIL  $name  ($reason)"
	echo "      events=$events log=$log"
	if [[ -n "${MAYBRAID_SMOKE_COPY_LOGS:-}" ]]; then
		mkdir -p "$MAYBRAID_SMOKE_COPY_LOGS"
		cp -f "$events" "$log" "$MAYBRAID_SMOKE_COPY_LOGS/" 2>/dev/null || true
		if [[ -s "$work/logs/panic.log" ]]; then
			cp -f "$work/logs/panic.log" "$MAYBRAID_SMOKE_COPY_LOGS/"
		fi
	fi
	exit 1
fi

echo "PASS  $name  ($analysis)"
