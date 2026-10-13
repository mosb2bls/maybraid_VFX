# Furniture components

Kit layer for painted furniture: `AssetPath`s, Blender→engine remap, and posed `SceneRef` + `MaterialRef` (no `MultiSceneMerge`).

Assemblies (`furniture-assemblies`) build `PlacedPart` lists; this crate instances the GLBs from [`art/furniture`](../../../../art/furniture/) once they are exported to `maybraid/assets` at the paths in [`src/assets.rs`](src/assets.rs).
