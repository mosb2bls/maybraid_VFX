use bevy::prelude::*;

/// Armature-local right.
pub const CHARACTER_RIGHT: Vec3 = Vec3::X;
/// Armature-local up.
pub const CHARACTER_UP: Vec3 = Vec3::Y;
/// Armature-local fight-forward. Jab targets use this axis.
///
/// Bevy cameras look along world −Z. That conversion is the character
/// [`GlobalTransform`], not a second hardcoded flip inside the solver.
pub const CHARACTER_FORWARD: Vec3 = Vec3::Z;

/// A point in armature-local space, including translation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterPoint(pub Vec3);

/// A point in Bevy world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPoint(pub Vec3);

impl CharacterPoint {
	/// Invert `character_world` and transform the point.
	///
	/// Non-finite results (zero scale, a singular transform) return `None`.
	/// Scale is included in the affine inverse; do not pass a direction here.
	pub fn from_world(target: WorldPoint, character_world: &GlobalTransform) -> Option<Self> {
		let inverse = character_world.affine().inverse();
		let point = inverse.transform_point3(target.0);
		if !point.is_finite() {
			return None;
		}
		Some(Self(point))
	}
}

/// Linear part of the character inverse. Directions stay directions.
pub fn character_direction_from_world(
	direction: Vec3,
	character_world: &GlobalTransform,
) -> Option<Vec3> {
	let inverse = character_world.affine().inverse();
	let direction = inverse.transform_vector3(direction);
	if !direction.is_finite() {
		return None;
	}
	Some(direction)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_point_follows_character_yaw() {
		let character = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(
			std::f32::consts::FRAC_PI_2,
		)));
		let local = CharacterPoint::from_world(WorldPoint(Vec3::X), &character).expect("invert");
		assert!(
			local.0.dot(Vec3::Z) > 0.9,
			"world +X in front of a +90° yaw character is local +Z, got {:?}",
			local.0
		);
	}

	#[test]
	fn direction_conversion_ignores_translation() {
		let character = GlobalTransform::from(Transform::from_translation(Vec3::splat(4.0)));
		let direction = character_direction_from_world(Vec3::Z, &character).expect("direction");
		assert!((direction - Vec3::Z).length() < 1e-5, "{direction:?}");
	}

	#[test]
	fn zero_scale_is_rejected() {
		let character = GlobalTransform::from(Transform::from_scale(Vec3::ZERO));
		assert!(CharacterPoint::from_world(WorldPoint(Vec3::X), &character).is_none());
	}
}
