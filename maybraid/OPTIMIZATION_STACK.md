# Optimization stack

Where CPU time still goes after the Gimme host split, and the order of work
that follows from Tracy — not a backlog of every possible render trick.

Related: [#800](https://github.com/ramate-io/maybraid/issues/800),
[#801](https://github.com/ramate-io/maybraid/issues/801),
[#802](https://github.com/ramate-io/maybraid/issues/802),
[#803](https://github.com/ramate-io/maybraid/issues/803).

## Already landed

| Step | What changed | Why it mattered |
| --- | --- | --- |
| Slim baseline / [#800](https://github.com/ramate-io/maybraid/issues/800) | Typed LOD refresh domains, less work in `Update` | Late-flight `Update` ~11.6 ms vs ~26.5 ms on `main` |
| [#803](https://github.com/ramate-io/maybraid/issues/803) | Flattened character visuals | Fewer nested visual hosts on the player |
| [#802](https://github.com/ramate-io/maybraid/issues/802) | `GimmeLodSceneHostIndex` for refresh / cull | Host cuboids left the Avian broadphase. Produce fill no longer climbs with collider count. |
| Shared produce fill | One `fill_lod_produce_cache` per host index; snapshots every `LodNode` | Vegetation `With<Camera>` and mob `With<LodViewer>` no longer each walk the index. Tracy should show **one** produce fill. Expected `Update` win ~1–1.5 ms vs the dual-fill captures. |
| Flatten building kits | `ComponentsOnly` High/Medium posed kits, not nested panel hosts | Urban host count dropped. `write_binned` still saw per-tile `Mesh3d`s. |
| Shared cull fill | One `fill_lod_cull_produce_cache` per host index; snapshots every `LodNode` | Same dual-`F` footgun as produce. After sharing, gameplay cull fill is **0.65 ms** (`more_rough_gameplay.csv`). |
| Stairs / doors on Medium | Circulation is High **and** Medium; furniture / labels stay High-only | Nested stair hosts used to keep their own band; flatten dropped the whole shell to Medium and hid stairs. |
| Unmerge shared wall kits | Stop baking city walls into [`MultiSceneMerge`](scene-ref/src/multi_merge.rs); leave posed [`SceneRef`](scene-ref/src/scene_ref.rs)s | Shared wall GLBs already instanced. Baking per-building layouts made unique meshes × 1 instance. `write_binned` / `visibility_propagate` got worse (`more_rough_gameplay.csv`). |
| Disable off-band LOD trees | Ready off-band / cull / present-hide trees get recursive [`Disabled`](https://docs.rs/bevy/latest/bevy/ecs/entity_disabling/struct.Disabled.html), not only [`Visibility::Hidden`](https://docs.rs/bevy/latest/bevy/render/view/enum.Visibility.html). Reveal prunes independently hidden nested trees; pending fulfill stays Hidden-only. Vis sync writes only on change. | Hidden warm roots still walked `visibility_propagate` / extract. `Disabled` drops them from default queries without a parent reveal enabling nested off-band roots. Last ~20s window (`last_30.csv`) had `visibility_propagate` **~3.3 ms**. |
| Cache present-cull keep ids | [`drain_lod_present_cull`](lod/lib/src/presentation/runtime.rs) rebuilds the keep [`HashSet`](https://doc.rust-lang.org/std/collections/struct.HashSet.html) only when the live keep AABB or [`SpatialIndex::membership_revision`](lod/lib/src/gen/spatial_index.rs) changes | Grove / canopy drains were paying `tracked_ids_for` + HashSet every frame (`last_30.csv` ~0.78 ms canopy + ~0.58 ms grove) while the ring was still. |
| Consume cullable-root marker | Region enqueue still lowers a stale desired level, then walks roots only if the level just dropped or [`LodHostHasCullableRoots`](lod/lib/src/scene/refresh/cull_regions/markers.rs) is present. Mob full-scan cull filters on the marker. Drop unused [`LodProduceCache`](lod/lib/src/scene/refresh/levels/produce.rs) `hit_entities` and the unused per-region cull-hit `Vec`s | Marker was written every sync and never read. Cull fill also stored both a per-tile `Vec` and a union set; only the union is scheduled. |
| Throttle scene-cull lattice | [`LodCullProduceCadence`](lod/lib/src/scene/refresh/cull_regions/cursor.rs) (vegetation: every 4 frames). Cursor does not advance on skipped ticks | Default OpenLattice is 1 km hole / 5 km outer / 500 m tiles → 112 cells. 1 tile/tick at 60 FPS is a ~1.9 s sweep and a cull-fill every frame (~0.78 ms). Every 4th frame is ~7.5 s and drops three of four fill ticks. |
| Event-drive padded terrain | [`generate_urbanization_padded_terrain`](world/layers/urbanization/richmond-playground/src/urbanization_stream.rs) / [`present_urbanization_padded_terrain`](world/layers/urbanization/richmond-playground/src/urbanization_stream.rs) skip when keep AABB, store / terrain revision, and (for present) 8 m viewer quant are unchanged. Present reuses one tracked-id set | `last_30.csv` paid ~0.32 ms generate + ~0.55 ms present every frame. Dirty-pad invalidation still runs; a removed pad forces generate. |

Crate layout after the split: [`lod`](lod/lib/) is the engine-agnostic runtime, [`lod-gimme`](lod/gimme/) owns the host index and refresh/cull plugins, [`lod-avian`](lod/avian/) keeps physics layers. Call sites use `gimme_host!`; unused `avian_host!` wrappers remain.

Capture `pre-alpha-custom-spatial-index-migration.tracy` (crate split, 4:05, 11 295 frames, ~46 FPS) shows **no regression** vs `pre-alpha-custom-spatial-index.tracy` (~44 FPS, `Update` 11.7 ms): `Update` **11.0 ms** mean, produce fills 1.34+1.74 ms, cull fill 1.03 ms, `reindex_moved_hosts` 0.18 ms. Fill systems name `lod_gimme::host::GimmeLodSceneHostIndex`.

Generate and present **ids** already live on typed [`SpatialIndex`](lod/lib/src/gen/spatial_index.rs)
resources (`ForestIndex`, urbanization, terrain, …). They were never the
Host-query death spiral. The unused Avian Generate / Present volume API is
deleted so those layers cannot be re-wired as Host-shaped colliders.

## What Tracy actually shows

Capture `pre-alpha-custom-spatial-index.tracy`: ~5 min, 13 058 frames, ~44 FPS
whole-flight. A ~140-frame late window (`rough_frames.csv`) is the better read
of “are we falling over.”

| Zone | Whole capture mean | Late ~140 frames |
| --- | ---: | ---: |
| `Update` | 11.7 ms | **10.2 ms** |
| `fill_lod_produce_cache` (`With<Camera>`) | 1.85 ms | **2.12 ms** |
| `fill_lod_produce_cache` (`With<LodViewer>`) | 1.41 ms | **1.66 ms** |
| `fill_lod_cull_produce_cache` | 1.58 ms | **1.89 ms** |
| `write_binned_instance_buffers<Opaque3d>` | 1.91 ms | **1.50 ms** |
| `check_visibility_cpu_culling` | 1.25 ms | **0.18 ms** |
| `PhysicsSchedule` | 1.14 ms (max 11 ms) | **0.12 ms** |
| `reindex_moved_hosts` | 0.18 ms | 0.19 ms |
| `sync_stick_colliders` | 0.077 ms | 0.062 ms |

Whole-capture visibility / collider / dirty-tree heat is load-in and hitch
**maxes**. Those dual-fill captures are the pre-shared-fill baseline. The live
budget after this pass should be **one** produce fill + cull fill, then opaque
instance write, then present drains. Gimme reindex is cheap. Physics is quiet
once spawned.

`fill_lod_produce_cache` is **once per host index**. Region production still uses
`With<Camera>` (vegetation) vs `With<LodViewer>` (mobs); fill snapshots every
`LodNode` so those filters cannot instantiate two `cache.clear()` walks.
`fill_lod_cull_produce_cache` is the same: once per index, every node. The
gameplay-window ~1.9 ms was already a **single** Camera walk (vegetation and
buildings share that filter). Sharing plus flatten dropped the named cull fill
to **0.65 ms**; it is no longer the lever.

Gameplay window (`rough_gameplay.csv`, ~270 frames after flatten + shared produce,
before wall merge): `Update` **7.4 ms**, produce **0.41 ms**, cull fill
**1.94 ms**, `write_binned` Opaque3d **1.98 ms**.

Gameplay window (`more_rough_gameplay.csv`, ~288 frames after wall merge):
`Update` **8.0 ms**, produce **1.13 ms**, cull fill **0.65 ms**,
`write_binned` **2.18 ms**, `visibility_propagate` **2.19 ms**,
`check_visibility` **0.61 ms**. Merge unique-baked the instance set. This
pass poses kits again; recapture should send `write_binned` back toward
**~1.5–2.0 ms**.

## Next work (this order)

### 1. Hygiene — Avian Generate / Present volumes

**Status: done in this crate pass.**

[`lod-avian`](lod/avian/) no longer exposes `AvianLodGenerateIndex`,
`AvianLodPresentIndex`, or Generate / Present marshallers and physics layers.
`PatchSceneBounds` stamps [`GimmeLodHostMarshaller`](lod/gimme/src/host.rs)
from [`lod-gimme`](lod/gimme/); those Avian volumes were not live dummy colliders. This does **not**
move the ~10 ms `Update`. It removes the footgun of putting Host-shaped
cuboids back onto generate / present.

Leftover: `AvianLodSceneHostIndex` / `AvianLodSceneBoundsMarshaller` still exist
as an unused Avian Host query path. Refresh plugins do not install them.

### 2. One shared produce fill

**Status: done in this crate pass.** Re-measure with Tracy: one
`fill_lod_produce_cache` zone, not Camera + LodViewer.

### 3. Flatten High / Medium building kits

**Status: done in this crate pass.** [`building_scene_chunks`](world/layers/urbanization/buildings/components/src/lib.rs) drains posed kits (weight 4, lazy) instead of a nested [`LodScene`](lod/lib/src/scene/lod_scene.rs) host per panel / partition / floor / …. Wizard’s Tower emit paths use the same flattened append. Fine-phase `PanelNode` plugins stay registered for leftover nested hosts.

### 4. Shared cull fill

**Status: done in this crate pass.** [`fill_lod_cull_produce_cache`](lod/lib/src/scene/refresh/cull_regions/cache.rs) is once per host index and snapshots every [`LodNode`](lod/lib/src/lod_ref/node.rs). Region production still filters drivers. Re-measure: one cull-fill zone (no Camera vs LodViewer pair).

### 5. Merge shared wall kits — reverted

**Status: reverted in this crate pass.** Same-kit wall tiles were already one
[`SceneRef`](scene-ref/src/scene_ref.rs). `write_binned` instanced them.
[`MultiSceneMerge`](scene-ref/src/multi_merge.rs) bakes transforms into a new mesh;
each building layout is a unique key → unique meshes × 1 instance. That is the
unique-merge failure mode at building grain.

Vegetation [`CollectionPresent::Merge`](world/layers/vegetation/components/src/foliage/present.rs)
is still the right merge: a **cacheable collection key** (cheap-balls / sticks),
not “all stone rectangles on this tower.”

Re-measure after unmerge: `write_binned` Opaque3d, `visibility_propagate`. Cull
fill should stay ~0.65 ms. Stairs should remain visible at Medium.

### 6. Disable off-band LOD trees (this crate pass)

**Status: implemented in this crate pass.** Recapture `visibility_propagate`.

Woody plants are posed kits under the **grove tile** host
([`nest_flattened_plant_chunk`](world/layers/vegetation/groves/src/grove/vc_compose.rs)) — not a
host per tree, and not a host per frond / stick. Isolated `/show` still uses one
plant host. Binning tiles into quadrants is a different lever and is **not**
this step.

The last-window vis cost is the **cardinality of plant/grove trees still in
Bevy’s visibility walk**, including Hidden warm roots. Rewriting `Hidden` every
frame also dirtied `Changed<Visibility>` and re-walked those subtrees.

Ready off-band, cull-inflight, and present-hide trees now stamp recursive
`Disabled` (Bevy does not cascade `Disabled` to children unless asked). Pending
fulfill roots stay Hidden-only so streamed children still enter
`visibility_propagate`, and pending cull never recursively enables descendants.
Each independently hidden root carries one private hide marker. Reveal walks the
tree it must re-enable but prunes at nested hide markers; a reveal beneath a
still-hidden ancestor defers removal until that ancestor reveals. There is no
per-descendant ownership component. Produce fill and Gimme
`reindex_moved_hosts` do **not** use `Allow<Disabled>`. Gimme intentionally
retains an existing host AABB while the host is Disabled (warm index, no
reveal-time reindex); refresh gates any such stale spatial hit before producing
work.
Raw Durham roots replaced by padded urban terrain also update `Visibility` only
when their replacement state changes, rather than dirtying
`Changed<Visibility>` every frame.

Do **not** unique-merge city walls. Do **not** widen High vegetation bands to
cut vis (more entities on-screen). Development-pad `exclusion_zones` still
help urban look; they do not replace this experiment.

### 8. Pre-quadrant runtime skips (this crate pass)

**Status: implemented in this crate pass.** Recapture present-cull, cull fill,
and padded-terrain zones before vegetation quadrants.

These four skip the leftover every-frame walks that still showed in
`last_30.csv` after off-band `Disabled`:

1. **Present-cull keep membership.** Grove and canopy `tracked_ids_for` already
   walk cell grids; the HashSet collect was still every tick. Cache by live
   keep AABB + index membership revision (`0` = unknown, always rebuild).
2. **`LodHostHasCullableRoots`.** Fill still snapshots every host in the tile
   (level lower needs them). Enqueue skips the bag walk unless the desired
   level just dropped or the host already has a non-current root. Full-scan
   mob cull filters on the marker. Produce-cache `hit_entities` and per-region
   cull `Vec`s were write-only.
3. **OpenLattice cadence.** Vegetation emits one tile every 4 frames. Tests and
   other playgrounds stay at 1. Rotating sweep is unchanged — only the clock.
4. **Padded urban terrain.** Generate still invalidates dirty pads, then skips
   `get_or_generate` when the keep AABB and store / terrain revisions match.
   Present skips when those plus an 8 m viewer quant match, and reuses one
   tracked-id set for band present + raw-root hide.

Do **not** unique-merge city walls. Do **not** implement vegetation quadrants
until Tracy says vis / `write_binned` still dominate after this pass.

### 7. After recapture

If `visibility_propagate` is still the late-window floor:

- Grove **tile** host count (`chico_grove_host_spawn`) vs plant flatten already
  done.
- Near-field present cull / fewer warm holds, not extra concentric bullseyes
  (level produce is already cell-gated).
- Vegetation **quadrants** (bin plant/grove hosts), not wider High bands.

Do **not** unique-merge city walls further.

## Not next (and why)

- **Moving generate onto Gimme for FPS.** Generate drains are tiny in the late
  window. Produce is already Gimme. The remaining fill cost is snapshot
  collection + host set size.
- **Caching stick compounds for frame time.** [`StickPhysicsAttached`](world/layers/vegetation/chico/src/stick_physics.rs)
  already stamps once (budget 8 hosts/frame). Late `sync_stick_colliders` is
  ~60 µs. Hitch maxes are **presence** in Fixed (and parenting under a dirty
  `GlobalTransform`), not rebuild. Near-field physics is the lever, not a
  recipe cache.
- **Attacking instances by making unique merged patches.** That explodes GPU
  memory and was the wall-merge regression. Share first (quantize / instance
  the same handle), then fold cardinality (fewer hosts). Vegetation merge stays
  collection-keyed.
- **Tighter `build_bone_maps` clamp.** It already rebuilds only when `Children` /
  `Name` change under that [`RigRoot`](rigs/src/bone_map.rs). The gameplay 0.92 ms
  is character-rig fulfill, not a leftover every-frame walk. A clamp would hide
  spawn hitch, not the steady budget.
- **`chico_grove_growth` in the mean FPS budget.** That zone is async
  `ChicoGrove::ensure_grown` (cap 4 tasks) plus one host spawn per present
  quantum — present hitch when tiles finish growing, not the 7.4 ms Update.

## How to re-measure

Prefer Tracy over hitch loggers.

1. `tracy-capture` a several-minute flight that includes urban.
2. Whole-capture CSV for maxes / load-in.
3. A late-window export (as `rough_frames.csv`) for the steady budget.
4. Compare `Update`, produce fill (**one** zone), cull fill (**one** zone),
   `drain_lod_present_cull` (grove / canopy), padded generate / present,
   `write_binned` Opaque3d, `visibility_propagate`, and `reindex_moved_hosts`.
   A late gameplay slice (~20s, `last_30.csv`) is the better read than
   whole-capture means. Cull fill should fire about every 4th frame.
