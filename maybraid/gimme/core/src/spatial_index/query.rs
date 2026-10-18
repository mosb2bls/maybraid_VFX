//! ID-and-bounds regional queries ([`SpatialIndexQuery`]).
//!
//! Distinct from [`crate::TypedSpatialIndex`], which reads and inserts typed
//! payloads. Grid internals (`cell_count`, `max_level`, buckets, `grid()` /
//! `store()`) stay on concrete types.

use bevy_math::bounding::Aabb3d;
use bevy_platform::collections::HashSet;

use super::index::SpatialIndex;

/// Regional ID query and destination-set accumulation.
///
/// Call through generics (static dispatch). Results of [`Self::query`] are
/// sorted by `Id`. [`Self::extend_intersecting`] does not sort; `out` is both
/// the uniqueness filter and the result.
pub trait SpatialIndexQuery<Id>
where
	Id: Copy + Eq + std::hash::Hash + Ord,
{
	/// Union intersecting ids into `out` without sorting or an extra hit `Vec`.
	fn extend_intersecting(&self, region: Aabb3d, out: &mut HashSet<Id>);

	/// All objects whose AABB intersects `region`, sorted by `Id`.
	fn query(&self, region: Aabb3d) -> Vec<Id> {
		let mut ids = HashSet::new();
		self.extend_intersecting(region, &mut ids);
		let mut result: Vec<_> = ids.into_iter().collect();
		result.sort();
		result
	}
}

impl<Id> SpatialIndexQuery<Id> for SpatialIndex<Id>
where
	Id: Copy + Eq + std::hash::Hash + Ord,
{
	fn extend_intersecting(&self, region: Aabb3d, out: &mut HashSet<Id>) {
		SpatialIndex::extend_intersecting(self, region, out);
	}

	fn query(&self, region: Aabb3d) -> Vec<Id> {
		SpatialIndex::query(self, region)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::bounding::IntersectsVolume;
	use bevy_math::{DVec3, Vec3};

	fn aabb(min: [f32; 3], max: [f32; 3]) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::from_array(min), Vec3::from_array(max))
	}

	fn populated() -> Result<SpatialIndex<u32>> {
		let mut idx = SpatialIndex::new(DVec3::ONE)?;
		idx.insert(3, aabb([1.0, 0.0, 0.0], [2.0, 1.0, 1.0]))?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(2, aabb([0.5, 0.0, 0.0], [1.5, 1.0, 1.0]))?;
		idx.insert(9, aabb([8.0, 0.0, 0.0], [9.0, 1.0, 1.0]))?;
		Ok(idx)
	}

	fn sorted(out: &HashSet<u32>) -> Vec<u32> {
		let mut got: Vec<_> = out.iter().copied().collect();
		got.sort();
		got
	}

	/// Linear scan of stored bounds. Second implementation of the query trait.
	struct LinearScan {
		entries: Vec<(u32, Aabb3d)>,
	}

	impl SpatialIndexQuery<u32> for LinearScan {
		fn extend_intersecting(&self, region: Aabb3d, out: &mut HashSet<u32>) {
			for &(id, bounds) in &self.entries {
				if bounds.intersects(&region) {
					out.insert(id);
				}
			}
		}
	}

	fn linear_from(idx: &SpatialIndex<u32>) -> LinearScan {
		LinearScan { entries: idx.iter_all().collect() }
	}

	#[test]
	fn trait_query_matches_inherent_order() -> Result<()> {
		let idx = populated()?;
		let region = aabb([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
		assert_eq!(SpatialIndexQuery::query(&idx, region), idx.query(region));
		assert_eq!(idx.query(region), vec![1, 2, 3]);
		Ok(())
	}

	#[test]
	fn trait_extend_matches_inherent_union() -> Result<()> {
		let idx = populated()?;
		let regions =
			[aabb([0.0, 0.0, 0.0], [1.2, 2.0, 2.0]), aabb([0.8, 0.0, 0.0], [2.0, 2.0, 2.0])];
		let mut inherent = HashSet::from_iter([99]);
		let mut via_trait = HashSet::from_iter([99]);
		for region in regions {
			idx.extend_intersecting(region, &mut inherent);
			SpatialIndexQuery::extend_intersecting(&idx, region, &mut via_trait);
		}
		assert_eq!(sorted(&via_trait), sorted(&inherent));
		assert_eq!(sorted(&inherent), vec![1, 2, 3, 99]);
		Ok(())
	}

	#[test]
	fn linear_scan_matches_spatial_index() -> Result<()> {
		let idx = populated()?;
		let scan = linear_from(&idx);
		let regions = [
			aabb([0.0, 0.0, 0.0], [1.2, 2.0, 2.0]),
			aabb([0.8, 0.0, 0.0], [2.0, 2.0, 2.0]),
			aabb([0.1, 0.1, 0.1], [0.2, 0.2, 0.2]),
		];

		for region in regions {
			assert_eq!(scan.query(region), idx.query(region));
		}

		let mut from_scan = HashSet::from_iter([77]);
		let mut from_idx = HashSet::from_iter([77]);
		for region in regions {
			scan.extend_intersecting(region, &mut from_scan);
			idx.extend_intersecting(region, &mut from_idx);
		}
		assert_eq!(sorted(&from_scan), sorted(&from_idx));
		Ok(())
	}

	#[test]
	fn default_query_matches_override() -> Result<()> {
		let idx = populated()?;
		let scan = linear_from(&idx);
		let region = aabb([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
		// `LinearScan` uses the trait default (extend then sort).
		assert_eq!(SpatialIndexQuery::query(&scan, region), SpatialIndex::query(&idx, region));
		Ok(())
	}
}
