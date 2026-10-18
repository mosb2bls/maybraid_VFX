//! LodScene recipe for Thumplus.
//!
//! [`Thumplus`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	pose::{ThumplusPose, THUMPLUS_OVERALL_SCALE},
	ThumplusColors, ThumplusConfig,
};
use crate::{
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{nodes as humanoid, BODY_WHALE},
};
use lod::scene::LodSceneLevel;

/// Semantic Thumplus data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Thumplus {
	pub colors: ThumplusColors,
}

impl Thumplus {
	pub fn from_config(config: &ThumplusConfig) -> Self {
		Self { colors: config.colors.clone() }
	}
}

impl Default for Thumplus {
	fn default() -> Self {
		Self::from_config(&ThumplusConfig::default_preview())
	}
}

impl CharacterComponents for Thumplus {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID.scaled(THUMPLUS_OVERALL_SCALE)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![humanoid::forelimbed_body_rig(ThumplusPose.resolve())
			.with_normalization(AssetNormalization::centroid(THUMPLUS_OVERALL_SCALE))])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("body", vec![humanoid::body_part("whale", BODY_WHALE.as_str())]).map(
			|part| {
				let color = self.colors.color_for_slot(part.slot);
				part.with_base_color(color)
			},
		)
	}
}
