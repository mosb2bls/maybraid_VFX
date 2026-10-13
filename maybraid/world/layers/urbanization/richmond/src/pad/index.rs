//! Uniform-grid broadphase over conservative pad-node support AABBs.

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use bevy::math::Vec2;
use procedural_common::Bounds2;

use super::node::PadNode;

#[derive(Clone)]
pub struct PadFootprintIndex {
	bounds: Bounds2,
	origin: Vec2,
	cell: f32,
	buckets: HashMap<(i32, i32), Vec<usize>>,
}

impl Debug for PadFootprintIndex {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PadFootprintIndex")
			.field("bounds", &self.bounds)
			.field("cell", &self.cell)
			.field("bucket_count", &self.buckets.len())
			.finish()
	}
}

impl PadFootprintIndex {
	pub fn build(bounds: Bounds2, nodes: &[PadNode]) -> Self {
		let short = bounds.extent().min_element().max(1.0);
		let cell = (short * 0.08).clamp(8.0, 32.0);
		let origin = bounds.min;
		let mut buckets: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
		for (id, node) in nodes.iter().enumerate() {
			let support = node.correction_index_bounds();
			let min = support.min.max(bounds.min);
			let max = support.max.min(bounds.max);
			if min.x > max.x || min.y > max.y {
				continue;
			}
			let x0 = ((min.x - origin.x) / cell).floor() as i32;
			let x1 = ((max.x - origin.x) / cell).floor() as i32;
			let z0 = ((min.y - origin.y) / cell).floor() as i32;
			let z1 = ((max.y - origin.y) / cell).floor() as i32;
			for x in x0..=x1 {
				for z in z0..=z1 {
					buckets.entry((x, z)).or_default().push(id);
				}
			}
		}
		Self { bounds, origin, cell, buckets }
	}

	pub fn candidates(&self, p: Vec2) -> &[usize] {
		if !self.bounds.contains(p) {
			return &[];
		}
		let x = ((p.x - self.origin.x) / self.cell).floor() as i32;
		let z = ((p.y - self.origin.y) / self.cell).floor() as i32;
		self.buckets.get(&(x, z)).map(Vec::as_slice).unwrap_or(&[])
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::pad::{PadNode, PadParams};

	#[test]
	fn index_returns_only_local_pad_candidates() -> anyhow::Result<()> {
		let nodes = vec![
			PadNode::rectangular_flatten(
				Vec2::new(10.0, 10.0),
				Vec2::splat(2.0),
				0.0,
				4.0,
				PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
			),
			PadNode::rectangular_flatten(
				Vec2::new(90.0, 90.0),
				Vec2::splat(2.0),
				0.0,
				8.0,
				PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
			),
		];
		let bounds = Bounds2::from_xz(0.0, 0.0, 100.0, 100.0);
		let index = PadFootprintIndex::build(bounds, &nodes);
		anyhow::ensure!(index.candidates(Vec2::new(10.0, 10.0)) == [0]);
		anyhow::ensure!(index.candidates(Vec2::new(90.0, 90.0)) == [1]);
		anyhow::ensure!(index.candidates(Vec2::new(50.0, 50.0)).is_empty());
		Ok(())
	}
}
