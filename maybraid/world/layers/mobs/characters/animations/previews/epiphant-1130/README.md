# Epiphant Bray / sleep previews ([#1130](https://github.com/ramate-io/maybraid/issues/1130))

Captured from `character-concepts-playground` on the clothed Epiphant recipe (quadruped V0 rig + meerkat head), using Lavapipe Vulkan (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`).

| File | Animation | Scale | Notes |
|------|-----------|-------|-------|
| `ordinary-bray.png` | `epiphant-bray` | 1× | `CROZON_PREVIEW_SAMPLE_PROGRESS=0.34` (flick apex) |
| `ordinary-sleep-enter.png` | `epiphant-sleep-enter` | 1× | progress `0.90` (settle) |
| `ordinary-sleep-hold.png` | `epiphant-sleep-hold` | 1× | progress `0.35` |
| `ordinary-wake.png` | `epiphant-wake` | 1× | progress `0.18` (front plant) |
| `ordinary-sleep-interrupted.png` | `epiphant-sleep-hold` → `epiphant-bray` | 1× | interrupt at 2.0 s |
| `boss-bray.png` … `boss-sleep-interrupted.png` | same | 10× | `CROZON_PREVIEW_ENCOUNTER_SCALE=10` (~20 m assembly) |

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
