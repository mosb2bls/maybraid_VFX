# Character playground

Character playground. 

Rough sketch...

```rust
use bevy::{
    mesh::skinning::SkinnedMesh,
    prelude::*,
    scene::SceneInstanceReady,
};
use std::collections::HashMap;

#[derive(Component)]
struct CharacterRig;

#[derive(Component)]
struct ModularBody;

#[derive(Component)]
struct NeedsSkinRemap {
    rig_root: Entity,
}

#[derive(Component, Default)]
struct BoneMap {
    by_name: HashMap<String, Entity>,
}

fn spawn_character(mut commands: Commands, assets: Res<AssetServer>) {
    let rig_root = commands
        .spawn((
            SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("humanoid_rig.glb"))),
            CharacterRig,
            BoneMap::default(),
        ))
        .id();

    commands.spawn((
        SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("humanoid_body.glb"))),
        ModularBody,
        NeedsSkinRemap { rig_root },
    ));
}
```

After the scenes instantiate, build a bone map for the rig:

```rust
fn build_rig_bone_map(
    mut rig_roots: Query<(Entity, &Children, &mut BoneMap), With<CharacterRig>>,
    children_q: Query<&Children>,
    names_q: Query<&Name>,
) {
    for (_rig_root, children, mut map) in &mut rig_roots {
        if !map.by_name.is_empty() {
            continue;
        }

        let mut stack: Vec<Entity> = children.iter().copied().collect();

        while let Some(entity) = stack.pop() {
            if let Ok(name) = names_q.get(entity) {
                map.by_name.insert(name.to_string(), entity);
            }

            if let Ok(children) = children_q.get(entity) {
                stack.extend(children.iter().copied());
            }
        }
    }
}
```

Then remap the body’s skin to the rig’s bones:

```rust
fn remap_body_skin_to_rig(
    mut commands: Commands,
    body_roots: Query<(Entity, &Children, &NeedsSkinRemap), With<ModularBody>>,
    rig_maps: Query<&BoneMap, With<CharacterRig>>,
    children_q: Query<&Children>,
    names_q: Query<&Name>,
    mut skinned_meshes: Query<(&mut SkinnedMesh, &Name)>,
) {
    for (body_root, children, remap) in &body_roots {
        let Ok(rig_map) = rig_maps.get(remap.rig_root) else {
            continue;
        };

        if rig_map.by_name.is_empty() {
            continue;
        }

        let mut stack: Vec<Entity> = children.iter().copied().collect();

        while let Some(entity) = stack.pop() {
            if let Ok((mut skin, _mesh_name)) = skinned_meshes.get_mut(entity) {
                let mut new_joints = Vec::with_capacity(skin.joints.len());

                for old_joint in &skin.joints {
                    let Ok(old_name) = names_q.get(*old_joint) else {
                        continue;
                    };

                    let Some(new_joint) = rig_map.by_name.get(old_name.as_str()) else {
                        warn!("No matching rig joint for body joint {}", old_name);
                        continue;
                    };

                    new_joints.push(*new_joint);
                }

                if new_joints.len() == skin.joints.len() {
                    skin.joints = new_joints;
                } else {
                    warn!("Body skin remap failed due to missing joints");
                }
            }

            if let Ok(children) = children_q.get(entity) {
                stack.extend(children.iter().copied());
            }
        }

        commands.entity(body_root).remove::<NeedsSkinRemap>();
    }
}
```

Register it:

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, spawn_character)
        .add_systems(Update, (build_rig_bone_map, remap_body_skin_to_rig))
        .run();
}
```

The body GLB still needs to contain its own copy of the rig when exported, because that is how the GLB stores the skin’s joint list and inverse bind poses. At runtime, you throw away the body’s duplicate live joint entities and replace them with the already-spawned rig entities. The baked vertex weights stay in the mesh. The important field you rewrite is:

```rust
skin.joints = new_joints;
```

Bevy’s `SkinnedMesh` is the component that stores the joint entity list and inverse bind poses, and Bevy’s skinned mesh examples show this same `SkinnedMesh { inverse_bindposes, joints }` data model. ([`Docs.rs`][1])

[1]: https://docs.rs/bevy/latest/bevy/mesh/skinning/index.html?utm_source=chatgpt.com "bevy::mesh::skinning - Rust - Docs.rs"

## Helpers

### Armature Hierarchy Dump

To help with armature usage, we will likely want to add an armature dump script that runs on pre-commit.

```py
import bpy

armature = bpy.data.objects["Armature"]

def dump_bone(bone, indent=0):
    print("  " * indent + bone.name)
    for child in bone.children:
        dump_bone(child, indent + 1)

for bone in armature.data.bones:
    if bone.parent is None:
        dump_bone(bone)
```