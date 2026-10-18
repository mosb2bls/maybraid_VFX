//! LodScene recipe for Topple.
//!
//! [`Topple`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::{ToppleBeakMesh, ToppleHeadMesh},
	pose::{TopplePose, TOPPLE_OVERALL_SCALE},
	ToppleColors, ToppleConfig,
};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{nodes as humanoid, EyeMesh, HairMesh},
};
use lod::scene::LodSceneLevel;

/// Cartoonishly large head relative to the ~2 ft whelp body.
const HEAD_RIG_SOCKET_SCALE: f32 = 1.85;

/// Semantic Topple data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Topple {
	pub beak: ToppleBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: ToppleColors,
}

impl Topple {
	pub fn from_config(config: &ToppleConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Topple {
	fn default() -> Self {
		Self::from_config(&ToppleConfig::default_preview())
	}
}

impl CharacterComponents for Topple {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID
			.scaled(TOPPLE_OVERALL_SCALE)
			.with_head_scale(HEAD_RIG_SOCKET_SCALE)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(TopplePose.resolve())
				.with_normalization(AssetNormalization::centroid(TOPPLE_OVERALL_SCALE)),
			humanoid::orthograde_head_rig_at(
				AssetNormalization::base_y(0.26),
				Transform::from_scale(Vec3::splat(HEAD_RIG_SOCKET_SCALE)),
			),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part("whelp", "characters/bodies/biped/whelp_bird.glb")],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				ToppleHeadMesh::Meerkat.label(),
				ToppleHeadMesh::Meerkat.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye),
			humanoid::eye_right(self.eye),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				self.beak.label(),
				self.beak.path().as_str(),
				AssetNormalization::centroid(0.35),
				"mouth_socket",
				humanoid::mouth_socket_local(),
			),
		];
		if let Some(hair) = humanoid::hair_scaled(
			self.hair,
			match self.hair {
				HairMesh::FeatherHawk => 0.4,
				_ => 1.0,
			},
		) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out.map(|part| {
			let color = self.colors.color_for_slot(part.slot);
			part.with_base_color(color)
		})
	}
}
