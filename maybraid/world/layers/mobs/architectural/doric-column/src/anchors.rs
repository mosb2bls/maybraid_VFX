//! Named anchor transforms in assembly space (+Y up, +Z forward).

use std::collections::BTreeMap;

use bevy::prelude::*;
use crate::layout::DoricColumnLayout;
use crate::spec::DoricColumnSpec;

/// Stable anchor names shared with authored GLBs (empty nodes / rig bones).
pub mod names {
	pub const GROUND_CONTACT: &str = "ground_contact";
	pub const PEDESTAL_TOP: &str = "pedestal_top";
	pub const SHAFT_TOP: &str = "shaft_top";
	pub const CAPITAL_TOP: &str = "capital_top";
	pub const CHAIN_ANCHOR_POS_X: &str = "chain_anchor_pos_x";
	pub const CHAIN_ANCHOR_NEG_X: &str = "chain_anchor_neg_x";
	pub const CHAIN_ANCHOR_POS_Z: &str = "chain_anchor_pos_z";
	pub const CHAIN_ANCHOR_NEG_Z: &str = "chain_anchor_neg_z";
}

pub const CHAIN_ANCHOR_NAMES: [&str; 4] = [
	names::CHAIN_ANCHOR_POS_X,
	names::CHAIN_ANCHOR_NEG_X,
	names::CHAIN_ANCHOR_POS_Z,
	names::CHAIN_ANCHOR_NEG_Z,
];

/// Marker on anchor child entities for chain attachment lanes.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DoricColumnChainAnchor {
	pub name: &'static str,
}

/// Marker on the ground-contact anchor.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DoricColumnGroundContact;

/// All named anchors for one assembly, in **assembly-local** space.
#[derive(Debug, Clone, PartialEq)]
pub struct DoricColumnAnchors {
	pub transforms: BTreeMap<&'static str, Transform>,
}

impl DoricColumnAnchors {
	pub fn for_spec(spec: &DoricColumnSpec) -> Self {
		let layout = DoricColumnLayout::from_spec(spec);
		let mut transforms = BTreeMap::new();
		transforms.insert(names::GROUND_CONTACT, Transform::IDENTITY);
		let pedestal_top_y = layout.plinth.height;
		let shaft_top_y = layout.shaft_top_y();
		let capital_top_y = layout.total_height();
		transforms.insert(
			names::PEDESTAL_TOP,
			Transform::from_translation(Vec3::new(0.0, pedestal_top_y, 0.0)),
		);
		transforms.insert(
			names::SHAFT_TOP,
			Transform::from_translation(Vec3::new(0.0, shaft_top_y, 0.0)),
		);
		transforms.insert(
			names::CAPITAL_TOP,
			Transform::from_translation(Vec3::new(0.0, capital_top_y, 0.0)),
		);

		let chain_y = shaft_top_y;
		let chain_r = layout.shaft.radius_top + layout.chain_anchor_outset;
		transforms.insert(
			names::CHAIN_ANCHOR_POS_X,
			chain_anchor(chain_r, chain_y, Vec3::X),
		);
		transforms.insert(
			names::CHAIN_ANCHOR_NEG_X,
			chain_anchor(chain_r, chain_y, -Vec3::X),
		);
		transforms.insert(
			names::CHAIN_ANCHOR_POS_Z,
			chain_anchor(chain_r, chain_y, Vec3::Z),
		);
		transforms.insert(
			names::CHAIN_ANCHOR_NEG_Z,
			chain_anchor(chain_r, chain_y, -Vec3::Z),
		);
		Self { transforms }
	}

	pub fn get(&self, name: &str) -> Option<&Transform> {
		self.transforms.get(name)
	}
}

fn chain_anchor(radius: f32, y: f32, outward: Vec3) -> Transform {
	let pos = outward.normalize() * radius + Vec3::new(0.0, y, 0.0);
	let rotation = Quat::from_rotation_arc(Vec3::Y, outward.normalize_or_zero());
	Transform::from_translation(pos).with_rotation(rotation)
}

/// World-space anchor position for tests and tooling.
pub fn anchor_world(spec: &DoricColumnSpec, root: &Transform, name: &str) -> Option<Vec3> {
	let anchors = DoricColumnAnchors::for_spec(spec);
	let local = anchors.get(name)?;
	Some(root.transform_point(local.translation))
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Color;
	use crate::{DoricColumnDimensions, DoricColumnMaterials, DoricColumnSpec};

	#[test]
	fn chain_anchors_scale_with_shaft_radius() {
		let small = DoricColumnSpec {
			dims: DoricColumnDimensions::default(),
			scale: Vec3::splat(0.5),
			yaw: 0.0,
			materials: DoricColumnMaterials::default(),
		};
		let large = DoricColumnSpec {
			dims: DoricColumnDimensions::default(),
			scale: Vec3::splat(2.0),
			yaw: 0.0,
			materials: DoricColumnMaterials::default(),
		};
		let a = DoricColumnAnchors::for_spec(&small);
		let b = DoricColumnAnchors::for_spec(&large);
		let small_x = a.get(names::CHAIN_ANCHOR_POS_X).unwrap().translation.x;
		let large_x = b.get(names::CHAIN_ANCHOR_POS_X).unwrap().translation.x;
		assert!(large_x > small_x);
		assert!((large_x / small_x - 4.0).abs() < 1e-3);
	}

	#[test]
	fn material_swap_does_not_move_anchors() {
		let base = DoricColumnSpec::default();
		let mut painted = DoricColumnSpec::default();
		painted.materials.shaft = painted.materials.shaft.with_palette([Color::srgb(0.9, 0.1, 0.1)]);
		let a = DoricColumnAnchors::for_spec(&base);
		let b = DoricColumnAnchors::for_spec(&painted);
		assert_eq!(a.transforms, b.transforms);
	}

	#[test]
	fn pedestal_top_matches_part_stack() {
		let spec = DoricColumnSpec::default();
		let layout = spec.layout();
		let anchors = DoricColumnAnchors::for_spec(&spec);
		let top = anchors.get(names::PEDESTAL_TOP).unwrap().translation.y;
		let expected = layout.plinth.height;
		assert!((top - expected).abs() < 1e-4);
	}
}
