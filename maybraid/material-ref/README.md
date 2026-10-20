# Material Reference

Deferred material identity (recipe name + palette + noise) resolved by a
pluggable `MaterialLib` — the same SystemParam pattern as LOD’s
`LodSceneRegionIndex`.

## Usage

```rust
use material_ref::{MaterialRef, MaterialRefRoot, StandardMaterialRefPlugin};
use bevy::prelude::*;

app.add_plugins(StandardMaterialRefPlugin);

// ECS: MaterialRefRoot → MeshMaterial3d<StandardMaterial> on fulfill
commands.spawn((
    Mesh3d(mesh),
    MaterialRefRoot(MaterialRef::named("tuft").with_palette([Color::srgb(0.2, 0.5, 0.2)])),
));

// WorldAsset / GLB: opt into applying the ref to Mesh3d descendants
commands.spawn((
    MaterialRefRoot(MaterialRef::default()),
    PropagateToDescendants,
    // SceneRefRoot / WorldAssetRoot …
));
```

### Custom multi-type lib

Implement `MaterialLib` on a `#[derive(SystemParam)]` that borrows every
`Assets<M>` / cache you need, fork on `MaterialId`, and insert the matching
`MeshMaterial3d<M>`. Register with `MaterialRefPlugin::<YourLib<'_, '_>>::default()`
after initializing any cache resources.

Install exactly one [`MaterialRefPlugin`] at the app composition root.
Domain plugins initialize caches and shaders only. A second resolver panics
instead of silently skipping. Composed libs should fall through to
`StandardMaterial` (Chico does).
