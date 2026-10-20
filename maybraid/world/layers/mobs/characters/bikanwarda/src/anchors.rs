//! Chain and attack socket frames in assembly-local space (+Y up, +Z forward).

use std::collections::BTreeMap;

use bevy::prelude::*;
use character_animations::animations::MOUTH_EMIT_SOCKET;
use characters::components::LocomotionCapsule;

pub mod names {
	pub const CHAIN_ANCHOR_POS_X: &str = "chain_anchor_pos_x";
	pub const CHAIN_ANCHOR_NEG_X: &str = "chain_anchor_neg_x";
	pub const CHAIN_ANCHOR_POS_Z: &str = "chain_anchor_pos_z";
	pub const CHAIN_ANCHOR_NEG_Z: &str = "chain_anchor_neg_z";
	pub const MOUTH_EMIT: &str = "mouth_emit";
}

pub const CHAIN_ANCHOR_NAMES: [&str; 4] = [
	names::CHAIN_ANCHOR_POS_X,
	names::CHAIN_ANCHOR_NEG_X,
	names::CHAIN_ANCHOR_POS_Z,
	names::CHAIN_ANCHOR_NEG_Z,
];

/// Sockets referenced by gameplay (chain + bray emission).
pub const CHAIN_SOCKET_NAMES: [&str; 5] = [
	names::CHAIN_ANCHOR_POS_X,
	names::CHAIN_ANCHOR_NEG_X,
	names::CHAIN_ANCHOR_POS_Z,
	names::CHAIN_ANCHOR_NEG_Z,
	MOUTH_EMIT_SOCKET,
];

const CHAIN_OUTSET: f32 = 0.08;

/// Marker on chain anchor child entities.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BikanwardaChainAnchor {
	pub name: &'static str,
}

/// Named anchors for one assembly (motor-local, pre-root-scale).
#[derive(Debug, Clone, PartialEq)]
pub struct BikanwardaAnchors {
	pub transforms: BTreeMap<&'static str, Transform>,
}

impl BikanwardaAnchors {
	pub fn for_capsule(capsule: LocomotionCapsule) -> Self {
		let hit = capsule
			.hit_capsule()
			.expect("Bikanwarda uses the Epiphant quadruped motor hull");
		let mut transforms = BTreeMap::new();
		let chain_y = capsule.radius * 0.65;
		let chain_r = hit.radius + CHAIN_OUTSET;
		transforms.insert(names::CHAIN_ANCHOR_POS_X, chain_anchor(chain_r, chain_y, Vec3::X));
		transforms.insert(names::CHAIN_ANCHOR_NEG_X, chain_anchor(chain_r, chain_y, -Vec3::X));
		transforms.insert(names::CHAIN_ANCHOR_POS_Z, chain_anchor(chain_r, chain_y, Vec3::Z));
		transforms.insert(names::CHAIN_ANCHOR_NEG_Z, chain_anchor(chain_r, chain_y, -Vec3::Z));
		// Bray lane follows the assembled nose socket; record a nominal point for tests.
		transforms.insert(
			names::MOUTH_EMIT,
			Transform::from_translation(Vec3::new(0.0, chain_y + capsule.radius, hit.along + hit.length * 0.35)),
		);
		Self { transforms }
	}

	pub fn get(&self, name: &str) -> Option<&Transform> {
		self.transforms.get(name)
	}
}

fn chain_anchor(radius: f32, y: f32, outward: Vec3) -> Transform {
	let pos = outward.normalize() * radius + Vec3::new(0.0, y, 0.0);
	let rotation = Quat::from_rotation_arc(Vec3::Y, outward.normalize_or_zero());
	Transform::from_translation(pos).with_rotation(rotation)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BikanwardaActorRecipe;
	use characters::{CharacterComponents, CharacterRecipe};

	#[test]
	fn chain_anchors_scale_with_encounter_capsule() {
		let actor = BikanwardaActorRecipe::default().components();
		let ordinary = actor.locomotion_capsule();
		let encounter = actor.scaled_locomotion_capsule();
		let a = BikanwardaAnchors::for_capsule(ordinary);
		let b = BikanwardaAnchors::for_capsule(encounter);
		let small_x = a.get(names::CHAIN_ANCHOR_POS_X).unwrap().translation.x;
		let large_x = b.get(names::CHAIN_ANCHOR_POS_X).unwrap().translation.x;
		assert!(large_x > small_x * 9.0);
	}

	#[test]
	fn mouth_emit_socket_name_matches_quadruped_bray() {
		assert_eq!(MOUTH_EMIT_SOCKET, "nose_socket");
	}
}
