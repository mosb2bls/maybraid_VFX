//! Inner [`CharacterComponents`] recipe for Bikanwarda.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use characters::{
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::epiphant::recipe::Epiphant,
	CharacterRecipe,
};
use lod::scene::LodSceneLevel;

use crate::presentation::BikanwardaPresentation;
use crate::spec::{BikanwardaForm, BikanwardaSpec};

/// Semantic Bikanwarda data on the assembly root.
#[derive(Component, Clone, PartialEq)]
pub struct BikanwardaActor {
	pub epiphant: Epiphant,
	pub form: BikanwardaForm,
	pub presentation: BikanwardaPresentation,
	pub uniform_scale: f32,
}

/// Config → components for all Bikanwarda forms.
#[derive(Debug, Clone, PartialEq)]
pub struct BikanwardaActorRecipe {
	pub spec: BikanwardaSpec,
}

impl Default for BikanwardaActorRecipe {
	fn default() -> Self {
		Self::for_form(BikanwardaForm::Encounter)
	}
}

impl BikanwardaActorRecipe {
	pub fn for_form(form: BikanwardaForm) -> Self {
		let spec = match form {
			BikanwardaForm::Encounter => BikanwardaSpec::encounter(),
			BikanwardaForm::Ward => BikanwardaSpec::ward(),
			BikanwardaForm::Enraged => BikanwardaSpec::enraged(),
		};
		Self { spec }
	}

	pub fn from_spec(spec: BikanwardaSpec) -> Self {
		Self { spec }
	}

	pub fn uniform_scale(&self) -> f32 {
		self.spec.uniform_scale()
	}

	pub fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.components().locomotion_capsule()
	}
}

impl CharacterRecipe for BikanwardaActorRecipe {
	type Components = BikanwardaActor;

	fn components(&self) -> Self::Components {
		let mut epiphant = Epiphant::from_config(&self.spec.epiphant);
		epiphant.colors = self.spec.presentation.colors.clone();
		BikanwardaActor {
			epiphant,
			form: self.spec.form,
			presentation: self.spec.presentation.clone(),
			uniform_scale: self.uniform_scale(),
		}
	}

	fn clothing_layers(&self) -> Vec<characters::ClothingLayer> {
		Vec::new()
	}
}

impl BikanwardaActor {
	/// Motor hull after encounter uniform scale (matches root [`Transform::scale`]).
	pub fn scaled_locomotion_capsule(&self) -> LocomotionCapsule {
		self.epiphant.locomotion_capsule().scaled(self.uniform_scale)
	}
}

impl CharacterComponents for BikanwardaActor {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.epiphant.locomotion_capsule()
	}

	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.epiphant.rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.epiphant.part_nodes_for_level(level)
	}

	fn preview_aabb(&self) -> Aabb3d {
		self.scaled_locomotion_capsule().preview_aabb()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_animations::animations::MOUTH_EMIT_SOCKET;
	use characters::assembly::CharacterPartSlot;
	use lod::scene::LodSceneLevel;

	#[test]
	fn bikanwarda_reuses_epiphant_nose_socket_for_bray() {
		let actor = BikanwardaActorRecipe::default().components();
		let parts = actor.part_nodes_for_level(LodSceneLevel::High).flatten();
		let nose = parts
			.iter()
			.find(|part| part.slot == CharacterPartSlot::Nose)
			.expect("epiphant nose part");
		assert_eq!(nose.socket.as_ref().and_then(|s| s.bone), Some(MOUTH_EMIT_SOCKET));
	}
}
