//! LodScene recipe for Braidman.
//!
//! [`Braidman`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{pose::BraidmanPose, sliders::BraidmanSliders, BraidmanColors, BraidmanConfig};
use crate::{
	assembly::CharacterPartSlot,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	species::common::{
		nodes as humanoid, BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
	},
};
use lod::scene::LodSceneLevel;

/// Semantic Braidman data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Braidman {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: BodyMesh,
	pub head: HeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	pub colors: BraidmanColors,
	pub sliders: BraidmanSliders,
}

impl Braidman {
	pub fn from_config(config: &BraidmanConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			body: config.body,
			head: config.head,
			eye: config.eye,
			nose: config.nose,
			mouth: config.mouth,
			ear: config.ear,
			hair: config.hair,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Braidman {
	fn default() -> Self {
		Self::from_config(&BraidmanConfig::default_preview())
	}
}

impl CharacterComponents for Braidman {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		let pose = BraidmanPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		};
		LocomotionCapsule::humanoid_from_pose(&pose.resolve(), 1.0)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose = BraidmanPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		}
		.resolve();
		Layers::from_free(vec![humanoid::humanoid_body_rig(pose), humanoid::orthograde_head_rig()])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled("body", vec![humanoid::body_mesh(self.body)]);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(self.head.label(), self.head.path().as_str())],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
			humanoid::eye_right(self.eye)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EyeRight)),
			humanoid::nose(self.nose)
				.with_feature(sliders.feature_transform(CharacterPartSlot::Nose)),
			humanoid::mouth(self.mouth)
				.with_feature(sliders.feature_transform(CharacterPartSlot::Mouth)),
			humanoid::ear_left(self.ear)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EarLeft)),
			humanoid::ear_right(self.ear)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EarRight)),
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
