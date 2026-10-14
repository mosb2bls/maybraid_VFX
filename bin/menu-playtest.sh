#!/usr/bin/env bash
# Run checked-in menu-*.jsonl scripts against throwaway saves/logs.
# Usage: bin/menu-playtest.sh [--seeds N] [--roster empty|one|several|auto] [script.jsonl ...]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
scripts_dir="$root/maybraid/agent/scripts"
fixture_one="$scripts_dir/fixtures/character-playtest-one.json"
bin="${MAYBRAID_BIN:-}"
seeds=""
roster="auto"
scripts=()

usage="usage: bin/menu-playtest.sh [--seeds N] [--roster empty|one|several|auto] [script.jsonl ...]"

while [[ $# -gt 0 ]]; do
	case "$1" in
		--seeds)
			seeds="${2:?$usage}"
			shift 2
			;;
		--roster)
			roster="${2:?$usage}"
			shift 2
			;;
		-h | --help)
			echo "$usage"
			exit 0
			;;
		*)
			scripts+=("$1")
			shift
			;;
	esac
done

if [[ ${#scripts[@]} -eq 0 ]]; then
	while IFS= read -r path; do
		scripts+=("$path")
	done < <(ls -1 "$scripts_dir"/menu-*.jsonl)
fi

if [[ -z "$bin" ]]; then
	if [[ -x "$root/target/release/maybraid" ]]; then
		bin="$root/target/release/maybraid"
	elif [[ -x "$root/target/debug/maybraid" ]]; then
		bin="$root/target/debug/maybraid"
	else
		echo "missing maybraid binary — build with cargo build -p maybraid" >&2
		exit 2
	fi
fi

if [[ ! -x "$bin" ]]; then
	echo "not executable: $bin" >&2
	exit 2
fi

seed_roster() {
	local dest="$1"
	local kind="$2"
	rm -rf "$dest"
	mkdir -p "$dest/characters"
	case "$kind" in
		empty) ;;
		one)
			cp "$fixture_one" "$dest/characters/00000000000000000000000000000001.json"
			printf '%s\n' '{"version":1,"id":"00000000000000000000000000000001"}' >"$dest/active.json"
			;;
		several)
			local i hex name
			for i in 1 2 3; do
				hex="$(printf '%032x' "$i")"
				name="Playtest $i"
				python3 - "$fixture_one" "$dest/characters/${hex}.json" "$hex" "$name" <<'PY'
import json, sys
src, dest, hex_id, name = sys.argv[1:]
data = json.load(open(src))
data["id"] = hex_id
data["name"] = name
json.dump(data, open(dest, "w"), indent=2)
print()
PY
			done
			printf '%s\n' '{"version":1,"id":"00000000000000000000000000000001"}' >"$dest/active.json"
			;;
		*)
			echo "unknown roster $kind" >&2
			return 2
			;;
	esac
}

roster_for_script() {
	local name="$1"
	if [[ "$roster" != "auto" ]]; then
		echo "$roster"
		return
	fi
	case "$name" in
		menu-create-back-mid-spin | menu-mash-home-* | menu-home-*) echo empty ;;
		*) echo one ;;
	esac
}

uses_start_at() {
	local name="$1"
	case "$name" in
		menu-create-back-mid-spin | menu-mash-home-* | menu-home-*) return 1 ;;
		*) return 0 ;;
	esac
}

rewrite_mash_seed() {
	local src="$1"
	local dest="$2"
	local seed="$3"
	python3 - "$src" "$dest" "$seed" <<'PY'
import json, sys
src, dest, seed = sys.argv[1:]
seed = int(seed)
out = []
with open(src) as fh:
	for line in fh:
		raw = line.strip()
		if not raw or raw.startswith("#") or raw.startswith("//"):
			out.append(line.rstrip("\n"))
			continue
		cmd = json.loads(raw)
		if cmd.get("cmd") == "mash":
			cmd["seed"] = seed
			out.append(json.dumps(cmd, separators=(",", ":")))
		else:
			out.append(raw)
open(dest, "w").write("\n".join(out) + "\n")
PY
}

analyze_events() {
	local events="$1"
	python3 - "$events" <<'PY'
import json, sys
path = sys.argv[1]
quit_seen = False
failed = []
invariants = []
ready = None
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
problems = []
if not quit_seen:
	problems.append("no quit event")
problems.extend(f"failed {item}" for item in failed)
problems.extend(f"invariant {item}" for item in invariants)
if problems:
	print("; ".join(problems))
	sys.exit(1)
print(f"ok ready={ready or '?'}")
PY
}

timeout_for() {
	local name="$1"
	case "$name" in
		menu-mash-*) echo "${MAYBRAID_PLAYTEST_TIMEOUT:-120}" ;;
		menu-leave-and-return) echo "${MAYBRAID_PLAYTEST_TIMEOUT:-180}" ;;
		*) echo "${MAYBRAID_PLAYTEST_TIMEOUT:-90}" ;;
	esac
}

work="$(mktemp -d "${TMPDIR:-/tmp}/maybraid-menu-playtest.XXXXXX")"
if [[ -z "${MAYBRAID_PLAYTEST_KEEP:-}" ]]; then
	trap 'rm -rf "$work"' EXIT
else
	echo "keeping playtest dir $work"
fi

pass=0
fail=0
declare -a results

run_one() {
	local script="$1"
	local seed_label="$2"
	local name
	name="$(basename "$script" .jsonl)"
	local label="$name"
	if [[ -n "$seed_label" ]]; then
		label="${name} seed=${seed_label}"
	fi
	local roster_kind
	roster_kind="$(roster_for_script "$name")"
	local run_dir="$work/${label//\//_}"
	mkdir -p "$run_dir/saves" "$run_dir/logs" "$run_dir/shots"
	seed_roster "$run_dir/saves" "$roster_kind"
	local play_script="$script"
	if [[ -n "$seed_label" ]]; then
		play_script="$run_dir/script.jsonl"
		rewrite_mash_seed "$script" "$play_script" "$seed_label"
	fi
	local events="$run_dir/events.jsonl"
	local log="$run_dir/game.log"
	local args=(--agent-script "$play_script" --agent-events "$events" --agent-quit)
	if uses_start_at "$name"; then
		args+=(--start-at=0,0)
	fi
	local timeout_secs
	timeout_secs="$(timeout_for "$name")"
	set +e
	if command -v timeout >/dev/null; then
		env \
			MAYBRAID_SAVES="$run_dir/saves" \
			MAYBRAID_LOGS="$run_dir/logs" \
			MAYBRAID_RESOLUTION="${MAYBRAID_RESOLUTION:-960x540}" \
			MAYBRAID_VSYNC="${MAYBRAID_VSYNC:-off}" \
			MAYBRAID_FULLSCREEN="${MAYBRAID_FULLSCREEN:-0}" \
			timeout --signal=KILL "${timeout_secs}s" "$bin" "${args[@]}" >"$log" 2>&1
	else
		env \
			MAYBRAID_SAVES="$run_dir/saves" \
			MAYBRAID_LOGS="$run_dir/logs" \
			MAYBRAID_RESOLUTION="${MAYBRAID_RESOLUTION:-960x540}" \
			MAYBRAID_VSYNC="${MAYBRAID_VSYNC:-off}" \
			MAYBRAID_FULLSCREEN="${MAYBRAID_FULLSCREEN:-0}" \
			"$bin" "${args[@]}" >"$log" 2>&1
	fi
	local rc=$?
	set -e
	local reason=""
	if [[ $rc -eq 124 || $rc -eq 137 ]]; then
		reason="timeout after ${timeout_secs}s"
	elif [[ $rc -ne 0 ]]; then
		reason="exit $rc"
	fi
	if [[ -s "$run_dir/logs/panic.log" ]]; then
		reason="${reason:+$reason; }panic.log"
	fi
	local analysis
	if analysis="$(analyze_events "$events")"; then
		:
	else
		reason="${reason:+$reason; }$analysis"
	fi
	if [[ -z "$reason" ]]; then
		echo "PASS  $label  ($analysis)  roster=$roster_kind"
		results+=("PASS $label")
		pass=$((pass + 1))
	else
		echo "FAIL  $label  ($reason)  roster=$roster_kind"
		echo "      events=$events log=$log"
		results+=("FAIL $label: $reason")
		fail=$((fail + 1))
	fi
}

for script in "${scripts[@]}"; do
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
	if [[ -n "$seeds" && "$name" == menu-mash-* ]]; then
		local_i=0
		while [[ $local_i -lt $seeds ]]; do
			run_one "$script" "$((42 + local_i))"
			local_i=$((local_i + 1))
		done
	else
		run_one "$script" ""
	fi
done

echo
echo "menu playtest: $pass passed, $fail failed"
if [[ $fail -ne 0 ]]; then
	printf '%s\n' "${results[@]}" | sed -n '/^FAIL /p'
	exit 1
fi
