//! LodScene recipe for Wumbus.
//!
//! [`Wumbus`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::{WumbusHeadMesh, WumbusHornMesh, WumbusMouthMesh},
	pose::WumbusPose,
	WumbusColors, WumbusConfig,
};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{nodes as humanoid, EyeMesh, HairMesh, EAR_FLANK},
};
use lod::scene::LodSceneLevel;

/// Semantic Wumbus data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Wumbus {
	pub horns: WumbusHornMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: WumbusColors,
}

impl Wumbus {
	pub fn from_config(config: &WumbusConfig) -> Self {
		Self {
			horns: config.horns,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Wumbus {
	fn default() -> Self {
		Self::from_config(&WumbusConfig::default_preview())
	}
}

fn wumbus_eye_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, 0.0, -0.12))
}

fn wumbus_ear_left_local() -> Transform {
	Transform::from_translation(Vec3::new(0.15, 0.3, -0.05))
		.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0))
}

fn wumbus_ear_right_local() -> Transform {
	Transform::from_translation(Vec3::new(-0.15, 0.3, -0.05))
		.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0))
}

impl CharacterComponents for Wumbus {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::humanoid_from_pose(&WumbusPose.resolve(), 1.0)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(WumbusPose.resolve()),
			humanoid::orthograde_head_rig_at(
				AssetNormalization::base_y(0.3),
				Transform::from_translation(Vec3::new(0.0, -0.2, 0.00)),
			),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part("wumbus", "characters/bodies/biped/wumbus_biped_full_body.glb"),
				humanoid::spine(
					"snail-back",
					"characters/spines/snail_back_full_exo.glb",
					"upper_back",
				),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				WumbusHeadMesh::OrthoBear.label(),
				WumbusHeadMesh::OrthoBear.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.L",
				wumbus_eye_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.R",
				wumbus_eye_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				WumbusMouthMesh::CanineSnout.label(),
				WumbusMouthMesh::CanineSnout.path().as_str(),
				AssetNormalization::centroid(0.4),
				"mouth_socket",
				humanoid::mouth_socket_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::EarLeft,
				"flank",
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.L",
				wumbus_ear_left_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EarRight,
				"flank",
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.R",
				wumbus_ear_right_local(),
			),
		];
		if self.horns != WumbusHornMesh::None {
			features.push(humanoid::horns(self.horns.label(), self.horns.path().as_str()));
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
