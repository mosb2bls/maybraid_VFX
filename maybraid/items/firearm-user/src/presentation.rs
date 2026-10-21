//! Held-mesh pose, sight aim, reticle, and combat rumble.

use bevy::prelude::*;
use characters::{CharacterMotionSystems, CharacterPoseOverlay};
use damage::DamageSystems;
use firearms::FirearmWeaponSystems;
use maybraid_audio::AudioSystems;
use maybraid_input::PadRumbleSystems;
use player::PlayerPoseSystems;
use player_camera::PlayerCameraSystems;

use crate::aim;
use crate::hold;
use crate::pose::{self, HeldFirearm};
use crate::reticle;
use crate::rumble;
use crate::swap;
use crate::HeldBy;

#[derive(Resource, Default)]
struct FirearmUserPresentation;

/// Pose / reticle / audio. Clocks stay on [`crate::register_firearm_user_runtime`].
pub fn register_firearm_user_presentation(app: &mut App) {
	if app.world().contains_resource::<FirearmUserPresentation>() {
		return;
	}
	app.init_resource::<FirearmUserPresentation>()
		.add_systems(
			Update,
			(pose::stamp_holding_arms, pose::pose_held_firearm, swap::apply_weapon_swap_pose)
				.chain()
				.in_set(PlayerPoseSystems::Item),
		)
		.add_systems(
			PostUpdate,
			swap::play_change_item
				.after(TransformSystems::Propagate)
				.before(AudioSystems::Sync),
		)
		.add_systems(Update, aim::write_sight_aim.in_set(PlayerCameraSystems::Aim))
		.add_systems(
			Update,
			hold::sync_hands_to_firearm
				.in_set(CharacterPoseOverlay::Firearm)
				.after(CharacterMotionSystems::Anim),
		)
		.add_systems(
			PostUpdate,
			(reticle::ingest_hit_markers, reticle::update_reticle)
				.chain()
				.after(TransformSystems::Propagate)
				.after(DamageSystems::Apply),
		)
		.add_systems(
			PostUpdate,
			rumble::pulse_combat_rumble
				.after(FirearmWeaponSystems::Fire)
				.after(DamageSystems::Apply)
				.before(PadRumbleSystems::FanOut),
		)
		.add_systems(Update, despawn_orphaned_held_firearms);
}

/// Held kits are world-posed, not parented. When the user is culled or
/// despawned without going through a drop path, [`HeldBy`] leaves and the gun
/// would otherwise float.
fn despawn_orphaned_held_firearms(
	mut commands: Commands,
	guns: Query<Entity, (With<HeldFirearm>, Without<HeldBy>)>,
) {
	for entity in &guns {
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod orphan_tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;

	use super::{despawn_orphaned_held_firearms, HeldFirearm};
	use crate::FirearmUser;

	#[test]
	fn orphaned_held_kit_despawns_when_the_user_is_gone() {
		let mut world = World::new();
		let gun = world.spawn(HeldFirearm { scale: 1.0 }).id();
		world.run_system_once(despawn_orphaned_held_firearms).expect("orphan");
		assert!(!world.entities().contains(gun));
	}

	#[test]
	fn held_kit_stays_while_linked() {
		let mut world = World::new();
		let gun = world.spawn(HeldFirearm { scale: 1.0 }).id();
		world.spawn(FirearmUser::holding(gun));
		world.flush();
		world.run_system_once(despawn_orphaned_held_firearms).expect("linked");
		assert!(world.entities().contains(gun));
	}
}
