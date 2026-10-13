//! LodScene recipe for Tuberwaber.
//!
//! [`Tuberwaber`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{pose::TuberwaberPose, sliders::TuberwaberSliders, TuberwaberColors, TuberwaberConfig};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	species::{
		common::{nodes as humanoid, EyeMesh, HairMesh, MouthMesh, NoseMesh, HORNS_HARROWED_CROWN},
		tuberwaber::assets::{TuberwaberBodyMesh, TuberwaberHeadMesh},
	},
};
use lod::scene::LodSceneLevel;

/// Semantic Tuberwaber data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Tuberwaber {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: TuberwaberBodyMesh,
	pub head: TuberwaberHeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub hair: HairMesh,
	pub colors: TuberwaberColors,
	pub sliders: TuberwaberSliders,
}

impl Tuberwaber {
	pub fn from_config(config: &TuberwaberConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			body: config.body,
			head: config.head,
			eye: config.eye,
			nose: config.nose,
			mouth: config.mouth,
			hair: config.hair,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Tuberwaber {
	fn default() -> Self {
		Self::from_config(&TuberwaberConfig::default_preview())
	}
}

fn tuberwaber_eye_left_local() -> Transform {
	Transform::from_translation(Vec3::new(0.3, 0.05, -0.12))
}

fn tuberwaber_eye_right_local() -> Transform {
	Transform::from_translation(Vec3::new(-0.3, 0.05, -0.12))
}

impl CharacterComponents for Tuberwaber {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		let pose = TuberwaberPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		};
		LocomotionCapsule::humanoid_from_pose(&pose.resolve(), 1.0)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose = TuberwaberPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		}
		.resolve();
		Layers::from_free(vec![humanoid::humanoid_body_rig(pose), humanoid::orthograde_head_rig()])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part(self.body.label(), self.body.path().as_str())],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(self.head.label(), self.head.path().as_str())],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.24),
				"eye_socket.L",
				tuberwaber_eye_left_local(),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.24),
				"eye_socket.R",
				tuberwaber_eye_right_local(),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeRight)),
			humanoid::head_feature(
				CharacterPartSlot::Nose,
				self.nose.label(),
				self.nose.path().as_str(),
				self.nose.normalization(),
				"nose_socket",
				Transform::from_translation(Vec3::new(0.0, 0.05, 0.1)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::Nose)),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				self.mouth.label(),
				self.mouth.path().as_str(),
				AssetNormalization::centroid(0.12),
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))
					.with_scale(Vec3::new(2.2, 1.0, 1.0)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::Mouth)),
			humanoid::head_feature(
				CharacterPartSlot::Horns,
				"harrowed-crown",
				HORNS_HARROWED_CROWN.as_str(),
				AssetNormalization::centroid(0.7),
				"crown_socket",
				Transform::IDENTITY,
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::Horns)),
		];
		if let Some(hair) = humanoid::hair(self.hair) {
			features.push(hair.with_feature(sliders.feature_transform(CharacterPartSlot::Hair)));
		}
		out.extend_labeled("features", features);
		out.map(|part| {
			let color = self.colors.color_for_slot(part.slot);
			part.with_base_color(color)
		})
	}
}

const _: TuberwaberBodyMesh = TuberwaberBodyMesh::Tuberwaber;
const _: TuberwaberHeadMesh = TuberwaberHeadMesh::Tuberwaber;
