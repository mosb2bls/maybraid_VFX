# Playgrounds

A playground is a **single-layer** developer app next to the crate it inspects (`buildings-playground`, `furniture-playground`, `durham-playground`, [`world-materials-playground`](world/materials/playground/), [`vfx-playground`](vfx/playground/), …). Assembled world — Durham terrain, streamed forest, urbanization, character — lives in [`maybraid-world`](world/) and runs as [`maybraid-world-playground`](world/playground/).

Do not keep a second assembled-world app. Parameters and streaming knobs belong on `WorldPlugin` / `maybraid-world`, not on a parallel vegetation-on-terrain binary.

## Retiring

When a playground (or other throwaway app) is no longer worth maintaining:

1. Record it under [Retired](#retired) **before** deleting files: crate path, the last commit that still contained it (`git rev-parse HEAD` at retirement), and a short description of what it did.
2. Remove the crate from the workspace (and any `cargo run -p` docs). Point remaining callers at the replacement.
3. Restore later with `git checkout <commit> -- <path>` and re-add the workspace member.

If the crate is also a **library** used by world or another host, retire the **binary** and record that. Keep the library until those types move into a non-playground crate.

## Retired

Last commit that still contained the trees below: [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47).

### `maybraid/world/layers/vegetation/sbs-trees-playground`

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Isolated `/show` plants and tiled groves (`vast-orchards`, `monster-grass-plains`, …), plus a leftover flat-ground `/forest` streamer. Forest generate / present / vegetation LOD view now live in `chico`.
- **Replacement:** [`maybraid-world-playground`](world/playground/) for streamed forest on Durham. Isolated plant `/show` is not hosted anywhere; restore this crate if that catalog is needed again.

### `maybraid/world/player` (binary)

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Small Durham fine-grid patch for iterating Chico groves on real ground. `/grove <kind>` tiled one grove type; `/forest` streamed the same generate/present/cull path as SBS, grown on Durham height. Character / free-look, canopy bump-outs, mesh stats.
- **Replacement:** [`maybraid-world-playground`](world/playground/) (`cargo run -p maybraid-world-playground`). The crate is now only the character/player host and diagnostics library for `maybraid-world`. Richmond and mobs no longer use it.

### `maybraid/world/layers/urbanization/richmond-playground`

- **Last commit:** [`4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e`](https://github.com/ramate-io/maybraid/commit/4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e)
- **Did:** Durham fine patch with Richmond developments, Chico groves and bump-outs, and Maputo furniture presented from the shared HCSG runtime. Terrain still came from the legacy `Generate::<PlaygroundMode, OnTerrain<Durham>>` stream. A command flag restarted the session, and an optional development focus narrowed it to one development.
- **Replacement:** [`maybraid-world-playground`](world/playground/) runs the same layers in Discovery. Retired with the legacy HCSG path, so rewrite it on `DurhamWorldPlugin` rather than restoring it.

### `maybraid/world/layers/mobs/barking-playground`

- **Last commit:** [`4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e`](https://github.com/ramate-io/maybraid/commit/4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e)
- **Did:** 4×4 Durham fine patch with a short authored mob list (herd, pack, or forced Hars/Ylter herds). No vegetation and no mob LOD stream. Used to tell grounding, routing hops and tethered High plants apart without world-stream noise.
- **Replacement:** [`maybraid-world-playground`](world/playground/). Retired with the legacy HCSG path, so rewrite it on `DurhamWorldPlugin` rather than restoring it.

### `maybraid/world/layers/mobs/characters/world-movements-playground` (`character-world-movements-playground`)

- **Last commit:** [`4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e`](https://github.com/ramate-io/maybraid/commit/4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e)
- **Did:** 4×4 Durham fine patch at the highest mesh band for character locomotion (walk, run, jump, facing, grounding on Avian colliders). `set-character <species>` and `stampede` (every biped and quadruped on its own capsule).
- **Replacement:** [`maybraid-world-playground`](world/playground/) for locomotion on streamed ground; [`character-concepts-playground`](menu/character-concepts-playground/) for concept sliders. Retired with the legacy HCSG path, so rewrite it on `DurhamWorldPlugin` rather than restoring it.

### `maybraid/intelligence/routing-playground`

- **Last commit:** [`4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e`](https://github.com/ramate-io/maybraid/commit/4f1ff3b7eeb85a3aa36c00bf3bb25a123498856e)
- **Did:** 4×4 Durham fine patch (survey camera, vegetation lighting, no groves). One NPC tethered to or stalked the player while gizmos drew coarse-to-fine routing corridors.
- **Replacement:** None. Retired with the legacy HCSG path, so rewrite it on `DurhamWorldPlugin` rather than restoring it.

### `maybraid/world/layers/urbanization/richmond-playground` (catalog-batch mode)

- **Last commit:** [`6925f413a28b1cc8e6a17f3f94e3922c37b92e66`](https://github.com/ramate-io/maybraid/commit/6925f413a28b1cc8e6a17f3f94e3922c37b92e66)
- **Did:** `own_terrain: true` filled a FinePatch all at once (Durham shaders + `generate_terrain`, no `BaseTerrainGenerationPlugin`). With a development focus it used the 300 m occupancy lattice instead of hopscotch leaves, then batch-spawned hosts. `urbanization: None` was the default.
- **Replacement:** The streamed crate that replaced it was itself retired at `4f1ff3b7` (above). Restore the batch path from `6925f413` if the lattice-without-hopscotch catalog is needed again.

### `playgrounds/terrain` (`terrain-playground`)

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Early SDF marching-cubes terrain LOD chunk viewer (`engine` + `procedures/terrain`).

### `playgrounds/objects` (`objects-playground`)

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Mesh inspection for walls, trees, and groves on the pre-Chico `vegetation-sdf` / `procedures/buildings` stack.

### `playgrounds/skill-map` (`skill-map-playground`)

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Skill-map demo (fireballs on pink squares, lock on blue) reusing the objects playground stack.

### `playgrounds/pathfinding` (`pathfinding-playground`)

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** 2D local pathfinding: a red agent steers around a wall toward the cursor via `procedures/intelligence`.

### `demos/naturescapes` (`naturescapes-demo`)

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Navigable naturescapes: procedures terrain SDF, Durham water shaders, and `vegetation-sdf`. Ancestor of Durham / Chico world composition.

### `engine`

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Shared Bevy chunk manager, marching cubes, and outline/leaf shaders used only by the trees above.

### `procedures/terrain`, `procedures/vegetation`, `procedures/buildings`, `procedures/skill-map`, `procedures/intelligence`

- **Last commit:** [`9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47`](https://github.com/ramate-io/maybraid/commit/9a9a74c6901ed4d7799a4e87d16f74d7dd3b9e47)
- **Did:** Pre-Maybraid generation: 2.5D height-oracle terrain SDF ([RFC-105](../rfc/rfc-000-000-105-procedural-terrain/README.md)), ball-stick `vegetation-sdf`, early building meshes, skill-map noise, and local pathfinding. Replaced by Durham, Chico, Richmond, and `maybraid/intelligence`.
- **Moved:** `comproc` (guillotine + noise) now lives at [`maybraid/procedural/comproc`](procedural/comproc/). SDF / ball-stick modules that only served the old stack were dropped.
