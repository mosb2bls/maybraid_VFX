#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="$SCRIPT_DIR"
WS_ROOT="$(cd "$SCRIPT_DIR/../../../../../../../.." && pwd)"
BIN="${WS_ROOT}/target/debug/character-concepts-playground"

export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export WGPU_BACKEND=vulkan
export CROZON_PREVIEW_HIDE_UI=1

# Nix-built playground + system Xvfb/ffmpeg must not share one LD_LIBRARY_PATH (glibc mismatch).
PLAYGROUND_LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}"
unset LD_LIBRARY_PATH

playground_ld_library_path() {
  local paths="$PLAYGROUND_LD_LIBRARY_PATH"
  local libs="libX11 libXcursor libXrandr libXi libXinerama libXxf86vm libxkbcommon libxkbcommon-x11 libxcb"
  local lib path
  for lib in $libs; do
    path=$(find /nix/store -maxdepth 3 -name "${lib}.so*" 2>/dev/null | head -1 || true)
    if [[ -n "$path" ]]; then
      paths="$(dirname "$path"):${paths}"
    fi
  done
  echo "${paths%:}"
}

# Allow asset load + mailbox pose before scrot (seconds).
SHOT_SECS="${CROZON_PREVIEW_SHOT_SECS:-28}"
QUIT_SECS="${CROZON_PREVIEW_QUIT_SECS:-32}"

if [[ ! -x "$BIN" ]]; then
  (cd "$WS_ROOT/maybraid/menu/character-concepts-playground" && cargo build -q --bin character-concepts-playground)
fi

# Set to 1 to record full-sequence MP4s (no frozen progress); stills are skipped when only videos run.
RUN_STILLS="${CROZON_CAPTURE_STILLS:-1}"
RUN_VIDEOS="${CROZON_CAPTURE_VIDEOS:-0}"

capture_video() {
  local name="$1"
  local animation="$2"
  local scale="$3"
  local duration="$4"
  local quit="$5"
  local interrupt_after="${6:-}"
  local interrupt_anim="${7:-}"
  local mouth_emit="${8:-0}"

  echo "Recording ${name}.mp4 (${animation}, scale=${scale}, ${duration}s)…"
  rm -f "${OUT}/${name}.mp4"

  export CROZON_PREVIEW_ENCOUNTER_SCALE="${scale}"
  export CROZON_PREVIEW_CAPTURE_QUIT_SECS="${quit}"
  unset CROZON_PREVIEW_SAMPLE_PROGRESS CROZON_PREVIEW_INTERRUPT_AFTER_SECS CROZON_PREVIEW_INTERRUPT_ANIMATION CROZON_PREVIEW_DRAW_MOUTH_EMIT
  if [[ -n "$interrupt_after" ]]; then
    export CROZON_PREVIEW_INTERRUPT_AFTER_SECS="${interrupt_after}"
    export CROZON_PREVIEW_INTERRUPT_ANIMATION="${interrupt_anim}"
  fi
  if [[ "$mouth_emit" == "1" ]]; then
    export CROZON_PREVIEW_DRAW_MOUTH_EMIT=1
  fi

  local ld_path
  ld_path="$(playground_ld_library_path)"
  xvfb-run -a -s '-screen 0 1280x720x24' bash -c "
    set -euo pipefail
    export LD_LIBRARY_PATH='${ld_path}'
    export VK_ICD_FILENAMES='${VK_ICD_FILENAMES}' WGPU_BACKEND='${WGPU_BACKEND}'
    env -u LD_LIBRARY_PATH /usr/bin/ffmpeg -y -loglevel error -f x11grab -framerate 24 -video_size 1280x720 -i \"\$DISPLAY\" -t ${duration} \"${OUT}/${name}.mp4\" &
    rec=\$!
    timeout ${quit} \"${BIN}\" epiphant preview --animation \"${animation}\" || true
    wait \"\$rec\" || true
  "

  if [[ ! -f "${OUT}/${name}.mp4" ]]; then
    echo "missing video for ${name}" >&2
    return 1
  fi
}

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

  local ld_path
  ld_path="$(playground_ld_library_path)"
  xvfb-run -a -s '-screen 0 1280x720x24' bash -c "
    export LD_LIBRARY_PATH='${ld_path}'
    export VK_ICD_FILENAMES='${VK_ICD_FILENAMES}' WGPU_BACKEND='${WGPU_BACKEND}'
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

if [[ "$RUN_STILLS" == "1" ]]; then
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
fi

if [[ "$RUN_VIDEOS" == "1" ]]; then
  # Full clip playback from progress 0 (mailbox clock); boss shots keep the 2 m scale axes in frame.
  capture_video ordinary-bray epiphant-bray 1 2.0 4 "" "" 1
  capture_video ordinary-sleep-enter epiphant-sleep-enter 1 2.0 4
  capture_video ordinary-sleep-hold epiphant-sleep-hold 1 8.0 10
  capture_video ordinary-wake epiphant-wake 1 2.0 4
  capture_video ordinary-sleep-interrupted epiphant-sleep-hold 1 36 38 18 epiphant-bray

  capture_video boss-bray epiphant-bray 10 2.0 4 "" "" 1
  capture_video boss-sleep-enter epiphant-sleep-enter 10 2.0 4
  capture_video boss-sleep-hold epiphant-sleep-hold 10 8.0 10
  capture_video boss-wake epiphant-wake 10 2.0 4
  capture_video boss-sleep-interrupted epiphant-sleep-hold 10 38 40 20 epiphant-bray
fi

echo "Done. Artifacts in ${OUT}"
[[ "$RUN_STILLS" == "1" ]] && sha256sum "${OUT}"/*.png
[[ "$RUN_VIDEOS" == "1" ]] && sha256sum "${OUT}"/*.mp4
