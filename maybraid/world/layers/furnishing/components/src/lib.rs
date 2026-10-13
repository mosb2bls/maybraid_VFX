//! Reusable furniture scene components: kit paths, slot remap, posed GLBs.
//!
//! Parallel to Chico vegetation-components:
//! Richmond [`AssetPath`](building_components::AssetPath) + [`Placement`](building_components::Placement)
//! → one [`scene_ref::SceneRef`] per part, painted with [`material_ref::MaterialRef`].
//! Assemblies (`furniture-assemblies`) explode Richmond slots into [`PlacedPart`]s.
//!
//! Blender sources live in `maybraid/art/furniture/`
//! ([`ecb5a8a`](https://github.com/ramate-io/maybraid/commit/ecb5a8a31d2894311fa690cfbe8ab01cee385fba)).
//! Export scene 0 into [`assets`]. Skip `.blend1`.

pub mod assets;
pub mod kit_space;
pub mod parts;
pub mod scene_children;

pub use kit_space::{
	latch_slab, place_kit, run_slab, shift, slab, slab_xz, BOX_KIT_MAX, BOX_KIT_MIN,
	BOX_KIT_TO_UNIT, HINGE_KIT_TO_UNIT, LATCH_KIT_MAX, LATCH_KIT_MIN, LATCH_KIT_TO_UNIT,
	LEG_KIT_MAX, LEG_KIT_MIN, LEG_KIT_TO_UNIT, RANGE_DOOR_KIT_TO_UNIT,
};
pub use parts::{pose_parts, FurnitureKitPart, PartKind, PlacedPart};
pub use scene_children::{assembly_scene, posed_kit, posed_kit_part};
