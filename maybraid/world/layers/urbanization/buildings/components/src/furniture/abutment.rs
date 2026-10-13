//! Wall side a furniture slot sits against, in building-local XZ.

use bevy::math::bounding::Aabb3d;
use std::f32::consts::{FRAC_PI_2, PI};

/// Host or partition face a packed slot flushes to.
///
/// Frame is building-local XZ (the packed AABB), the same space as
/// [`crate::placed::Placement`]. World pose is the `DevelopmentHost` transform
/// composed with that placement.
///
/// Kit facing is local **+Z** after the Blender \(Z\)-up remap (authoring \(+Y\)
/// for chair back / bed head). [`Self::facing_yaw`] points that axis **toward**
/// this wall so a headboard or chair back sits on the abutment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FurnitureAbutment {
	NegX,
	PosX,
	NegZ,
	PosZ,
}

impl FurnitureAbutment {
	/// Flush detection tolerance (metres), matched to bedroom wall-flush tests.
	pub const FLUSH_EPS: f32 = 0.08;

	/// The opposite host face.
	pub const fn opposite(self) -> Self {
		match self {
			Self::NegX => Self::PosX,
			Self::PosX => Self::NegX,
			Self::NegZ => Self::PosZ,
			Self::PosZ => Self::NegZ,
		}
	}

	/// Yaw about \(+Y\) that aims kit \(+Z\) at this wall.
	pub const fn facing_yaw(self) -> f32 {
		match self {
			Self::PosZ => 0.0,
			Self::PosX => FRAC_PI_2,
			Self::NegZ => PI,
			Self::NegX => -FRAC_PI_2,
		}
	}

	/// Kit-local scale so the unit cube fills `slot` **after** [`Self::facing_yaw`].
	///
	/// [`Placement::scale`] is applied before rotation. World AABB extents as
	/// scale plus a ±90° yaw swing the long run out of the packed box.
	pub fn local_scale(self, slot: &Aabb3d) -> bevy::math::Vec3 {
		local_scale_for_yaw(slot, self.facing_yaw())
	}

	/// Longest flush contact between `slot` and `host`, if any face is within
	/// [`Self::FLUSH_EPS`].
	pub fn from_flush(slot: &Aabb3d, host: &Aabb3d) -> Option<Self> {
		let mut best: Option<(f32, Self)> = None;
		let consider = |best: &mut Option<(f32, Self)>, hit: bool, contact: f32, side: Self| {
			if hit && contact > 1e-4 {
				if best.is_none_or(|(len, _)| contact > len) {
					*best = Some((contact, side));
				}
			}
		};
		let x_contact = (slot.max.z - slot.min.z).max(0.0);
		let z_contact = (slot.max.x - slot.min.x).max(0.0);
		consider(
			&mut best,
			(slot.min.x - host.min.x).abs() < Self::FLUSH_EPS,
			x_contact,
			Self::NegX,
		);
		consider(
			&mut best,
			(slot.max.x - host.max.x).abs() < Self::FLUSH_EPS,
			x_contact,
			Self::PosX,
		);
		consider(
			&mut best,
			(slot.min.z - host.min.z).abs() < Self::FLUSH_EPS,
			z_contact,
			Self::NegZ,
		);
		consider(
			&mut best,
			(slot.max.z - host.max.z).abs() < Self::FLUSH_EPS,
			z_contact,
			Self::PosZ,
		);
		best.map(|(_, side)| side)
	}

	/// Free-slot facing: kit \(+Z\) toward the nearer host wall on the **shorter**
	/// plan axis (head / back on a short side, biased to the room periphery).
	pub fn free_facing_yaw(slot: &Aabb3d, host: &Aabb3d) -> f32 {
		let center = (slot.min + slot.max) * 0.5;
		let span_x = slot.max.x - slot.min.x;
		let span_z = slot.max.z - slot.min.z;
		if span_x + 1e-4 >= span_z {
			let to_neg = (center.z - host.min.z).abs();
			let to_pos = (host.max.z - center.z).abs();
			if to_neg <= to_pos {
				Self::NegZ.facing_yaw()
			} else {
				Self::PosZ.facing_yaw()
			}
		} else {
			let to_neg = (center.x - host.min.x).abs();
			let to_pos = (host.max.x - center.x).abs();
			if to_neg <= to_pos {
				Self::NegX.facing_yaw()
			} else {
				Self::PosX.facing_yaw()
			}
		}
	}
}

/// Kit-local `(along, height, depth)` for a yaw that aims \(+Z\) at a wall.
pub fn local_scale_for_yaw(slot: &Aabb3d, yaw: f32) -> bevy::math::Vec3 {
	let span = slot.max - slot.min;
	if yaw.cos().abs() < 0.5 {
		bevy::math::Vec3::new(span.z, span.y, span.x)
	} else {
		bevy::math::Vec3::new(span.x, span.y, span.z)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::Vec3;

	fn box_at(min: Vec3, max: Vec3) -> Aabb3d {
		Aabb3d::from_min_max(min, max)
	}

	#[test]
	fn flush_prefers_the_longer_contact_on_a_corner() -> anyhow::Result<()> {
		let host = box_at(Vec3::ZERO, Vec3::new(8.0, 3.0, 6.0));
		let bed = box_at(Vec3::ZERO, Vec3::new(2.0, 0.55, 1.6));
		let side = FurnitureAbutment::from_flush(&bed, &host)
			.ok_or_else(|| anyhow::anyhow!("expected a flush wall"))?;
		assert_eq!(side, FurnitureAbutment::NegZ);
		assert!((side.facing_yaw() - PI).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn free_facing_uses_the_shorter_axis_toward_the_nearer_wall() {
		let host = box_at(Vec3::ZERO, Vec3::new(10.0, 3.0, 8.0));
		let bed = box_at(Vec3::new(3.0, 0.0, 0.5), Vec3::new(5.0, 0.55, 2.1));
		assert!((FurnitureAbutment::free_facing_yaw(&bed, &host) - PI).abs() < 1e-5);
	}

	#[test]
	fn east_wall_local_scale_swaps_xz() {
		let slot = box_at(Vec3::new(5.2, 0.0, 1.0), Vec3::new(6.0, 1.0, 5.0));
		let scale = FurnitureAbutment::PosX.local_scale(&slot);
		assert!((scale.x - 4.0).abs() < 1e-4);
		assert!((scale.y - 1.0).abs() < 1e-4);
		assert!((scale.z - 0.8).abs() < 1e-4);
	}
}
