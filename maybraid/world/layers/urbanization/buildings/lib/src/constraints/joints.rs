//! Joints on boundary faces.

use bevy_math::{bounding::Aabb2d, Vec3};

use crate::constraints::face::FACE_EPS;

/// Incoming boundary geometry sample before the joint point.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreJointSweep(pub Vec<Vec3>);

/// Joint coordinate in boundary-local \([0, 1]\) space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointCoordinate {
	/// Distance along the boundary segment.
	pub t: f32,
	/// Height up the boundary segment.
	pub h: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JointEntry(pub Vec<(JointCoordinate, PreJointSweep)>);

impl JointEntry {
	pub fn clip_to_coverage(&self, coverage: Aabb2d) -> Self {
		let size = coverage.max - coverage.min;
		let sx = size.x.max(FACE_EPS);
		let sy = size.y.max(FACE_EPS);
		Self(
			self.0
				.iter()
				.filter_map(|(coord, sweep)| {
					if coord.t < coverage.min.x - FACE_EPS
						|| coord.t > coverage.max.x + FACE_EPS
						|| coord.h < coverage.min.y - FACE_EPS
						|| coord.h > coverage.max.y + FACE_EPS
					{
						return None;
					}
					Some((
						JointCoordinate {
							t: ((coord.t - coverage.min.x) / sx).clamp(0.0, 1.0),
							h: ((coord.h - coverage.min.y) / sy).clamp(0.0, 1.0),
						},
						sweep.clone(),
					))
				})
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
