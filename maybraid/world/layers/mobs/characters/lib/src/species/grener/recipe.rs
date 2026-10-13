//! LodScene recipe for Grener.
//!
//! [`Grener`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	pose::{GrenerPose, GRENER_OVERALL_SCALE},
	GrenerColors, GrenerConfig,
};
use crate::{
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{nodes as humanoid, BODY_SHARK},
};
use lod::scene::LodSceneLevel;

/// Semantic Grener data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Grener {
	pub colors: GrenerColors,
}

impl Grener {
	pub fn from_config(config: &GrenerConfig) -> Self {
		Self { colors: config.colors.clone() }
	}
}

impl Default for Grener {
	fn default() -> Self {
		Self::from_config(&GrenerConfig::default_preview())
	}
}

impl CharacterComponents for Grener {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID.scaled(GRENER_OVERALL_SCALE)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![humanoid::forelimbed_body_rig(GrenerPose.resolve())
			.with_normalization(AssetNormalization::centroid(GRENER_OVERALL_SCALE))])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("body", vec![humanoid::body_part("shark", BODY_SHARK.as_str())]).map(
			|part| {
				let color = self.colors.color_for_slot(part.slot);
				part.with_base_color(color)
			},
		)
	}
}
