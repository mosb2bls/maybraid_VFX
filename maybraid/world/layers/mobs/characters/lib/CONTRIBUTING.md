# Contributing to the `characters` crate

## General

## Adding a new species

A species is a self-contained module under `src/species/` that owns its config,
baseline pose, LodScene recipe, and color `enums`. Shared mesh catalogs (body, eye,
hair, clothing, etc.) live in `src/species/common/assets.rs`; species-specific
meshes and swatches stay in the species module.

Use **`Brodler`** as the template for a fixed-silhouette species (config + catalog
enums + pose, no user sliders). Use **`Braidman`** when the species needs
gender/build presets and rig or feature sliders.

Orthograde humanoid species that share the standard head-rig sockets should
implement [`CharacterComponents`](src/components.rs) with builders in
[`species/common/nodes.rs`](src/species/common/nodes.rs) (`eye_left` /
`eye_right`, `ear_left` / `ear_right`, `orthograde_head_rig`, …). Right-side
left-authored GLBs use [`SceneRef::reflected`](../../../../../scene-ref/src/scene_ref.rs)
for handedness; socket locals stay placement-only (no `scale.x = -1`). Clothing
is [`Clothed<T>`](src/components.rs) via [`CharacterRecipe::clothed`](src/components.rs),
not part of the inner species. Apps that spawn characters add
[`CharacterHostsPlugin`](src/hosts.rs) (scene-ref, material-ref, LOD refresh,
[`CharacterComponentsPlugin`](src/plugin.rs), [`character-motion`](../motion/),
and every clothed species). A single species can instead call
[`add_character_components_host::<Clothed<T>>`](src/plugin.rs).
[`RigNode`](src/nodes/rig_node.rs) / [`PartNode`](src/nodes/part.rs) are
registered once. Those nodes stamp socket/skin/rig/part/transform from
[`LodScene::host`](../../../../../lod/lib/src/scene/lod_scene.rs); part materials
(`MaterialRefRoot` + `PropagateToDescendants`) live on the **part host**, not
the GLB child, so live paint is an insert on the member you query. Nested hosts
get [`MemberOf`](src/member.rs) after fulfill (walk [`ChildOf`] to
[`CharacterRoot`](src/member.rs)); socket/skin fulfill is scoped to that
character. Proportion pose is [`ActiveRigPose`](src/rig.rs) on the rig member
([`maintain_resolved_pose`](src/pose.rs)). Clips are [`AnimRefRoot`](src/anim.rs)
on the body-rig host (default still). [`AnimClip`](../motion/src/clip.rs)
is the identity (variant + sampler knobs); the mailbox transitions on
[`AnimId`](../motion/src/clip.rs) when the variant changes, not when
knobs retune. [`CharacterComponentsPlugin`](src/plugin.rs) owns membership, bone
map, socket/skin fulfill, and pose. The mailbox and terrain pitch live in
[`character-motion`](../motion/README.md): host markers
(`AnimateBones` / `AnimateEffects` / `ApplyTerrainPitch`) are synced from the
shown LOD band (and the look frustum for visible High Mid/Far plants) and filter
the expensive systems. Do not
grow a post-spawn prepare pass that copies materials or poses.

Runtime world, mob, and player visuals spawn a **fixed assembly**
([`spawn_fixed_character_visual`](src/fixed.rs)) — ordinary [`CharacterRoot`](src/member.rs)
/ rig / part entities without query-only [`LodSceneHost`](../../../../../lod/lib/src/scene/lod_scene.rs)s.
[`MobScene`](../../mobs/mobs/src/scene.rs) remains the only LOD gate for mob members.
Concepts and menu previews still spawn LodScene ([`CharacterRecipe::clothed`](src/components.rs)
via [`ComponentsOnly`](src/components.rs) [`LodScene::host`](../../../../../lod/lib/src/scene/lod_scene.rs)).
[`CharacterComponents`](src/components.rs) is the only character recipe.
Part colors live on [`PartNode::material`](src/nodes/part.rs) (`*Colors::color_for_slot`
and [`ClothingLayer`](src/components.rs) stamp palette[0]). Live playground tint
inserts [`MaterialRefRoot`](src/nodes/part.rs) on the part host (change-driven
[`material-ref`](../../../../../material-ref) fulfill); [`PreviewAssetTarget`](../../../../../menu/character-concepts-playground/src/preview.rs)
stays UI mapping, not the paint path. Live animation inserts [`AnimRefRoot`](src/anim.rs)
on the body member (`ConceptAnimation` maps at the playground/menu edge).
Playground live updates query [`CharacterMembers`](src/member.rs) of the preview
root — not the whole world.

### 1. `characters` (this crate)

1. Create `src/species/<name>.rs` with:
   - `*Config` and `*Colors` structs (`*Colors::color_for_slot` for recipe tints)
   - species-local `enums` (skin, eye, head, mouth, etc.) with `VALUES`, `label()`,
     and `color()` where needed
   - `impl CharacterRecipe for *Config`
2. Create `src/species/<name>/recipe.rs` with the inner [`CharacterComponents`](src/components.rs)
   recipe (`rig_nodes_for_level` / `part_nodes_for_level`). Stamp part colors from
   `color_for_slot` onto `PartNode.material` (`with_base_color`).
3. Create `src/species/<name>/assets.rs` when the species has a local mesh catalog:
   - mesh `enums` with `VALUES`, `label()`, and `path()`
   - omit parts the species does not use (for example `Mygr` has no nose)
4. Create `src/species/<name>/pose.rs` when the species has a fixed baseline:
   - compose [`RigPoseLayer`](../rigs/src/pose.rs) scales via
     `BraidmanSliders::apply_*` helpers for leg length, thigh thickness, etc.
5. Register the module in [`src/species.rs`](src/species.rs).
6. Add shared asset paths to [`src/species/common/assets.rs`](src/species/common/assets.rs)
   when a mesh may be reused across species.
7. Wire menu traits in [`src/menu_traits.rs`](src/menu_traits.rs):
   - `impl_menu_identity!` for list/cycle `enums`
   - `impl_asset_option!` for thumbnail meshes
   - `SwatchOption` for species color `enums`

### 2. `character-creation-menus`

1. Add `src/characters/<name>.rs` — menu structs, `From` conversions to/from
   `*Config`, and `camera_focus_for_field`.
2. Register the module in [`src/characters.rs`](../../../../../menu/character-creation-menus/src/characters.rs).
3. Extend [`src/character.rs`](../../../../../menu/character-creation-menus/src/character.rs):
   - `ConceptSpecies::<Name>`
   - `CharacterMenu::<name>` field
   - `from_<name>`, `apply_<name>`, config accessor
4. Extend [`src/event.rs`](../../../../../menu/character-creation-menus/src/event.rs) with any new
   `CharacterField`, `AssetValue`, and `SwatchValue` variants.

### 3. `character-concepts-playground`

1. Add `src/commands/<name>.rs` for CLI preview `args`.
2. Extend [`src/preview.rs`](../../../../../menu/character-concepts-playground/src/preview.rs):
   - `ConceptPreviewConfig::<Name>` (add the variant to `with_clothed_recipe!`)
   - `PreviewTarget::<Name>*` variants and `preview_asset_target` mapping
   - `*Colors::color_for_slot` (recipe stamps `PartNode.material`; spawn /
     `lod_rig_nodes` / `lod_part_nodes` go through the macro). Live tint writes
     `MaterialRefRoot` on the part member of the preview [`CharacterRoot`].
3. Update [`src/menu_listeners.rs`](../../../../../menu/character-concepts-playground/src/menu_listeners.rs),
   [`src/species_session.rs`](../../../../../menu/character-concepts-playground/src/species_session.rs),
   [`src/focus_reference.rs`](../../../../../menu/character-concepts-playground/src/focus_reference.rs),
   and [`src/preview_color.rs`](../../../../../menu/character-concepts-playground/src/preview_color.rs).

### Verify

```bash
cargo test -p characters
cargo test -p character-creation-menus
cargo check -p character-concepts-playground
```

World locomotion (walk / run / jump on streamed Durham ground) runs in
[`maybraid-world-playground`](../../../../playground/):

```bash
cargo run -p maybraid-world-playground
```

Spawn from the playground UI (species picker) or CLI (`character-concepts` is
[`CONCEPTS_CLI_NAME`](../../../../../menu/character-concepts-playground/src/commands.rs)):

```bash
cargo run -p character-concepts-playground -- mygr preview --skin ginger --eyes green
```

Preview socket/skin debug:

```bash
CROZON_PREVIEW_DEBUG=1 cargo run -p character-concepts-playground -- hars preview
```

### Socketing, scale, and shear

Parts attach with [`SocketRef`](src/socket.rs): a `ChildOf(bone)` plus a
local `Transform`. Bevy propagates the full parent affine, so **non-uniform scale
on an ancestor combined with rotation on or under that socket shears** the
attached mesh. Intermediate bones do not fix this if the part remains a transform
child of the scaled chain.

Nested armatures use [`RigId::Neck`](src/socket.rs) /
[`CharacterPartSlot::NeckRig`](src/assembly.rs) (+ optional `NeckMesh`):

1. Socket the neck OwnRig to the body `head_socket`.
2. Apply **pitch** (and optional **uniform** scale) via the neck [`RigNode`](src/nodes/rig_node.rs) pose.
3. Socket the head to the neck tip `head_socket` (counter-pitch on the tip bone).

With a dedicated neck armature/mesh, **prefer authored length + pitch + uniform
scale** — do not lengthen via `BoneScale::length` / bind translation on that path.
Check armature rest orientation: a 90° export flip can make local-X pitch yaw in
world space (Hars raises about local −Z on the triple-join neck).

Until rigid (no-scale) sockets land ([#516](https://github.com/ramate-io/maybraid/issues/516)):

- Prefer **uniform** scale on any bone that still parents a pitched socket or head.
- Avoid `BoneScale::length` / `thickness` on bones that rotate or parent a rotated
  `head_socket` / feature socket.
- Keep mesh and armature as separate assets when needed (`NeckRig` armature +
  `NeckMesh` skinned to it), matching the head rig / head mesh split.

[#516](https://github.com/ramate-io/maybraid/issues/516) tracks an opt-in rigid
socket path (follow bone translation + orthonormal rotation, ignore scale/shear).
