//! Pad rumble for the followed player's fire and landed hits.

use std::time::Duration;

use bevy::prelude::*;
use damage::{DamageApplied, HitPayload, DEFAULT_HIT};
use firearms::{ProjectileLoad, Weapon, WeaponFired};
use maybraid_input::PadRumble;
use player::CameraFollow;

use crate::FirearmUser;

/// Catalog speed span used to invert duration. Faster shot → shorter pulse.
const SPEED_SLOW: f32 = 80.0;
const SPEED_FAST: f32 = 400.0;
const FIRE_MS_SLOW: f32 = 180.0;
const FIRE_MS_FAST: f32 = 70.0;
const HIT_MS_SLOW: f32 = 280.0;
const HIT_MS_FAST: f32 = 110.0;
/// 25 DPC is weight 1. Headshots and heavy rolls go above.
const DAMAGE_REF: f32 = DEFAULT_HIT;
const FIRE_WEAK: f32 = 0.32;
const FIRE_STRONG: f32 = 0.28;
const HIT_WEAK: f32 = 0.5;
const HIT_STRONG: f32 = 0.72;
/// Laser tick is ~150 ms; stay under that so the beam pulses instead of drones.
const LASER_MS: u64 = 90;
const LASER_WEAK: f32 = 0.22;
const LASER_STRONG: f32 = 0.16;

pub(crate) fn laser_rumble() -> PadRumble {
	PadRumble::motors(Duration::from_millis(LASER_MS), LASER_WEAK, LASER_STRONG)
}

pub(crate) fn fire_rumble(speed: f32, damage: f32) -> PadRumble {
	ballistic_rumble(speed, damage, FIRE_MS_SLOW, FIRE_MS_FAST, FIRE_WEAK, FIRE_STRONG)
}

pub(crate) fn hit_rumble(speed: f32, damage: f32) -> PadRumble {
	ballistic_rumble(speed, damage, HIT_MS_SLOW, HIT_MS_FAST, HIT_WEAK, HIT_STRONG)
}

fn ballistic_rumble(
	speed: f32,
	damage: f32,
	ms_slow: f32,
	ms_fast: f32,
	weak: f32,
	strong: f32,
) -> PadRumble {
	let weight = rumble_weight(damage);
	PadRumble::motors(
		Duration::from_millis(duration_ms(speed, ms_slow, ms_fast)),
		weak * weight,
		strong * weight,
	)
}

fn duration_ms(speed: f32, slow_ms: f32, fast_ms: f32) -> u64 {
	let t = ((speed - SPEED_SLOW) / (SPEED_FAST - SPEED_SLOW)).clamp(0.0, 1.0);
	(slow_ms + (fast_ms - slow_ms) * t).round() as u64
}

fn rumble_weight(damage: f32) -> f32 {
	(damage / DAMAGE_REF).clamp(0.35, 1.8)
}

fn projectile_speed(weapon: &Weapon) -> f32 {
	match weapon.load {
		ProjectileLoad::Bolt(spec) => spec.speed,
		ProjectileLoad::Bullet(spec) => spec.speed,
		ProjectileLoad::Laser(_) => 0.0,
	}
}

fn is_laser(weapon: &Weapon) -> bool {
	matches!(weapon.load, ProjectileLoad::Laser(_))
}

fn fire_delay(dt: f32, age: f32) -> Duration {
	Duration::from_secs_f32((dt - age).max(0.0))
}

pub(crate) fn pulse_combat_rumble(
	time: Res<Time>,
	mut fired: MessageReader<WeaponFired>,
	mut hits: MessageReader<DamageApplied>,
	followed: Query<(Entity, &FirearmUser), With<CameraFollow>>,
	guns: Query<(&Weapon, Option<&HitPayload>)>,
	mut rumble: MessageWriter<PadRumble>,
) {
	let dt = time.delta_secs();
	let shots: Vec<WeaponFired> = fired.read().copied().collect();
	let applied: Vec<DamageApplied> = hits.read().copied().collect();
	if shots.is_empty() && applied.is_empty() {
		return;
	}
	for (player, user) in &followed {
		let gun = guns.get(user.held).ok();
		let laser = gun.is_some_and(|(weapon, _)| is_laser(weapon));
		let speed = gun.map(|(weapon, _)| projectile_speed(weapon)).unwrap_or(SPEED_SLOW);
		let payload = gun
			.and_then(|(_, payload)| payload.map(|payload| payload.amount))
			.unwrap_or(DEFAULT_HIT);
		for event in &shots {
			if event.shooter != player {
				continue;
			}
			let pulse = if laser { laser_rumble() } else { fire_rumble(speed, payload) };
			debug!(
				"pad_rumble: fire laser={laser} speed={speed:.0} damage={payload:.1} age={:.4}",
				event.age
			);
			rumble.write(pulse.after(fire_delay(dt, event.age)));
		}
		if laser {
			continue;
		}
		for hit in &applied {
			if hit.source != Some(player) {
				continue;
			}
			debug!("pad_rumble: hit speed={speed:.0} damage={:.1}", hit.amount);
			rumble.write(hit_rumble(speed, hit.amount));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use firearms::Weapon;

	#[test]
	fn faster_shot_is_shorter() {
		let slow = fire_rumble(SPEED_SLOW, DEFAULT_HIT);
		let fast = fire_rumble(SPEED_FAST, DEFAULT_HIT);
		assert!(fast.duration < slow.duration);
		assert_eq!(slow.duration, Duration::from_millis(FIRE_MS_SLOW as u64));
		assert_eq!(fast.duration, Duration::from_millis(FIRE_MS_FAST as u64));
	}

	#[test]
	fn more_damage_is_heavier() {
		let light = fire_rumble(180.0, 10.0);
		let heavy = fire_rumble(180.0, 40.0);
		assert!(heavy.intensity.strong_motor > light.intensity.strong_motor);
		assert_eq!(light.duration, heavy.duration);
	}

	#[test]
	fn hit_is_heavier_than_fire_at_the_same_stats() {
		let fire = fire_rumble(180.0, DEFAULT_HIT);
		let hit = hit_rumble(180.0, DEFAULT_HIT);
		assert!(hit.duration > fire.duration);
		assert!(hit.intensity.strong_motor > fire.intensity.strong_motor);
	}

	#[test]
	fn laser_pulse_is_constant() {
		assert_eq!(laser_rumble().duration, Duration::from_millis(LASER_MS));
		assert!((laser_rumble().intensity.weak_motor - LASER_WEAK).abs() < 1e-5);
	}

	fn collect_pad_rumble(app: &mut App) -> Vec<PadRumble> {
		let messages = app.world().resource::<Messages<PadRumble>>();
		let mut cursor = messages.get_cursor();
		cursor.read(messages).copied().collect()
	}

	fn combat_app(weapon: Weapon, damage: f32) -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<WeaponFired>()
			.add_message::<DamageApplied>()
			.add_message::<PadRumble>()
			.add_systems(Update, pulse_combat_rumble);
		let gun = app.world_mut().spawn((weapon, HitPayload { amount: damage })).id();
		let player = app.world_mut().spawn((CameraFollow, FirearmUser::holding(gun))).id();
		(app, player)
	}

	#[test]
	fn followed_player_fire_and_hit_rumble() {
		let (mut app, player) = combat_app(Weapon::bolt(), 25.0);
		let target = app.world_mut().spawn_empty().id();
		app.world_mut().write_message(WeaponFired::new(player, 0.04));
		app.world_mut().write_message(DamageApplied {
			target,
			source: Some(player),
			amount: 25.0,
			applied: 25.0,
			remaining: 75.0,
			point: Vec3::ZERO,
		});
		app.update();
		let pulses = collect_pad_rumble(&mut app);
		assert_eq!(pulses.len(), 2);
		assert_eq!(pulses[0].duration, fire_rumble(180.0, 25.0).duration);
		assert_eq!(pulses[0].intensity, fire_rumble(180.0, 25.0).intensity);
		assert_eq!(pulses[1], hit_rumble(180.0, 25.0));
	}

	#[test]
	fn laser_stays_one_low_pulse_even_on_hit() {
		let (mut app, player) = combat_app(Weapon::laser(), 40.0);
		let target = app.world_mut().spawn_empty().id();
		app.world_mut().write_message(WeaponFired::new(player, 0.0));
		app.world_mut().write_message(DamageApplied {
			target,
			source: Some(player),
			amount: 40.0,
			applied: 40.0,
			remaining: 60.0,
			point: Vec3::ZERO,
		});
		app.update();
		let pulses = collect_pad_rumble(&mut app);
		assert_eq!(pulses, vec![laser_rumble()]);
	}

	#[test]
	fn npc_shots_do_not_rumble() {
		let (mut app, player) = combat_app(Weapon::bolt(), 25.0);
		let npc = app.world_mut().spawn_empty().id();
		app.world_mut().write_message(WeaponFired::new(npc, 0.08));
		app.world_mut().write_message(DamageApplied {
			target: player,
			source: Some(npc),
			amount: 25.0,
			applied: 25.0,
			remaining: 75.0,
			point: Vec3::ZERO,
		});
		app.update();
		assert!(collect_pad_rumble(&mut app).is_empty());
	}

	#[test]
	fn each_shot_gets_its_own_delayed_pulse() {
		let (mut app, player) = combat_app(Weapon::bolt(), 25.0);
		app.world_mut()
			.write_message(WeaponFired { shooter: player, recoil: 0.04, age: 0.012 });
		app.world_mut()
			.write_message(WeaponFired { shooter: player, recoil: 0.04, age: 0.006 });
		app.world_mut()
			.write_message(WeaponFired { shooter: player, recoil: 0.04, age: 0.0 });
		app.update();
		let pulses = collect_pad_rumble(&mut app);
		assert_eq!(pulses.len(), 3);
		for pulse in &pulses {
			assert_eq!(pulse.duration, fire_rumble(180.0, 25.0).duration);
			assert_eq!(pulse.intensity, fire_rumble(180.0, 25.0).intensity);
		}
		assert!(pulses[0].delay <= pulses[1].delay);
		assert!(pulses[1].delay <= pulses[2].delay);
	}

	#[test]
	fn a_second_followed_user_does_not_silence_rumble() {
		let (mut app, player) = combat_app(Weapon::bolt(), 25.0);
		let extra_gun = app.world_mut().spawn(Weapon::bolt()).id();
		app.world_mut().spawn((CameraFollow, FirearmUser::holding(extra_gun)));
		app.world_mut().write_message(WeaponFired::new(player, 0.04));
		app.update();
		let pulses = collect_pad_rumble(&mut app);
		assert_eq!(pulses.len(), 1);
		assert_eq!(pulses[0].duration, fire_rumble(180.0, 25.0).duration);
	}

	#[test]
	fn fire_delay_is_the_rest_of_the_frame() {
		assert_eq!(fire_delay(0.016, 0.016), Duration::ZERO);
		assert_eq!(fire_delay(0.016, 0.0), Duration::from_secs_f32(0.016));
		assert_eq!(fire_delay(0.016, 0.1), Duration::ZERO);
	}
}
