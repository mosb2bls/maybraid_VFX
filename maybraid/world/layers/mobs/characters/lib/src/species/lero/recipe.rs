//! LodScene recipe for Lero.
//!
//! [`Lero`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::{LeroHeadMesh, LeroMouthMesh},
	pose::LeroPose,
	LeroColors, LeroConfig,
};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{nodes as humanoid, HairMesh, BODY_FULL, EYE_STANDARD, TAIL_LERODON},
};
use lod::scene::LodSceneLevel;

const SNOUT_Z_SCALE: f32 = 2.5;

/// Semantic Lero data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Lero {
	pub mouth: LeroMouthMesh,
	pub hair: HairMesh,
	pub colors: LeroColors,
}

impl Lero {
	pub fn from_config(config: &LeroConfig) -> Self {
		Self { mouth: config.mouth, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Lero {
	fn default() -> Self {
		Self::from_config(&LeroConfig::default_preview())
	}
}

fn lero_eye_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, -0.05, -0.12))
}

impl CharacterComponents for Lero {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::humanoid_from_pose(&LeroPose.resolve(), 1.0)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(LeroPose.resolve()),
			humanoid::orthograde_head_rig_at(AssetNormalization::base_y(0.28), Transform::IDENTITY),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part("leron", BODY_FULL.as_str()),
				humanoid::tail("lerodon-tail", TAIL_LERODON.as_str(), "root"),
				humanoid::spine(
					"lerodon-spine",
					"characters/spines/spiked_lerodon_full_exo.glb",
					"upper_back",
				),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				LeroHeadMesh::OrthoTee.label(),
				LeroHeadMesh::OrthoTee.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				"standard",
				EYE_STANDARD.as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.L",
				lero_eye_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				"standard",
				EYE_STANDARD.as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.R",
				lero_eye_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				self.mouth.label(),
				self.mouth.path().as_str(),
				AssetNormalization::centroid(0.4),
				"mouth_socket",
				humanoid::mouth_socket_local().with_scale(Vec3::new(1.0, 1.0, SNOUT_Z_SCALE)),
			),
		];
		if let Some(hair) = humanoid::hair(self.hair) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out.map(|part| {
			let color = self.colors.color_for_slot(part.slot);
			part.with_base_color(color)
		})
	}
}
