//! Mix-and-match firearm: required body plus optional kit slots.

use firearms_components::assets::guns;
use firearms_components::{FirearmComponents, Layers, PartNode, RigNode};
use lod::scene::LodSceneLevel;

use crate::parts::{BarrelMesh, BodyMesh, GripMesh, SightMesh, StockMesh, TriggerBoxMesh};
use crate::pose::{BoneFit, FirearmPose};

/// Assembled firearm: receiver + body, and whatever other slots are filled.
///
/// Barrel, trigger box, grip, stock, and sight may be [`None`](BarrelMesh::None). Body
/// is always present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bevy::prelude::Component)]
pub struct FirearmKit {
	pub body: BodyMesh,
	pub barrel: BarrelMesh,
	pub trigger_box: TriggerBoxMesh,
	pub grip: GripMesh,
	pub stock: StockMesh,
	pub sight: SightMesh,
}

impl FirearmKit {
	pub fn body(body: BodyMesh) -> Self {
		Self { body, ..Self::default() }
	}

	/// Collapse unused stock / barrel bones so icon extents ignore empty sockets.
	pub fn icon_pose(self) -> FirearmPose {
		FirearmPose {
			barrel: BoneFit::collapsed_if(matches!(self.barrel, BarrelMesh::None)),
			stock: BoneFit::collapsed_if(matches!(self.stock, StockMesh::None)),
			..FirearmPose::default()
		}
	}

	pub fn label(self) -> String {
		format!(
			"body={} barrel={} trigger-box={} grip={} stock={} sight={}",
			self.body.label(),
			self.barrel.label(),
			self.trigger_box.label(),
			self.grip.label(),
			self.stock.label(),
			self.sight.label(),
		)
	}
}

fn receiver_rig() -> RigNode {
	RigNode::receiver("firearm-rig", guns::FIREARM_RIG.as_str())
}

fn optional_layer(name: &'static str, node: Option<PartNode>) -> Layers<PartNode> {
	match node {
		Some(node) => Layers::from_labeled(name, vec![node]),
		None => Layers::new(),
	}
}

impl FirearmComponents for FirearmKit {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_labeled("receiver", vec![receiver_rig()])
	}

	fn body_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("body", vec![self.body.node()])
	}

	fn barrel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("barrel", self.barrel.node())
	}

	fn trigger_box_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("trigger_box", self.trigger_box.node())
	}

	fn grip_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("grip", self.grip.node())
	}

	fn stock_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("stock", self.stock.node())
	}

	fn sight_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("sight", self.sight.node())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::pose::BoneFit;
	use firearms_components::{FirearmPartSlot, SocketRef};

	#[test]
	fn default_kit_is_body_only() {
		let kit = FirearmKit::default();
		assert_eq!(kit.body, BodyMesh::Bullpup);
		assert_eq!(kit.body_nodes_for_level(LodSceneLevel::High).len(), 1);
		assert!(kit.barrel_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(kit.trigger_box_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(kit.grip_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(kit.stock_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(kit.sight_nodes_for_level(LodSceneLevel::High).is_empty());
	}

	#[test]
	fn holorand_sockets_on_the_sight_landmark() {
		let kit = FirearmKit { sight: SightMesh::Holorand, ..FirearmKit::body(BodyMesh::Bullpup) };
		let sights = kit.sight_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sights[0].slot, FirearmPartSlot::Sight);
		assert_eq!(
			sights[0].socket,
			Some(SocketRef::bone("sight_socket").with_local(bevy::prelude::Transform::from_scale(
				bevy::prelude::Vec3::splat(0.10)
			)))
		);
		assert_eq!(sights[0].scene.path, guns::HOLORAND_SIGHT.as_str());
	}

	#[test]
	fn optional_barrel_sockets_when_set() {
		let kit = FirearmKit { barrel: BarrelMesh::Laznard, ..FirearmKit::body(BodyMesh::Silopup) };
		let bodies = kit.body_nodes_for_level(LodSceneLevel::High).flatten();
		let barrels = kit.barrel_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(bodies[0].slot, FirearmPartSlot::Body);
		assert_eq!(bodies[0].socket, Some(SocketRef::bone("body")));
		assert_eq!(barrels[0].slot, FirearmPartSlot::Barrel);
		assert_eq!(barrels[0].socket, Some(SocketRef::bone("barrel")));
		assert_eq!(barrels[0].scene.path, guns::LAZNARD_BARREL.as_str());
	}

	#[test]
	fn icon_pose_collapses_absent_stock_and_barrel() {
		let body_only = FirearmKit::body(BodyMesh::Snailer);
		let pose = body_only.icon_pose();
		assert_eq!(pose.barrel, BoneFit::COLLAPSED);
		assert_eq!(pose.stock, BoneFit::COLLAPSED);
		assert_eq!(pose.body, BoneFit::BIND);

		let with_barrel =
			FirearmKit { barrel: BarrelMesh::Bullpup, ..FirearmKit::body(BodyMesh::Silopup) };
		let posed = with_barrel.icon_pose();
		assert_eq!(posed.barrel, BoneFit::BIND);
		assert_eq!(posed.stock, BoneFit::COLLAPSED);
	}
}
