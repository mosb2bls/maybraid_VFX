# Structural LOD collectors (models)

This note describes a **future** presentation layer for bulk internal/external geometry in `-models` crates. It is not implemented yet. Today, Richmond uses per-node [`ParentConfines`](../../world/layers/urbanization/buildings/components/src/parent_confines.rs) and [`LodSceneHost`](../lib/src/scene/host.rs) switches (Wizard’s Tower, partition mesh tiers).

## Fine phase vs broad phase

Camera-driven LOD today runs a **fine phase**: for each relevant host / confined node, evaluate bounds / `scene_lod_level` / confines and update `LodSceneLevel` or visibility.

A later **broad phase** should decide *which* hosts enter that fine phase (region interest, cascade cells, stream-in sets). Do not conflate the two: fine-phase checks stay correct but expensive at city scale without a broad cull.

## Why collectors

Per-node `ParentConfines` is a high-performance fast path for a single building. At city scale, models may want **layer collectors** similar to hydro / terrain cells:

- An `InternalStructures` (name TBD) spatial index gathers internal walls, floors, furniture, etc.
- An external / silhouette layer gathers façade primitives.
- Presenters toggle whole layers (or cell batches) instead of walking every authored hierarchy.

## Boundary ownership (min-corner)

Structural primitives often **cross cell boundaries**. Collectors must not double-present them.

**Rule:** each primitive has one canonical owner cell — the cell containing its **lower-left-southern** corner (minimum of its AABB, or an explicit authoring origin). Only that cell’s collector emits the primitive.

Ownership for **collection** is separate from **LOD confine**. A wall can be owned by cell A and still carry `ParentConfines::Internal(parent_ball)` for when it draws.

## Leveraging this in models

1. Keep IR nodes (`WallNode`, `FloorNode`, …) as the authored truth, including `ParentConfines`.
2. On materialize, register each primitive into the appropriate collector cell using min-corner ownership.
3. Present collectors with `LodScene` / hosts: coarse levels hide entire internal layers; fine phase (and later broad phase) refine what remains.
4. Prefer `ParentConfines` on nodes until duplication/boundaries force a lattice; then add collectors without replacing the confine policy.

## Related

- [Richmond CONTRIBUTING — LodScene](../../world/layers/urbanization/CONTRIBUTING.md#lodscene-on-buildings)
- [`LodScene`](../lib/src/scene/lod_scene.rs) and [`lod::hcsg::PresentationPlugin`](../lib/src/hcsg/presentation.rs)
