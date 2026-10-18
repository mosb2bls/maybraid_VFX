//! LodScene recipe for Croconot.
//!
//! [`Croconot`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::{SNOUT_XY_SCALE, SNOUT_Z_SCALE},
	pose::CroconotPose,
	sliders::CroconotSliders,
	CroconotColors, CroconotConfig,
};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	socket::{RigId, SocketRef},
	species::{
		common::{nodes as humanoid, EarMesh, EyeMesh, EAR_FLANK, TAIL_LERODON_QUADRUPED},
		croconot::assets::{
			CroconotBodyMesh, CroconotHeadMesh, CroconotHornMesh, CroconotMouthMesh,
		},
	},
};
use lod::scene::LodSceneLevel;

/// Semantic Croconot data attached to the character root entity.
///
/// This species has no clothing catalog; [`crate::CharacterRecipe::clothed`] wraps the
/// inner recipe with an empty clothing layer list.
#[derive(Component, Clone, PartialEq)]
pub struct Croconot {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: CroconotHornMesh,
	pub eye: EyeMesh,
	pub colors: CroconotColors,
	pub sliders: CroconotSliders,
}

impl Croconot {
	pub fn from_config(config: &CroconotConfig) -> Self {
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

impl Default for Croconot {
	fn default() -> Self {
		Self::from_config(&CroconotConfig::default_preview())
	}
}

impl CharacterComponents for Croconot {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		let pose = CroconotPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		};
		LocomotionCapsule::quadruped_for_limb_length(pose.rest_limb_scale())
			.with_pose_girdle(&pose.resolve())
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose = CroconotPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		};
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
					CroconotBodyMesh::Dragloon.label(),
					CroconotBodyMesh::Dragloon.path().as_str(),
				),
				humanoid::tail(
					"lerodon-tail",
					TAIL_LERODON_QUADRUPED.as_str(),
					"haunch_vertical_thickness",
				)
				.socketed(
					SocketRef::on(RigId::Body, "haunch_vertical_thickness")
						.with_local(Transform::from_translation(Vec3::new(0.0, -0.05, -0.05))),
				),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				CroconotHeadMesh::Canine.label(),
				CroconotHeadMesh::Canine.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.4),
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.05, -0.12)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.4),
				"eye_socket.R",
				Transform::from_translation(Vec3::new(0.0, -0.05, -0.12)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeRight)),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				CroconotMouthMesh::Lerodon.label(),
				CroconotMouthMesh::Lerodon.path().as_str(),
				AssetNormalization::centroid(0.4),
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)).with_scale(Vec3::new(
					SNOUT_XY_SCALE,
					SNOUT_XY_SCALE,
					SNOUT_Z_SCALE,
				)),
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
		if self.horns != CroconotHornMesh::None {
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
const _: CroconotBodyMesh = CroconotBodyMesh::Dragloon;
const _: CroconotHeadMesh = CroconotHeadMesh::Canine;
