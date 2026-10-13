//! Per-AABB ridge / wall / eave candidates and overhang resolution.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use super::{EndCap, Overhang};

pub(super) const EPS: f32 = 1e-3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LongAxis {
	X,
	Z,
}

impl LongAxis {
	pub fn from_extents(extent_x: f32, extent_z: f32) -> Self {
		if extent_x >= extent_z {
			Self::X
		} else {
			Self::Z
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LineSeg {
	pub a: Vec3,
	pub b: Vec3,
}

impl LineSeg {
	pub fn new(a: Vec3, b: Vec3) -> Self {
		Self { a, b }
	}

	pub fn as_tuple(self) -> (Vec3, Vec3) {
		(self.a, self.b)
	}

	pub fn end(self, end: usize) -> Vec3 {
		if end == 0 {
			self.a
		} else {
			self.b
		}
	}

	pub fn set_end(&mut self, end: usize, p: Vec3) {
		if end == 0 {
			self.a = p;
		} else {
			self.b = p;
		}
	}
}

/// Unconstrained roof rails for one massing box (before junction truncation).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct VolumeCandidate {
	pub aabb: Aabb3d,
	pub long_axis: LongAxis,
	pub ridge: LineSeg,
	/// Short-axis sides: `[0]` = negative side, `[1]` = positive side.
	pub wall: [LineSeg; 2],
	pub eave: [LineSeg; 2],
	pub short_span: f32,
	pub side_overhang: f32,
	/// End `0` = long-axis min, end `1` = long-axis max.
	pub end_free: [bool; 2],
}

impl VolumeCandidate {
	pub fn from_aabb(aabb: Aabb3d, side_overhang: Overhang) -> Self {
		let min = Vec3::from(aabb.min);
		let max = Vec3::from(aabb.max);
		let extent_x = (max.x - min.x).max(EPS);
		let extent_z = (max.z - min.z).max(EPS);
		let long_axis = LongAxis::from_extents(extent_x, extent_z);
		let short_span = match long_axis {
			LongAxis::X => extent_z,
			LongAxis::Z => extent_x,
		};
		let oh = side_overhang.resolve(short_span);
		let y_ridge = max.y;
		let y_wall = min.y;

		let (ridge, wall, eave) = match long_axis {
			LongAxis::X => {
				let z_mid = 0.5 * (min.z + max.z);
				let ridge = LineSeg::new(
					Vec3::new(min.x, y_ridge, z_mid),
					Vec3::new(max.x, y_ridge, z_mid),
				);
				let wall = [
					LineSeg::new(Vec3::new(min.x, y_wall, min.z), Vec3::new(max.x, y_wall, min.z)),
					LineSeg::new(Vec3::new(min.x, y_wall, max.z), Vec3::new(max.x, y_wall, max.z)),
				];
				let eave = [
					LineSeg::new(
						Vec3::new(min.x, y_wall, min.z - oh),
						Vec3::new(max.x, y_wall, min.z - oh),
					),
					LineSeg::new(
						Vec3::new(min.x, y_wall, max.z + oh),
						Vec3::new(max.x, y_wall, max.z + oh),
					),
				];
				(ridge, wall, eave)
			}
			LongAxis::Z => {
				let x_mid = 0.5 * (min.x + max.x);
				let ridge = LineSeg::new(
					Vec3::new(x_mid, y_ridge, min.z),
					Vec3::new(x_mid, y_ridge, max.z),
				);
				let wall = [
					LineSeg::new(Vec3::new(min.x, y_wall, min.z), Vec3::new(min.x, y_wall, max.z)),
					LineSeg::new(Vec3::new(max.x, y_wall, min.z), Vec3::new(max.x, y_wall, max.z)),
				];
				let eave = [
					LineSeg::new(
						Vec3::new(min.x - oh, y_wall, min.z),
						Vec3::new(min.x - oh, y_wall, max.z),
					),
					LineSeg::new(
						Vec3::new(max.x + oh, y_wall, min.z),
						Vec3::new(max.x + oh, y_wall, max.z),
					),
				];
				(ridge, wall, eave)
			}
		};

		Self {
			aabb,
			long_axis,
			ridge,
			wall,
			eave,
			short_span,
			side_overhang: oh,
			end_free: [true, true],
		}
	}

	pub fn plan_min(&self) -> (f32, f32) {
		let min = Vec3::from(self.aabb.min);
		(min.x, min.z)
	}

	pub fn plan_max(&self) -> (f32, f32) {
		let max = Vec3::from(self.aabb.max);
		(max.x, max.z)
	}

	/// Apply free-end hip / gable insets. Junction ends are left alone.
	pub fn apply_end_caps(&mut self, end_cap: EndCap) {
		for end in 0..2 {
			if !self.end_free[end] {
				continue;
			}
			match end_cap {
				EndCap::Hip => {
					// Bank hips: shorten ridge by half the eave-to-eave span.
					let inset = (self.short_span * 0.5 + self.side_overhang).max(0.0);
					self.inset_ridge_end(end, inset);
				}
				EndCap::Gable { ridge, eave } => {
					// Wall plate stays at the massing end; ridge / eave project past it
					// so half-gable walling reads as a barge overhang.
					let ridge_oh = ridge.resolve(self.short_span);
					let eave_oh = eave.resolve(self.short_span);
					self.extend_ridge_end(end, ridge_oh);
					self.extend_eave_end(end, eave_oh);
				}
			}
		}
	}

	fn long_dir(&self) -> Vec3 {
		match self.long_axis {
			LongAxis::X => Vec3::X,
			LongAxis::Z => Vec3::Z,
		}
	}

	fn inset_ridge_end(&mut self, end: usize, inset: f32) {
		let dir = self.long_dir();
		let sign = if end == 0 { 1.0 } else { -1.0 };
		let p = self.ridge.end(end) + dir * (sign * inset);
		self.ridge.set_end(end, p);
	}

	fn extend_ridge_end(&mut self, end: usize, oh: f32) {
		let dir = self.long_dir();
		let sign = if end == 0 { -1.0 } else { 1.0 };
		let p = self.ridge.end(end) + dir * (sign * oh);
		self.ridge.set_end(end, p);
	}

	fn extend_eave_end(&mut self, end: usize, oh: f32) {
		let dir = self.long_dir();
		let sign = if end == 0 { -1.0 } else { 1.0 };
		let delta = dir * (sign * oh);
		for i in 0..2 {
			self.eave[i].set_end(end, self.eave[i].end(end) + delta);
		}
	}

	/// Pitch plane through ridge and one eave (three non-colinear samples).
	pub fn pitch_plane(&self, side: usize) -> Option<Plane> {
		let r0 = self.ridge.a;
		let r1 = self.ridge.b;
		let e0 = self.eave[side].a;
		Plane::from_points(r0, r1, e0)
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Plane {
	pub n: Vec3,
	pub d: f32,
}

impl Plane {
	pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Option<Self> {
		let n = (b - a).cross(c - a);
		if n.length_squared() < 1e-10 {
			return None;
		}
		let n = n.normalize();
		Some(Self { n, d: n.dot(a) })
	}

	pub fn intersect(self, other: Self) -> Option<(Vec3, Vec3)> {
		let dir = self.n.cross(other.n);
		if dir.length_squared() < 1e-10 {
			return None;
		}
		let dir = dir.normalize();
		// Solve n1·x = d1, n2·x = d2, prefer a point in the plane pair.
		let n1 = self.n;
		let n2 = other.n;
		let abs_x = dir.x.abs();
		let abs_y = dir.y.abs();
		let abs_z = dir.z.abs();
		let point = if abs_x >= abs_y && abs_x >= abs_z {
			// x free → set x=0
			let det = n1.y * n2.z - n1.z * n2.y;
			if det.abs() < 1e-10 {
				return None;
			}
			let y = (self.d * n2.z - other.d * n1.z) / det;
			let z = (n1.y * other.d - n2.y * self.d) / det;
			Vec3::new(0.0, y, z)
		} else if abs_y >= abs_z {
			let det = n1.x * n2.z - n1.z * n2.x;
			if det.abs() < 1e-10 {
				return None;
			}
			let x = (self.d * n2.z - other.d * n1.z) / det;
			let z = (n1.x * other.d - n2.x * self.d) / det;
			Vec3::new(x, 0.0, z)
		} else {
			let det = n1.x * n2.y - n1.y * n2.x;
			if det.abs() < 1e-10 {
				return None;
			}
			let x = (self.d * n2.y - other.d * n1.y) / det;
			let y = (n1.x * other.d - n2.x * self.d) / det;
			Vec3::new(x, y, 0.0)
		};
		Some((point, dir))
	}
}
