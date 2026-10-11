#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="$SCRIPT_DIR"
WS_ROOT="$(cd "$SCRIPT_DIR/../../../../../../../.." && pwd)"
BIN="${WS_ROOT}/target/debug/character-concepts-playground"

export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export WGPU_BACKEND=vulkan
export CROZON_PREVIEW_HIDE_UI=1

# Allow asset load + mailbox pose before scrot (seconds).
SHOT_SECS="${CROZON_PREVIEW_SHOT_SECS:-28}"
QUIT_SECS="${CROZON_PREVIEW_QUIT_SECS:-32}"

if [[ ! -x "$BIN" ]]; then
  (cd "$WS_ROOT/maybraid/menu/character-concepts-playground" && cargo build -q --bin character-concepts-playground)
fi

capture() {
  local name="$1"
  local animation="$2"
  local scale="$3"
  local progress="${4:-}"
  local shot_at="${5:-$SHOT_SECS}"
  local quit="${6:-$QUIT_SECS}"
  local interrupt_after="${7:-}"
  local interrupt_anim="${8:-}"
  local mouth_emit="${9:-0}"

  echo "Capturing ${name} (${animation}, scale=${scale})…"
  rm -f /tmp/epiphant-capture.png

  export CROZON_PREVIEW_ENCOUNTER_SCALE="${scale}"
  export CROZON_PREVIEW_CAPTURE_QUIT_SECS="${quit}"
  unset CROZON_PREVIEW_SAMPLE_PROGRESS CROZON_PREVIEW_INTERRUPT_AFTER_SECS CROZON_PREVIEW_INTERRUPT_ANIMATION CROZON_PREVIEW_DRAW_MOUTH_EMIT
  if [[ -n "$progress" ]]; then
    export CROZON_PREVIEW_SAMPLE_PROGRESS="${progress}"
  fi
  if [[ -n "$interrupt_after" ]]; then
    export CROZON_PREVIEW_INTERRUPT_AFTER_SECS="${interrupt_after}"
    export CROZON_PREVIEW_INTERRUPT_ANIMATION="${interrupt_anim}"
  fi
  if [[ "$mouth_emit" == "1" ]]; then
    export CROZON_PREVIEW_DRAW_MOUTH_EMIT=1
  fi

  xvfb-run -a -s '-screen 0 1280x720x24' bash -c "
    (sleep ${shot_at} && scrot -o /tmp/epiphant-capture.png) &
    timeout ${quit} \"${BIN}\" epiphant preview --animation \"${animation}\" || true
    wait || true
  "

  if [[ -f /tmp/epiphant-capture.png ]]; then
    mv /tmp/epiphant-capture.png "${OUT}/${name}.png"
  else
    echo "missing screenshot for ${name}" >&2
    return 1
  fi
}

capture ordinary-bray epiphant-bray 1 0.34 "$SHOT_SECS" "$QUIT_SECS" "" "" 1
capture ordinary-sleep-enter epiphant-sleep-enter 1 0.90
capture ordinary-sleep-hold epiphant-sleep-hold 1 0.35
capture ordinary-wake epiphant-wake 1 0.18
capture ordinary-sleep-interrupted epiphant-sleep-hold 1 "" 31 35 18 epiphant-bray

capture boss-bray epiphant-bray 10 0.34 "$SHOT_SECS" "$QUIT_SECS" "" "" 1
capture boss-sleep-enter epiphant-sleep-enter 10 0.90
capture boss-sleep-hold epiphant-sleep-hold 10 0.35
capture boss-wake epiphant-wake 10 0.18
capture boss-sleep-interrupted epiphant-sleep-hold 10 "" 34 38 20 epiphant-bray

echo "Done. PNGs in ${OUT}"
sha256sum "${OUT}"/*.png
