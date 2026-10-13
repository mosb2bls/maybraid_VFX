# Furniture

Painted assemblies that fill Richmond [`FurnitureNode`](../urbanization/buildings/components/src/furniture/node.rs) slots. Generate is a pass over building High slots ([`collect_furniture_slots`](assemblies/src/generation.rs)). World present is a neighborhood of 50 m [`FurnitureCell`](maputo/src/host.rs) hosts — flattened kits, not Richmond children. Each host fulfills one painted GLB per chunk quantum; generate / present admit one cell per frame. Keep / refresh AABBs are camera-centered on Y (Durham is not at sea level) and overlap developments on XZ.

[`Maputo`](maputo/) is the furnishing model: it reads [`FurnitureSlots`](maputo/src/slots.rs) from `Urbanization<Richmond<_>>` and bins those slots into 50 m cells. Vegetation and mobs stay on urbanization; nothing wraps `Furnishing<_>`.

- [`components`](components/) — kit `AssetPath`s, Blender→engine remap, posed `SceneRef` + `MaterialRef`
- [`assemblies`](assemblies/) — `FooParams` → `build()` → exploded parts
- [`usage-areas`](usage-areas/) — packed usage regions (`BitesCounter`, `BitesKitchen`) → furniture ensembles
- [`shaders`](shaders/) — BotW-warm `MaterialRef` recipes (wood, cloth, marble, ornate)
- [`maputo`](maputo/) — 50 m cell index, generate stream, and presentation
- [`playground`](playground/) — isolated `/show` catalog (unit kits + Richmond slot gallery)

Blender sources live in [`art/furniture`](../../../../art/furniture/) ([`ecb5a8a`](https://github.com/ramate-io/maybraid/commit/ecb5a8a31d2894311fa690cfbe8ab01cee385fba)). Export scene 0 into the paths in [`components/src/assets.rs`](components/src/assets.rs) (skip `.blend1`).
