//! Historical query-and-insert adapter, reconstructed from public APIs.
//!
//! Main filled with `out.extend(query_pairs → Vec<Id>)`. `query_iter` still
//! uses that sorted pair walk. This type lives here so production `SpatialIndex`
//! does not grow a benchmark-only method.

use bevy_math::bounding::Aabb3d;
use bevy_platform::collections::HashSet;
use gimme_core::{BaseScale, SpatialIndex, SpatialIndexQuery};

/// Main's sorted pair-walk fill (`out.extend(query_pairs → Vec<Id>)`).
pub struct PairWalkFill<'a, Id> {
	index: &'a SpatialIndex<Id>,
}

impl<'a, Id> PairWalkFill<'a, Id>
where
	Id: Copy + Eq + std::hash::Hash + Ord,
{
	pub fn new(index: &'a SpatialIndex<Id>) -> Self {
		Self { index }
	}

	pub fn query_ids(&self, region: Aabb3d) -> Vec<Id> {
		self.index
			.query_iter(region, BaseScale::levels_through(self.index.max_level()))
			.map(|(id, _)| id)
			.collect()
	}

	pub fn extend_hosts_in_region(&mut self, region: Aabb3d, out: &mut HashSet<Id>) {
		out.extend(self.query_ids(region));
	}
}

/// Destination-union fill used by the Gimme `LodSceneHostIndex` override.
///
/// Calls [`SpatialIndexQuery::extend_intersecting`] so the bench path matches
/// the adapter (static dispatch; no `dyn`).
pub struct ExtendIntersectingFill<'a, Id> {
	index: &'a SpatialIndex<Id>,
}

impl<'a, Id> ExtendIntersectingFill<'a, Id>
where
	Id: Copy + Eq + std::hash::Hash + Ord,
{
	pub fn new(index: &'a SpatialIndex<Id>) -> Self {
		Self { index }
	}

	pub fn extend_hosts_in_region(&mut self, region: Aabb3d, out: &mut HashSet<Id>) {
		SpatialIndexQuery::extend_intersecting(self.index, region, out);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::{DVec3, Vec3};
	use gimme_core::SpatialIndex;

	fn aabb(min: [f32; 3], max: [f32; 3]) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::from_array(min), Vec3::from_array(max))
	}

	#[test]
	fn pair_walk_matches_sorted_query() -> Result<()> {
		let mut idx = SpatialIndex::new(DVec3::ONE)?;
		idx.insert(3, aabb([1.0, 0.0, 0.0], [2.0, 1.0, 1.0]))?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(2, aabb([0.5, 0.0, 0.0], [1.5, 1.0, 1.0]))?;
		let region = aabb([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
		assert_eq!(PairWalkFill::new(&idx).query_ids(region), idx.query(region));
		assert_eq!(idx.query(region), vec![1, 2, 3]);
		Ok(())
	}
}
