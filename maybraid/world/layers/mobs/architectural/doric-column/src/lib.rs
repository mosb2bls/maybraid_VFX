//! Reusable Doric column + pedestal assembly for Bikanwarda ([#1133](https://github.com/ramate-io/maybraid/issues/1133)).
//!
//! Shaft, capital, and pedestal are separate kit parts with stable anchors,
//! Fixed-layer cover colliders, and a terrain ground-contact contract. Boss AI
//! and Lubikan logic stay out of this crate — encounters and summons only spawn
//! [`spawn_doric_column_encounter`] / [`spawn_doric_column_summon`].
//!
//! Authoring guidance for final GLBs: [`AUTHORING.md`](AUTHORING.md).

pub mod anchors;
pub mod assets;
pub mod authoring;
pub mod colliders;
pub mod layout;
pub mod materials;
pub mod parts;
pub mod placeholder;
pub mod plugin;
pub mod spawn;
pub mod spec;
pub mod terrain;

pub use authoring::{
	NORMALIZED_LOWER_SHAFT_DIAMETER, uniform_placement_scale, placement_scale_vec,
};
pub use anchors::{
	CHAIN_ANCHOR_NAMES, DoricColumnAnchors, DoricColumnChainAnchor, DoricColumnGroundContact,
};
pub use colliders::{cover_aabb, cover_collider, cover_collider_shapes};
pub use materials::DoricColumnMaterials;
pub use parts::{
	DoricColumnComponents, DoricColumnPartNode, DoricColumnPartSlot, DoricColumnUseAuthoredGlb,
};
pub use plugin::DoricColumnPlugin;
pub use spawn::{
	spawn_doric_column_encounter, spawn_doric_column_summon, DoricColumnCoverCollider,
	DoricColumnEncounter, DoricColumnRoot, DoricColumnSummon,
};
pub use spec::{DoricColumnDimensions, DoricColumnRatios, DoricColumnSpec};
pub use terrain::{snap_doric_columns_to_terrain, DoricColumnTerrainPlacement};
