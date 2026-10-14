# Tanz user

Tanzer control mode, starter Tanz definitions, and the STANDARD / TANZER HUD.

When a Tanz is executed, a top-center callout shows that Tanz's captured
silhouette and display name, then fades over 1.25s. A newer trigger replaces
the previous callout. The callout is display-only.

## Icons

Starter Tanzes store a Bevy asset path on [`TanzDefinition::icon`]:

| Tanz | Path |
| --- | --- |
| Healing Salute | `iconography/tanz/healing_salute.png` |
| Left Strafe | `iconography/tanz/left_strafe.png` |
| Right Strafe | `iconography/tanz/right_strafe.png` |
| Forward Dash | `iconography/tanz/forward_dash.png` |

Those PNGs are 128×128 white silhouettes with transparency, captured from a
base nude Braidman (`BraidmanConfig::default_preview()`, no clothing / hair)
in `characters-playground` under lavapipe. Poses are the readable clip beats:
salute hold (`jab` 0.36), strafe slide peak (`power-strafe` 0.48), dash
stomp/strike (`power-dash` 0.42).

### Regenerate

```bash
# lavapipe + xvfb. Builds characters-playground if needed.
maybraid/items/tanz-user/tools/capture-tanz-icons.sh
```

That script:

1. Runs `characters-playground capture-tanz-icons --out-dir <raw>` with
   `WGPU_BACKEND=vulkan WGPU_FORCE_FALLBACK_ADAPTER=1
   WGPU_ADAPTER_NAME=llvmpipe VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`.
2. Converts the magenta-backed screenshots with
   `tools/silhouette_from_capture.py` into the committed 128px icons.

Manual playground commands for one pose:

```text
/character assemble --species braidman --animation power-dash
/character playback --pause --progress 0.42
/character camera three-quarter
```
