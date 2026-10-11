# Spatial rumble playground

Isolated host for [`maybraid-spatial-rumble`](../lib). Dispatch Bray, heavy
landing, or a stacked pair and judge camera shake plus the ground sample.

```text
cargo run -p spatial-rumble-playground
cargo run -p spatial-rumble-playground -- demo --quit-after 2.4
```

Keyboard (no controller required):

| Key | Dispatch |
|---|---|
| `1` | Bray at the origin |
| `2` | Heavy landing at the origin |
| `3` | Both presets at once |
| `4` | Bray 28 m away (falloff) |
| `P` | Pause / resume |

WASD + mouse look, Space / Shift up-down. The ground plane scales with
[`GroundRumbleSample`](../lib/src/channels.rs); rings visualize amplitude. A
connected pad receives `PadRumble` through the existing input plugin; keyboard
and mouse play without one.
