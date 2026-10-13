#!/usr/bin/env bash
# Bounded startup from /tmp. Fail on loader errors. Do not claim GPU success.
set -euo pipefail

[[ $# -ge 1 ]] || { echo "usage: packaging/smoke.sh <binary>" >&2; exit 2; }
bin="$1"
sec="${MAYBRAID_SMOKE_SECONDS:-8}"
log="$(mktemp)"

echo "==> smoke (cwd=/tmp, ${sec}s): $bin"
(
	cd /tmp
	"$bin"
) >"$log" 2>&1 &
pid=$!

elapsed=0
while kill -0 "$pid" 2>/dev/null && (( elapsed < sec )); do
	sleep 1
	elapsed=$((elapsed + 1))
done

if kill -0 "$pid" 2>/dev/null; then
	kill -TERM "$pid" 2>/dev/null || true
	wait "$pid" || true
	sed -n '1,20p' "$log"
	echo "smoke: process stayed up ${sec}s (loader check only; GPU not validated)"
	rm -f "$log"
	exit 0
fi

code=0
wait "$pid" || code=$?
sed -n '1,40p' "$log"
if grep -Eiq 'Library not loaded|dyld\[|error while loading shared libraries|cannot open shared object|GLIBC_' "$log"; then
	echo "smoke: loader failure (exit $code)" >&2
	rm -f "$log"
	exit 1
fi
echo "smoke: exited $code before ${sec}s; not a runtime pass"
rm -f "$log"
exit 0
