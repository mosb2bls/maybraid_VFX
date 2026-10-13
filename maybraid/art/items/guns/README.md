# Firearm kit art

Blender sources for kit-assembled firearms. Runtime GLBs mirror this layout under `maybraid/assets/items/guns/`. Path constants live in [`firearms-components` `assets.rs`](../../../items/firearms-components/src/assets.rs). The shared armature is [`rigs/firearm_rig.blend`](rigs/firearm_rig.blend).

## Kit slots

Each folder is one replaceable mesh. Trigger boxes used to be modeled into the body; they are their own slot so a receiver hull can swap fire-control geometry independently.

| Folder | Socket bone | Role |
|--------|-------------|------|
| [`bodies/`](bodies/) | `body` | Receiver hull |
| [`barrels/`](barrels/) | `barrel` | Barrel / bore |
| [`trigger_boxes/`](trigger_boxes/) | `trigger_box` | Fire-control box (trigger, guard, related housing) |
| [`grips/`](grips/) | `grip` | Pistol grip / handle |
| [`stocks/`](stocks/) | `stock` | Stock |
| [`sights/`](sights/) | `sight_socket` | Optic (attachment on the sight landmark) |
| [`concepts/`](concepts/) | (none) | Baked one-mesh kits; skip assembly |
| [`rigs/`](rigs/) | — | Shared receiver armature |

Empty directories are placeholders (`stocks/`), not missing exports.

Runtime kits always have a body. Barrel, trigger box, grip, stock, and sight are optional (`none` in the playground `kit` command). Sights are authored on a 1 m cube and rest-scaled onto `sight_socket`; they do not lengthen a kit bone. First-person focus stays on `sight_camera_socket`.

Trigger-box meshes: `keelripe_box`, `paddle_box`, `reltor_box`. The grip catalog is `bump_handle`. Sight meshes: `holorand` (1–3× ADS), `leskop` (3–5× ADS). Keelripe is a trigger box, not a body.

## Receiver armature

Rest hierarchy in [`firearm_rig.blend`](rigs/firearm_rig.blend):

```
body
  barrel
  grip
  grip_arm
    grip_point
stock
trigger_box
trigger_arm
  trigger_ledge
    trigger_point
sight_arm
  sight_camera_socket
  sight_boom
    sight_socket
```

`body`, `stock`, and `trigger_box` are sibling roots. `barrel` / `grip` / `grip_arm` hang off `body`. That keeps stock and trigger-box length independent of receiver length.

### Kit bones vs hand bones

| Bone | Kind | Use |
|------|------|-----|
| `body`, `barrel`, `trigger_box`, `grip`, `stock` | Length | Socket the matching kit GLB here |
| `grip_arm` → `grip_point` | Hand chain | Support / grip hand target |
| `trigger_arm` → `trigger_ledge` → `trigger_point` | Hand chain | Trigger hand target |
| `sight_arm` → `sight_boom` → `sight_socket` | Attachment | Optic mesh |
| `sight_arm` → `sight_camera_socket` | Camera | First-person focus |

Do not socket kit meshes onto `_point` / `_arm` / `_ledge` bones, or onto `sight_camera_socket`. Those exist so a hand or camera can follow a named landmark without riding the kit part. The optic GLB goes on `sight_socket`.

## Author along the bone

Kit meshes are authored in **bone space**: the GLB's **+Y is that socket bone's length axis** (Blender bone +Y, head → tail). Fulfill uses an identity [`SocketRef`](../../../items/firearms-components/src/nodes.rs). [`BoneScale::length`](../../../rigs/src/pose.rs) is already “scale local Y”; a part that matches the bone stretches with it.

Same rule on every slot, even when bones point different ways in armature space. In this rest pose (Blender, Z-up):

- `body` / `barrel` run along armature **+Z** (muzzle)
- `stock` runs along armature **−Z** (butt)
- `grip` / `trigger_box` run along armature **−Y** (down the handle)

After glTF the armature node already carries Blender’s +90° X (−Y forward, +Z up → runtime bore **+Z**, grip **−Y**). [`aim_plus_x`](../../../items/firearms/src/pose.rs) is a Y yaw onto world +X. Pitching around X puts the bore into the ground. Do not re-author each mesh into world-horizontal.

### Why not `AlongBone`

A runtime remap from “item space” onto bone Y would duplicate `BoneScale::length` and leave identity sockets as a silent wrong default. Author the rotation into the GLB instead.

### Why not extra `_socket` bones on kit parts

Character `head_socket` bones are **attachments**: they place a child, they are not a length of the parent. Perpendicular socket bones on a barrel or stock would attach the mesh, but scaling the chain bone would not scale the part. Kit length lives on the bone the mesh is parented to.

Hand landmarks (`grip_point`, `trigger_point`) **are** that attachment pattern. They are for hands, not for kit GLBs. `sight_socket` is the same pattern for optics; `sight_camera_socket` is the first-person camera landmark.

## Lengthening a chain

Non-uniform scale on an ancestor shears any rotated descendant ([character socket notes](../../../world/layers/mobs/characters/lib/CONTRIBUTING.md#socketing-scale-and-shear)). `body` still parents `barrel`, `grip`, and `grip_arm`, so **do not** `BoneScale::length` `body` to stretch the receiver if those children must stay unsheared.

Lengthen a kit piece by scaling **that part's host**, or push child joints with [`BoneTranslation::length`](../../../rigs/src/pose.rs) and leave the parent bone unscaled.

## Export

```bash
blender --background maybraid/art/items/guns/rigs/firearm_rig.blend \
  --python scripts/glb/main.py -- maybraid/assets/items/guns/rigs/firearm_rig.glb
blender --background maybraid/art/items/guns/rigs/firearm_rig.blend \
  --python scripts/armature-dump/main.py -- maybraid/assets/items/guns/rigs/firearm_rig.armature_dump
```

Kit meshes use the same `scripts/glb/main.py` path into the matching `assets/items/guns/<slot>/` file. Re-export the armature after bone edits so [`RECEIVER_LANDMARKS`](../../../items/firearms-components/src/nodes.rs) can index the new names.

## See also

- [`firearms-components` README](../../../items/firearms-components/README.md) — slots, sockets, recipe trait
- [`firearms` README](../../../items/firearms/README.md) — named kits on top of those nodes
