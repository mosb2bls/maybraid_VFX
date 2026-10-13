//! LodScene recipe for Tapp.
//!
//! [`Tapp`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::{TappBeakMesh, TappHeadMesh},
	pose::{TappPose, TAPP_OVERALL_SCALE},
	TappColors, TappConfig,
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

/// Cartoonishly large head relative to the ~2 ft whelp body (same as Topple).
const HEAD_RIG_SOCKET_SCALE: f32 = 1.85;

/// Semantic Tapp data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Tapp {
	pub beak: TappBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: TappColors,
}

impl Tapp {
	pub fn from_config(config: &TappConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Tapp {
	fn default() -> Self {
		Self::from_config(&TappConfig::default_preview())
	}
}

impl CharacterComponents for Tapp {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID
			.scaled(TAPP_OVERALL_SCALE)
			.with_head_scale(HEAD_RIG_SOCKET_SCALE)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(TappPose.resolve())
				.with_normalization(AssetNormalization::centroid(TAPP_OVERALL_SCALE)),
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
				TappHeadMesh::Meerkat.label(),
				TappHeadMesh::Meerkat.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye),
			humanoid::eye_right(self.eye),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				self.beak.label(),
				self.beak.path().as_str(),
				AssetNormalization::centroid(0.5),
				"mouth_socket",
				humanoid::mouth_socket_local().with_scale(Vec3::new(0.8, 0.8, 1.8)),
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
