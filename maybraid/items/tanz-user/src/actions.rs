//! Starter Tanz definitions. Effects compose existing movement / health APIs.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use characters::AnimClip;

use crate::input::{TanzButton, TANZ_MIN_COHERENCE, TANZ_MIN_MAGNITUDE};
use crate::stack::{TanzDefinition, TanzId, TanzInputRange, TanzStack, TanzTiming};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TanzAction {
	/// A positive `cruise` is an absolute ground speed (m/s) applied with the
	/// shared burst envelope. `cruise == 0` keeps the motor jog cap.
	Move {
		local: Vec2,
		cruise: f32,
	},
	Heal {
		amount: f32,
	},
}

pub const HEALING_SALUTE_AMOUNT: f32 = 25.0;

/// Held-slide cruise so the 0.38 s envelope covers about 10 m.
///
/// Push ramps 0→cruise, hold stays at cruise, brake ramps cruise→0. Jog-capped
/// Active-only wishes only covered ~0.9 m; 45 m/s is the smallest cruise that
/// lands near 10 m without changing Tanz wall time.
pub const FORWARD_DASH_CRUISE: f32 = 45.0;

/// Lateral PowerStrafe cruise on the same 0.38 s envelope, a bit under the dash.
///
/// 34 m/s lands near 7.6 m. Visual `ArmatureOffset` stays a centimetre-scale
/// follow-through and does not add this travel.
pub const POWER_STRAFE_CRUISE: f32 = 34.0;

/// Must match `character_animations::animations` PowerDash windows.
pub const DASH_COMPRESS_END: f32 = 0.10;
pub const DASH_PUSH_END: f32 = 0.25;
pub const DASH_HOLD_END: f32 = 0.68;
pub const DASH_BRAKE_END: f32 = 0.85;

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
		action: TanzAction::Move { local: Vec2::NEG_X, cruise: POWER_STRAFE_CRUISE },
		timing: MOVE_TIMING,
		clip: AnimClip::power_strafe_left(),
	}
}

pub fn right_strafe() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::RightStrafe,
		input: strong_sector(Vec2::X, TanzButton::None),
		action: TanzAction::Move { local: Vec2::X, cruise: POWER_STRAFE_CRUISE },
		timing: MOVE_TIMING,
		clip: AnimClip::power_strafe_right(),
	}
}

pub fn forward_dash() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::ForwardDash,
		input: strong_sector(Vec2::Y, TanzButton::None),
		action: TanzAction::Move { local: Vec2::Y, cruise: FORWARD_DASH_CRUISE },
		timing: MOVE_TIMING,
		clip: AnimClip::power_dash(),
	}
}

pub fn starter_stack() -> TanzStack {
	TanzStack::starter()
}

fn smoothstep(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

/// Ground speed along the shared Tanz burst silhouette: 0 in compress, cruise
/// through the held slide, 0 again after brake. PowerDash and PowerStrafe both
/// use this; `ArmatureOffset` does not add capsule travel.
pub fn dash_ground_speed(progress: f32, cruise: f32) -> f32 {
	let t = progress.clamp(0.0, 1.0);
	if t < DASH_COMPRESS_END {
		0.0
	} else if t < DASH_PUSH_END {
		cruise * smoothstep((t - DASH_COMPRESS_END) / (DASH_PUSH_END - DASH_COMPRESS_END))
	} else if t < DASH_HOLD_END {
		cruise
	} else if t < DASH_BRAKE_END {
		cruise * (1.0 - smoothstep((t - DASH_HOLD_END) / (DASH_BRAKE_END - DASH_HOLD_END)))
	} else {
		0.0
	}
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
		assert_eq!(
			left_strafe().action,
			TanzAction::Move { local: Vec2::NEG_X, cruise: POWER_STRAFE_CRUISE }
		);
		assert_eq!(
			right_strafe().action,
			TanzAction::Move { local: Vec2::X, cruise: POWER_STRAFE_CRUISE }
		);
		Ok(())
	}

	#[test]
	fn forward_dash_uses_power_dash() -> anyhow::Result<()> {
		assert_eq!(forward_dash().clip.id(), AnimClip::power_dash().id());
		assert_eq!(
			forward_dash().action,
			TanzAction::Move { local: Vec2::Y, cruise: FORWARD_DASH_CRUISE }
		);
		Ok(())
	}

	#[test]
	fn dash_speed_windows_match_power_dash_clip() {
		assert_eq!(DASH_COMPRESS_END, character_animations::animations::COMPRESS_END);
		assert_eq!(DASH_PUSH_END, character_animations::animations::PUSH_END);
		assert_eq!(DASH_HOLD_END, character_animations::animations::HOLD_END);
		assert_eq!(DASH_BRAKE_END, character_animations::animations::BRAKE_END);
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
			wall, dash.timing.startup, dash.timing.active, dash.timing.recovery, wall,
		);
		Ok(())
	}

	#[test]
	fn forward_dash_capsule_travel_is_about_ten_metres() -> anyhow::Result<()> {
		let report = simulate_burst_travel(FORWARD_DASH_CRUISE, 1.0 / 60.0);
		eprintln!(
			"forward dash {:.3}s @ cruise {:.1} m/s: total {:.3} m (push {:.3}, hold {:.3}, brake {:.3}); max speed {:.2}; hold foot-slip {:.3} m ({:.3} m/frame); leftover v {:.2} m/s",
			forward_dash().timing.total(),
			FORWARD_DASH_CRUISE,
			report.total,
			report.push,
			report.hold,
			report.brake,
			report.max_speed,
			report.hold_slip,
			report.hold_slip_per_frame,
			report.leftover_speed,
		);
		assert!(report.total > 9.5 && report.total < 10.5, "expected ~10 m, got {}", report.total);
		assert!(
			report.leftover_speed < 1.5,
			"recover should kill the burst, leftover {}",
			report.leftover_speed
		);
		assert!(report.hold > 7.0 && report.hold < 8.0, "hold should carry most of the dash");
		Ok(())
	}

	#[test]
	fn strafe_capsule_travel_is_about_seven_to_eight_metres() -> anyhow::Result<()> {
		let report = simulate_burst_travel(POWER_STRAFE_CRUISE, 1.0 / 60.0);
		eprintln!(
			"power strafe {:.3}s @ cruise {:.1} m/s: total {:.3} m (push {:.3}, hold {:.3}, brake {:.3}); max speed {:.2}; hold foot-slip {:.3} m ({:.3} m/frame); leftover v {:.2} m/s",
			left_strafe().timing.total(),
			POWER_STRAFE_CRUISE,
			report.total,
			report.push,
			report.hold,
			report.brake,
			report.max_speed,
			report.hold_slip,
			report.hold_slip_per_frame,
			report.leftover_speed,
		);
		assert!(report.total > 7.0 && report.total < 8.2, "expected ~7–8 m, got {}", report.total);
		assert!(
			report.total < 9.5,
			"strafe should stay shorter than the ~10 m dash, got {}",
			report.total
		);
		assert!(
			report.leftover_speed < 1.5,
			"recover should kill the burst, leftover {}",
			report.leftover_speed
		);
		assert!(report.hold > 5.0 && report.hold < 6.5, "hold should carry most of the strafe");
		Ok(())
	}

	struct BurstTravel {
		total: f32,
		push: f32,
		hold: f32,
		brake: f32,
		max_speed: f32,
		hold_slip: f32,
		hold_slip_per_frame: f32,
		leftover_speed: f32,
	}

	fn simulate_burst_travel(cruise: f32, dt: f32) -> BurstTravel {
		fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
			let delta = target - current;
			if delta.abs() <= max_delta {
				target
			} else {
				current + delta.signum() * max_delta
			}
		}

		let total_secs = left_strafe().timing.total();
		let mut velocity = 0.0f32;
		let mut total = 0.0f32;
		let mut push = 0.0f32;
		let mut hold = 0.0f32;
		let mut brake = 0.0f32;
		let mut max_speed = 0.0f32;
		let mut age = 0.0f32;
		while age < total_secs {
			let progress = (age / total_secs).clamp(0.0, 1.0);
			let target = dash_ground_speed(progress, cruise);
			let accel = (target.max(velocity) * 30.0).max(40.0);
			velocity = move_toward(velocity, target, accel * dt);
			total += velocity * dt;
			max_speed = max_speed.max(velocity);
			if progress >= DASH_COMPRESS_END && progress < DASH_PUSH_END {
				push += velocity * dt;
			} else if progress >= DASH_PUSH_END && progress < DASH_HOLD_END {
				hold += velocity * dt;
			} else if progress >= DASH_HOLD_END && progress < DASH_BRAKE_END {
				brake += velocity * dt;
			}
			age += dt;
		}
		BurstTravel {
			total,
			push,
			hold,
			brake,
			max_speed,
			hold_slip: hold,
			hold_slip_per_frame: cruise * dt,
			leftover_speed: velocity,
		}
	}

	#[test]
	fn strafe_move_wish_is_lateral_body_x_at_identity_yaw() -> anyhow::Result<()> {
		use crate::look_wish;

		assert_eq!(look_wish(0.0, Vec2::NEG_X), Vec3::NEG_X);
		assert_eq!(look_wish(0.0, Vec2::X), Vec3::X);
		Ok(())
	}
}
