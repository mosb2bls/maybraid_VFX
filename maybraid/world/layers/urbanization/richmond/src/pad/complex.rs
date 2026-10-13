//! Indexed pad complex: member skirt nodes + sample-time flatten / ease.

use bevy::math::Vec2;
use durham::terrain::{ElevationModulation, TerrainSdf};
use procedural_common::Bounds2;
use urbanization_developments::{PadParams, PadPlan};

use super::index::PadFootprintIndex;
use super::node::{PadNode, PadStage};

/// Bag of pad nodes blended by occupancy softmax, with flatten terraces exact.
///
/// [`Self::bounds`] is the union of each node's yawed support AABB (flatten +
/// ease), not the development cell. Early-out is a conservative OBB AABB;
/// classification still uses the yawed footprint SDF.
#[derive(Debug, Clone)]
pub struct PadComplex {
	pub bounds: Bounds2,
	pub pads: Vec<PadNode>,
	index: PadFootprintIndex,
}

impl PadComplex {
	pub fn new(bounds: Bounds2) -> Self {
		Self { bounds, pads: Vec::new(), index: PadFootprintIndex::build(bounds, &[]) }
	}

	pub fn with_pads(mut self, pads: Vec<PadNode>) -> Self {
		self.pads = pads;
		self.bounds = union_pad_bounds(&self.pads);
		self.index = PadFootprintIndex::build(self.bounds, &self.pads);
		self
	}

	pub fn from_nodes(mut pads: Vec<PadNode>) -> Self {
		// Development cells live in a HashMap. Canonicalize the merged order so
		// equivalent terrain cells produce the same blend ties and mesh identity.
		pads.sort_by_cached_key(|node| format!("{node:?}"));
		let bounds = union_pad_bounds(&pads);
		let index = PadFootprintIndex::build(bounds, &pads);
		Self { bounds, pads, index }
	}

	/// Flatten / grade / ease must blend in one pass. Sequential complexes let a
	/// later ease skirt undo an earlier terrace and smear overlapping path
	/// skirts into a general lift.
	pub fn union_all(complexes: impl IntoIterator<Item = Self>) -> Self {
		Self::from_nodes(complexes.into_iter().flat_map(|c| c.pads).collect())
	}

	/// One rectangular flatten terrace for a yawed building plan.
	pub fn building_skirt(
		center: Vec2,
		building_half_extents: Vec2,
		yaw: f32,
		height: f32,
		params: PadParams,
	) -> Self {
		Self::from_nodes(vec![PadNode::rectangular_flatten(
			center,
			building_half_extents,
			yaw,
			height,
			params,
		)])
	}

	/// One graded reach node per polyline segment, analog of hydro `nodes_from_polyline`.
	pub fn graded_polyline(
		path: &[Vec2],
		levels: &[f32],
		half_width: f32,
		params: PadParams,
	) -> Self {
		Self::from(&PadPlan::graded_polyline(path, levels, half_width, params))
	}

	pub fn is_empty(&self) -> bool {
		self.pads.is_empty()
	}

	pub fn nodes_intersecting(&self, p: Vec2) -> impl Iterator<Item = &PadNode> {
		self.index
			.candidates(p)
			.iter()
			.filter_map(|&id| self.pads.get(id))
			.filter(move |node| node.contains_index_point(p))
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return elevation;
		}
		let nodes = self.nodes_intersecting(p);
		PadNode::elevation_blend(nodes, elevation, p)
	}

	pub fn classification_at(&self, x: f32, z: f32) -> Option<PadStage> {
		let p = Vec2::new(x, z);
		let mut best: Option<(&PadNode, f32)> = None;
		for node in self.nodes_intersecting(p) {
			let phi = node.phi(p);
			if best.map(|(_, d)| phi < d).unwrap_or(true) {
				best = Some((node, phi));
			}
		}
		best.and_then(|(node, phi)| node.classification_at_distance(phi))
	}
}

impl From<&PadPlan> for PadComplex {
	fn from(plan: &PadPlan) -> Self {
		Self::from_nodes(plan.forms.iter().map(PadNode::from).collect())
	}
}

impl urbanization_layer_model::PadOps for PadComplex {
	fn modify_elevation(&self, height: f32, x: f32, z: f32) -> f32 {
		PadComplex::modify_elevation(self, height, x, z)
	}
}

impl ElevationModulation for PadComplex {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		PadComplex::modify_elevation(self, elevation, x, z)
	}

	fn mesh_identity(&self) -> String {
		// The footprint index contains HashMap buckets. They accelerate sampling
		// but do not affect geometry and must not enter persistent mesh keys.
		format!("PadComplex {{ bounds: {:?}, pads: {:?} }}", self.bounds, self.pads)
	}
}

fn union_pad_bounds(pads: &[PadNode]) -> Bounds2 {
	let mut min = Vec2::splat(f32::INFINITY);
	let mut max = Vec2::splat(f32::NEG_INFINITY);
	for node in pads {
		let b = node.correction_index_bounds();
		min = min.min(b.min);
		max = max.max(b.max);
	}
	if !min.is_finite() {
		return Bounds2::from_xz(0.0, 0.0, 0.0, 0.0);
	}
	Bounds2 { min, max }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn identity_is_independent_of_merged_node_order() {
		let first = PadNode::rectangular_flatten_default(
			Vec2::new(10.0, 20.0),
			Vec2::splat(4.0),
			0.25,
			3.0,
		);
		let second = PadNode::rectangular_flatten_default(
			Vec2::new(-30.0, 5.0),
			Vec2::new(8.0, 2.0),
			-0.5,
			7.0,
		);
		let forward = PadComplex::from_nodes(vec![first.clone(), second.clone()]);
		let reverse = PadComplex::from_nodes(vec![second, first]);

		assert_eq!(forward.mesh_identity(), reverse.mesh_identity());
	}
}
