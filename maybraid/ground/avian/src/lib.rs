//! Avian-backed [`ElevationProbe`](ground::ElevationProbe).
//!
//! Walks a downward column through [`PhysicsInteractionLayer::Fixed`] colliders
//! (terrain trimeshes, buildings, tree stick compounds). LOD Host / Generate /
//! Present volumes and Animated capsules are ignored. Near-start hits are
//! skipped so a ray that begins inside a canopy or solid AABB does not count as
//! ground; among the rest, the **lowest** hit wins so a tree canopy cannot
//! steal the Durham trimesh.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::entity::Entity;
use bevy::ecs::system::SystemParam;
use bevy::math::{Dir3, Vec3};
use ground::{ElevationProbe, GroundHit};
use lod_avian::PhysicsInteractionLayer;

/// Matches `character_motion::PROBE_LIFT`: pitch rays start this far
/// above the body. Hits closer than this are canopy / solid-start volumes.
pub const MIN_GROUND_DROP: f32 = 2.0;

/// Grove tiles and overlapping High-band plants on one plumb line.
const MAX_COLUMN_HITS: usize = 8;

/// [`SystemParam`] Avian implementation of [`ElevationProbe`].
///
/// Register `apply_terrain_pitch::<AvianElevationProbe>` (or any system that
/// takes this param) after physics so hits match this frame’s colliders.
#[derive(SystemParam)]
pub struct AvianElevationProbe<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

/// The [`MAX_COLUMN_HITS`] nearest hits on one plumb line, sorted by distance.
///
/// One `ray_hits_callback` traversal fills this. It selects the same hit as
/// casting the closest ray, excluding that collider, and repeating
/// [`MAX_COLUMN_HITS`] times, without a traversal and an excluded-entity set
/// insert per step.
#[derive(Clone, Copy, Debug)]
struct ColumnHits {
	hits: [(f32, Entity, Vec3); MAX_COLUMN_HITS],
	len: usize,
}

impl Default for ColumnHits {
	fn default() -> Self {
		Self { hits: [(f32::INFINITY, Entity::PLACEHOLDER, Vec3::ZERO); MAX_COLUMN_HITS], len: 0 }
	}
}

impl ColumnHits {
	fn push(&mut self, distance: f32, entity: Entity, normal: Vec3) {
		if self.len == MAX_COLUMN_HITS {
			if distance >= self.hits[MAX_COLUMN_HITS - 1].0 {
				return;
			}
			self.len -= 1;
		}
		let at = self.hits[..self.len].partition_point(|&(d, _, _)| d <= distance);
		self.hits.copy_within(at..self.len, at + 1);
		self.hits[at] = (distance, entity, normal);
		self.len += 1;
	}

	/// Lowest kept hit at least `min_drop` below the origin.
	fn ground(&self, min_drop: f32) -> Option<(f32, Entity, Vec3)> {
		let last = *self.hits[..self.len].last()?;
		(last.0 >= min_drop).then_some(last)
	}
}

impl ElevationProbe for AvianElevationProbe<'_, '_> {
	fn hit_down(
		&mut self,
		origin: Vec3,
		max_distance: f32,
		exclude: &[Entity],
	) -> Option<GroundHit> {
		if max_distance <= 0.0 {
			return None;
		}
		let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
		let mut column = ColumnHits::default();
		self.spatial
			.ray_hits_callback(origin, Dir3::NEG_Y, max_distance, true, &filter, |hit| {
				if !exclude.contains(&hit.entity) {
					column.push(hit.distance, hit.entity, hit.normal);
				}
				true
			});
		column.ground(MIN_GROUND_DROP).map(|(distance, entity, normal)| GroundHit {
			point: origin + Vec3::NEG_Y * distance,
			normal,
			entity,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The former column walk: closest hit, exclude it, repeat
	/// [`MAX_COLUMN_HITS`] times; the last kept hit past `min_drop` wins.
	fn iterative_column_distance(distances: &[f32], min_drop: f32) -> Option<f32> {
		let mut left: Vec<f32> = distances.to_vec();
		let mut best = None;
		for _ in 0..MAX_COLUMN_HITS {
			let Some((index, distance)) =
				left.iter().copied().enumerate().min_by(|a, b| a.1.total_cmp(&b.1))
			else {
				break;
			};
			left.swap_remove(index);
			if distance >= min_drop {
				best = Some(distance);
			}
		}
		best
	}

	fn select_column_distance(distances: &[f32], min_drop: f32) -> Option<f32> {
		let mut column = ColumnHits::default();
		for (i, distance) in distances.iter().enumerate() {
			column.push(*distance, Entity::from_bits(i as u64 + 1), Vec3::Y);
		}
		column.ground(min_drop).map(|(distance, _, _)| distance)
	}

	#[test]
	fn single_traversal_matches_iterative_column_walk() {
		let mut state = 0x2545_f491_4f6c_dd1d_u64;
		let mut unit = move || {
			state ^= state << 13;
			state ^= state >> 7;
			state ^= state << 17;
			(state >> 40) as f32 / (1_u32 << 24) as f32
		};
		for case in 0..2_000 {
			let count = case % 14;
			let distances: Vec<f32> = (0..count).map(|_| unit() * 12.0).collect();
			assert_eq!(
				select_column_distance(&distances, MIN_GROUND_DROP),
				iterative_column_distance(&distances, MIN_GROUND_DROP),
				"{distances:?}"
			);
		}
	}

	#[test]
	fn column_keeps_only_the_nearest_hits() {
		let mut distances: Vec<f32> = (0..MAX_COLUMN_HITS).map(|i| 0.1 * i as f32).collect();
		distances.push(9.0);
		assert_eq!(select_column_distance(&distances, MIN_GROUND_DROP), None);
		assert_eq!(iterative_column_distance(&distances, MIN_GROUND_DROP), None);
	}

	#[test]
	fn column_keeps_the_lowest_hit_past_canopy() {
		assert_eq!(select_column_distance(&[0.4, 2.8, 9.1], MIN_GROUND_DROP), Some(9.1));
	}

	#[test]
	fn column_ignores_only_near_start_hits() {
		assert_eq!(select_column_distance(&[0.4, 1.9], MIN_GROUND_DROP), None);
		assert_eq!(select_column_distance(&[8.5], MIN_GROUND_DROP), Some(8.5));
	}
}
