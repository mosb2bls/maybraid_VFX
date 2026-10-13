//! Catalog [`FirearmSpec`] → assembled [`FirearmKit`], with slot looks painted on.

use character_items::{
	FirearmBarrel, FirearmGrip, FirearmKitSpec, FirearmMesh, FirearmSight, FirearmSpec,
	FirearmStock, FirearmTriggerBox, SlotLook,
};
use firearms::{
	BarrelMesh, BodyMesh, FirearmComponents, FirearmKit, GripMesh, Layers, PartNode, RigNode,
	SightMesh, StockMesh, TriggerBoxMesh,
};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

/// Painted inventory kit used by the world, range, mobs, and character-menu inspect.
///
/// Socket rest scales (including the 1 m sight cube) come from [`FirearmKit`]; this
/// wrapper only stamps catalog materials. Pose stays bind, matching held kits — the
/// world then applies a uniform [`crate::HeldFirearm`] root scale.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeneratedFirearm {
	pub spec: FirearmSpec,
	pub kit: FirearmKit,
}

impl GeneratedFirearm {
	pub fn from_spec(spec: FirearmSpec) -> Self {
		Self { spec, kit: kit_from_spec(spec) }
	}

	/// Capture / inspect pose: unused stock and barrel bones scale to zero.
	pub fn icon_pose(self) -> firearms::FirearmPose {
		self.kit.icon_pose()
	}

	fn look_material(look: SlotLook) -> MaterialRef {
		MaterialRef::named(look.material.recipe_id()).with_palette([look.color.color()])
	}

	fn paint(layers: Layers<PartNode>, look: SlotLook) -> Layers<PartNode> {
		let material = Self::look_material(look);
		let mut out = Layers::new();
		out.extend_free(layers.free.into_iter().map(|node| node.with_material(material.clone())));
		for (layer, nodes) in layers.labeled {
			out.extend_labeled(
				layer,
				nodes.into_iter().map(|node| node.with_material(material.clone())),
			);
		}
		out
	}
}

/// Map a rolled catalog spec onto the combat [`FirearmKit`].
pub fn kit_from_spec(spec: FirearmSpec) -> FirearmKit {
	kit_from_parts(spec.kit)
}

fn kit_from_parts(kit: FirearmKitSpec) -> FirearmKit {
	FirearmKit {
		body: match kit.body {
			FirearmMesh::Bullpup => BodyMesh::Bullpup,
			FirearmMesh::Silopup => BodyMesh::Silopup,
			FirearmMesh::Reltor => BodyMesh::Reltor,
			FirearmMesh::Samsonist => BodyMesh::Samsonist,
			FirearmMesh::Snailer => BodyMesh::Snailer,
		},
		barrel: match kit.barrel {
			FirearmBarrel::None => BarrelMesh::None,
			FirearmBarrel::Bullpup => BarrelMesh::Bullpup,
			FirearmBarrel::Laznard => BarrelMesh::Laznard,
		},
		trigger_box: match kit.trigger_box {
			FirearmTriggerBox::None => TriggerBoxMesh::None,
			FirearmTriggerBox::Keelripe => TriggerBoxMesh::Keelripe,
			FirearmTriggerBox::Paddle => TriggerBoxMesh::Paddle,
			FirearmTriggerBox::Reltor => TriggerBoxMesh::Reltor,
		},
		grip: match kit.grip {
			FirearmGrip::None => GripMesh::None,
			FirearmGrip::BumpHandle => GripMesh::BumpHandle,
		},
		stock: match kit.stock {
			FirearmStock::None => StockMesh::None,
		},
		sight: match kit.sight {
			FirearmSight::None => SightMesh::None,
			FirearmSight::Holorand => SightMesh::Holorand,
			FirearmSight::Leskop => SightMesh::Leskop,
		},
	}
}

impl FirearmComponents for GeneratedFirearm {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.kit.rig_nodes_for_level(level)
	}

	fn body_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.body_nodes_for_level(level), self.spec.looks.body)
	}

	fn barrel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.barrel_nodes_for_level(level), self.spec.looks.barrel)
	}

	fn trigger_box_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.trigger_box_nodes_for_level(level), self.spec.looks.trigger_box)
	}

	fn grip_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.grip_nodes_for_level(level), self.spec.looks.grip)
	}

	fn stock_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.stock_nodes_for_level(level), self.spec.looks.stock)
	}

	fn sight_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.sight_nodes_for_level(level), self.spec.looks.sight)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{FirearmMaterial, FirearmMesh, ItemColor};
	use firearms::{SocketRef, StockMesh};

	#[test]
	fn kit_mapping_preserves_bullpup_concept_slots() {
		let kit = kit_from_spec(FirearmSpec::from_mesh(FirearmMesh::Bullpup));
		assert_eq!(kit.body, BodyMesh::Bullpup);
		assert_eq!(kit.barrel, BarrelMesh::Bullpup);
		assert_eq!(kit.grip, GripMesh::BumpHandle);
	}

	#[test]
	fn kit_mapping_preserves_holorand_sight() {
		let mut spec = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		spec.kit.sight = FirearmSight::Holorand;
		let kit = kit_from_spec(spec);
		assert_eq!(kit.sight, SightMesh::Holorand);
	}

	#[test]
	fn kit_mapping_preserves_reltor_trigger_box() {
		let kit = kit_from_spec(FirearmSpec::from_mesh(FirearmMesh::Reltor));
		assert_eq!(kit.body, BodyMesh::Reltor);
		assert_eq!(kit.trigger_box, TriggerBoxMesh::Reltor);
	}

	#[test]
	fn paints_the_body_look_onto_the_kit_node() {
		let mut spec = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		spec.looks.body = SlotLook::new(FirearmMaterial::LavaVeins, ItemColor::Red);
		let rolled = GeneratedFirearm::from_spec(spec);
		let nodes = rolled.body_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 1);
		assert_eq!(nodes[0].material, GeneratedFirearm::look_material(spec.looks.body));
	}

	#[test]
	fn icon_pose_collapses_empty_stock() {
		let rolled = GeneratedFirearm::from_spec(FirearmSpec::from_mesh(FirearmMesh::Snailer));
		assert_eq!(rolled.kit.stock, StockMesh::None);
		assert_eq!(rolled.icon_pose().stock.length, 0.0);
	}

	#[test]
	fn holorand_keeps_the_kit_sight_socket_and_rest_scale() {
		let mut spec = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		spec.kit.sight = FirearmSight::Holorand;
		let rolled = GeneratedFirearm::from_spec(spec);
		let sights = rolled.sight_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(
			sights[0].socket,
			Some(SocketRef::bone("sight_socket").with_local(bevy::prelude::Transform::from_scale(
				bevy::prelude::Vec3::splat(SightMesh::Holorand.rest_scale())
			)))
		);
	}
}
