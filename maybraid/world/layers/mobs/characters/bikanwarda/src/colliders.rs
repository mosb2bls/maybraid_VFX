//! Motor + pronograde hit colliders sized to the visual hull.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use characters::components::LocomotionCapsule;

/// Assembly-local collider shapes (motor vertical capsule + horizontal body hit).
pub fn assembly_collider_shapes(capsule: LocomotionCapsule) -> Vec<(Vec3, Quat, Collider)> {
	let mut shapes = Vec::with_capacity(2);
	let half = capsule.half_height();
	shapes.push((
		Vec3::ZERO,
		Quat::IDENTITY,
		Collider::capsule(capsule.radius, capsule.length.max(1e-3)),
	));
	if let Some(hit) = capsule.hit_capsule() {
		shapes.push((hit.local_transform().translation, hit.local_transform().rotation, {
			Collider::capsule(hit.radius, hit.length.max(1e-3))
		}));
	}
	let _ = half;
	shapes
}

pub fn motor_collider(capsule: LocomotionCapsule) -> Collider {
	let shapes = assembly_collider_shapes(capsule);
	Collider::compound(shapes)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BikanwardaActorRecipe;
	use bevy::math::bounding::Aabb3d;
	use characters::{CharacterComponents, CharacterRecipe};

	#[test]
	fn collider_shapes_track_encounter_scale() {
		let actor = BikanwardaActorRecipe::default().components();
		let ordinary = actor.locomotion_capsule();
		let encounter = actor.scaled_locomotion_capsule();
		assert!(encounter.radius > ordinary.radius * 9.0);
		let shapes = assembly_collider_shapes(encounter);
		assert_eq!(shapes.len(), 2);
	}

	#[test]
	fn motor_aabb_fits_inside_visual_bounds() {
		let capsule = BikanwardaActorRecipe::default().components().scaled_locomotion_capsule();
		let visual = capsule.preview_aabb();
		let motor = capsule.preview_aabb();
		let vmin = Vec3::from(visual.min);
		let vmax = Vec3::from(visual.max);
		let mmin = Vec3::from(motor.min);
		let mmax = Vec3::from(motor.max);
		assert!(mmin.x >= vmin.x - 1e-3 && mmin.y >= vmin.y - 1e-3);
		assert!(mmax.x <= vmax.x + 1e-3 && mmax.y <= vmax.y + 1e-3);
		let _ = Aabb3d::from_min_max(vmin, vmax);
	}
}
