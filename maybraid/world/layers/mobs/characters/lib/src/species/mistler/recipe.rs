//! LodScene recipe for Mistler.
//!
//! [`Mistler`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	pose::{MistlerPose, MISTLER_OVERALL_SCALE},
	MistlerColors, MistlerConfig,
};
use crate::{
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{nodes as humanoid, BODY_SPRITE_FISH},
};
use lod::scene::LodSceneLevel;

/// Semantic Mistler data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Mistler {
	pub colors: MistlerColors,
}

impl Mistler {
	pub fn from_config(config: &MistlerConfig) -> Self {
		Self { colors: config.colors.clone() }
	}
}

impl Default for Mistler {
	fn default() -> Self {
		Self::from_config(&MistlerConfig::default_preview())
	}
}

impl CharacterComponents for Mistler {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID.scaled(MISTLER_OVERALL_SCALE)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![humanoid::forelimbed_body_rig(MistlerPose.resolve())
			.with_normalization(AssetNormalization::centroid(MISTLER_OVERALL_SCALE))])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled(
			"body",
			vec![humanoid::body_part("sprite-fish", BODY_SPRITE_FISH.as_str())],
		)
		.map(|part| {
			let color = self.colors.color_for_slot(part.slot);
			part.with_base_color(color)
		})
	}
}
