# Epiphant Bray / sleep previews ([#1130](https://github.com/ramate-io/maybraid/issues/1130))

Captured from `character-concepts-playground` on the clothed Epiphant recipe (quadruped V0 rig + meerkat head), using Lavapipe Vulkan (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`).

| File | Animation | Scale | Notes |
|------|-----------|-------|-------|
| `ordinary-bray.png` | `epiphant-bray` | 1× | `CROZON_PREVIEW_SAMPLE_PROGRESS=0.34` (flick apex) |
| `ordinary-sleep-enter.png` | `epiphant-sleep-enter` | 1× | progress `0.90` (settle) |
| `ordinary-sleep-hold.png` | `epiphant-sleep-hold` | 1× | progress `0.35` |
| `ordinary-wake.png` | `epiphant-wake` | 1× | progress `0.18` (front plant) |
| `ordinary-sleep-interrupted.png` | `epiphant-sleep-hold` → `epiphant-bray` | 1× | interrupt at 18 s, frame ~31 s (mid-bray rise) |
| `boss-bray.png` … `boss-sleep-interrupted.png` | same | 10× | `CROZON_PREVIEW_ENCOUNTER_SCALE=10`; boss interrupt at 20 s, frame ~34 s |

Capture hides the creator panel and command HUD (`CROZON_PREVIEW_HIDE_UI`), spawns a ground plane aligned each frame to the lowest **body-mesh** world Y (scaled foot contact, with Epiphant locomotion-capsule anchor before meshes load), frames the camera from live mesh bounds (centered, feet + ground in view), and draws the bray mouth-emit gizmo (cyan origin, yellow horizontal forward) when `CROZON_PREVIEW_DRAW_MOUTH_EMIT=1`.

Regenerate:

```bash
./capture.sh
```

Manual single frame (Liam):

```bash
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
CROZON_PREVIEW_SAMPLE_PROGRESS=0.34 CROZON_PREVIEW_CAPTURE_QUIT_SECS=60 \
  cargo run -p character-concepts-playground -- epiphant preview --animation epiphant-bray
```
