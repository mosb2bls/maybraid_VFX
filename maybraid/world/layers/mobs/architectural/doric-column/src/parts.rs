//! Per-part scene nodes (authored GLB or procedural placeholder).

use bevy::prelude::*;
use building_components::Placement;
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use crate::assets;
use crate::spec::DoricColumnSpec;

/// Which kit piece this node instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoricColumnPartSlot {
	Pedestal,
	Shaft,
	Capital,
}

impl DoricColumnPartSlot {
	pub const fn asset_path(self) -> &'static str {
		match self {
			Self::Pedestal => assets::PEDESTAL_001.as_str(),
			Self::Shaft => assets::SHAFT_001.as_str(),
			Self::Capital => assets::CAPITAL_001.as_str(),
		}
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Pedestal => "pedestal",
			Self::Shaft => "shaft",
			Self::Capital => "capital",
		}
	}
}

/// When `false`, procedural placeholders are used until Liam's GLBs land.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DoricColumnUseAuthoredGlb(pub bool);

/// One posed part in the column assembly.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct DoricColumnPartNode {
	pub slot: DoricColumnPartSlot,
	pub placement: Placement,
	pub material: MaterialRef,
	pub use_authored_glb: bool,
}

impl DoricColumnPartNode {
	pub fn for_spec(
		spec: &DoricColumnSpec,
		slot: DoricColumnPartSlot,
		placement: Placement,
	) -> Self {
		let material = match slot {
			DoricColumnPartSlot::Pedestal => spec.materials.pedestal.clone(),
			DoricColumnPartSlot::Shaft => spec.materials.shaft.clone(),
			DoricColumnPartSlot::Capital => spec.materials.capital.clone(),
		};
		Self { slot, placement, material, use_authored_glb: false }
	}

	pub fn transform(&self) -> Transform {
		let p = self.placement;
		Transform { translation: p.translation, rotation: p.rotation(), scale: p.scale }
	}
}

/// Structural composition API (parallel to [`firearms_components::FirearmComponents`]).
pub trait DoricColumnComponents {
	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Vec<DoricColumnPartNode>;
}

impl DoricColumnComponents for DoricColumnSpec {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Vec<DoricColumnPartNode> {
		use crate::layout::part_placements;
		let parts = part_placements(self);
		vec![
			DoricColumnPartNode::for_spec(self, DoricColumnPartSlot::Pedestal, parts.pedestal),
			DoricColumnPartNode::for_spec(self, DoricColumnPartSlot::Shaft, parts.shaft),
			DoricColumnPartNode::for_spec(self, DoricColumnPartSlot::Capital, parts.capital),
		]
	}
}
