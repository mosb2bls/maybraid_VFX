# World

Assembled world model. Streamed forest at 1 km grove fill / 3 km selection
generate, with canopy bump-outs in the 1–5 km present keep (spawned only where
160 m fine cells already exist). Fine terrain is the [#675](https://github.com/ramate-io/maybraid/pull/675)
~2.6 km disk plus short 2×/4× macro rings. Character mode is the default, with a
soft sky-dome wash from 350 m to 1200 m. The world uses the shared player camera
and firearm presentation stack: R3 / look-stick click toggles first person,
focus blends onto the firearm sight (LT / right mouse uses the optic FOV; left
bumper / middle mouse keeps iron FOV), and the third-person camera
shapecasts against Fixed terrain and vegetation sticks so it does not clip
through. Outgoing hits show FFA-style score markers, and incoming damage shows
directional indicators; health bars remain hidden. Vegetation continues to own
the player capsule locomotion. Downed players leave a ragdoll corpse under a
fading red screen glaze, wait four active-gameplay seconds, then receive a fresh
loadout-equipped body at a terrain-fitted nearby urban, interior, or vegetation
POI.

```bash
cargo run -p maybraid-world-playground
```

In-game: `/` console, `Y` or `F1` drawer. Character: WASD, mouse look, Space jump.
`mode free` restores the fly camera. FPS is on-screen (toggle with `stats fps`).

Forest stream knobs live on each mode's `VegetationLayerConfig`.
