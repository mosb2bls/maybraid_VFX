use bevy::prelude::*;

use super::BoneId;

/// Reusable local transforms indexed by [`BoneId`].
///
/// Allocate to the definition length once. Sampling copies rest into this buffer
/// and overwrites posed bones; it does not allocate when the length already matches.
#[derive(Clone, Debug, PartialEq)]
pub struct PoseBuffer {
	pub local: Vec<Transform>,
}

impl PoseBuffer {
	pub fn identity(len: usize) -> Self {
		Self { local: vec![Transform::IDENTITY; len] }
	}

	pub fn len(&self) -> usize {
		self.local.len()
	}

	pub fn is_empty(&self) -> bool {
		self.local.is_empty()
	}

	pub fn get(&self, bone: BoneId) -> Option<Transform> {
		self.local.get(bone.index()).copied()
	}

	pub fn rotation(&self, bone: BoneId) -> Quat {
		self.local.get(bone.index()).map(|t| t.rotation).unwrap_or(Quat::IDENTITY)
	}

	pub fn set_rotation(&mut self, bone: BoneId, rotation: Quat) {
		if let Some(transform) = self.local.get_mut(bone.index()) {
			transform.rotation = rotation;
		}
	}

	pub fn copy_from(&mut self, other: &Self) {
		if self.local.len() != other.local.len() {
			self.local.clone_from(&other.local);
			return;
		}
		self.local.copy_from_slice(&other.local);
	}

	/// Quaternion blend of two poses that share a definition.
	///
	/// Weight 0 copies `from`; weight 1 copies `to`. Translation, rotation, and
	/// scale all participate.
	pub fn blend_into(from: &Self, to: &Self, weight: f32, out: &mut Self) {
		let t = weight.clamp(0.0, 1.0);
		let n = from.local.len().min(to.local.len()).min(out.local.len());
		for index in 0..n {
			let a = from.local[index];
			let b = to.local[index];
			out.local[index] = Transform {
				translation: a.translation.lerp(b.translation, t),
				rotation: a.rotation.slerp(b.rotation, t),
				scale: a.scale.lerp(b.scale, t),
			};
		}
	}
}

/// Scratch pairs for Mix / Smooth / Transition.
///
/// Depth 0 uses [`Self::a`] / [`Self::b`]. Nested composites allocate extra
/// pairs so evaluating an inner child cannot overwrite an outer saved pose.
#[derive(Clone, Debug)]
pub struct PoseScratch {
	pub a: PoseBuffer,
	pub b: PoseBuffer,
	nested: Vec<(PoseBuffer, PoseBuffer)>,
	pub depth: usize,
}

impl PoseScratch {
	pub fn identity(len: usize) -> Self {
		Self {
			a: PoseBuffer::identity(len),
			b: PoseBuffer::identity(len),
			nested: Vec::new(),
			depth: 0,
		}
	}

	fn ensure_nested(&mut self, depth: usize) {
		if depth == 0 {
			return;
		}
		let len = self.a.len();
		while self.nested.len() < depth {
			self.nested.push((PoseBuffer::identity(len), PoseBuffer::identity(len)));
		}
	}

	pub fn capture_from(&mut self, depth: usize, pose: &PoseBuffer) {
		if depth == 0 {
			self.a.copy_from(pose);
			return;
		}
		self.ensure_nested(depth);
		self.nested[depth - 1].0.copy_from(pose);
	}

	pub fn capture_to(&mut self, depth: usize, pose: &PoseBuffer) {
		if depth == 0 {
			self.b.copy_from(pose);
			return;
		}
		self.ensure_nested(depth);
		self.nested[depth - 1].1.copy_from(pose);
	}

	pub fn blend_saved(&self, depth: usize, weight: f32, out: &mut PoseBuffer) {
		let (from, to) = if depth == 0 {
			(&self.a, &self.b)
		} else {
			let pair = &self.nested[depth - 1];
			(&pair.0, &pair.1)
		};
		PoseBuffer::blend_into(from, to, weight, out);
	}
}

/// Visual offset of the armature relative to its bind, in armature-parent space.
///
/// Absent motion is [`Self::IDENTITY`]. Translation is added to the bind,
/// rotation is pre-multiplied, and scale is multiplied. Gameplay locomotion
/// displacement is not this offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmatureOffset(pub Transform);

impl Default for ArmatureOffset {
	fn default() -> Self {
		Self::IDENTITY
	}
}

impl ArmatureOffset {
	pub const IDENTITY: Self = Self(Transform::IDENTITY);

	pub fn from_translation(translation: Vec3) -> Self {
		Self(Transform::from_translation(translation))
	}

	pub fn from_rotation(rotation: Quat) -> Self {
		Self(Transform::from_rotation(rotation))
	}

	pub fn is_identity(self) -> bool {
		self.0.translation == Vec3::ZERO
			&& self.0.rotation == Quat::IDENTITY
			&& self.0.scale == Vec3::ONE
	}

	pub fn blend(from: Self, to: Self, weight: f32) -> Self {
		let t = weight.clamp(0.0, 1.0);
		Self(Transform {
			translation: from.0.translation.lerp(to.0.translation, t),
			rotation: from.0.rotation.slerp(to.0.rotation, t),
			scale: from.0.scale.lerp(to.0.scale, t),
		})
	}

	pub fn apply_to_bind(self, bind: Transform) -> Transform {
		let mut armature = bind;
		armature.translation += self.0.translation;
		armature.rotation = self.0.rotation * armature.rotation;
		armature.scale *= self.0.scale;
		armature
	}
}

/// Curve shared by Smooth and mailbox transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendCurve {
	#[default]
	SmoothStep,
	Linear,
}

impl BlendCurve {
	pub fn sample(self, t: f32) -> f32 {
		let t = t.clamp(0.0, 1.0);
		match self {
			Self::Linear => t,
			Self::SmoothStep => t * t * (3.0 - 2.0 * t),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blend_endpoints_and_midpoint_cover_translation_rotation_and_scale() {
		let from = PoseBuffer::identity(1);
		let mut to = PoseBuffer::identity(1);
		to.local[0] = Transform {
			translation: Vec3::X,
			rotation: Quat::from_rotation_y(1.0),
			scale: Vec3::splat(3.0),
		};
		let mut out = PoseBuffer::identity(1);
		PoseBuffer::blend_into(&from, &to, 0.0, &mut out);
		assert_eq!(out.local[0], from.local[0]);
		PoseBuffer::blend_into(&from, &to, 1.0, &mut out);
		assert!((out.local[0].translation - Vec3::X).length() < 1e-5);
		assert!((out.local[0].scale - Vec3::splat(3.0)).length() < 1e-5);
		PoseBuffer::blend_into(&from, &to, 0.5, &mut out);
		assert!((out.local[0].translation - Vec3::X * 0.5).length() < 1e-5);
		assert!((out.local[0].scale - Vec3::splat(2.0)).length() < 1e-4);
		assert!(out.local[0].rotation.dot(Quat::IDENTITY).abs() < 0.999);
	}

	#[test]
	fn identity_offset_blend_fades_rotation() {
		let from = ArmatureOffset::IDENTITY;
		let to = ArmatureOffset::from_rotation(Quat::from_rotation_x(1.0));
		let mid = ArmatureOffset::blend(from, to, 0.5);
		assert!(mid.0.rotation.dot(Quat::IDENTITY).abs() < 0.999);
		assert!(
			(ArmatureOffset::blend(from, to, 0.0).0.rotation.dot(Quat::IDENTITY).abs() - 1.0).abs()
				< 1e-5
		);
		assert!(
			(ArmatureOffset::blend(from, to, 1.0).0.rotation.dot(to.0.rotation).abs() - 1.0).abs()
				< 1e-5
		);
	}

	#[test]
	fn copy_from_does_not_grow_capacity() {
		let mut dst = PoseBuffer::identity(4);
		let cap = dst.local.capacity();
		let src = PoseBuffer::identity(4);
		dst.copy_from(&src);
		assert_eq!(dst.local.capacity(), cap);
	}
}
