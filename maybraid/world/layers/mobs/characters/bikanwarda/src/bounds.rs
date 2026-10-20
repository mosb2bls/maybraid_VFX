//! Visual bounds helpers (rest pose).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Vec3;
use characters::components::CharacterComponents;

use crate::recipe::BikanwardaActor;

/// Rest-pose visual bounds for LOD / camera framing (motor envelope).
pub fn visual_bounds_aabb(actor: &BikanwardaActor) -> Aabb3d {
	actor.preview_aabb()
}

/// Half-extents for quick camera framing tests.
pub fn visual_bounds_half_extents(aabb: Aabb3d) -> Vec3 {
	let min = Vec3::from(aabb.min);
	let max = Vec3::from(aabb.max);
	(max - min) * 0.5
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::measure::{encounter_standing_height_m, ENCOUNTER_STANDING_HEIGHT_M, STANDING_HEIGHT_TOLERANCE_M};
	use crate::recipe::BikanwardaActorRecipe;
	use characters::CharacterRecipe;

	#[test]
	fn encounter_bounds_height_matches_standing_measurement() {
		let actor = BikanwardaActorRecipe::default().components();
		let aabb = visual_bounds_aabb(&actor);
		let height = Vec3::from(aabb.max).y - Vec3::from(aabb.min).y;
		assert!((height - encounter_standing_height_m()).abs() < 1e-3);
		assert!((height - ENCOUNTER_STANDING_HEIGHT_M).abs() < STANDING_HEIGHT_TOLERANCE_M);
	}
}
