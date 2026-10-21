//! Throw aim, launch, and landing helpers.

use bevy::prelude::*;
use character_items::GrenadeStats;
use player::{PlayerLook, look_forward};

/// First person follows the camera. Third person uses body heading; orbit pitch is not a throw angle.
pub fn throw_aim(facing: Vec3, look: &PlayerLook) -> Vec3 {
	if look.first_person {
		return look_forward(look);
	}
	Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z)
}

/// Avian default |g|. Thrown grenades use unit gravity scale and no linear damping.
pub const THROW_GRAVITY: f32 = 9.81;

/// Chest hold (`1.28 + hold_up`) plus the release toss offset (`0.12`).
pub const STANDING_RELEASE_HEIGHT: f32 = 1.43;

pub fn launch_velocity(aim: Vec3, stats: &GrenadeStats, inherit: Vec3) -> Vec3 {
	let tossed = (aim + Vec3::Y * stats.upward_bias).normalize_or(Vec3::Y);
	tossed * stats.launch_speed + inherit * stats.inherit_velocity
}

/// Seconds until a vacuum throw from `height` returns to y = 0.
pub fn landing_time(height: f32, vy: f32, gravity: f32) -> f32 {
	if gravity <= 1e-6 {
		return 0.0;
	}
	let disc = vy * vy + 2.0 * gravity * height.max(0.0);
	(vy + disc.sqrt()) / gravity
}

/// Horizontal meters a vacuum throw covers before y = 0.
pub fn landing_range(height: f32, velocity: Vec3, gravity: f32) -> f32 {
	Vec2::new(velocity.x, velocity.z).length() * landing_time(height, velocity.y, gravity)
}
