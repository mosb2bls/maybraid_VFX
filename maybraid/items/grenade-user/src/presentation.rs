//! Held-mesh pose, cook HUD, throw jingle, and blast feel.

use bevy::prelude::*;
use characters::{CharacterMotionSystems, CharacterPoseOverlay, CharacterPoseSystems};
use maybraid_spatial_rumble::SpatialRumbleSystems;
use player::PlayerPoseSystems;

use crate::cook_hud;
use crate::feel;
use crate::hold::{self, HeldGrenade};
use crate::throw::{self, advance_throw};
use crate::HeldByGrenade;

#[derive(Resource, Default)]
struct GrenadeUserPresentation;

/// HUD / audio / mesh realization. Clocks stay on
/// [`crate::register_grenade_user_runtime`].
pub fn register_grenade_user_presentation(app: &mut App) {
	if app.world().contains_resource::<GrenadeUserPresentation>() {
		return;
	}
	app.init_resource::<GrenadeUserPresentation>()
		.add_systems(Startup, cook_hud::spawn_cook_fuse_hud)
		.add_systems(
			Update,
			(
				hold::stamp_holding_grenade,
				hold::pose_held_grenade,
				hold::apply_grenade_swap_pose,
				hold::sync_held_visibility,
			)
				.chain()
				.in_set(PlayerPoseSystems::Item),
		)
		.add_systems(
			Update,
			hold::sync_throw_arm
				.in_set(CharacterPoseOverlay::Grenade)
				.after(CharacterMotionSystems::Anim),
		)
		.add_systems(
			Update,
			throw::play_throw_jingles
				.after(CharacterPoseSystems::Overlay)
				.before(advance_throw),
		)
		.add_systems(Update, cook_hud::sync_cook_fuse_hud.after(advance_throw))
		.add_systems(
			Update,
			feel::pulse_blast_feel
				.after(grenades::apply_grenade_blasts)
				.before(SpatialRumbleSystems::Ingest),
		)
		.add_systems(
			Update,
			(hold::realize_held_grenade_visuals, despawn_orphaned_held_grenades).chain(),
		);
}

pub(crate) fn despawn_orphaned_held_grenades(
	mut commands: Commands,
	held: Query<Entity, (With<HeldGrenade>, Without<HeldByGrenade>)>,
) {
	for entity in &held {
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;
	use crate::GrenadeUser;

	#[test]
	fn orphaned_held_grenade_despawns_when_the_user_is_gone() {
		let mut world = World::new();
		let held = world.spawn(HeldGrenade { scale: 1.0 }).id();
		world.run_system_once(despawn_orphaned_held_grenades).expect("orphan");
		assert!(!world.entities().contains(held));
	}

	#[test]
	fn held_grenade_stays_while_linked() {
		let mut world = World::new();
		let held = world.spawn(HeldGrenade { scale: 1.0 }).id();
		world.spawn(GrenadeUser::holding(held));
		world.flush();
		world.run_system_once(despawn_orphaned_held_grenades).expect("linked");
		assert!(world.entities().contains(held));
	}
}
