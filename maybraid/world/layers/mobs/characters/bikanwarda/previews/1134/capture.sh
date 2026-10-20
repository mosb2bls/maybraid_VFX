#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="$SCRIPT_DIR"
WS_ROOT="$(cd "$SCRIPT_DIR/../../../../../../../.." && pwd)"
BIN="${WS_ROOT}/target/debug/character-concepts-playground"

export CXX="${CXX:-g++-13}"
export CC="${CC:-gcc-13}"
export LIBRARY_PATH="${LIBRARY_PATH:-/usr/lib/gcc/x86_64-linux-gnu/13}"
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export WGPU_BACKEND=vulkan
export CROZON_PREVIEW_HIDE_UI=1

SCALE="$(cd "$WS_ROOT" && cargo run -q --example scale -p bikanwarda-character)"

SHOT_SECS="${CROZON_PREVIEW_SHOT_SECS:-30}"
QUIT_SECS="${CROZON_PREVIEW_QUIT_SECS:-34}"

if [[ ! -x "$BIN" ]]; then
  (cd "$WS_ROOT" && cargo build -q -p character-concepts-playground)
fi

capture() {
  local name="$1"
  local animation="$2"
  local progress="${3:-}"

  echo "Capturing ${name} (${animation}, scale=${SCALE})…"
  rm -f /tmp/bikanwarda-capture.png

  export CROZON_PREVIEW_ENCOUNTER_SCALE="${SCALE}"
  export CROZON_PREVIEW_CAPTURE_QUIT_SECS="${QUIT_SECS}"
  unset CROZON_PREVIEW_SAMPLE_PROGRESS
  if [[ -n "$progress" ]]; then
    export CROZON_PREVIEW_SAMPLE_PROGRESS="${progress}"
  fi

  xvfb-run -a -s '-screen 0 1280x720x24' bash -c "
    export VK_ICD_FILENAMES='${VK_ICD_FILENAMES}' WGPU_BACKEND='${WGPU_BACKEND}'
    (sleep ${SHOT_SECS} && scrot -o /tmp/bikanwarda-capture.png) &
    timeout ${QUIT_SECS} \"${BIN}\" epiphant preview --animation \"${animation}\" || true
    wait || true
  "

  if [[ -f /tmp/bikanwarda-capture.png ]]; then
    cp /tmp/bikanwarda-capture.png "${OUT}/${name}.png"
  else
    echo "missing still for ${name}" >&2
    return 1
  fi
}

capture boss-assembly-still still
capture boss-bray-apex epiphant-bray 0.34
capture boss-sleep-hold epiphant-sleep-hold 0.35

echo "Wrote PNGs to $OUT (encounter scale=$SCALE)"
