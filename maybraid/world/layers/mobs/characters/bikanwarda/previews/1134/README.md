# Bikanwarda assembly validation ([#1134](https://github.com/ramate-io/maybraid/issues/1134))

Boss-scale Epiphant stills reuse the [#1130](https://github.com/ramate-io/maybraid/issues/1130) capture harness; encounter scale comes from [`encounter_uniform_scale()`](../../src/measure.rs) in `bikanwarda-character` (~10.5× for a 20 m motor envelope).

| File | Notes |
|------|-------|
| `boss-assembly-still.png` | Encounter scale, still pose |
| `boss-bray-apex.png` | `epiphant-bray` @ progress 0.34 |
| `boss-sleep-hold.png` | `epiphant-sleep-hold` @ 0.35 |

Regenerate:

```bash
./capture.sh
```

Manual frame:

```bash
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export WGPU_BACKEND=vk
SCALE=$(cargo run -q --example scale -p bikanwarda-character)
CROZON_PREVIEW_ENCOUNTER_SCALE="$SCALE" CROZON_PREVIEW_CAPTURE_QUIT_SECS=45 \
  cargo run -p character-concepts-playground -- epiphant preview --animation epiphant-bray
```
