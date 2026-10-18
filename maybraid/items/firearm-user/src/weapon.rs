//! Catalog [`FirearmStats`] → live [`Weapon`] / cadence / payload.

use bevy::prelude::*;
use character_items::{FireMode, FirearmMesh, FirearmSpec, FirearmStats, ProjectileKind};
use damage::{HitPayload, DEFAULT_HIT};
use firearms::{
	BoltSpec, BulletSpec, FireControl, LaserSpec, ProjectileLoad, Weapon, WeaponRecoil,
	IRON_SIGHT_FOV,
};
use std::hash::{Hash, Hasher};

/// Kick range (radians) per catalog recoil unit. Positive pitch is look up.
pub const RECOIL_PITCH_PER_UNIT: f32 = 0.02;

/// Minimum seconds between semi-auto rising edges.
pub const SEMI_INTERVAL_FLOOR: f32 = 1.0 / 20.0;

/// Catalog recoil for the default 25 DPC bolt (duel / unspec'd spawn).
const DEFAULT_CATALOG_RECOIL: f32 = 1.9;

/// Components stamped on a held [`firearms::FirearmRoot`] at spawn.
#[derive(Component, Clone, Copy, Debug)]
pub struct LiveWeapon {
	pub weapon: Weapon,
	pub payload: HitPayload,
	pub fire: FireControl,
	pub recoil: WeaponRecoil,
	/// Stable noise seed for [`RecoilPattern`]. Hashed from catalog stats, or
	/// from [`FirearmSpec`] via [`Self::with_weapon_identity`].
	pub recoil_seed: u64,
	/// ADS vertical FOV from the rolled optic.
	pub sight_fov: f32,
}

impl Default for LiveWeapon {
	fn default() -> Self {
		Self {
			weapon: Weapon::bolt(),
			payload: HitPayload { amount: DEFAULT_HIT },
			fire: FireControl::auto(),
			recoil: WeaponRecoil(DEFAULT_CATALOG_RECOIL * RECOIL_PITCH_PER_UNIT),
			recoil_seed: weapon_noise_seed(&FirearmSpec::from_mesh(FirearmMesh::Bullpup)),
			sight_fov: IRON_SIGHT_FOV,
		}
	}
}

impl LiveWeapon {
	/// Replace the recoil noise seed with a hash of `weapon` (typically [`FirearmSpec`]).
	pub fn with_weapon_identity<T: Hash>(mut self, weapon: &T) -> Self {
		self.recoil_seed = weapon_noise_seed(weapon);
		self
	}
}

/// Per-weapon strength-based kick sequence. `shot` advances on each recoiling fire.
/// `remaining` is the untraveled yaw/pitch path; `time_left` is the lerp window.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct RecoilPattern {
	pub seed: u64,
	pub shot: u32,
	pub remaining: Vec2,
	pub time_left: f32,
}

impl RecoilPattern {
	pub fn from_seed(seed: u64) -> Self {
		Self { seed, shot: 0, remaining: Vec2::ZERO, time_left: 0.0 }
	}
}

pub(crate) fn weapon_noise_seed<T: Hash>(weapon: &T) -> u64 {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	weapon.hash(&mut hasher);
	hasher.finish()
}

/// Bake integer catalog stats plus the wearer's outgoing DPC bonus.
pub fn live_weapon_from_stats(stats: FirearmStats, outgoing_damage_bonus: i16) -> LiveWeapon {
	let (fire, interval) = cadence_from_stats(stats);
	let load = load_from_stats(stats);
	let amount = (f32::from(stats.damage) + f32::from(outgoing_damage_bonus)).max(0.0);
	LiveWeapon {
		weapon: Weapon::new(load, interval),
		payload: HitPayload { amount },
		fire,
		recoil: WeaponRecoil(stats.recoil * RECOIL_PITCH_PER_UNIT),
		recoil_seed: weapon_noise_seed(&stats),
		sight_fov: stats.sight_fov,
	}
}

fn cadence_from_stats(stats: FirearmStats) -> (FireControl, f32) {
	match stats.fire {
		Some(FireMode::FullAuto { rpm }) => (FireControl::auto(), interval_from_rpm(rpm)),
		Some(FireMode::Burst { rounds, rpm }) => {
			(FireControl::burst(rounds), interval_from_rpm(rpm))
		}
		Some(FireMode::SemiAuto) => (FireControl::semi(), SEMI_INTERVAL_FLOOR),
		Some(FireMode::Gated { recharge_tenths }) => {
			(FireControl::gated(), f32::from(recharge_tenths) / 10.0)
		}
		None => match stats.projectile {
			ProjectileKind::Laser => (FireControl::auto(), 0.0),
			_ => (FireControl::auto(), Weapon::bolt().interval),
		},
	}
}

fn interval_from_rpm(rpm: u16) -> f32 {
	60.0 / f32::from(rpm.max(1))
}

fn load_from_stats(stats: FirearmStats) -> ProjectileLoad {
	let speed = f32::from(stats.speed.max(1));
	let range = f32::from(stats.range);
	let travel = (range / speed) + 0.25;
	match stats.projectile {
		ProjectileKind::Bolt => {
			let bolt = BoltSpec::default();
			ProjectileLoad::Bolt(BoltSpec {
				speed,
				max_range: range,
				penetration: f32::from(stats.penetration) / 1000.0,
				max_age: bolt.max_age.max(travel),
				..bolt
			})
		}
		ProjectileKind::Bullet => {
			let bullet = BulletSpec::default();
			ProjectileLoad::Bullet(BulletSpec {
				speed,
				max_range: range,
				penetration: f32::from(stats.penetration) / 1000.0,
				max_age: bullet.max_age.max(travel),
				..bullet
			})
		}
		ProjectileKind::Laser => ProjectileLoad::Laser(LaserSpec {
			max_length: range,
			max_time: travel.clamp(0.2, 2.0),
			..LaserSpec::default()
		}),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{FirearmMesh, FirearmSpec};
	use firearms::Cadence;

	fn bolt_auto() -> FirearmStats {
		FirearmStats {
			projectile: ProjectileKind::Bolt,
			speed: 180,
			penetration: 600,
			range: 40,
			fire: Some(FireMode::FullAuto { rpm: 600 }),
			recoil: 4.0,
			damage: 25,
			weight: 10,
			sight_fov: IRON_SIGHT_FOV,
		}
	}

	#[test]
	fn auto_rpm_becomes_interval() {
		let live = live_weapon_from_stats(bolt_auto(), 0);
		assert!((live.weapon.interval - 0.1).abs() < 1e-5);
		assert_eq!(live.fire.cadence, Cadence::Auto);
		assert_eq!(live.payload.amount, 25.0);
	}

	#[test]
	fn clothing_damage_adds_to_dpc() {
		let live = live_weapon_from_stats(bolt_auto(), 10);
		assert_eq!(live.payload.amount, 35.0);
	}

	#[test]
	fn penetration_millunits_become_through() {
		let ProjectileLoad::Bolt(spec) = live_weapon_from_stats(bolt_auto(), 0).weapon.load else {
			panic!("expected bolt");
		};
		assert!((spec.penetration - 0.6).abs() < 1e-5);
		assert_eq!(spec.speed, 180.0);
		assert_eq!(spec.max_range, 40.0);
	}

	#[test]
	fn gated_tenths_become_seconds() {
		let mut stats = bolt_auto();
		stats.fire = Some(FireMode::Gated { recharge_tenths: 12 });
		let live = live_weapon_from_stats(stats, 0);
		assert!((live.weapon.interval - 1.2).abs() < 1e-5);
		assert_eq!(live.fire.cadence, Cadence::Gated);
	}

	#[test]
	fn burst_keeps_round_count() {
		let mut stats = bolt_auto();
		stats.fire = Some(FireMode::Burst { rounds: 3, rpm: 900 });
		let live = live_weapon_from_stats(stats, 0);
		assert_eq!(live.fire.cadence, Cadence::Burst);
		assert_eq!(live.fire.burst_rounds, 3);
		assert!((live.weapon.interval - 60.0 / 900.0).abs() < 1e-5);
	}

	#[test]
	fn semi_uses_the_click_floor() {
		let mut stats = bolt_auto();
		stats.fire = Some(FireMode::SemiAuto);
		let live = live_weapon_from_stats(stats, 0);
		assert_eq!(live.fire.cadence, Cadence::Semi);
		assert!((live.weapon.interval - SEMI_INTERVAL_FLOOR).abs() < 1e-5);
	}

	#[test]
	fn semi_floor_rejects_a_second_shot_inside_the_window() {
		use firearms::advance_shot_clock;
		let mut stats = bolt_auto();
		stats.fire = Some(FireMode::SemiAuto);
		let live = live_weapon_from_stats(stats, 0);
		let mut cooldown = 0.0;
		let dt = 0.016;
		assert_eq!(advance_shot_clock(&mut cooldown, live.weapon.interval, dt, 1).shots, 1);
		let mut elapsed = dt;
		loop {
			let fired = advance_shot_clock(&mut cooldown, live.weapon.interval, dt, 1).shots;
			elapsed += dt;
			if fired > 0 {
				assert!(elapsed + 1e-4 >= SEMI_INTERVAL_FLOOR, "{elapsed}");
				return;
			}
			if elapsed > SEMI_INTERVAL_FLOOR + 0.1 {
				panic!("semi never rearmed");
			}
		}
	}

	#[test]
	fn laser_uses_range_as_length() {
		let stats = FirearmStats {
			projectile: ProjectileKind::Laser,
			speed: 90,
			penetration: 0,
			range: 80,
			fire: None,
			recoil: 0.0,
			damage: 18,
			weight: 8,
			sight_fov: IRON_SIGHT_FOV,
		};
		let live = live_weapon_from_stats(stats, 0);
		let ProjectileLoad::Laser(spec) = live.weapon.load else {
			panic!("expected laser");
		};
		assert_eq!(spec.max_length, 80.0);
		assert_eq!(live.payload.amount, 18.0);
	}

	#[test]
	fn recoil_becomes_kick_strength() {
		let live = live_weapon_from_stats(bolt_auto(), 0);
		assert!((live.recoil.0 - 0.08).abs() < 1e-5);
	}

	#[test]
	fn different_weapons_hash_to_different_seeds() {
		let mut other = bolt_auto();
		other.recoil = 7.0;
		assert_ne!(
			live_weapon_from_stats(bolt_auto(), 0).recoil_seed,
			live_weapon_from_stats(other, 0).recoil_seed
		);
	}

	#[test]
	fn spec_identity_overrides_stats_seed() {
		let spec = FirearmSpec::from_mesh(FirearmMesh::Snailer);
		let from_stats = live_weapon_from_stats(bolt_auto(), 0);
		let from_spec = from_stats.with_weapon_identity(&spec);
		assert_ne!(from_stats.recoil_seed, from_spec.recoil_seed);
		assert_eq!(from_spec.recoil_seed, weapon_noise_seed(&spec));
	}

	#[test]
	fn optic_fov_is_copied_onto_the_live_weapon() {
		let mut stats = bolt_auto();
		stats.sight_fov = 18.0_f32.to_radians();
		let live = live_weapon_from_stats(stats, 0);
		assert!((live.sight_fov - stats.sight_fov).abs() < 1e-5);
	}
}
