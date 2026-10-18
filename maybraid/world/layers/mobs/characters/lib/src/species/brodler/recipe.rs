//! LodScene recipe for Brodler.
//!
//! [`Brodler`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{assets::HornMesh, pose::BrodlerPose, BrodlerColors, BrodlerConfig, BrodlerHeadMesh};
use crate::{
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		nodes as humanoid, BodyMesh, EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh,
	},
};
use lod::scene::LodSceneLevel;

/// Semantic Brodler data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`crate::CharacterRecipe::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Brodler {
	pub head: BrodlerHeadMesh,
	pub horns: HornMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	pub colors: BrodlerColors,
}

impl Brodler {
	pub fn from_config(config: &BrodlerConfig) -> Self {
		Self {
			head: config.head,
			horns: config.horns,
			eye: config.eye,
			nose: config.nose,
			mouth: config.mouth,
			ear: config.ear,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Brodler {
	fn default() -> Self {
		Self::from_config(&BrodlerConfig::default_preview())
	}
}

impl CharacterComponents for Brodler {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::humanoid_from_pose(&BrodlerPose.resolve(), 1.0)
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(BrodlerPose.resolve()),
			humanoid::orthograde_head_rig(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled("body", vec![humanoid::body_mesh(BodyMesh::Standard)]);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(self.head.label(), self.head.path().as_str())],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye),
			humanoid::eye_right(self.eye),
			humanoid::nose(self.nose),
			humanoid::mouth(self.mouth),
			humanoid::ear_left(self.ear),
			humanoid::ear_right(self.ear),
			humanoid::horns(self.horns.label(), self.horns.path().as_str()),
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
