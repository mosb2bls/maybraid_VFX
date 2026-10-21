//! Capsule player, visual, and handoff slots for camera / pose drivers.

mod assign;
mod body;
mod buoyancy;
mod contact;
mod flinch;
mod hit;
mod identity;
mod locomotion;
mod look;
mod motor;
mod movement;
mod separation;
mod spawn;
mod stance;

use bevy::prelude::*;
use characters::{CharacterMotionSystems, CharacterPoseSystems};
use maybraid_character_controller::CharacterControlSystems;
use maybraid_controllers_core::ControllerPipeline;

pub use body::{
	apply_character_controller, apply_character_mobility, apply_locomotion_capsule,
	ground_plane_for_wish, tick_jump, walkable_contact_normal, wish_on_ground, CharacterController,
	CharacterLocomotion, Grounded, JumpPhase, JumpWish, Jumping, MoveSpeedCap, MoveWish,
	PlayerControlSystems, Sprinting, WalkableGround,
};
pub use buoyancy::{Buoyant, Wading, WaterRegime};
pub use characters::{HeadCapsule, HitCapsule};
pub use contact::{
	motor_traction_bundle, register_motor_traction_physics, MotorTraction, MotorTractionHooks,
};
pub use hit::HitVolume;
pub use identity::{
	CameraFollow, Npc, Player, PlayerCameraAim, PlayerCameraPose, PlayerCapsule, PlayerLook,
	PlayerUse, PlayerVisual, PlayerYawOwner,
};
pub use locomotion::drive_player_locomotion;
pub use look::{clamp_aim_pitch, look_delta, look_forward, wrap_pi, yaw_xz, AIM_PITCH_MARGIN};
pub use motor::{
	apply_motor, compose_motor, MotorContributor, MotorIntent, MotorModifier, MotorRecovery,
	MotorRequestStage, MotorSample, MotorSelection, MotorSuppressionToken, ResolvedMotorRequest,
	SpeedCapRequest, SprintRequest, StanceRequest,
};
pub use separation::{SoftBump, NPC_SEPARATION};
pub use spawn::{
	capsule_spawn_height, needs_npc_visual, needs_player_visual, spawn_npc, spawn_npc_visual,
	spawn_npc_with_hidden_capsule, spawn_npc_with_hull, spawn_player, spawn_player_visual,
	spawn_player_with_hidden_capsule, spawn_player_with_hull, LocomotionCapsule, CAPSULE_LENGTH,
	CAPSULE_RADIUS,
};
pub use stance::{CharacterStance, RestLocomotionCapsule, StanceKind};

/// Capsule physics and move/jump intents.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerSystems {
	Intent,
	Body,
	Locomotion,
}

/// Handoff for item-user crates. `Item` poses held meshes; `Overlay` runs after clips.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerPoseSystems {
	Item,
	Overlay,
}

/// Locomotion-independent item presentation schedule.
///
/// Applications with their own capsule controller can install this without
/// adding [`PlayerPlugin`].
pub struct PlayerPresentationPlugin;

impl Plugin for PlayerPresentationPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				PlayerPoseSystems::Item.after(CharacterMotionSystems::Elevation),
				CharacterPoseSystems::Overlay.after(PlayerPoseSystems::Item),
				PlayerPoseSystems::Overlay
					.after(PlayerPoseSystems::Item)
					.in_set(CharacterPoseSystems::Overlay),
			),
		);
	}
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PlayerPresentationPlugin>() {
			app.add_plugins(PlayerPresentationPlugin);
		}
		if !app.is_plugin_added::<maybraid_controllers_core::ControllersCorePlugin>() {
			app.add_plugins(maybraid_controllers_core::ControllersCorePlugin);
		}
		MotorRequestStage::configure(app);
		app.init_resource::<CharacterLocomotion>()
			.init_resource::<maybraid_controllers_core::InputRouter>()
			.add_message::<maybraid_controllers_core::RoutingEvent>()
			.add_systems(Update, assign::assign_local_player.before(CharacterControlSystems))
			.configure_sets(
				Update,
				(
					PlayerSystems::Intent
						.after(CharacterControlSystems)
						.before(ControllerPipeline::Motor),
					PlayerSystems::Body
						.after(PlayerSystems::Intent)
						.in_set(ControllerPipeline::Motor)
						.in_set(PlayerControlSystems),
					PlayerSystems::Locomotion
						.after(PlayerSystems::Body)
						.before(CharacterMotionSystems::Anim),
				),
			)
			.add_systems(
				Update,
				(motor::ensure_motor_intents, motor::reset_motor_intents)
					.chain()
					.in_set(ControllerPipeline::Interpret),
			)
			.add_systems(
				Update,
				separation::declare_npc_soft_bump
					.in_set(ControllerPipeline::MotorRequest)
					.after(MotorRequestStage::Standard),
			)
			.add_systems(Update, (compose_motor, apply_motor).chain().in_set(PlayerSystems::Body))
			.add_systems(
				Update,
				(
					body::update_grounded,
					buoyancy::apply_buoyancy,
					body::apply_wish_movement,
					body::apply_wish_jump,
					stance::stand_when_airborne,
					stance::advance_stance_blend,
					stance::apply_stance_hulls,
					stance::sync_stance_pitch,
					body::advance_jump_phases,
				)
					.chain()
					.in_set(PlayerSystems::Body)
					.after(apply_motor),
			)
			.add_systems(
				PostUpdate,
				(
					body::sync_character_locomotion,
					hit::maintain_hit_volumes,
					hit::align_hit_volumes,
				)
					.chain(),
			)
			.add_systems(
				Update,
				(locomotion::face_wish_yaw, drive_player_locomotion, hit::align_hit_volumes)
					.chain()
					.in_set(PlayerSystems::Locomotion),
			);
		flinch::configure_flinch(app);
		movement::configure_movement(app);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Resource, Default)]
	struct PresentationOrder(Vec<&'static str>);

	fn note_elevation(mut order: ResMut<PresentationOrder>) {
		order.0.push("elevation");
	}

	fn note_item(mut order: ResMut<PresentationOrder>) {
		order.0.push("item");
	}

	fn note_overlay(mut order: ResMut<PresentationOrder>) {
		order.0.push("overlay");
	}

	#[test]
	fn presentation_runs_after_elevation_in_order() {
		let mut app = App::new();
		app.init_resource::<PresentationOrder>()
			.add_plugins(PlayerPresentationPlugin)
			.add_systems(
				Update,
				(
					note_elevation.in_set(CharacterMotionSystems::Elevation),
					note_item.in_set(PlayerPoseSystems::Item),
					note_overlay.in_set(PlayerPoseSystems::Overlay),
				),
			);
		app.update();
		assert_eq!(app.world().resource::<PresentationOrder>().0, ["elevation", "item", "overlay"]);
	}

	#[test]
	fn pose_overlay_stays_after_item_and_elevation() {
		let mut app = App::new();
		app.init_resource::<PresentationOrder>();
		CharacterPoseSystems::configure(&mut app);
		app.add_plugins(PlayerPresentationPlugin).add_systems(
			Update,
			(
				note_elevation.in_set(CharacterMotionSystems::Elevation),
				note_item.in_set(PlayerPoseSystems::Item),
				note_overlay.in_set(CharacterPoseSystems::Overlay),
			),
		);
		app.update();
		assert_eq!(app.world().resource::<PresentationOrder>().0, ["elevation", "item", "overlay"]);
	}
}
