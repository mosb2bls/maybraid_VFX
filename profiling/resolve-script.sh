# Sourced by the flight scripts. `resolve_script <name-or-path>` sets:
#   script    absolute path to the JSONL script
#   scenario  measure-window scenario name (last non-`-warmup` start mark)
# A bare name resolves to maybraid/agent/scripts/<name>.jsonl; anything that
# exists as a file (e.g. a script downloaded from a Traces release) is used as is.

resolve_script() {
	local arg="$1"
	if [[ -f "$arg" ]]; then
		script="$(cd "$(dirname "$arg")" && pwd)/$(basename "$arg")"
	else
		script="$root/maybraid/agent/scripts/${arg}.jsonl"
	fi
	if [[ ! -f "$script" ]]; then
		echo "missing script: $script" >&2
		exit 2
	fi
	scenario="$(python3 - "$script" <<'PY'
import json, sys
name = None
for raw in open(sys.argv[1]):
    line = raw.strip()
    if not line or line.startswith("#") or line.startswith("//"):
        continue
    cmd = json.loads(line)
    if cmd.get("cmd") == "scenario" and cmd.get("mark") == "start":
        if not cmd["name"].endswith("-warmup"):
            name = cmd["name"]
print(name or "")
PY
)"
	if [[ -z "$scenario" ]]; then
		echo "no measure scenario (non -warmup start mark) in $script" >&2
		exit 2
	fi
}
