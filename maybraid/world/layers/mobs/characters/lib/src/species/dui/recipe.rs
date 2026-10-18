//! LodScene recipe for Dui.
//!
//! [`Dui`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiNoseMesh},
	pose::DuiPose,
	DuiColors, DuiConfig,
};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{nodes as humanoid, HairMesh},
};
use lod::scene::LodSceneLevel;

/// Semantic Dui data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Dui {
	pub nose: DuiNoseMesh,
	pub hair: HairMesh,
	pub colors: DuiColors,
}

impl Dui {
	pub fn from_config(config: &DuiConfig) -> Self {
		Self { nose: config.nose, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Dui {
	fn default() -> Self {
		Self::from_config(&DuiConfig::default_preview())
	}
}

fn dui_eye_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, -0.1, 0.05))
}

impl CharacterComponents for Dui {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::humanoid_from_pose(&DuiPose.resolve(), 1.0)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(DuiPose.resolve()),
			humanoid::orthograde_head_rig_at(AssetNormalization::base_y(0.4), Transform::IDENTITY),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part("igeo", "characters/bodies/biped/igeo_biped_full_body.glb")],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				DuiHeadMesh::BarredBowl.label(),
				DuiHeadMesh::BarredBowl.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				DuiEyeMesh::Thorn.label(),
				DuiEyeMesh::Thorn.path().as_str(),
				AssetNormalization::centroid(0.6),
				"eye_socket.L",
				dui_eye_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				DuiEyeMesh::Thorn.label(),
				DuiEyeMesh::Thorn.path().as_str(),
				AssetNormalization::centroid(0.6),
				"eye_socket.R",
				dui_eye_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				DuiMouthMesh::SmallCommon.label(),
				DuiMouthMesh::SmallCommon.path().as_str(),
				AssetNormalization::centroid(0.08),
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)),
			),
		];
		if self.nose != DuiNoseMesh::None {
			features.push(humanoid::head_feature(
				CharacterPartSlot::Nose,
				self.nose.label(),
				self.nose.path().as_str(),
				AssetNormalization::centroid(0.05),
				"nose_socket",
				humanoid::nose_socket_local(),
			));
		}
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
