# Scene Reference

Scene loading via references to avoid loading the same scene multiple times.

## Usage

```rust
use scene_ref::{
    MirrorAxis, MultiSceneMerge, MultiScenePart, SceneRef, SceneRefHandles, SceneRefPlugin,
};
use bevy::prelude::Transform;

app.add_plugins(SceneRefPlugin);

// Author a scene with a shared GLB root (`SceneRefRoot` → `WorldAssetRoot` when ready;
// AssetServer + SceneRefHandles keep one strong handle per SceneRef).
let scene = SceneRef::glb("urban/floors/foo.glb").scene();

// Axis-mirrored rebuild (vertex/winding only; positive Transform scale at the caller):
let mirrored = SceneRef::glb("urban/panels/unit_right_triangle.glb")
    .mirrored(MirrorAxis::X)
    .scene();

// Instance reflection (verts + conjugated hierarchy; for skinned / nested GLBs):
let reflected = SceneRef::glb("characters/ears/round_scoop_lateral_ear_left.glb")
    .reflected(MirrorAxis::X)
    .scene();

// Merge several scenes into one mesh WorldAsset (per-part transforms baked into verts):
let merged = MultiSceneMerge::new([
    MultiScenePart::identity(SceneRef::glb("foliage/tuft_a.glb")),
    MultiScenePart::new(
        SceneRef::glb("foliage/tuft_b.glb"),
        Transform::from_xyz(1.0, 0.0, 0.0),
    ),
])
.scene();

// Or fetch the handle explicitly (e.g. preload / spawn outside BSN):
fn preload(mut handles: ResMut<SceneRefHandles>, assets: Res<AssetServer>) {
    let _ = handles.handle(&SceneRef::glb("urban/floors/foo.glb"), &assets);
}
```

[`SceneRef::glb`] accepts a path relative to the Bevy asset root. If no `#SceneN`
label is present, scene `0` is used. [`SceneRef::mirrored`] rebuilds positions,
normals, and tangents with reversed winding (caller places the instance).
[`SceneRef::reflected`] does that and conjugates each entity `Transform`, matching
a parent axis-flip without negative Transform scale at the caller.

[`MultiSceneMerge`] resolves each part through [`SceneRefHandles`] (including mirrors),
bakes each mesh entity’s hierarchy transform plus the part transform into vertices,
and concatenates into a single mesh. Source glTF materials are **not** carried over:
the merged asset uses one shared placeholder [`StandardMaterial`] so the mesh still
enters the PBR pipeline. Kit-local positions are written to vertex `COLOR.xyz` before
the bake (see `multi_merge` module docs) so downstream shaders can recover per-kit
variation after the merge.

## Fulfill budgets

[`SceneRefPlugin`] installs [`SceneRefAdmitBudget`] (default: unlimited). Fulfill
systems cap how many pending [`SceneRefRoot`] / [`MultiSceneMergeRoot`] entities they
probe and admit per frame:

| Field | Limits |
|-------|--------|
| `per_frame` | Max roots fulfilled (insert [`WorldAssetRoot`]) per frame |
| `new_merge_meshes_per_frame` | Max **first-time** [`MultiSceneMerge`] bakes (`meshes.add`); cache hits still count only against `per_frame` |
| `inspect_per_frame` | Max pending-root probes per frame (round-robin so unready roots at the front of the query do not starve the rest) |

Production hosts that spawn large drain waves should lower these (for example
[`chico` `view.rs`](../world/layers/vegetation/chico/src/view.rs) uses `256` /
`64` / `256`). Set `per_frame` or `inspect_per_frame` to `0` to disable fulfill
for that frame.

## glTF node scale

On first resolve, [`SceneRefHandles::try_resolve`] bakes **odd negative** glTF node
scale into mesh winding (`GltfNodeScale` / `bake_odd_scales_in_world`) so default
materials keep Back-face cull after import. Mirrored and merged paths run after
that bake.
