# Doric column kit (`doric-column`)

Reusable pedestal + shaft + capital assembly for Bikanwarda ([#1133](https://github.com/ramate-io/maybraid/issues/1133)): configurable [`DoricColumnSpec`](src/spec.rs), chain anchors, Fixed-layer cover colliders, terrain ground contact, and encounter/summon spawn entry points.

**Authoring (Liam):** [`AUTHORING.md`](AUTHORING.md)

**Preview:**

```bash
cargo run -p doric-column-playground
```

WASD + right-drag look, Space/Shift vertical. Spawns an encounter column (left) and a slightly scaled summon column (right) with debug gizmos for cover and anchors.

**Headless capture** (CI / cloud VM): fixed 1280×720 window, `import -window` (not `scrot -u`, which fails on xvfb):

```bash
DORIC_PREVIEW_SHOT=wide|capital xvfb-run -a ./target/debug/doric-column-playground &
# after ~12s once the window exists:
WID=$(xdotool search --name doric-column-playground | head -1)
import -window "$WID" /path/to/frame.png
```
