//! Starter Tanz definitions. Effects compose existing movement / health APIs.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use characters::AnimClip;

use crate::input::{TanzButton, TANZ_MIN_COHERENCE, TANZ_MIN_MAGNITUDE};
use crate::stack::{TanzDefinition, TanzId, TanzInputRange, TanzStack, TanzTiming};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TanzAction {
	Move { local: Vec2 },
	Heal { amount: f32 },
}

pub const HEALING_SALUTE_AMOUNT: f32 = 25.0;

const STRONG: RangeInclusive<f32> = TANZ_MIN_MAGNITUDE..=1.5;
const SECTOR: f32 = 40_f32 * std::f32::consts::PI / 180.0;

const MOVE_TIMING: TanzTiming = TanzTiming {
	startup: 0.04,
	active: 0.22,
	recovery: 0.12,
	cooldown: 0.35,
	chain_window: 0.22,
	buffer_ttl: 0.40,
};

const SALUTE_TIMING: TanzTiming = TanzTiming {
	startup: 0.08,
	active: 0.28,
	recovery: 0.20,
	cooldown: 0.80,
	chain_window: 0.24,
	buffer_ttl: 0.40,
};

fn strong_sector(direction: Vec2, button: TanzButton) -> TanzInputRange {
	TanzInputRange {
		direction,
		half_angle: SECTOR,
		magnitude: STRONG,
		min_coherence: TANZ_MIN_COHERENCE,
		button,
	}
}

pub fn healing_salute() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::HealingSalute,
		input: strong_sector(Vec2::Y, TanzButton::APress),
		action: TanzAction::Heal { amount: HEALING_SALUTE_AMOUNT },
		timing: SALUTE_TIMING,
		clip: AnimClip::jab(),
	}
}

pub fn left_strafe() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::LeftStrafe,
		input: strong_sector(Vec2::NEG_X, TanzButton::None),
		action: TanzAction::Move { local: Vec2::NEG_X },
		timing: MOVE_TIMING,
		clip: AnimClip::power_strafe_left(),
	}
}

pub fn right_strafe() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::RightStrafe,
		input: strong_sector(Vec2::X, TanzButton::None),
		action: TanzAction::Move { local: Vec2::X },
		timing: MOVE_TIMING,
		clip: AnimClip::power_strafe_right(),
	}
}

pub fn forward_dash() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::ForwardDash,
		input: strong_sector(Vec2::Y, TanzButton::None),
		action: TanzAction::Move { local: Vec2::Y },
		timing: MOVE_TIMING,
		clip: AnimClip::power_dash(),
	}
}

pub fn starter_stack() -> TanzStack {
	TanzStack::starter()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn salute_uses_jab_and_heal() -> anyhow::Result<()> {
		let salute = healing_salute();
		assert_eq!(salute.clip.id(), AnimClip::jab().id());
		assert_eq!(salute.action, TanzAction::Heal { amount: HEALING_SALUTE_AMOUNT });
		assert_eq!(salute.input.button, TanzButton::APress);
		Ok(())
	}

	#[test]
	fn strafe_tanzes_use_power_strafe_with_matching_side() -> anyhow::Result<()> {
		assert_eq!(left_strafe().clip, AnimClip::power_strafe_left());
		assert_eq!(right_strafe().clip, AnimClip::power_strafe_right());
		assert_eq!(left_strafe().clip.id(), right_strafe().clip.id());
		assert_eq!(left_strafe().action, TanzAction::Move { local: Vec2::NEG_X });
		Ok(())
	}

	#[test]
	fn forward_dash_uses_power_dash() -> anyhow::Result<()> {
		assert_eq!(forward_dash().clip.id(), AnimClip::power_dash().id());
		assert_eq!(forward_dash().action, TanzAction::Move { local: Vec2::Y });
		Ok(())
	}

	#[test]
	fn forward_dash_clip_wall_time_matches_move_timing() -> anyhow::Result<()> {
		let dash = forward_dash();
		let wall = dash.timing.total();
		assert!((wall - 0.38).abs() < 1e-4, "MOVE_TIMING total={wall}");
		assert_eq!(dash.clip.default_speed(), 1.0);
		// Tanz drives normalized progress from execution age; cycle speed is irrelevant on that path.
		eprintln!(
			"forward dash wall time {:.3}s (startup {:.2} + active {:.2} + recovery {:.2}); AnimProgress = age / {:.3}",
			wall,
			dash.timing.startup,
			dash.timing.active,
			dash.timing.recovery,
			wall,
		);
		Ok(())
	}

	#[test]
	fn forward_dash_capsule_travel_during_active_phase() -> anyhow::Result<()> {
		// Matches `player::body` jog cap / accel / brake (not re-exported).
		const JOG_SPEED: f32 = 6.0;
		const MOVE_ACCEL: f32 = 40.0;
		const MOVE_BRAKE: f32 = 50.0;

		fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
			let delta = target - current;
			if delta.abs() <= max_delta {
				target
			} else {
				current + delta.signum() * max_delta
			}
		}

		let timing = forward_dash().timing;
		let dt = 1.0 / 60.0;
		let mut velocity = 0.0f32;
		let mut distance = 0.0f32;
		let mut age = 0.0f32;
		while age < timing.total() {
			let in_active = age >= timing.startup_end() && age < timing.active_end();
			let target = if in_active { JOG_SPEED } else { 0.0 };
			let rate = if target > 0.0 { MOVE_ACCEL } else { MOVE_BRAKE };
			velocity = move_toward(velocity, target, rate * dt);
			if in_active {
				distance += velocity * dt;
			}
			age += dt;
		}

		eprintln!(
			"capsule forward travel during active {:.2}s (tanzer, no sprint): {:.3} m @ JOG_SPEED cap {:.1} m/s",
			timing.active,
			distance,
			JOG_SPEED,
		);
		assert!(distance > 0.7 && distance < 1.1, "unexpected travel {distance}");
		Ok(())
	}

	#[test]
	fn strafe_move_wish_is_lateral_body_x_at_identity_yaw() -> anyhow::Result<()> {
		use crate::look_wish;

		assert_eq!(look_wish(0.0, Vec2::NEG_X), Vec3::NEG_X);
		assert_eq!(look_wish(0.0, Vec2::X), Vec3::X);
		Ok(())
	}
}
