//! LodScene recipe for Brenal.
//!
//! [`Brenal`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{pose::BrenalPose, sliders::BrenalSliders, BrenalColors, BrenalConfig};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	socket::RigId,
	species::{
		brenal::assets::{BrenalBodyMesh, BrenalHeadMesh, BrenalHornMesh, BrenalMouthMesh},
		common::{nodes as humanoid, EarMesh, EyeMesh, EAR_FLANK, TAIL_CAT},
	},
};
use lod::scene::LodSceneLevel;

/// Semantic Brenal data attached to the character root entity.
///
/// This species has no clothing catalog; [`crate::CharacterRecipe::clothed`] wraps the
/// inner recipe with an empty clothing layer list.
#[derive(Component, Clone, PartialEq)]
pub struct Brenal {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: BrenalHornMesh,
	pub eye: EyeMesh,
	pub colors: BrenalColors,
	pub sliders: BrenalSliders,
}

impl Brenal {
	pub fn from_config(config: &BrenalConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			horns: config.horns,
			eye: config.eye,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Brenal {
	fn default() -> Self {
		Self::from_config(&BrenalConfig::default_preview())
	}
}

impl CharacterComponents for Brenal {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		let pose =
			BrenalPose { gender: self.gender, build: self.build, sliders: self.sliders.clamped() };
		LocomotionCapsule::quadruped_for_limb_length(pose.rest_limb_scale())
			.with_pose_girdle(&pose.resolve())
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose =
			BrenalPose { gender: self.gender, build: self.build, sliders: self.sliders.clamped() };
		Layers::from_free(vec![
			humanoid::quadruped_body_rig(pose.resolve()),
			humanoid::pronograde_head_rig(
				AssetNormalization::base_y(0.2),
				RigId::Body,
				"head_socket",
				Transform::IDENTITY,
			),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part(
					BrenalBodyMesh::Gumbus.label(),
					BrenalBodyMesh::Gumbus.path().as_str(),
				),
				humanoid::tail("cat-tail", TAIL_CAT.as_str(), "tailbone"),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				BrenalHeadMesh::Canine.label(),
				BrenalHeadMesh::Canine.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.4),
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.25)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.4),
				"eye_socket.R",
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.25)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeRight)),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				BrenalMouthMesh::CanineSnout.label(),
				BrenalMouthMesh::CanineSnout.path().as_str(),
				AssetNormalization::centroid(0.8),
				"mouth_socket",
				Transform::IDENTITY,
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::Mouth)),
			humanoid::head_feature(
				CharacterPartSlot::EarLeft,
				EarMesh::Flank.label(),
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.L",
				Transform::IDENTITY,
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EarLeft)),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EarRight,
				EarMesh::Flank.label(),
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.R",
				Transform::IDENTITY,
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EarRight)),
		];
		if self.horns != BrenalHornMesh::None {
			features.push(
				humanoid::head_feature(
					CharacterPartSlot::Horns,
					self.horns.label(),
					self.horns.path().as_str(),
					AssetNormalization::centroid(0.7),
					"crown_socket",
					Transform::IDENTITY,
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::Horns)),
			);
		}
		out.extend_labeled("features", features);
		out.map(|part| {
			let color = self.colors.color_for_slot(part.slot);
			part.with_base_color(color)
		})
	}
}

// Keep fixed mesh enums referenced for compile-time asset wiring checks.
const _: BrenalBodyMesh = BrenalBodyMesh::Gumbus;
const _: BrenalHeadMesh = BrenalHeadMesh::Canine;
