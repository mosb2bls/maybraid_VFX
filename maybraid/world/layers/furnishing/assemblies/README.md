# Furniture assemblies

Chico-shaped furniture kits: `BedParams` / `ChairParams` / `ChestParams` / `CounterParams` / sit-on and kitchen params → `build()` → exploded parts. `unit_from_num` changes [`MaterialRef`](../../../../material-ref/) palette only.

Parts are floor-origin slabs in the unit slot cube, then [`furniture-components`](../components/) maps each authored GLB through `kit_to_unit` and the Richmond `FurnitureNode` placement (slot yaw already carries abutment facing). Covers use the same transform as the mattress.

[`generation`](src/generation.rs) walks Richmond [`BuildingComponents`](../../urbanization/buildings/components/src/lib.rs) High furniture slots and expands [`FurnitureUsageNode`](../../urbanization/buildings/components/src/furniture/usage.rs) regions through [`furniture-usage-areas`](../usage-areas/). [`maputo`](../maputo/) bins those slots into 50 m [`FurnitureCell`](../maputo/src/host.rs) hosts and presents a neighborhood around the camera (one cell generate / present / despawn per frame). Keep / Gimme refresh follow the camera on Y so hosts on Durham (`y ≈ -140`) stay in the query. Each host's `scene_chunks` is one lazy slot that expands to one weighted chunk per painted GLB. Do not parent kits under Richmond development hosts.

Do not `MultiSceneMerge`. One `SceneRef` per part.
