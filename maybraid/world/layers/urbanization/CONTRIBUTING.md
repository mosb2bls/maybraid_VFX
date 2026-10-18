# Contributing to Richmond

Richmond is the cellular urbanization stack: footprints, rooms, ornamental kit,
and building composition. This document covers how **buildings** should author
and present geometry on top of [`building-components`](buildings/components/).

## Crates

| Crate | Role |
|-------|------|
| [`building-components`](buildings/components/) | Domain IR + kit assets. Authoring types are `*Node` values (`FloorNode`, `PartitionNode`, `StairNode`, `DoorNode`, `RoofNode`, `PanelNode`, `FurnitureNode`, `LabelNode`): **style + geometry + placement** (+ optional [`ParentConfines`](buildings/components/src/parent_confines.rs); labels also carry a debug string). Each node implements [`LodScene`](../../../lod/lib/src/scene/lod_scene.rs). Tessellation into kit pieces is private to the domain. Partition IR is primitive (no portals). Labels render as colored wireframes; face text is a playground gizmo pass (scaled/wrapped to each face). |
| [`buildings`](buildings/) | Building procedures. Compose constraints, layouts, and helpers (`paneling` / `arcs` / `portals`, `ArcSpire`, …) into domain nodes via [`BuildingComponents`](buildings/components/src/lib.rs). Present component-only buildings as [`ComponentsOnly`](buildings/components/src/lib.rs)`<T>` for `LodScene`; keep a custom `LodScene` when hosts, silhouettes, or non-node extras are required. Playground joinery demos live under `wall_demo`. |
| [`buildings-playground`](buildings/playground/) | Preview / CLI. Spawns hosts once; LOD flips update [`LodSceneLevel`](../../../lod/lib/src/scene/level.rs) in place (no whole-tree despawn). |

Shared pose helpers (`pose`, `posed_glb`, `with_pose`, `scene_children`, `append_component_scenes`, `ComponentsOnly`) live in building-components and should not be reimplemented per building.

## Authoring model

Buildings **emit authored domain types**; they do not tessellate kit pieces or call style→GLB mapping themselves.

```text
constraints / layout helpers
        │
        ▼
  BuildingComponents (*_nodes_for_level)
        │
        ├── ComponentsOnly<T> → LodScene (component-only)
        └── custom LodScene (host / silhouette / lights)
              optionally via append_component_scenes
```

Preferred shape for a storey or room:

```rust
use building_components::{BuildingComponents, Layers};

pub struct ExampleFloor {
    pub floors: Vec<FloorNode>,
    pub partitions: Vec<PartitionNode>,
    pub stairs: Vec<StairNode>,
}

impl BuildingComponents for ExampleFloor {
    fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
        Layers::from_free(self.floors.clone())
    }
    fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
        Layers::from_free(self.partitions.clone())
    }
    fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
        Layers::from_free(self.stairs.clone())
    }
}

// Present: ComponentsOnly(&example).scene_with_lod(lod_ref)
```

Parents **merge** children’s layered node maps via [`Layers::extend`](buildings/components/src/layer.rs), they do not nest child `LodScene`s when both sides are component-only.

[`Layer`](buildings/components/src/layer.rs) is a **provenance** record (e.g. `"closet"`, `"envelope"`), not a stand-in for node type. Domain type stays on the trait method (`panel_nodes_for_level` vs `partition_nodes_for_level`). Higher-order types use layer names to decide what to do with that geometry; use [`Layers::free`](buildings/components/src/layer.rs) until a provenance label is useful.

Helpers such as [`portal_ring_wall`](buildings/lib/src/arcs/portal_ring.rs) / paneling strips / `ArcSpire` are fine when they **produce** `PartitionNode` / `PanelNode` / `StairNode` via `BuildingComponents`. Use **partition** for arc/linear kit IR, **panel** for rectangle/triangle kits, and **portals** for opening assignment along a path. Door leaves stay empty until portal → `DoorNode` authorship exists.

## Allocate cells, fill in children

Higher-order room types own **layout**: they `subset` child AABBs from [`CellConstraints`](buildings/lib/src/constraints.rs) and construct lower-order types. Lower-order types own **fill**.

Constructors take the child's [`CellConstraints`](buildings/lib/src/constraints.rs). Do not pass a parent `&CellConstraints` “for context”.

Residential program fill now lives under
[`usage_areas`](buildings/lib/src/usage_areas.rs) (`CommonBedroom`, livable quarters)
via the Fit / parameterized → plan path and the shared
[`placer`](buildings/lib/src/placer.rs) KindSpec trier — not hierarchical
`CellConstraints` bedroom trees.

## `LodScene` on buildings

`LodScene` is a compatibility alias for [`SemanticLodScene`](../../../lod/lib/src/scene/lod_scene.rs) (main-world spawn). Per-view render LOD is [`VisualLodScene`](../../../lod/lib/src/scene/lod_scene.rs); do not put packed forest in `scene_chunks_with_level`.

Most buildings should implement [`BuildingComponents`](buildings/components/src/lib.rs) and present via [`ComponentsOnly`](buildings/components/src/lib.rs)`<T>`. High / Medium drain **posed kits** ([`building_scene_chunks`](buildings/components/src/lib.rs) / [`flattened_component_scene`](buildings/components/src/lib.rs)) — one structural host, not a nested `LodScene` per panel. Fine-phase node types stay `LodScene` for playgrounds and leftover nested hosts.

Types with silhouettes, lights, or late-bound [`ParentConfines`](buildings/components/src/parent_confines.rs) (e.g. Wizard’s Tower) implement `BuildingComponents` and keep a custom `LodScene`. Prefer [`append_flattened_component_scenes`](buildings/components/src/lib.rs) for the node portion. [`append_component_scenes`](buildings/components/src/lib.rs) still nests `LodScene::host` when you need a per-node host.

`LodScene` methods:

- `scene_lod_level` — desired [`LodSceneLevel`](../../../lod/lib/src/scene/level.rs) (cheap).
- `scene_lod_status` — `Unchanged` or `Changed(level)`.
- `scene_lod_culls` — inactive [`LodLevelRoot`](../../../lod/lib/src/scene/host.rs)s this type is willing to **despawn** ([`LodSceneCulls`](../../../lod/lib/src/scene/cull.rs); default `None` keeps roots warm). Prefer helpers ([`cull_non_adjacent_bands`](../../../lod/lib/src/scene/cull.rs), [`cull_offset_bands`](../../../lod/lib/src/scene/cull.rs) / [`cull_bands_with_adjacent_depth`](../../../lod/lib/src/scene/cull.rs), [`cull_named_from_factor`](../../../lod/lib/src/scene/cull.rs)) over ad-hoc lists; “not current” alone is not a cull reason. Host GC never despawns the current level. After despawn, Sync + Fulfill bring the desired level back via `scene_with_level` (same as first spawn). **Do not casually cull the immediately adjacent band** — re-entering it forces an expensive respawn; prefer non-adjacent GC, or offset bands (halfway in) only when the adjacent root is heavy.
- `scene_with_level` — primary builder for one level root.
- `scene_with_lod` — first present via [`lod_host_scene`](../../../lod/lib/src/scene/host.rs).

Hosts flip level-root visibility / lazily spawn missing roots. Nested hosts are independent. World buildings should not nest a host per kit — structural `ComponentsOnly` / flattened append is the urban path.

**Refresh pass:** [`LodRefreshCorePlugin`](../../../lod/lib/src/scene/refresh.rs) tracks [`LodNode`](../../../lod/lib/src/lod_ref.rs) poses. Prefer the Avian region stack (Bullseye / Spotlight → index → levels → chunk sync; OpenLattice cull) as in the buildings and sbs-trees playgrounds, or [`LodSceneRefreshPlugin`](../../../lod/lib/src/scene/refresh.rs) generally. Hosts present via [`lod_host_scene_pending`](../../../lod/lib/src/scene/host.rs) / [`LodScene::host`](../../../lod/lib/src/scene/lod_scene.rs) and override [`scene_chunks_with_level`](../../../lod/lib/src/scene/lod_scene.rs) when content should amortize — see [scene chunks](../../../lod/docs/scene-chunks.md). Cameras are playground-only (`LodViewer` requires `LodNode` on the fly-cam). See also [structural LOD collectors](../../../lod/docs/structural-lod-collectors.md).

### `ParentConfines` (building-components only)

[`ParentConfines`](buildings/components/src/parent_confines.rs) is an **IR field** on nodes — not part of general `lod`:

- `External` — façade / silhouette candidates; normal distance/extent mesh banding.
- `Internal(InternalShape)` — detail gated by [`INTERNAL_REVEAL_FACTOR`](buildings/components/src/parent_confines.rs) (`5`) × radius:
  - [`InternalShape::Ball`](buildings/components/src/parent_confines.rs) — **floor- or room-compartment** ball. Prefer authoring at this grain.
  - [`InternalShape::Capsule`](buildings/components/src/parent_confines.rs) — long non-compartmentalized volumes (e.g. one continuous vertical spire); distance to the medial segment.

Do **not** hang one Internal ball on an entire multi-storey building. Pass the compartment footprint — do not pre-multiply by the reveal factor.

### Wizard’s Tower levels

LOD uses **capsule surface distance** (meters outside a vertical footprint capsule through the full tower AABB). Tall height no longer inflates the Low cutoff:

| Level | Content | Distance |
|-------|---------|----------|
| High | Exterior + per-storey internals + spire capsule | ≤ 5 × footprint radius |
| Medium | Exterior walls only | ≤ [`LOW_RES_CUTOFF_METERS`](buildings/lib/src/wizards_tower/tower_lod.rs) (raw world meters) |
| Low | Cylinder silhouette | beyond |

Scale-dependent [`ParentConfines`](buildings/components/src/parent_confines.rs) may still reveal internals inside High even when that radius reaches farther than a short capsule-based feel — that clash is acceptable.

### Partition mesh resolution

Warm high/mid/low SceneRef roots under a **single** partition-node host. Parent banding uses linear factors (`distance / max_extent`):

| Band | Factor (linear / polyline parent) |
|------|--------|
| High | ≤ [`LINEAR_HIGH_FACTOR`](buildings/components/src/partitions/geometry/linear.rs) (5) |
| Medium | ≤ [`LINEAR_MEDIUM_FACTOR`](buildings/components/src/partitions/geometry/linear.rs) (20) |
| Low | ≤ [`LINEAR_LOW_FACTOR`](buildings/components/src/partitions/geometry/linear.rs) (500) |
| UltraLow | elsewhere (shares low mesh for now) |

**Polyline** is a short-run primitive: one LOD parent for the whole run (kits are content, not nested hosts). Prefer splitting long paths in buildings/paneling. Joint kits under a polyline follow the parent level (high/mid GLBs only; omitted at Low). Lone joint leaf banding uses tighter factors (High ≤ 3, Medium ≤ 12).

### Roof mesh resolution

Roof kits use the same distance / extent probe shape ([`RoofLodProbe`](buildings/components/src/roofs/lod.rs)) but **tighter** High / Medium thresholds than walls:

| Band | Factor |
|------|--------|
| High | ≤ [`ROOF_HIGH_FACTOR`](buildings/components/src/roofs/lod.rs) (2) |
| Medium | ≤ [`ROOF_MEDIUM_FACTOR`](buildings/components/src/roofs/lod.rs) (3) |
| Low | ≤ [`ROOF_LOW_FACTOR`](buildings/components/src/roofs/lod.rs) (8) |
| UltraLow | elsewhere (shares low mesh for now) |

### Panel mesh resolution

Panel kits reuse the roof distance factors ([`PANEL_*_FACTOR`](buildings/components/src/panels/lod.rs)) but treat UltraLow as a **dedicated** host root: every style swaps to the shared flat low-res rectangle / right-triangle GLB (`urban/panels/flat/…_low_res.glb`).

| Band | Content |
|------|---------|
| High / Medium / Low | Style triad (`*_high_res` / `*_mid_res` / `*_low_res`) |
| UltraLow | Flat low-res ([`PANEL_ULTRA_LOW_RECTANGLE`](buildings/components/src/panels/lod.rs) / [`PANEL_ULTRA_LOW_RIGHT_TRIANGLE`](buildings/components/src/panels/lod.rs)) |

Shared mapping lives in [`lod_band`](buildings/components/src/lod_band.rs). Fine-phase `update_*_host_levels` still run for leftover nested node hosts (playgrounds). World `ComponentsOnly` buildings pick kit GLBs from the **structural** High / Medium / Low root via `scene_with_level`.

## Internal vs external emission

At **High**, each floor/room emits its own Internal ball for compartment geometry. Continuous vertical features (the tower spire) share one [`ParentConfines::capsule`](buildings/components/src/parent_confines.rs) (`Internal(Capsule)`) so higher storeys do not pop in awkwardly while you are inside the shaft. **Medium** omits internals from the scene.

```rust
fn emit_internal_features(
    &self,
    children: &mut Vec<Box<dyn Scene>>,
    lod_ref: &LodRef,
) {
    let confines = ParentConfines::internal(
        self.storey_confine_center(),
        self.storey_confine_radius(),
    );
    for floor in &self.floors {
        children.push(Box::new(
            floor.clone().with_confines(confines).scene_with_lod(lod_ref),
        ));
    }
}
```

## What not to do

- Do not put `ParentConfines` in the general `lod` crate.
- Do not despawn a whole building host on LOD flips — update `LodSceneLevel`.
- Do not treat “camera moved” as `Changed` by itself.

## Related reading

- [building-components README](buildings/components/README.md)
- [buildings README](buildings/lib/README.md) (urban kit taxonomy for higher-order authorship)
- [buildings CONTRIBUTING](buildings/lib/CONTRIBUTING.md) (Les Halles parameterized → plan → full / openings / usage areas)
- [Urban art README](../../../art/urban/README.md)
- [`LodScene`](../../../lod/lib/src/scene/lod_scene.rs)
- [Structural LOD collectors](../../../lod/docs/structural-lod-collectors.md)
- [Maybraid contributing: `-models` crates](../../../CONTRIBUTING.md#-models-crates)
