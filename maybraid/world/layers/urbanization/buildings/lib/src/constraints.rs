//! Cell constraint representation for Richmond building generation.
//!
//! Cells are rectangular prisms describing authoring bounds. Authored geometry
//! need not be strictly rectangular.

pub mod circulation;
pub mod face;
pub mod joints;
pub mod ownership;
pub mod region;
pub mod thickness;

pub use circulation::{CirculationEntry, CirculationRequestStatus};
pub use face::FaceKind;
pub use joints::{JointCoordinate, JointEntry, PreJointSweep};
pub use ownership::{BoundaryOwnershipEntry, BoundaryOwnershipStatus};
pub use region::BoundaryRegionList;
pub use thickness::BoundaryThicknessEntry;

use bevy_math::bounding::Aabb3d;
use face::FaceKind as Face;

/// Failure modes for [`CellConstraints::subset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsetError {
	/// Child AABB is not contained in the parent cell AABB.
	NotContained,
}

/// The complete boundary table for the cell.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellBoundaryTable<T> {
	/// Bevy +Y
	pub top: Option<T>,
	/// Bevy -Y
	pub bottom: Option<T>,
	/// Bevy -X (double-check this)
	pub left: Option<T>,
	/// Bevy +X
	pub right: Option<T>,
	/// Bevy -Z (double-check this)
	pub front: Option<T>,
	/// Bevy +Z
	pub back: Option<T>,
}

impl<T> CellBoundaryTable<T> {
	pub fn get(&self, face: FaceKind) -> Option<&T> {
		match face {
			FaceKind::Top => self.top.as_ref(),
			FaceKind::Bottom => self.bottom.as_ref(),
			FaceKind::Left => self.left.as_ref(),
			FaceKind::Right => self.right.as_ref(),
			FaceKind::Front => self.front.as_ref(),
			FaceKind::Back => self.back.as_ref(),
		}
	}

	pub fn set(&mut self, face: FaceKind, value: Option<T>) {
		match face {
			FaceKind::Top => self.top = value,
			FaceKind::Bottom => self.bottom = value,
			FaceKind::Left => self.left = value,
			FaceKind::Right => self.right = value,
			FaceKind::Front => self.front = value,
			FaceKind::Back => self.back = value,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellConstraints {
	/// Within these bounds, the cell has write authority.
	pub aabb: Aabb3d,
	pub boundary_thickness: CellBoundaryTable<BoundaryThicknessEntry>,
	pub boundary_ownership: CellBoundaryTable<BoundaryOwnershipEntry>,
	pub circulation: CellBoundaryTable<CirculationEntry>,
	pub joints: CellBoundaryTable<JointEntry>,
}

impl CellConstraints {
	/// Minimal cell-owned constraints for `aabb`.
	pub fn cell_owned(aabb: Aabb3d) -> Self {
		Self {
			aabb,
			boundary_thickness: CellBoundaryTable {
				top: Some(BoundaryThicknessEntry::default()),
				bottom: Some(BoundaryThicknessEntry::default()),
				left: Some(BoundaryThicknessEntry::default()),
				right: Some(BoundaryThicknessEntry::default()),
				front: Some(BoundaryThicknessEntry::default()),
				back: Some(BoundaryThicknessEntry::default()),
			},
			boundary_ownership: CellBoundaryTable {
				top: Some(BoundaryOwnershipEntry::default()),
				bottom: Some(BoundaryOwnershipEntry::default()),
				left: Some(BoundaryOwnershipEntry::default()),
				right: Some(BoundaryOwnershipEntry::default()),
				front: Some(BoundaryOwnershipEntry::default()),
				back: Some(BoundaryOwnershipEntry::default()),
			},
			circulation: CellBoundaryTable::default(),
			joints: CellBoundaryTable::default(),
		}
	}

	/// Subset this cell into a child AABB.
	///
	/// - Child AABB must be contained in `self.aabb` (epsilon-tolerant).
	/// - Coincident faces inherit and clip boundary-local tables into the child's face space.
	/// - Interior faces become cell-owned with default thickness and empty circulation/joints.
	/// - Circulation on non-cell-owned regions is dropped.
	pub fn subset(&self, child_aabb: Aabb3d) -> Result<CellConstraints, SubsetError> {
		if !Face::contains_aabb(&self.aabb, &child_aabb) {
			return Err(SubsetError::NotContained);
		}

		let mut ownership = CellBoundaryTable::default();
		let mut thickness = CellBoundaryTable::default();
		let mut circulation = CellBoundaryTable::default();
		let mut joints = CellBoundaryTable::default();

		for face in FaceKind::ALL {
			if !face.is_coincident(&self.aabb, &child_aabb) {
				ownership
					.set(face, Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Cell)));
				thickness.set(face, Some(BoundaryThicknessEntry::default()));
				circulation.set(face, None);
				joints.set(face, None);
				continue;
			}

			let coverage = face.child_coverage_in_parent_local(&self.aabb, &child_aabb);
			let owned = self
				.boundary_ownership
				.get(face)
				.map(|entry| entry.clip_to_coverage(coverage))
				.unwrap_or_default();
			let thick = self
				.boundary_thickness
				.get(face)
				.map(|entry| entry.clip_to_coverage(coverage))
				.unwrap_or_default();
			let circ = self
				.circulation
				.get(face)
				.map(|entry| entry.clip_to_coverage(coverage).filter_cell_owned(&owned))
				.unwrap_or_default();
			let joint = self
				.joints
				.get(face)
				.map(|entry| entry.clip_to_coverage(coverage))
				.unwrap_or_default();

			ownership.set(face, Some(owned));
			thickness.set(face, Some(thick));
			circulation.set(face, circ.into_option());
			joints.set(face, joint.into_option());
		}

		Ok(CellConstraints {
			aabb: child_aabb,
			boundary_thickness: thickness,
			boundary_ownership: ownership,
			circulation,
			joints,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::{bounding::Aabb2d, Vec2, Vec3};

	#[test]
	fn subset_rejects_uncontained_child() -> anyhow::Result<()> {
		let parent = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(10.0, 10.0, 10.0),
		));
		let child = Aabb3d::from_min_max(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0));
		assert_eq!(parent.subset(child).err(), Some(SubsetError::NotContained));
		Ok(())
	}

	#[test]
	fn subset_interior_faces_are_cell_owned() -> anyhow::Result<()> {
		let parent = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(10.0, 10.0, 10.0),
		));
		let child = Aabb3d::from_min_max(Vec3::new(2.0, 2.0, 2.0), Vec3::new(4.0, 4.0, 4.0));
		let sub = parent.subset(child).map_err(|e| anyhow::anyhow!("subset failed: {e:?}"))?;
		assert_eq!(
			sub.boundary_ownership.left,
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Cell))
		);
		assert!(sub.circulation.left.is_none());
		Ok(())
	}

	#[test]
	fn subset_shared_bottom_inherits_and_filters_circulation() -> anyhow::Result<()> {
		let mut parent = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(10.0, 10.0, 10.0),
		));
		parent.boundary_ownership.bottom =
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Parent));
		parent.circulation.bottom = Some(CirculationEntry(vec![(
			Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(1.0, 1.0) },
			vec![CirculationRequestStatus::Required],
		)]));

		let child = Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0));
		let sub = parent.subset(child).map_err(|e| anyhow::anyhow!("subset failed: {e:?}"))?;
		assert_eq!(
			sub.boundary_ownership.bottom,
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Parent))
		);
		assert!(sub.circulation.bottom.is_none());
		Ok(())
	}
}
