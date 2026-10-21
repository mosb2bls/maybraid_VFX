//! Thrown grenade body, fuse, blast hits, and detonation message.
//!
//! Execution lives in [`runtime`]; mesh / material / audio live in
//! [`presentation`].

mod material;
mod presentation;
mod runtime;
mod sound;

use bevy::prelude::{App, Plugin};

pub use material::{grenade_material, GrenadeMaterial, GrenadeMaterialPlugin};
pub use presentation::{grenade_mesh, register_grenade_presentation, spawn_thrown_grenade};
pub use runtime::{
	apply_grenade_blasts, blast_amount, grenade_layers, register_grenade_runtime,
	tick_grenade_fuses, DropLiveGrenade, GrenadeDetonated, GrenadeEffect, GrenadeFuse,
	ThrownGrenade, ThrownGrenadeSpawn, BLAST_FALLOFF_K,
};
pub use sound::{
	GrenadeSounds, BOUNCE_SPATIAL_RADIUS, CHARGE_IMPACT, CHARGE_RUMBLE, CHARGE_THROW,
	IMPACT_SPATIAL_RADIUS, RUMBLE_SPATIAL_RADIUS, THROW_SPATIAL_RADIUS,
};

pub struct GrenadesPlugin;

impl Plugin for GrenadesPlugin {
	fn build(&self, app: &mut App) {
		register_grenade_runtime(app);
		register_grenade_presentation(app);
	}
}
