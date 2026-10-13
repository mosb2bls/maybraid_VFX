//! Shared humanoid asset catalog used by multiple species.

pub mod assets;
pub mod nodes;

use crate::presets::BuildPreset;

/// Rest-pose quadruped support height relative to [`crate::LocomotionCapsule::QUADRUPED`].
///
/// Length layers multiply: species baseline × max(front, hind) slider × lanky
/// (or `1.0` when the build is not Lanky).
pub fn quadruped_rest_limb_scale(
	species_limb: f32,
	arm_length: f32,
	leg_length: f32,
	lanky_scale: f32,
	build: BuildPreset,
) -> f32 {
	let lanky = if build == BuildPreset::Lanky { lanky_scale } else { 1.0 };
	species_limb * arm_length.max(leg_length) * lanky
}

pub use assets::{
	BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh, BODY_DRAGLOON, BODY_FULL,
	BODY_GUMBUS, BODY_RIG, BODY_RUMBLER, BODY_SHARK, BODY_SPRITE_FISH, BODY_STANDARD, BODY_WHALE,
	EAR_FLANK, EAR_ROUND, EAR_STANDARD, EYE_FALCON, EYE_STANDARD, FORELIMBED_RIG,
	HAIR_THICK_BRAIDS, HEAD_CANINE, HEAD_CAOLE, HEAD_COWDER, HEAD_FULL, HEAD_GAUNT,
	HEAD_ORTHO_BEAR, HEAD_RIG, HEAD_STANDARD, HORNS_HARROWED_CROWN, HORNS_LORKEN_CROWN,
	MOUTH_CANINE_SNOUT, MOUTH_COW_SNOUT, MOUTH_LERODON_SNOUT, MOUTH_ROBREK_SNOUT, MOUTH_STANDARD,
	NECK_BASIC, NECK_TRIPLE_JOIN, NOSE_BALLOON, NOSE_BROAD, NOSE_LOAF, NOSE_STANDARD,
	PRONOGRADE_HEAD_RIG, QUADRUPED_RIG, TAIL_CAT, TAIL_LERODON, TAIL_LERODON_QUADRUPED,
};

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rest_limb_scale_multiplies_species_slider_and_lanky() {
		assert!(
			(quadruped_rest_limb_scale(1.35, 1.0, 1.0, 1.05, BuildPreset::Average) - 1.35).abs()
				< 1e-5
		);
		assert!(
			(quadruped_rest_limb_scale(1.35, 1.0, 1.2, 1.05, BuildPreset::Average) - 1.35 * 1.2)
				.abs() < 1e-5
		);
		assert!(
			(quadruped_rest_limb_scale(1.35, 1.1, 1.0, 1.05, BuildPreset::Lanky)
				- 1.35 * 1.1 * 1.05)
				.abs() < 1e-5
		);
	}
}
