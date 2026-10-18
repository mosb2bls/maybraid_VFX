//! Named firearm presets: a body plus whatever optional slots that kit fills.

use lod::scene::LodSceneLevel;

use crate::kit::FirearmKit;
use crate::parts::{BarrelMesh, BodyMesh, GripMesh, SightMesh, StockMesh, TriggerBoxMesh};
use firearms_components::assets::guns;
use firearms_components::{FirearmComponents, Layers, PartNode, RigNode};

/// Authored firearm concepts currently in `maybraid/art/items/guns/{bodies,barrels,grips,…}/`.
///
/// Each concept is a [`FirearmKit`]: body is required, other slots may be empty.
/// Mix arbitrary parts in the playground with `kit --barrel laznard` (etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bevy::prelude::Component)]
pub enum FirearmConcept {
	/// Body + barrel + bump-handle grip.
	#[default]
	Bullpup,
	Silopup,
	Reltor,
	Samsonist,
	Snailer,
}

impl FirearmConcept {
	pub const ALL: [Self; 5] =
		[Self::Bullpup, Self::Silopup, Self::Reltor, Self::Samsonist, Self::Snailer];

	pub fn label(self) -> &'static str {
		match self {
			Self::Bullpup => "bullpup",
			Self::Silopup => "silopup",
			Self::Reltor => "reltor",
			Self::Samsonist => "samsonist",
			Self::Snailer => "snailer",
		}
	}

	pub fn body(self) -> BodyMesh {
		match self {
			Self::Bullpup => BodyMesh::Bullpup,
			Self::Silopup => BodyMesh::Silopup,
			Self::Reltor => BodyMesh::Reltor,
			Self::Samsonist => BodyMesh::Samsonist,
			Self::Snailer => BodyMesh::Snailer,
		}
	}

	/// Kit this named concept expands to (empty optional slots stay `none`).
	pub fn kit(self) -> FirearmKit {
		match self {
			Self::Bullpup => FirearmKit {
				body: BodyMesh::Bullpup,
				barrel: BarrelMesh::Bullpup,
				trigger_box: TriggerBoxMesh::None,
				grip: GripMesh::BumpHandle,
				stock: StockMesh::None,
				sight: SightMesh::None,
			},
			Self::Reltor => FirearmKit {
				body: BodyMesh::Reltor,
				trigger_box: TriggerBoxMesh::Reltor,
				..FirearmKit::default()
			},
			other => FirearmKit::body(other.body()),
		}
	}

	/// Baked one-mesh concept, when the kit was also exported as a single GLB.
	pub fn baked_concept(self) -> Option<firearms_components::AssetPath> {
		match self {
			Self::Bullpup => Some(guns::BULLPUP_FULL_CONCEPT),
			Self::Silopup => Some(guns::SILOPUP_FULL_CONCEPT),
			_ => None,
		}
	}
}

impl std::str::FromStr for FirearmConcept {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::ALL
			.iter()
			.copied()
			.find(|concept| concept.label() == s)
			.ok_or_else(|| format!("unknown firearm concept {s:?}"))
	}
}

impl FirearmComponents for FirearmConcept {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.kit().rig_nodes_for_level(level)
	}

	fn body_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.kit().body_nodes_for_level(level)
	}

	fn barrel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.kit().barrel_nodes_for_level(level)
	}

	fn trigger_box_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.kit().trigger_box_nodes_for_level(level)
	}

	fn grip_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.kit().grip_nodes_for_level(level)
	}

	fn stock_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.kit().stock_nodes_for_level(level)
	}

	fn sight_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.kit().sight_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use firearms_components::{FirearmPartSlot, SocketRef};

	#[test]
	fn bullpup_assembles_body_barrel_grip_on_the_receiver() {
		let gun = FirearmConcept::Bullpup;
		let kit = gun.kit();
		assert_eq!(kit.body, BodyMesh::Bullpup);
		assert_eq!(kit.barrel, BarrelMesh::Bullpup);
		assert_eq!(kit.grip, GripMesh::BumpHandle);
		let rigs = gun.rig_nodes_for_level(LodSceneLevel::High).flatten();
		let bodies = gun.body_nodes_for_level(LodSceneLevel::High).flatten();
		let barrels = gun.barrel_nodes_for_level(LodSceneLevel::High).flatten();
		let grips = gun.grip_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(rigs[0].scene.path, guns::FIREARM_RIG.as_str());
		assert_eq!(bodies[0].slot, FirearmPartSlot::Body);
		assert_eq!(barrels[0].slot, FirearmPartSlot::Barrel);
		assert_eq!(grips[0].slot, FirearmPartSlot::Grip);
		assert_eq!(bodies[0].socket, Some(SocketRef::bone("body")));
		assert_eq!(barrels[0].socket, Some(SocketRef::bone("barrel")));
		assert_eq!(grips[0].socket, Some(SocketRef::bone("grip")));
		assert!(gun.trigger_box_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(gun.stock_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(gun.sight_nodes_for_level(LodSceneLevel::High).is_empty());
	}

	#[test]
	fn every_concept_emits_the_shared_receiver_and_a_body() {
		for gun in FirearmConcept::ALL {
			let kit = gun.kit();
			assert_eq!(kit.body, gun.body());
			let rigs = gun.rig_nodes_for_level(LodSceneLevel::High).flatten();
			assert_eq!(rigs.len(), 1);
			assert_eq!(rigs[0].scene.path, guns::FIREARM_RIG.as_str());
			assert_eq!(gun.body_nodes_for_level(LodSceneLevel::High).len(), 1);
		}
	}

	#[test]
	fn reltor_includes_its_trigger_box() {
		let gun = FirearmConcept::Reltor;
		let boxes = gun.trigger_box_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(boxes[0].slot, FirearmPartSlot::TriggerBox);
		assert_eq!(boxes[0].socket, Some(SocketRef::bone("trigger_box")));
		assert_eq!(boxes[0].scene.path, guns::RELTOR_BOX.as_str());
	}

	#[test]
	fn body_only_concepts_have_no_kit_attachments() {
		for gun in [FirearmConcept::Silopup, FirearmConcept::Samsonist, FirearmConcept::Snailer] {
			assert_eq!(gun.body_nodes_for_level(LodSceneLevel::High).len(), 1);
			assert!(gun.barrel_nodes_for_level(LodSceneLevel::High).is_empty());
			assert!(gun.trigger_box_nodes_for_level(LodSceneLevel::High).is_empty());
			assert!(gun.grip_nodes_for_level(LodSceneLevel::High).is_empty());
			assert!(gun.stock_nodes_for_level(LodSceneLevel::High).is_empty());
			assert!(gun.sight_nodes_for_level(LodSceneLevel::High).is_empty());
		}
	}
}
