//! Muzzle flash, laser grow, impacts, and fire audio.
//!
//! [`crate::projectiles::fire_weapons`] still owns the mixed GPU fire path
//! (clock + spawn). Runtime registration does not add that system.

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use damage::DamageSystems;
use firearms_components::FirearmHostSystems;
use maybraid_audio::{AudioPlugin, AudioSystems};
use projectiles::ProjectileSystems;

use crate::impact::{setup_impact_effects, tick_impact_bursts};
use crate::muzzle_flame::{init_muzzle_flame_caches, MuzzleFlameMaterialPlugin};
use crate::projectiles::{
	fire_weapons, setup_muzzle_flash, spawn_impacts_from_contacts, tick_laser_hits, tick_lasers,
	tick_muzzle_flashes, FirearmWeaponSystems,
};
use crate::runtime::register_firearm_runtime;
use crate::sound::{copy_flight_audio_velocity, ensure_camera_spatial_listener, setup_fire_sounds};

#[derive(Resource, Default)]
struct FirearmPresentation;

/// GPU / audio systems. Requires [`register_firearm_runtime`].
pub fn register_firearm_presentation(app: &mut App) {
	if app.world().contains_resource::<FirearmPresentation>() {
		return;
	}
	register_firearm_runtime(app);
	if !app.is_plugin_added::<bevy_hanabi::HanabiPlugin>() {
		app.add_plugins(bevy_hanabi::HanabiPlugin);
	}
	init_muzzle_flame_caches(app);
	if !app.is_plugin_added::<MuzzleFlameMaterialPlugin>() {
		app.add_plugins(MuzzleFlameMaterialPlugin);
	}
	if !app.is_plugin_added::<AudioPlugin>() {
		app.add_plugins(AudioPlugin);
	}
	app.init_resource::<FirearmPresentation>()
		.add_systems(Startup, (setup_impact_effects, setup_muzzle_flash, setup_fire_sounds))
		.add_systems(Update, ensure_camera_spatial_listener)
		.add_systems(
			PostUpdate,
			(
				fire_weapons
					.in_set(FirearmWeaponSystems::Fire)
					.before(VisibilitySystems::VisibilityPropagate),
				tick_muzzle_flashes
					.after(FirearmWeaponSystems::Fire)
					.after(bevy_hanabi::EffectSystems::TickSpawners),
				tick_lasers.after(FirearmWeaponSystems::Fire),
				tick_laser_hits.in_set(DamageSystems::Collect).after(FirearmWeaponSystems::Fire),
				copy_flight_audio_velocity.after(FirearmWeaponSystems::Fire),
				spawn_impacts_from_contacts,
				tick_impact_bursts.after(bevy_hanabi::EffectSystems::TickSpawners),
			)
				.after(TransformSystems::Propagate)
				.after(FirearmHostSystems::Pose)
				.after(ProjectileSystems::Sweep)
				.before(AudioSystems::Sync),
		);
}
