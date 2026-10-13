//! Uniform-grid broadphase over conservative node AABBs.

use crate::primitive::node::HydroNode;
use bevy_math::Vec2;
use procedural_common::Bounds2;
use std::collections::HashMap;

/// Uniform-grid broadphase over conservative AABBs (not exact intersection bake).
#[derive(Debug, Clone)]
pub struct FootprintIndex {
	bounds: Bounds2,
	origin: Vec2,
	cell: f32,
	buckets: HashMap<(i32, i32), Vec<u16>>,
}

impl FootprintIndex {
	pub fn empty() -> Self {
		Self {
			bounds: Bounds2::from_xz(0.0, 0.0, 0.0, 0.0),
			origin: Vec2::ZERO,
			cell: 1.0,
			buckets: HashMap::new(),
		}
	}

	/// Broadphase over node hydraulic AABBs expanded by each node's index pad.
	pub fn build_nodes(bounds: Bounds2, nodes: &[HydroNode], cell: f32) -> Self {
		let cell = cell.max(1.0);
		let origin = bounds.min;
		let mut buckets: HashMap<(i32, i32), Vec<u16>> = HashMap::new();
		for (i, node) in nodes.iter().enumerate() {
			let id = i as u16;
			let support = node.correction_index_bounds();
			let mn = support.min.max(bounds.min);
			let mx = support.max.min(bounds.max);
			if mn.x > mx.x || mn.y > mx.y {
				continue;
			}
			let i0 = ((mn.x - origin.x) / cell).floor() as i32;
			let i1 = ((mx.x - origin.x) / cell).floor() as i32;
			let j0 = ((mn.y - origin.y) / cell).floor() as i32;
			let j1 = ((mx.y - origin.y) / cell).floor() as i32;
			for ix in i0..=i1 {
				for iz in j0..=j1 {
					buckets.entry((ix, iz)).or_default().push(id);
				}
			}
		}
		Self { bounds, origin, cell, buckets }
	}

	pub fn candidates(&self, p: Vec2) -> &[u16] {
		if !self.bounds.contains(p) {
			return &[];
		}
		let ix = ((p.x - self.origin.x) / self.cell).floor() as i32;
		let iz = ((p.y - self.origin.y) / self.cell).floor() as i32;
		self.buckets.get(&(ix, iz)).map(|v| v.as_slice()).unwrap_or(&[])
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::hydro::{
		Ellipse, HydroElevation, HydroFootprint, HydroPrimitive, RadialBowl,
	};
	use crate::primitive::parameters::HydroParams;

	fn ellipse_node(center: Vec2, radii: Vec2) -> HydroNode {
		let primitive = HydroPrimitive {
			footprint: HydroFootprint::Ellipse(Ellipse { center, radii, rotation: 0.0 }),
			elevation: HydroElevation::Radial(RadialBowl { surface: 10.0, center_depth: 2.0 }),
			influence_pad: 0.0,
		};
		HydroNode::new(primitive, HydroParams::default(), 0.0)
	}

	#[test]
	fn oversized_node_only_populates_local_buckets() {
		let bounds = Bounds2::from_xz(0.0, 0.0, 160.0, 160.0);
		let node = ellipse_node(Vec2::new(80.0, 80.0), Vec2::splat(2_000.0));
		let index = FootprintIndex::build_nodes(bounds, &[node], 10.0);

		assert_eq!(index.buckets.len(), 17 * 17);
		assert_eq!(index.candidates(Vec2::new(80.0, 80.0)), &[0]);
		assert!(index.candidates(Vec2::new(161.0, 80.0)).is_empty());
		assert!(index.candidates(Vec2::new(1_000.0, 1_000.0)).is_empty());
	}

	#[test]
	fn node_outside_local_bounds_is_not_indexed() {
		let bounds = Bounds2::from_xz(0.0, 0.0, 160.0, 160.0);
		let node = ellipse_node(Vec2::splat(1_000.0), Vec2::splat(10.0));
		let index = FootprintIndex::build_nodes(bounds, &[node], 10.0);

		assert!(index.buckets.is_empty());
	}
}
