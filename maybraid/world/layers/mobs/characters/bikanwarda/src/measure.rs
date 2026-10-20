//! Documented standing-height measurement for Bikanwarda.

use bevy::prelude::Vec3;
use characters::{
	components::{CharacterComponents, LocomotionCapsule},
	species::epiphant::recipe::Epiphant,
};

/// Target boss standing height at encounter scale (accessories excluded).
pub const ENCOUNTER_STANDING_HEIGHT_M: f32 = 20.0;

/// Acceptance tolerance on [`ENCOUNTER_STANDING_HEIGHT_M`].
pub const STANDING_HEIGHT_TOLERANCE_M: f32 = 0.35;

/// Standing height from the rest locomotion motor + pronograde hit hull (Y extent only).
///
/// Accessories (eyes, ears, nose meshes) are excluded by design — see issue #1134 open
/// decisions in the PR body.
pub fn standing_height_m(capsule: LocomotionCapsule) -> f32 {
	let aabb = capsule.preview_aabb();
	(Vec3::from(aabb.max).y - Vec3::from(aabb.min).y).max(0.0)
}

/// Ordinary-scale preview hull for the default Bikanwarda Epiphant recipe (scale 1).
pub fn ordinary_standing_height_m() -> f32 {
	standing_height_m(Epiphant::default().locomotion_capsule())
}

/// Uniform scale that maps the ordinary preview hull to [`ENCOUNTER_STANDING_HEIGHT_M`].
pub fn encounter_uniform_scale() -> f32 {
	let ordinary = ordinary_standing_height_m().max(f32::EPSILON);
	ENCOUNTER_STANDING_HEIGHT_M / ordinary
}

/// Encounter standing height after applying [`encounter_uniform_scale`] to the default recipe.
pub fn encounter_standing_height_m() -> f32 {
	standing_height_m(
		Epiphant::default()
			.locomotion_capsule()
			.scaled(encounter_uniform_scale()),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use characters::CharacterComponents;

	#[test]
	fn ordinary_preview_is_near_two_meters() {
		let h = ordinary_standing_height_m();
		assert!(h > 1.75 && h < 2.15, "ordinary standing height {h} m");
	}

	#[test]
	fn encounter_scale_hits_twenty_meter_target() {
		let h = encounter_standing_height_m();
		assert!(
			(h - ENCOUNTER_STANDING_HEIGHT_M).abs() < STANDING_HEIGHT_TOLERANCE_M,
			"encounter height {h} m, target {ENCOUNTER_STANDING_HEIGHT_M} m"
		);
	}

	#[test]
	fn scale_factor_matches_height_ratio() {
		let factor = encounter_uniform_scale();
		assert!(factor > 9.5 && factor < 11.0, "unexpected encounter factor {factor}");
		let epiphant = Epiphant::default();
		let scaled = epiphant.locomotion_capsule().scaled(factor);
		let h = standing_height_m(scaled);
		assert!((h - ENCOUNTER_STANDING_HEIGHT_M).abs() < STANDING_HEIGHT_TOLERANCE_M);
	}
}
