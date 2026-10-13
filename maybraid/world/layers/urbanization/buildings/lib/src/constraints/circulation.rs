//! Circulation requirements on boundary regions.

use bevy_math::{
	bounding::{Aabb2d, Aabb3d},
	Vec3,
};

use crate::constraints::face::{FaceKind, FACE_EPS};
use crate::constraints::ownership::BoundaryOwnershipEntry;
use crate::constraints::region::{BoundaryRegionList, BoundaryRegionListExt};
use crate::constraints::CellConstraints;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CirculationRequestStatus {
	/// Already fulfilled in the hierarchy (often implied by absence).
	#[default]
	Satisfied,
	Required,
	Desired,
	Optional,
}

/// Circulation requirements for regions on one boundary face.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CirculationEntry(pub BoundaryRegionList<Vec<CirculationRequestStatus>>);

impl CirculationEntry {
	pub fn clip_to_coverage(&self, coverage: Aabb2d) -> Self {
		Self(self.0.clip_to_coverage(coverage))
	}

	/// Drop regions that are not cell-owned after subsetting.
	pub fn filter_cell_owned(self, ownership: &BoundaryOwnershipEntry) -> Self {
		Self(
			self.0
				.into_iter()
				.filter(|(region, _)| ownership.owns_region_as_cell(*region))
				.collect(),
		)
	}

	pub fn into_option(self) -> Option<Self> {
		if self.0.is_empty() {
			None
		} else {
			Some(self)
		}
	}
}

impl CirculationRequestStatus {
	/// Statuses that still need clear approach space into the cell.
	pub fn needs_clearance(self) -> bool {
		matches!(self, Self::Required | Self::Desired | Self::Optional)
	}
}

impl CellConstraints {
	/// World-space volumes carved inward from outstanding circulation regions.
	///
	/// Each boundary-local region is mapped onto its face, then extruded into the
	/// cell by the region's **primary-axis world width** (door / opening width).
	/// Child allocation should avoid intersecting these boxes.
	pub fn circulation_exclusion_zones(&self) -> Vec<Aabb3d> {
		let mut zones = Vec::new();
		for face in FaceKind::ALL {
			let Some(entry) = self.circulation.get(face) else {
				continue;
			};
			for (region, statuses) in &entry.0 {
				if !statuses.iter().copied().any(CirculationRequestStatus::needs_clearance) {
					continue;
				}
				if let Some(zone) = exclusion_zone_for_region(&self.aabb, face, *region) {
					zones.push(zone);
				}
			}
		}
		zones
	}
}

/// Project a face-local circulation region inward by its along-face width.
fn exclusion_zone_for_region(aabb: &Aabb3d, face: FaceKind, region: Aabb2d) -> Option<Aabb3d> {
	let size = aabb.max - aabb.min;
	let t0 = region.min.x.clamp(0.0, 1.0);
	let t1 = region.max.x.clamp(0.0, 1.0);
	let h0 = region.min.y.clamp(0.0, 1.0);
	let h1 = region.max.y.clamp(0.0, 1.0);
	if t1 <= t0 + FACE_EPS || h1 <= h0 + FACE_EPS {
		return None;
	}

	let (min, max) = match face {
		FaceKind::Front => {
			let width = (t1 - t0) * size.x.max(FACE_EPS);
			let x0 = aabb.min.x + t0 * size.x;
			let x1 = aabb.min.x + t1 * size.x;
			let y0 = aabb.min.y + h0 * size.y;
			let y1 = aabb.min.y + h1 * size.y;
			let z0 = aabb.min.z;
			let z1 = aabb.min.z + width;
			(Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1.min(aabb.max.z)))
		}
		FaceKind::Back => {
			let width = (t1 - t0) * size.x.max(FACE_EPS);
			let x0 = aabb.min.x + t0 * size.x;
			let x1 = aabb.min.x + t1 * size.x;
			let y0 = aabb.min.y + h0 * size.y;
			let y1 = aabb.min.y + h1 * size.y;
			let z1 = aabb.max.z;
			let z0 = aabb.max.z - width;
			(Vec3::new(x0, y0, z0.max(aabb.min.z)), Vec3::new(x1, y1, z1))
		}
		FaceKind::Left => {
			let width = (t1 - t0) * size.z.max(FACE_EPS);
			let z0 = aabb.min.z + t0 * size.z;
			let z1 = aabb.min.z + t1 * size.z;
			let y0 = aabb.min.y + h0 * size.y;
			let y1 = aabb.min.y + h1 * size.y;
			let x0 = aabb.min.x;
			let x1 = aabb.min.x + width;
			(Vec3::new(x0, y0, z0), Vec3::new(x1.min(aabb.max.x), y1, z1))
		}
		FaceKind::Right => {
			let width = (t1 - t0) * size.z.max(FACE_EPS);
			let z0 = aabb.min.z + t0 * size.z;
			let z1 = aabb.min.z + t1 * size.z;
			let y0 = aabb.min.y + h0 * size.y;
			let y1 = aabb.min.y + h1 * size.y;
			let x1 = aabb.max.x;
			let x0 = aabb.max.x - width;
			(Vec3::new(x0.max(aabb.min.x), y0, z0), Vec3::new(x1, y1, z1))
		}
		FaceKind::Bottom => {
			let width = (t1 - t0) * size.x.max(FACE_EPS);
			let x0 = aabb.min.x + t0 * size.x;
			let x1 = aabb.min.x + t1 * size.x;
			let z0 = aabb.min.z + h0 * size.z;
			let z1 = aabb.min.z + h1 * size.z;
			let y0 = aabb.min.y;
			let y1 = aabb.min.y + width;
			(Vec3::new(x0, y0, z0), Vec3::new(x1, y1.min(aabb.max.y), z1))
		}
		FaceKind::Top => {
			let width = (t1 - t0) * size.x.max(FACE_EPS);
			let x0 = aabb.min.x + t0 * size.x;
			let x1 = aabb.min.x + t1 * size.x;
			let z0 = aabb.min.z + h0 * size.z;
			let z1 = aabb.min.z + h1 * size.z;
			let y1 = aabb.max.y;
			let y0 = aabb.max.y - width;
			(Vec3::new(x0, y0.max(aabb.min.y), z0), Vec3::new(x1, y1, z1))
		}
	};

	if min.x > max.x + FACE_EPS || min.y > max.y + FACE_EPS || min.z > max.z + FACE_EPS {
		None
	} else {
		Some(Aabb3d::from_min_max(min, max))
	}
}

/// True when `a` and `b` overlap with positive volume (epsilon-tolerant).
pub fn aabb3d_intersects(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x - FACE_EPS
		&& a.max.x > b.min.x + FACE_EPS
		&& a.min.y < b.max.y - FACE_EPS
		&& a.max.y > b.min.y + FACE_EPS
		&& a.min.z < b.max.z - FACE_EPS
		&& a.max.z > b.min.z + FACE_EPS
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::constraints::CellConstraints;
	use bevy_math::Vec2;

	#[test]
	fn front_door_projects_inward_by_opening_width() -> anyhow::Result<()> {
		let mut cell =
			CellConstraints::cell_owned(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0)));
		// Door spanning t ∈ [0.25, 0.5] → world width 1.0 on a 4m face.
		cell.circulation.front = Some(CirculationEntry(vec![(
			Aabb2d { min: Vec2::new(0.25, 0.0), max: Vec2::new(0.5, 0.9) },
			vec![CirculationRequestStatus::Required],
		)]));
		let zones = cell.circulation_exclusion_zones();
		assert_eq!(zones.len(), 1);
		let z = &zones[0];
		assert!((z.min.x - 1.0).abs() < 1e-3);
		assert!((z.max.x - 2.0).abs() < 1e-3);
		assert!((z.min.z - 0.0).abs() < 1e-3);
		assert!((z.max.z - 1.0).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn satisfied_only_regions_are_not_excluded() -> anyhow::Result<()> {
		let mut cell =
			CellConstraints::cell_owned(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0)));
		cell.circulation.front = Some(CirculationEntry(vec![(
			Aabb2d { min: Vec2::ZERO, max: Vec2::ONE },
			vec![CirculationRequestStatus::Satisfied],
		)]));
		assert!(cell.circulation_exclusion_zones().is_empty());
		Ok(())
	}
}
