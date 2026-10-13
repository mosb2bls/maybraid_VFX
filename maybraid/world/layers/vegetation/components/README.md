# `vegetation-components`

Domain IR for Chico vegetation: **geometry + placement → node (`LodScene`)**.

Higher-order trees implement [`VegetationComponents`](src/lib.rs) and present via [`FlattenedComponentsOnly`](src/lib.rs)`<`[`PlacedVegetation`](src/placed_vegetation.rs)`<Arc<T>>>`, mirroring Richmond's [`BuildingComponents`](../../urbanization/buildings/components/). Tuft groves without [`LodScene`](../../lod/lib) yet still use [`ComponentsOnly`](src/lib.rs).

## Domains

| Node | Role |
|------|------|
| [`StickNode`](src/sticks.rs) | Trunk / branch segments. Kit: \(Y \in [0, 1]\), \(X = Z \in [-0.2, 0.2]\). |
| [`FoliageNode`](src/foliage.rs) | Canopy leaves (layered ball, cheap ball, frond, collections). |

Stick [`StickGeometry::{Segment,Trunk}`](src/sticks/geometry.rs) picks the GLB triad under `maybraid/assets/vegetation/sticks/standard/` (`001_*` vs `trunk_001_*`) and the nested mesh-LOD extent policy. Foliage [`FoliageGeometry`](src/foliage/geometry.rs) picks the kit under `maybraid/assets/vegetation/foliage/standard/` (layered ball, cheap ball, frond, or a collection of those).

## Apps

Add [`VegetationProceduralPlugin`](src/procedural.rs) before spawning vegetation scenes (registers unit stick/ball meshes and fulfills plane-splay foliage).
