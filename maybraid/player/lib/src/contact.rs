//! Motor-driven capsules own traction; solver contacts only block penetration.
//!
//! Floor materials keep high grip for ragdolls, props, and other dynamics.
//! [`CoefficientCombine::Max`] on terrain / buildings beats the capsule's
//! `Friction::ZERO` + `Min`, so this hook zeros friction on motor contacts
//! instead of relying on the capsule material alone. Register
//! [`register_motor_traction_physics`] (or `PhysicsPlugins` with
//! [`MotorTractionHooks`]) before any other Avian plugin, and stamp
//! [`MotorTraction`] plus [`ActiveCollisionHooks::MODIFY_CONTACTS`].

use avian3d::prelude::{
	ActiveCollisionHooks, CollisionHooks, ContactPair, PhysicsPlugins, PhysicsSchedulePlugin,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Marker: this rigid body writes walk velocity; contact friction must not.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct MotorTraction;

/// Avian plugins with [`MotorTractionHooks`]. No-op if physics is already added.
///
/// Hooks cannot be attached after [`PhysicsPlugins`], so the first physics
/// registration in an app must go through here (or the same
/// `with_collision_hooks` call).
pub fn register_motor_traction_physics(app: &mut App) {
	if app.is_plugin_added::<PhysicsSchedulePlugin>() {
		return;
	}
	app.add_plugins(PhysicsPlugins::default().with_collision_hooks::<MotorTractionHooks>());
}

/// Bundle stamped onto motor capsules with the dynamic character controller.
pub fn motor_traction_bundle() -> impl Bundle {
	(MotorTraction, ActiveCollisionHooks::MODIFY_CONTACTS)
}

/// Zero friction on contacts that involve a [`MotorTraction`] body or collider.
#[derive(SystemParam)]
pub struct MotorTractionHooks<'w, 's> {
	motors: Query<'w, 's, (), With<MotorTraction>>,
}

impl CollisionHooks for MotorTractionHooks<'_, '_> {
	fn modify_contacts(&self, contacts: &mut ContactPair, _commands: &mut Commands) -> bool {
		if motor_contact(&self.motors, contacts) {
			for manifold in &mut contacts.manifolds {
				manifold.friction = 0.0;
			}
		}
		true
	}
}

fn motor_contact(motors: &Query<(), With<MotorTraction>>, contacts: &ContactPair) -> bool {
	motors.contains(contacts.collider1)
		|| motors.contains(contacts.collider2)
		|| contacts.body1.is_some_and(|body| motors.contains(body))
		|| contacts.body2.is_some_and(|body| motors.contains(body))
}
