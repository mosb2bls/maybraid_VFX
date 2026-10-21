//! Firearm execution contract: armed flag, cadence, and the fire clock.
//!
//! Shot spawn / muzzle / impact stay in [`crate::presentation`]. Headless
//! tests and production run [`crate::projectiles::decide_weapon_fire`].

use bevy::prelude::*;

use crate::cadence::{WeaponFired, WeaponShotRequest};
use crate::projectiles::{FirearmWeaponSystems, WeaponsArmed, decide_weapon_fire};

#[derive(Resource, Default)]
struct FirearmRuntime;

/// Messages, armed flag, and the authoritative fire clock.
/// Does not spawn meshes or register `fire_weapons`.
pub fn register_firearm_runtime(app: &mut App) {
	if app.world().contains_resource::<FirearmRuntime>() {
		return;
	}
	app.init_resource::<FirearmRuntime>()
		.init_resource::<WeaponsArmed>()
		.add_message::<WeaponFired>()
		.add_message::<WeaponShotRequest>()
		.add_systems(PostUpdate, decide_weapon_fire.in_set(FirearmWeaponSystems::Fire));
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::WeaponRecoil;
	use crate::cadence::{FireControl, WeaponShotRequest};
	use crate::projectiles::{FireOnTrigger, Weapon, WeaponTrigger};
	use bevy::ecs::system::RunSystemOnce;

	fn tick_fire(app: &mut App) {
		app.world_mut()
			.resource_mut::<Time>()
			.advance_by(std::time::Duration::from_secs_f32(1.0 / 30.0));
		app.world_mut().run_system_once(decide_weapon_fire).expect("decide weapon fire");
	}

	#[test]
	fn decide_weapon_fire_emits_one_shot_request() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_firearm_runtime(&mut app);
		let gun = app
			.world_mut()
			.spawn((
				Weapon::bolt(),
				WeaponTrigger(true),
				FireOnTrigger,
				FireControl::auto(),
				WeaponRecoil(0.08),
			))
			.id();
		tick_fire(&mut app);
		let shots: Vec<WeaponShotRequest> =
			app.world_mut().resource_mut::<Messages<WeaponShotRequest>>().drain().collect();
		assert_eq!(shots.len(), 1);
		assert_eq!(shots[0].weapon, gun);
		assert!((shots[0].recoil - 0.08).abs() < 1e-5);
		let fired: Vec<WeaponFired> =
			app.world_mut().resource_mut::<Messages<WeaponFired>>().drain().collect();
		assert_eq!(fired.len(), 1);
		assert_eq!(fired[0].shooter, gun);
	}

	#[test]
	fn register_firearm_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_firearm_runtime(&mut app);
		register_firearm_runtime(&mut app);
		app.world_mut().spawn((Weapon::bolt(), WeaponTrigger(true), FireOnTrigger));
		tick_fire(&mut app);
		assert_eq!(
			app.world_mut().resource_mut::<Messages<WeaponShotRequest>>().drain().count(),
			1
		);
	}
}
