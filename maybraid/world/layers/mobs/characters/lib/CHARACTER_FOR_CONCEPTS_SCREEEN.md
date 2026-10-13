# Character Concepts Screen Spec

## Key Recommendations

Use a layered, bone-driven design:

1. **Menu model** — editable UI state: species, gender, build, features, clothes, active animation, and raw control values.
2. **Resolved character config** — normalized game-facing data: concrete species definitions, chosen assets, resolved field values, active feature effects, and clothing choices.
3. **Rig pose / render assembly** — converts resolved config into a `RigPose`, spawned scenes, attached features, clothing fits, and animation preview.
4. **Actual character struct** — final runtime/gameplay data, built from resolved config rather than UI widgets.

The preview must not mutate the final character field-by-field, and it must not use scene-root scaling to express species proportions. Proportions are layered on bones: `GLTF bind pose -> species base bone scales -> gender/build refinements -> user slider deltas -> Bevy bone marshaling, skin remap, and animation`.

Asset authoring scale is separate from character proportions. If an imported mesh or rig was authored as a 1m-radius working asset, give that asset a documented local normalization transform; sliders then multiply the normalized baseline.

---

# Character Concepts Screen

## Goal

The Character Concepts Screen is an early create-a-character scene used to gather feedback on character silhouettes, species identity, extended features, animation readability, and clothing fit behavior.

The screen is not a final character creator. It is a concept review tool for assembling character rigs, previewing species variations, testing feature controls, and validating clothing fit behavior.

Paints, albedos, and material customization are out of scope for this iteration.

---

# Core Flow

The screen should follow this pipeline:

```rust
Menu State
    -> Resolved Character Config
    -> RigPose from bind pose + layered bone effects
    -> Feature/Clothing Assembly
    -> Preview Entity
    -> Final Character Struct, when needed
```

In pseudocode:

```rust
fn update_character_preview(
    menu: Res<CharacterCreationMenuState>,
    species_defs: Res<SpeciesDefinitions>,
    bind_poses: Res<RigBindPoses>,
    mut preview: ResMut<CharacterPreview>,
) {
    if !menu.is_changed() {
        return;
    }

    let resolved = resolve_character_config(&menu, &species_defs);
    let bind_pose = bind_poses.for_species(resolved.species);

    let rig_pose = build_rig_pose_from_bind_pose(&resolved, bind_pose);
    let features = build_feature_instances(&resolved);
    let clothing = fit_clothing(&resolved, &rig_pose);

    preview.apply(rig_pose, features, clothing);
}
```

When the game needs an actual character:

```rust
fn commit_character(
    menu: &CharacterCreationMenuState,
    species_defs: &SpeciesDefinitions,
) -> Character {
    let resolved = resolve_character_config(menu, species_defs);

    Character::from_resolved_config(resolved)
}
```

This keeps the menu flexible while preventing UI structure from leaking into gameplay structs. In the preview hierarchy, scene-root transforms should usually be identity; skinned body/head/clothing meshes follow their remapped rigs.

---

# Menu State

The menu state is the editable UI-facing state.

```rust
pub struct CharacterCreationMenuState {
    pub species: SpeciesId,
    pub gender: PresetId,
    pub build: PresetId,
    pub fields: HashMap<FieldId, FieldValue>,
    pub active_animation: AnimationPreview,
    pub focused_field: Option<FieldId>,
}
```

A field represents anything editable in the character creator:

* Species selector
* Gender selector
* Build selector
* Feature selector
* Feature toggle
* Feature slider
* Rig slider
* Clothing selector
* Animation tab

```rust
pub enum FieldValue {
    Select(String),
    Toggle(bool),
    Slider(f32),
    MultiSelect(Vec<String>),
}
```

Every field should carry camera focus metadata.

```rust
pub struct CharacterCreationField {
    pub id: FieldId,
    pub label: String,
    pub value: FieldValue,
    pub control: FieldControl,
    pub camera_target: CameraTargetSpec,
}
```

```rust
pub enum FieldControl {
    Select { options: Vec<FieldOption> },
    Toggle,
    Slider { min: f32, max: f32, step: Option<f32> },
    MultiSelect { options: Vec<FieldOption> },
}
```

This replaces the idea that only sliders suggest a camera transform. Any field can focus the camera.

---

# Marshaling Menu State Into Character Data

The menu should be marshaled in stages.

## Stage 1: Menu State to Resolved Character Config

```rust
pub struct ResolvedCharacterConfig {
    pub species: SpeciesId,
    pub gender: PresetId,
    pub build: PresetId,
    pub rig_sliders: Vec<RigSlider>,
    pub non_rig_controls: Vec<ResolvedControlEffect>,
    pub features: Vec<ResolvedFeature>,
    pub clothing: Vec<ClothingItemId>,
}
```

Conceptually:

```rust
fn resolve_character_config(
    menu: &CharacterCreationMenuState,
    species_defs: &SpeciesDefinitions,
) -> ResolvedCharacterConfig {
    let species = species_defs.get(menu.species);

    let gender = species.gender_preset(menu.gender);
    let build = species.build_preset(menu.build);

    let mut resolved = ResolvedCharacterConfig::default();

    resolved.species = species.id;
    resolved.gender = gender.id;
    resolved.build = build.id;

    gender.apply_defaults(&mut resolved);
    build.apply_defaults(&mut resolved);

    for field in species.fields() {
        let value = menu.fields.get(&field.id).unwrap_or(&field.default_value);

        field.resolve(value, &mut resolved);
    }

    resolved
}
```

Important rule:

```rust
Preset defaults initialize values.
Menu fields override preset defaults.
Species constraints clamp final values.
```

Suggested resolution order:

1. Species base defaults
2. Gender preset defaults
3. Build preset defaults
4. Current menu field values
5. Clamp/validate against species rules
6. Produce resolved config

---

## Stage 2: Resolved Config to `RigPose`

`RigPose` is the single transform budget for the skinned preview. Start from a snapshot of the GLTF bind pose, then compose species, preset, and user effects onto that rest pose.

```rust
impl RigPose {
    pub fn apply_sliders<S: Slidable>(&mut self, sliders: &[S], rest: &RigPose) {
        for slider in sliders {
            let bone_effects = slider.slider_bone_effects();

            for bone_effect in bone_effects {
                self.compose_bone_effect(rest, bone_effect);
            }
        }
    }
}
```

The resolved config should collect all currently active rig sliders, then build the pose in order:

```rust
fn build_rig_pose_from_bind_pose(
    config: &ResolvedCharacterConfig,
    bind_pose: &RigPose,
) -> RigPose {
    let mut pose = bind_pose.clone();

    pose.apply_species_base(config.species, bind_pose);
    pose.apply_gender_preset(config.gender, bind_pose);
    pose.apply_build_preset(config.build, bind_pose);

    pose.apply_sliders(&config.rig_sliders, bind_pose);

    pose
}
```

Rules:

* Bone effects compose with bind-pose translation, rotation, and scale; they never replace a full bone `Transform`.
* A slider value of `1.0` means "species baseline", not "write identity transform".
* Species width, height, and depth are bone scales or named bone-group effects, not scene-root downscales.
* Reuse `character_rigs` helpers such as `RigPose::apply_sliders`, articulation helpers, axis-aware translation deltas, and bind-pose debug output.

Because `Slidable` uses concrete generic types, you may want an enum wrapper for heterogeneous rig sliders:

```rust
pub enum RigSlider {
    HipWidth(HipWidthSlider),
    TorsoLength(TorsoLengthSlider),
    ShoulderWidth(ShoulderWidthSlider),
    HeadScale(HeadScaleSlider),
}
```

Then implement `Slidable` for the enum:

```rust
impl Slidable for RigSlider {
    fn slider(&self) -> Slider {
        match self {
            RigSlider::HipWidth(s) => s.slider(),
            RigSlider::TorsoLength(s) => s.slider(),
            RigSlider::ShoulderWidth(s) => s.slider(),
            RigSlider::HeadScale(s) => s.slider(),
        }
    }

    fn slider_bone_effects(&self) -> Vec<SliderBoneEffect> {
        match self {
            RigSlider::HipWidth(s) => s.slider_bone_effects(),
            RigSlider::TorsoLength(s) => s.slider_bone_effects(),
            RigSlider::ShoulderWidth(s) => s.slider_bone_effects(),
            RigSlider::HeadScale(s) => s.slider_bone_effects(),
        }
    }
}
```

This allows:

```rust
pub struct ResolvedCharacterConfig {
    pub rig_sliders: Vec<RigSlider>,
    // ...
}
```

And then:

```rust
rig_pose.apply_sliders(&config.rig_sliders, bind_pose);
```

This is cleaner than using `Vec<Box<dyn Slidable>>`, especially since the current `Sliders<S: Slidable>` trait is generic and Rust will be happier with a concrete enum.

---

# Reconciling Rig Sliders With Future Slider Types

Right now, most sliders map directly to rig sliders. That is good and should remain the default path.

However, not all future sliders will affect bones. Some may affect:

* Feature scale
* Feature offset
* Feature variant
* Clothing fit parameter
* Animation preview parameter
* Material or paint later
* Spawned asset visibility
* Physics/collider shape

So the character creator should distinguish between:

1. **UI slider**
2. **Rig slider**
3. **Resolved effect**

A UI slider is just an editable field:

```rust
CharacterCreationField {
    id,
    label,
    value: FieldValue::Slider(0.5),
    control: FieldControl::Slider { min, max, step },
    camera_target,
}
```

A rig slider is a domain object implementing `Slidable`:

```rust
pub trait Slidable {
    fn slider(&self) -> Slider;
    fn slider_bone_effects(&self) -> Vec<SliderBoneEffect>;
}
```

A resolved effect is what the slider actually does:

```rust
pub enum ResolvedControlEffect {
    RigSlider(RigSlider),
    FeatureTransform {
        feature: FeatureSlotId,
        transform: Transform,
    },
    ClothingFitParameter {
        clothing: ClothingItemId,
        parameter: String,
        value: f32,
    },
}
```

Recommended rule:

```rust
All sliders are fields.
Some fields resolve into rig sliders.
Some fields resolve into other effects.
```

That gives you a stable UI model while preserving your existing rig slider machinery.

---

# Field Definitions

Each species should expose a list of fields.

```rust
pub trait CharacterCreationFieldProvider {
    fn fields(&self) -> Vec<CharacterCreationField>;
}
```

A species field may resolve into one or more effects.

```rust
pub trait ResolvableCharacterField {
    fn resolve(
        &self,
        value: &FieldValue,
        resolved: &mut ResolvedCharacterConfig,
    );
}
```

Example:

```rust
pub struct HipWidthField;

impl ResolvableCharacterField for HipWidthField {
    fn resolve(
        &self,
        value: &FieldValue,
        resolved: &mut ResolvedCharacterConfig,
    ) {
        let FieldValue::Slider(value) = value else {
            return;
        };

        resolved.rig_sliders.push(RigSlider::HipWidth(
            HipWidthSlider { value: *value }
        ));
    }
}
```

Example for a non-rig slider:

```rust
pub struct HornScaleField;

impl ResolvableCharacterField for HornScaleField {
    fn resolve(
        &self,
        value: &FieldValue,
        resolved: &mut ResolvedCharacterConfig,
    ) {
        let FieldValue::Slider(value) = value else {
            return;
        };

        resolved.non_rig_controls.push(
            ResolvedControlEffect::FeatureTransform {
                feature: FeatureSlotId::Horns,
                transform: Transform::from_scale(Vec3::splat(*value)),
            }
        );
    }
}
```

---

# Species

A species is the highest-level character definition.

Each species defines:

* Base rig
* Base `gltf` assets
* Allowed gender presets
* Allowed build presets
* Species-level fields
* Allowed extended features
* Base bone scales and offset rules, grouped by named bone sets
* Feature attachment points
* Animation compatibility
* Clothing fit compatibility

Species should own constraints and base silhouette behavior. Put readable proportion constants on species rig asset types, not in scattered resolution helpers.

A species can expose fields like:

```rust
SpeciesAFields {
    hip_width: SliderField,
    torso_length: SliderField,
    horns: FeatureSelectField,
    horn_height: SliderField,
    spines: FeatureSelectField,
}
```

But the runtime should see the normalized field list:

```rust
species.fields() -> Vec<CharacterCreationField>
```

---

# Gender Presets

Gender presets are initialization presets.

Typical examples:

* Male
* Female
* Non-binary

Gender presets may:

* Set initial field values
* Set initial rig slider values
* Enable default extended features
* Refine base rig proportions with documented bone-scale or bone-offset effects
* Select default assets

Gender presets must not restrict slider ranges. Defaults are allowed to be non-neutral, but they must be visible in resolution and debug output.

---

# Build Presets

Build presets are second-layer initialization presets.

Examples:

* Slender
* Heavy
* Stocky
* Tall
* Scary
* Lanky

Build presets may:

* Set initial field values
* Refine bone scales or slider values after gender defaults
* Enable or disable default features
* Select a stronger silhouette

Build presets must not restrict slider ranges. Like gender presets, their effects should compose on top of the species baseline rather than writing absolute bone transforms.

---

# Extended Features

Extended features are optional or selectable additions to the character.

Examples:

* Horns
* Spines
* Hair
* Eye variants
* Tails
* Ears
* Fins
* Crests

Extended features may include their own fields:

* Variant selector
* Enabled toggle
* Scale slider
* Length slider
* Offset slider
* Rotation slider

Feature fields should resolve into feature effects, not necessarily rig sliders.

---

# Clothes

Clothing can be equipped after species, presets, features, and sliders have been selected.

A clothing item may define:

* Supported species
* Unsupported species
* Fit implementation
* Required attachment points
* Optional shadow rig
* Preview metadata

```rust
pub enum FitResult {
    NoFit,
    NoChanges,
    Shadow(Rig),
}
```

```rust
pub trait Fit {
    fn fit_bones(&self, rig: &Rig) -> FitResult;
}
```

Most clothing items should use `NoChanges` or `NoFit`.

---

# Animations

The preview scene should support cycling between core animations while editing:

* Walk
* Run
* Two-footed jump
* Front flip

Animation preview is itself a field and should also suggest a camera target.

For example:

* Walk: full-body side/front view
* Run: full-body side view
* Jump: full-body view with more vertical framing
* Front flip: zoomed-out full-body framing

---

# Topical Camera Placement

Every editable field must provide camera metadata.

This includes:

* Species
* Gender
* Build
* Feature selectors
* Feature sliders
* Rig sliders
* Clothing selectors
* Animation tabs

```rust
pub struct CharacterCreationField {
    pub id: FieldId,
    pub label: String,
    pub value: FieldValue,
    pub control: FieldControl,
    pub camera_target: CameraTargetSpec,
}
```

Recommended camera target form:

```rust
pub enum CameraTargetSpec {
    Topic(CameraTopic),
    Explicit {
        focus_bone: Option<Name>,
        local_offset: Vec3,
        distance: f32,
        pitch: f32,
        yaw: f32,
    },
}
```

```rust
pub enum CameraTopic {
    FullBody,
    Head,
    Face,
    Torso,
    Back,
    Arms,
    Hands,
    Legs,
    Feet,
    Feature(FeatureSlotId),
    Clothing(ClothingSlotId),
    Animation(AnimationPreview),
}
```

The camera system should not need to know why a field is active. It only needs the active field’s `CameraTargetSpec`.

```rust
fn update_camera_for_focused_field(
    menu: Res<CharacterCreationMenuState>,
    fields: Res<CharacterCreationFields>,
    mut camera: Query<&mut Transform, With<CharacterCreationCamera>>,
) {
    let Some(field_id) = menu.focused_field else {
        return;
    };

    let Some(field) = fields.get(field_id) else {
        return;
    };

    let target = resolve_camera_target(&field.camera_target);

    smoothly_move_camera_to(target, &mut camera);
}
```

---

# Bevy System Order

Suggested system sets:

```rust
pub enum CharacterConceptSet {
    Input,
    FieldResolution,
    CharacterResolution,
    RigUpdate,
    FeatureUpdate,
    ClothingUpdate,
    AnimationUpdate,
    CameraUpdate,
    UiUpdate,
}
```

Suggested order:

1. Read UI input.
2. Update `CharacterCreationMenuState`.
3. Resolve active fields.
4. Marshal menu state into `ResolvedCharacterConfig`.
5. Build or update `RigPose`.
6. Spawn/update extended features.
7. Refit clothing.
8. Update animation preview.
9. Move camera based on focused field.
10. Refresh UI display state.

---

# Dirty Flags

```rust
pub struct CharacterDirtyFlags {
    pub fields: bool,
    pub resolved_config: bool,
    pub rig: bool,
    pub features: bool,
    pub clothing: bool,
    pub animation: bool,
    pub camera: bool,
    pub ui: bool,
}
```

Recommended behavior:

* Species change dirties everything.
* Gender change dirties resolved config, rig, features, clothing, camera, and UI.
* Build change dirties resolved config, rig, features, clothing, camera, and UI.
* Rig slider change dirties resolved config, rig, clothing, and camera.
* Feature selection change dirties resolved config, features, clothing, and camera.
* Feature slider change dirties resolved config, features, and camera.
* Clothing change dirties resolved config, clothing, and camera.
* Animation tab change dirties animation and camera.
* Focused field change dirties camera only.

---

# Display Helpers

Instead of having display helpers mutate commands directly, prefer having them describe fields.

Less ideal:

```rust
pub trait CharacterBuilder {
    fn relative_camera_transform() -> Transform;
    fn display_menu(&mut Commands);
}
```

Recommended:

```rust
pub trait CharacterCreationFieldProvider {
    fn fields(&self) -> Vec<CharacterCreationField>;
}
```

Then a UI system renders those fields.

```rust
fn spawn_character_creation_ui(
    fields: Res<CharacterCreationFields>,
    mut commands: Commands,
) {
    for field in fields.iter() {
        spawn_control_for_field(&mut commands, field);
    }
}
```

This keeps authoring, runtime resolution, and UI rendering separate.

---

# Main Design Rule

The cleanest reconciliation is:

```rust
Menu fields are generic.
Rig sliders are one kind of resolved field effect.
The actual character struct is built from resolved config, not from UI controls.
Every field provides camera metadata.
```

This lets the current rig-slider system remain simple while leaving room for feature sliders, clothing sliders, animation controls, and later paint/material controls.

--- 

# Characters

> [!WARNING]
> When implementing these characters always stop and report if a given asset is missing.

Asset paths in this section use the shorthand `assets/…` (linked to [`maybraid/assets/characters/`](../../assets/characters/)). At runtime, load them as `characters/…` under the `maybraid/assets` root (same convention as [`characters-playground`](../playground/src/character.rs)).

Definitions and resolution logic belong in [`characters`](./); the concepts screen wires them through [`character-concepts-playground`](../../../../menu/character-concepts-playground/).

## Assembly

Each species preview is a small hierarchy of spawned GLB scenes:

1. **Body rig** (`Humanoid`) — animation target and bone map for body-skinned parts.
2. **Body mesh** — skin remapped onto the body rig.
3. **Head rig** (`OrthogradeHeadRig`) — socketed child of the body rig at `upper_neck`. Own bone map for head-skinned parts.
4. **Head mesh** — skinned to the head rig (not the body rig).
5. **Features** (eyes, nose, mouth, ears, hair) — skinned to the head rig and/or socketed on head-rig bones. See attachment table below.
6. **Clothes** — skinned to `Humanoid`; each layer remaps onto the body rig. `NoChanges` fit unless a garment defines otherwise.

`*_left` feature meshes denote the authored side; mirror them for the right (scale/attachment on the paired head-rig bone).

The playground’s fixed `HEAD_SCALE` socket is a stopgap. Here, head placement and overall silhouette come from species **Height / Width / Depth** plus rig sliders, not a hard-coded scale factor.

Keep skinned scene-root transforms minimal. Body width, head proportions, and clothing fit should come from the relevant body/head rig bones after skin remap.

### Feature attachment (`Braidman`)

| Feature | Head-rig socket bone | Skinned to |
|---|---|---|
| Head mesh | (root of head rig) | `OrthogradeHeadRig` |
| Eyes | `eye.L`, `eye.R` | `OrthogradeHeadRig` |
| Nose | `nose` | `OrthogradeHeadRig` |
| Mouth | `mouth` | `OrthogradeHeadRig` |
| Ears | `cheek.L`, `cheek.R` | `LateralEarRig` (per ear mesh) |
| Hair | `crown` | `OrthogradeHeadRig` |

### Authored asset scale (`Braidman`)

These are one-time asset-local normalization scales for 1m-radius authored assets. They are not species proportions, rig sliders, or preview-root scales.

* Head rig and head-sized hair: `0.12`.
* Eyes: `0.04`.
* Standard, Broad, and Loaf noses: `0.05`.
* Balloon nose: `0.08`.
* Mouth: `0.02`.
* Ears: `0.08`.

The head asset is base anchored: +Z in Blender / +Y in Bevy. The other feature assets are roughly centroid anchored, so their normalization scale should not imply the same base-anchor rule.

Ears are authored turned sideways, facing +X. That orientation matters for socket attachment metadata, but it does not change the normalization scale listed here.

## Sliders and scale

**Species Height / Width / Depth** are baseline bone-scale groups, not scene-root scale values. Everything else deforms from that species baseline.

Resolution order (see [Stage 1](#stage-1-menu-state-to-resolved-character-config) above): `species baseline -> gender preset -> build preset -> user-facing slider values -> clamp -> map to rig effects`.

Two layers:

* **User-facing sliders** — what the concepts screen exposes (grouped by body, head, feature, etc.).
* **Rig sliders** — underlying `Slidable` bone effects in `character-rigs`; one user slider may drive one or more rig sliders, and each effect composes with the bind pose.

When the same label appears at head and feature scope, they compose rather than override:

| User-facing control | Scope | Effect |
|---|---|---|
| Eye Spacing | Head | Move `eye.L` / `eye.R` apart on the head rig |
| Eye Width | Eye | Horizontal scale of the eye mesh |
| Eye Height | Eye | Vertical scale of the eye mesh |
| Eye Tilt | Eye | Rotate the eye mesh (degrees) |
| Nose Width | Head | Span of nose-region layout bones |
| Nose Width | Nose | Horizontal scale of the nose mesh |
| Nose Length | Head | Nose protrusion along the head rig |
| Nose Length / Depth | Nose | Scale of the nose mesh |

Gender and build presets below adjust the same body rig sliders as percent offsets on the current value.

## `Braidman`

`Braidman` is a relatively plain humanoid species.

- **Body Rig:** `Humanoid` `Armature` in [`assets/bodies/humanoid_rig.glb`](../../assets/characters/bodies/biped/humanoid_rig.glb)
    - **Base bone scales:**
        - Lower chest width group: `0.8`.
        - Waist width group: `1.2`.
        - Other body bones: species baseline `1.0`.
- **Body Meshes:** 
    - **Standard:** `HumanoidFullBody` `Mesh` in [`assets/bodies/humanoid_full_body.glb`](../../assets/characters/bodies/biped/humanoid_full_body.glb)
    - **Full:** `LeronBipedFullBody` `Mesh` in [`assets/bodies/leron_biped_full_body.glb`](../../assets/characters/bodies/biped/leron_biped_full_body.glb)
- **Body Rig Sliders:**
    - **Height:** from 0.5 to 1.5 of the species baseline height.
        - Scaling height increases the length of the legs, arms, and spinal bones. 
    - **Shoulder Width:** from 0.8 to 1.2 of the species baseline shoulder width.
        - Scaling shoulder width increases the length of the shoulder bones.
    - **Hip Width:** from 0.8 to 1.4 of the species baseline hip width.
        - Scaling hip width increases the length of the pelvis bones.
    - **Trunk Thickness:** from 0.8 to 1.2 of the species baseline trunk thickness.
        - Scaling trunk thickness increases the X-Z scale of the bones in the back pelvis and shoulders. 
    - **Chest Thickness:** from 0.8 to 1.2 of the species baseline chest thickness.
        - Scaling chest thickness increases the length of the chest thickness bones.
    - **Back Thickness:** from 0.8 to 1.2 of the species baseline back thickness.
        - Scaling back thickness increases the length of the back thickness bones.
    - **Belly Thickness:** from 0.8 to 1.2 of the species baseline belly thickness (`upper_belly`, `lower_belly`).
        - Scaling belly thickness increases the length of the belly thickness bones.
    - **Buttocks Thickness:** from 0.8 to 1.2 of the species baseline buttock thickness.
        - Scaling buttocks thickness increases the length of the buttocks-thickness bones.
    - **Arm Length:** from 0.8 to 1.2 of the species baseline arm length.
        - Scaling arm length increases the length of the arm bones.
    - **Arm Thickness:** from 0.8 to 1.2 of the species baseline arm thickness.
        - Scaling arm thickness increases the X-Z scale of the bones in the arms.
    - **Leg Length:** from 0.8 to 1.2 of the species baseline leg length.
        - Scaling leg length increases the length of the leg bones.
    - **Leg Thickness:** from 0.8 to 1.2 of the species baseline leg thickness.
        - Scaling leg thickness increases the X-Z scale of the bones in the legs.
    - **Neck Length:** from 0.8 to 1.2 of the species baseline neck length.
        - Scaling neck length increases the length of the neck bones.
    - **Neck Thickness:** from 0.8 to 1.2 of the species baseline neck thickness.
        - Scaling neck thickness increases the X-Z scale of the bones in the neck.
- **Head Rig:** `OrthogradeHeadRig` `Armature` in [`assets/heads/orthograde_head.glb`](../../assets/characters/heads/orthograde_head.glb), bones dumped in [`assets/heads/orthograde_head.armature_dump`](../../assets/characters/heads/orthograde_head.armature_dump). Socket on body rig `upper_neck`.
- **Heads:**
    - **Standard:** `MeerkatHead` `Mesh` in [`assets/heads/meerkat_head.glb`](../../assets/characters/heads/meerkat_head.glb)
    - **Gaunt:** `GauntOrthoHumanoidHead` `Mesh` in [`assets/heads/gaunt_ortho_humanoid_head.glb`](../../assets/characters/heads/gaunt_ortho_humanoid_head.glb)
    - **Full:** `FullOrthHumanoidHead` `Mesh` in [`assets/heads/full_ortho_humanoid_head.glb`](../../assets/characters/heads/full_ortho_humanoid_head.glb)
- **Head Sliders:** (layout on the head rig; see [slider table](#sliders-and-scale) for naming)
    - **Head Width:** from 0.8 to 1.2 of the species baseline head width.
        - Scaling head width increases the length of lateral bones in the head rig.
    - **Head Height:** from 0.8 to 1.2 of the species baseline head height.
        - Scaling head height increases the length of medial vertical bones in the head rig.
    - **Head Depth:** from 0.8 to 1.2 of the species baseline head depth.
        - Scaling head depth increases the length of medial horizontal bones in the head rig.
    - **Eye Spacing:** from 0.8 to 1.2 of the species baseline inter-eye distance.
    - **Eye-line Height:** from 0.8 to 1.2 of the species baseline eye height.
    - **Nose Width:** from 0.8 to 1.2 of the species baseline nose-region span.
    - **Nose Length:** from 0.8 to 1.2 of the species baseline nose length along the head rig.
    - **Nose Protrusion:** from 0.8 to 1.2 of the species baseline nose protrusion.
- **Eyes:** (author `*_left`; mirror for the right eye)
    - **Standard:** `HumanoidEyeLeft` `Mesh` in [`assets/eyes/humanoid_eye_left.glb`](../../assets/characters/eyes/humanoid_eye_left.glb)
    - **Falcon:** `FalconEyeLeft` `Mesh` in [`assets/eyes/falcon_eye_left.glb`](../../assets/characters/eyes/falcon_eye_left.glb)
- **Eye Sliders:** (per-eye mesh scale/rotation; compose with head **Eye Spacing**)
    - **Eye Width:** from 0.8 to 1.2 of the selected eye mesh baseline width.
    - **Eye Height:** from 0.8 to 1.2 of the selected eye mesh baseline height.
    - **Eye Tilt:** -5 to 5 degrees.
- **Noses:**
    - **Standard:** `HumanoidNose` `Mesh` in [`assets/noses/humanoid_nose.glb`](../../assets/characters/noses/humanoid_nose.glb)
    - **Broad:** `BroadHumanoidNose` `Mesh` in [`assets/noses/broad_humanoid_nose.glb`](../../assets/characters/noses/broad_humanoid_nose.glb)
    - **Loaf:** `LoafHumanoidNose` `Mesh` in [`assets/noses/loaf_nose.glb`](../../assets/characters/noses/loaf_nose.glb)
    - **Balloon:** `MumbusNose` `Mesh` in [`assets/noses/mumbus_nose.glb`](../../assets/characters/noses/mumbus_nose.glb)
- **Nose Sliders:** (per-nose mesh scale; compose with head nose layout sliders)
    - **Nose Width:** from 0.8 to 1.2 of the selected nose mesh baseline width.
    - **Nose Length:** from 0.8 to 1.2 of the selected nose mesh baseline length.
    - **Nose Depth:** from 0.8 to 1.2 of the selected nose mesh baseline depth.
- **Mouths:**
    - **Standard:** `CommonMouth` `Mesh` in [`assets/mouths/common_mouth.glb`](../../assets/characters/mouths/common_mouth.glb)
- **Mouth Sliders:**
    - **Mouth Width:** from 0.8 to 1.2 of the selected mouth mesh baseline width.
    - **Mouth Height:** from 0.8 to 1.2 of the selected mouth mesh baseline height.
    - **Mouth Depth:** from 0.8 to 1.2 of the selected mouth mesh baseline depth.
- **Ears:** (author `*_left`; mirror for the right ear)
    - **Standard:** `RoundScoopLateralEarLeft` `Mesh` in [`assets/ears/round_scoop_lateral_ear_left.glb`](../../assets/characters/ears/round_scoop_lateral_ear_left.glb)
    - **Round:** `RoundLateralEarLeft` `Mesh` in [`assets/ears/round_lateral_ear_left.glb`](../../assets/characters/ears/round_lateral_ear_left.glb)
    - **Flank:** `FlankLateralEarLeft` `Mesh` in [`assets/ears/flank_lateral_ear_left.glb`](../../assets/characters/ears/flank_lateral_ear_left.glb)
- **Ear Sliders:**
    - **Ear Width:** from 0.8 to 1.2 of the selected ear mesh baseline width.
    - **Ear Height:** from 0.8 to 1.2 of the selected ear mesh baseline height.
    - **Ear Depth:** from 0.8 to 1.2 of the selected ear mesh baseline depth.
- **Extended Features:**
    - **Head Hair:** 
        - **None:** no hair on head.
        - **Thick Braids:** `ThickBraids` `Mesh` in [`assets/hair/thick_braids.glb`](../../assets/characters/hair/thick_braids.glb)
        - **Flowing Curls:** `FlowingCurls` `Mesh` in [`assets/hair/flowing_curls.glb`](../../assets/characters/hair/flowing_curls.glb)
        - **Wrapping Braids:** `WrappingBraids` `Mesh` in [`assets/hair/wrapping_braids.glb`](../../assets/characters/hair/wrapping_braids.glb)
        - **Wrapping Braids Hanging Locks:** `Wrapping Braids Hanging Locks` `Mesh` in [`assets/hair/wrapping_braids_hanging_locks.glb`](../../assets/characters/hair/wrapping_braids_hanging_locks.glb)
        - **Braid Hawk:** `BraidHawk` `Mesh` in [`assets/hair/braid_hawk.glb`](../../assets/characters/hair/braid_hawk.glb)
        - **Feather Hawk:** `FeatherHawk` `Mesh` in [`assets/hair/feather_hawk.glb`](../../assets/characters/hair/feather_hawk.glb)
        - **Flowing Edgy Curls:** `FlowingEdgyCurls` `Mesh` in [`assets/hair/flowing_edgy_curls.glb`](../../assets/characters/hair/flowing_edgy_curls.glb)
        - **Perm Braid:** `PermBraid` `Mesh` in [`assets/hair/perm_braid.glb`](../../assets/characters/hair/perm_braid.glb)
        - **Techno Edge:** `TechnoEdge` `Mesh` in [`assets/hair/techno_edge.glb`](../../assets/characters/hair/techno_edge.glb)
- **Clothes:** (can wear as many as you want at the same time; each remaps to the body rig, `NoChanges` fit)
    - **Tunic:** `Tunic` `Mesh` in [`assets/clothes/tunic.glb`](../../assets/characters/clothes/body/tunic.glb).
    - **Long Dress:** `LongDress` `Mesh` in [`assets/clothes/long_dress.glb`](../../assets/characters/clothes/body/long_dress.glb).
    - **Short Dress:** `ShortDress` `Mesh` in [`assets/clothes/short_dress.glb`](../../assets/characters/clothes/body/short_dress.glb).
    - **Fitted Coat:** `FittedCoat` `Mesh` in [`assets/clothes/fitted_coat.glb`](../../assets/characters/clothes/body/fitted_coat.glb).
    - **Robe Coat:** `RobeCoat` `Mesh` in [`assets/clothes/robe_coat.glb`](../../assets/characters/clothes/body/robe_coat.glb).
    - **Robe:** `Robe` `Mesh` in [`assets/clothes/robe.glb`](../../assets/characters/clothes/body/robe.glb).
- **Animations:** walk, run, two-footed jump, tucked flip (humanoid-compatible; see [`character-animations`](../animations/)).
- **Genders:** (percent offsets on body rig sliders after species baseline)
    - **Male:**
        - Increase shoulder width by 5%.
    - **Female:**
        - Decrease shoulder width by 5%.
        - Increase chest thickness by 20%.
        - Increase hip width by 10%.
        - Increase buttocks thickness by 10%.
    - **Non-binary:**
        - Decrease shoulder width by 5%.
- **Builds:** (percent offsets on body rig sliders after gender preset)
    - **Slender:**
        - Decrease shoulder width by 5%.
        - Decrease chest thickness by 10%.
        - Decrease hip width by 10%.
    - **Athletic:**
        - Increase shoulder width by 5%.
        - Increase chest thickness by 10%.
        - Increase hip width by 10%.
        - Increase arm length by 10%.
        - Increase arm thickness by 10%.
        - Increase leg length by 10%.
    - **Heavy:**
        - Increase shoulder width by 5%.
        - Increase chest thickness by 10%.
        - Increase hip width by 10%.
        - Increase belly thickness by 20%.
    - **Stocky:**
        - Increase shoulder width by 10%.
        - Increase chest thickness by 10%.
        - Increase arm thickness by 10%.
        - Increase leg thickness by 10%.
    - **Lanky:**
        - Decrease shoulder width by 5%.
        - Decrease chest thickness by 10%.
        - Decrease hip width by 10%.
        - Decrease belly thickness by 20%.
        - Increase neck length by 10%.
        - Increase leg length by 10%.
        - Increase arm length by 10%.
