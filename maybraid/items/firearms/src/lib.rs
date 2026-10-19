//! Maybraid firearm recipes: kit assembly over [`firearms_components`].
//!
//! [`FirearmKit`] is the character-recipe analogue: a required body plus optional
//! barrel / trigger-box / grip / stock / sight, socketed onto the matching bones.
//! [`FirearmConcept`] is a named preset of that kit.

pub mod cadence;
pub mod concepts;
pub mod energy;
pub mod impact;
pub mod kit;
pub mod muzzle_flame;
pub mod parts;
pub mod plugin;
pub mod pose;
pub mod projectiles;
pub mod sound;

pub use ::projectiles::{BoltSpec, BulletSpec, Flight, PenetrationCost, ProjectileSource};
pub use cadence::{
	advance_shot_clock, catch_up_limit, idle_shot_clock, Cadence, FireControl, ShotClockTick,
	WeaponFired, WeaponRecoil, CATCH_UP_SHOTS,
};
pub use concepts::FirearmConcept;
pub use energy::{
	init_energy_material_caches, is_energy_recipe, laser_hex_ref, laser_pulse_ref, laser_tail_ref,
	EnergyKind, EnergyMaterial, EnergyMaterialLib, EnergyMaterialPlugin, EnergyMaterialRefCache,
	KIND_HEX, KIND_PULSE, KIND_TAIL, LASER_HEX_RECIPE, LASER_PULSE_RECIPE, LASER_TAIL_RECIPE,
};
pub use firearms_components::{
	add_firearm_components_host, assembled_firearm_bounds, firearm_bounds, firearm_preview_camera,
	spawn_firearm_components, ActiveRigPose, AssetPath, BindPose, BoneMap, BoneScale,
	ComponentsOnly, FirearmComponents, FirearmComponentsPlugin, FirearmHostSystems, FirearmMembers,
	FirearmPartSlot, FirearmRoot, Layer, Layers, MemberOf, PartNode, ResolvedRigPose, RigNode,
	RigPoseLayer, RigRoot, SocketRef, SocketRefApplied, SocketRefRoot, RECEIVER_LANDMARKS,
};
pub use kit::FirearmKit;
pub use muzzle_flame::{
	init_muzzle_flame_caches, muzzle_flame_ref, MuzzleFlameMaterial, MuzzleFlameMaterialLib,
	MuzzleFlameMaterialPlugin, MuzzleFlameMaterialRefCache, MUZZLE_FLAME_RECIPE,
};
pub use parts::{
	fov_at_zoom, BarrelMesh, BodyMesh, GripMesh, KitBone, SightMesh, StockMesh, TriggerBoxMesh,
	IRON_SIGHT_FOV,
};
pub use plugin::FirearmHostsPlugin;
pub use pose::{aim_plus_x, BoneFit, FirearmPose};
pub use projectiles::{
	muzzle_world, FireOnTrigger, FirearmWeaponSystems, FirearmWeaponsPlugin, LaserBeam, LaserSpec,
	ProjectileLoad, Weapon, WeaponTrigger, WeaponsArmed, BARREL_REST_LENGTH,
};
pub use sound::{
	FirearmFireSounds, FlightFizz, FIRE_LISTENER_GAP, FIRE_SPATIAL_RADIUS, FIRE_SPATIAL_SCALE,
	FIRE_VOLUME, FIZZ_SPATIAL_RADIUS, FIZZ_SPATIAL_SCALE, FIZZ_VOLUME, HAMMER_SPATIAL_RADIUS,
	HAMMER_SPATIAL_SCALE, HAMMER_VOLUME, IMPACT_SPATIAL_RADIUS, IMPACT_SPATIAL_SCALE,
	IMPACT_VOLUME, LASER_MUZZLE_LOCAL, WEAPON_FIRE, WEAPON_FIZZ, WEAPON_HAMMER, WEAPON_IMPACT,
};

/// Config → inner [`FirearmComponents`] recipe.
///
/// Mirrors character recipes without clothing. Concepts expand to a [`FirearmKit`].
pub trait FirearmRecipe {
	type Components: FirearmComponents + Clone + Default + Send + Sync + 'static;

	fn components(&self) -> Self::Components;
}

impl FirearmRecipe for FirearmKit {
	type Components = Self;

	fn components(&self) -> Self::Components {
		*self
	}
}

impl FirearmRecipe for FirearmConcept {
	type Components = FirearmKit;

	fn components(&self) -> Self::Components {
		self.kit()
	}
}
