//! Geom-free placement for vegetation kits.

use bevy_math::{EulerRot, Quat, Vec3};

/// Translation / yaw / pitch / roll / scale for a kit piece in tree-local space.
///
/// Stick kits are authored vertically: \(Y \in [0, 1]\), \(X = Z \in [-0.2, 0.2]\).
/// Placement maps that kit into tree space (base at segment start, \(+Y\) along the
/// segment). Rotation order is intrinsic [`EulerRot::YXZ`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
	pub translation: Vec3,
	pub yaw: f32,
	pub pitch: f32,
	pub roll: f32,
	pub scale: Vec3,
}

impl Placement {
	pub const IDENTITY: Self =
		Self { translation: Vec3::ZERO, yaw: 0.0, pitch: 0.0, roll: 0.0, scale: Vec3::ONE };

	pub fn new(translation: Vec3, yaw: f32) -> Self {
		Self { translation, yaw, pitch: 0.0, roll: 0.0, scale: Vec3::ONE }
	}

	pub fn at_origin() -> Self {
		Self::IDENTITY
	}

	pub fn with_pitch(mut self, pitch: f32) -> Self {
		self.pitch = pitch;
		self
	}

	pub fn with_roll(mut self, roll: f32) -> Self {
		self.roll = roll;
		self
	}

	pub fn with_scale(mut self, scale: Vec3) -> Self {
		self.scale = scale;
		self
	}

	pub fn with_rotation(mut self, rotation: Quat) -> Self {
		let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
		self.yaw = yaw;
		self.pitch = pitch;
		self.roll = roll;
		self
	}

	pub fn rotation(self) -> Quat {
		Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, self.roll)
	}

	/// Compose a child placement into this parent's space (translation scaled, rotations add).
	pub fn compose_child(self, child: Placement) -> Placement {
		Placement {
			translation: self.translation + self.rotation() * (child.translation * self.scale),
			yaw: self.yaw + child.yaw,
			pitch: self.pitch + child.pitch,
			roll: self.roll + child.roll,
			scale: self.scale * child.scale,
		}
	}

	/// Stick segment: base at `start`, \(+Y\) along `dir`, length / girth from kit space.
	///
	/// Kit half-extent on \(X/Z\) is [`crate::sticks::STICK_KIT_HALF`]; world radius `radius`
	/// maps to scale \(=\texttt{radius} / \texttt{STICK\_KIT\_HALF}\).
	pub fn stick_segment(start: Vec3, dir: Vec3, length: f32, radius: f32) -> Option<Self> {
		let len_sq = dir.length_squared();
		if len_sq < 1e-12 || length < 1e-6 {
			return None;
		}
		let d = dir / len_sq.sqrt();
		let rotation = Quat::from_rotation_arc(Vec3::Y, d);
		let girth = (radius / crate::sticks::STICK_KIT_HALF).max(1e-4);
		Some(
			Self::new(start, 0.0)
				.with_rotation(rotation)
				.with_scale(Vec3::new(girth, length, girth)),
		)
	}

	/// Uniform foliage ball / splay at `center` with world radius `radius`.
	///
	/// Orientation is a spatial hash of `center`: independent yaw / pitch / roll in
	/// \([0, 2\pi)\) so cheap-ball cards do not share a grove-wide frame. Nearby
	/// positions stay uncorrelated (white, not band-limited noise).
	pub fn foliage_uniform(center: Vec3, radius: f32) -> Self {
		let (yaw, pitch, roll) = hashed_ball_euler(center);
		Self { translation: center, yaw, pitch, roll, scale: Vec3::splat(radius.max(1e-4)) }
	}

	/// Straight frond segment: base at `start`, \(+Y\) along `dir`, blade width along kit \(X\).
	///
	/// Authored kit: \(Y \in [0, 1]\), \(X \in [-\texttt{FROND\_KIT\_HALF\_X}, \texttt{FROND\_KIT\_HALF\_X}]\),
	/// \(Z\) negligible. World full width `width` maps to
	/// \(X\)-scale \(=\texttt{width} / (2 \cdot \texttt{FROND\_KIT\_HALF\_X})\); \(Z\)-scale matches
	/// \(X\) so the already-flat mesh stays thin.
	pub fn frond_segment(start: Vec3, dir: Vec3, length: f32, width: f32) -> Option<Self> {
		let len_sq = dir.length_squared();
		if len_sq < 1e-12 || length < 1e-6 {
			return None;
		}
		let d = dir / len_sq.sqrt();
		let rotation = Quat::from_rotation_arc(Vec3::Y, d);
		let scale_x = (width * 0.5 / crate::FROND_KIT_HALF_X).max(1e-4);
		Some(
			Self::new(start, 0.0)
				.with_rotation(rotation)
				.with_scale(Vec3::new(scale_x, length, scale_x)),
		)
	}
}

impl Default for Placement {
	fn default() -> Self {
		Self::IDENTITY
	}
}

/// White-noise Euler from position bits. Lanes keep yaw / pitch / roll uncorrelated.
fn hashed_ball_euler(p: Vec3) -> (f32, f32, f32) {
	const TAU: f32 = std::f32::consts::TAU;
	(unit_hash(p, 1) * TAU, unit_hash(p, 2) * TAU, unit_hash(p, 3) * TAU)
}

/// Deterministic sample in `[0, 1)` from `p`'s IEEE bits and a decorrelation lane.
///
/// Integer mixing (lowbias32) so nearby positions and large-magnitude float seeds
/// stay distinct — the same class of failure as authored jitter in `chico-sbs-geometry`.
fn unit_hash(p: Vec3, lane: u32) -> f32 {
	let mut h = p.x.to_bits()
		^ p.y.to_bits().rotate_left(11)
		^ p.z.to_bits().rotate_left(22)
		^ lane.wrapping_mul(0x9E37_79B9);
	h ^= h >> 16;
	h = h.wrapping_mul(0x7FEB_352D);
	h ^= h >> 15;
	h = h.wrapping_mul(0x846C_A68B);
	h ^= h >> 16;
	(h >> 8) as f32 / (1 << 24) as f32
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn foliage_uniform_is_deterministic() {
		let a = Placement::foliage_uniform(Vec3::new(1.25, 4.0, -2.5), 0.8);
		let b = Placement::foliage_uniform(Vec3::new(1.25, 4.0, -2.5), 0.8);
		assert_eq!(a, b);
		assert_eq!(a.translation, Vec3::new(1.25, 4.0, -2.5));
		assert!((a.scale - Vec3::splat(0.8)).abs().max_element() < 1e-6);
	}

	#[test]
	fn foliage_uniform_axes_are_uncorrelated() {
		let p = Placement::foliage_uniform(Vec3::new(3.0, 1.0, 7.0), 1.0);
		assert!((p.yaw - p.pitch).abs() > 1e-3, "yaw collided with pitch");
		assert!((p.yaw - p.roll).abs() > 1e-3, "yaw collided with roll");
		assert!((p.pitch - p.roll).abs() > 1e-3, "pitch collided with roll");
		for angle in [p.yaw, p.pitch, p.roll] {
			assert!((0.0..std::f32::consts::TAU).contains(&angle), "out of range: {angle}");
		}
	}

	#[test]
	fn nearby_centers_do_not_share_a_frame() {
		let a = Placement::foliage_uniform(Vec3::new(0.0, 2.0, 0.0), 1.0);
		let b = Placement::foliage_uniform(Vec3::new(0.05, 2.0, 0.0), 1.0);
		assert!((a.yaw - b.yaw).abs() > 1e-3, "yaw correlated");
		assert!((a.pitch - b.pitch).abs() > 1e-3, "pitch correlated");
		assert!((a.roll - b.roll).abs() > 1e-3, "roll correlated");
	}
}
