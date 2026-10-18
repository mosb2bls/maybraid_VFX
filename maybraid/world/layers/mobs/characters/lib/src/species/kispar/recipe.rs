//! LodScene recipe for Kispar.
//!
//! [`Kispar`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::{KisparBeakMesh, KisparHeadMesh},
	pose::{KisparPose, KISPAR_OVERALL_SCALE},
	KisparColors, KisparConfig,
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

/// Semantic Kispar data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Kispar {
	pub beak: KisparBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: KisparColors,
}

impl Kispar {
	pub fn from_config(config: &KisparConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Kispar {
	fn default() -> Self {
		Self::from_config(&KisparConfig::default_preview())
	}
}

impl CharacterComponents for Kispar {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID.scaled(KISPAR_OVERALL_SCALE)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(KisparPose.resolve())
				.with_normalization(AssetNormalization::centroid(KISPAR_OVERALL_SCALE)),
			humanoid::orthograde_head_rig(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part("sparrow", "characters/bodies/biped/sparrow_body.glb")],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				KisparHeadMesh::Meerkat.label(),
				KisparHeadMesh::Meerkat.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye),
			humanoid::eye_right(self.eye),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				self.beak.label(),
				self.beak.path().as_str(),
				AssetNormalization::centroid(0.45),
				"mouth_socket",
				humanoid::mouth_socket_local().with_scale(Vec3::new(0.85, 0.85, 1.65)),
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
