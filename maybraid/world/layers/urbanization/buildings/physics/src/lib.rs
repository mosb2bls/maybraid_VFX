//! Fixed-layer walk colliders from Richmond domain IR.
//!
//! LOD hosts are indexed by Gimme, not Avian Host volumes. Walk geometry is
//! spawned as child [`RigidBody::Static`] colliders on
//! [`lod_avian::PhysicsInteractionLayer::Fixed`] so movers can stand on floors,
//! triangle landings, and stair ramps without treating LOD host AABBs as contact.

mod colliders;

use avian3d::prelude::{CoefficientCombine, Friction};
use bevy::prelude::*;

pub use colliders::{spawn_building_walk_colliders, BuildingWalkCollider, BuildingWalkShapes};

/// Dirt / stone grip. [`CoefficientCombine::Max`] beats the capsule's
/// `Friction::ZERO` + `Min`. Motor-driven capsules zero contact friction
/// separately; this material still holds ragdolls and props.
pub const BUILDING_FRICTION: Friction = Friction {
	dynamic_coefficient: 0.75,
	static_coefficient: 0.95,
	combine_rule: CoefficientCombine::Max,
};

/// Friction applied to new building walk cuboids.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct BuildingFrictionConfig(pub Friction);

impl Default for BuildingFrictionConfig {
	fn default() -> Self {
		Self(BUILDING_FRICTION)
	}
}

/// Registers [`BuildingFrictionConfig`] and attaches one Fixed compound per host.
pub struct BuildingWalkColliderPlugin;

impl Plugin for BuildingWalkColliderPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BuildingFrictionConfig>()
			.add_systems(Update, colliders::attach_building_walk_colliders);
	}
}
