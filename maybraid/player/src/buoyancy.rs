//! Column-sampled spring bob. Water is not an Avian collider.

use crate::body::{CharacterController, Grounded, Jumping, MOVE_SPEED};
use avian3d::prelude::{Gravity, GravityScale, LinearVelocity};
use bevy::ecs::query::Has;
use bevy::prelude::*;
use characters::LocomotionCapsule;
use durham::{DurhamSurface, WaterColumn};

/// Draft as a fraction of hull height (head out).
pub const FLOAT_DRAFT: f32 = 0.55;
/// Shallower than this stays wade (terrain `Grounded`).
pub const WADE_MAX_DEPTH: f32 = 0.6;
/// Submersion below [`FLOAT_DRAFT`] stays wade. `0.35` is the typical ankle/knee band.
const SPRING_K: f32 = 40.0;
const SPRING_ZETA: f32 = 0.45;
const SPRING_C: f32 = 2.0 * SPRING_ZETA * 6.3245554; // 2ζ√40
const FLOAT_XZ_DRAG: f32 = 6.0;
const WADE_XZ_DRAG: f32 = 3.0;
const DEFAULT_GRAVITY: f32 = 9.81;
/// Swim wish cap as a fraction of [`MOVE_SPEED`].
pub const SWIM_SPEED_SCALE: f32 = 0.5;
pub const WADE_SPEED_SCALE: f32 = 0.7;
const SURFACE_JUMP_MIN: f32 = 0.35;
const SURFACE_JUMP_MAX: f32 = 0.7;
pub const WATER_JUMP_SCALE: f32 = 0.7;

/// Floating on a wet column. Sparse, same idea as [`JumpWish`].
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(storage = "SparseSet")]
pub struct Buoyant {
	pub submersion: f32,
}

impl Buoyant {
	pub fn can_surface_jump(&self) -> bool {
		(SURFACE_JUMP_MIN..=SURFACE_JUMP_MAX).contains(&self.submersion)
	}
}

/// Ankle-to-waist wet contact. Stays on the terrain walk plane.
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(storage = "SparseSet")]
pub struct Wading;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterRegime {
	Dry,
	Wade,
	Float,
}

pub fn submersion_fraction(feet_y: f32, height: f32, surface: f32) -> f32 {
	if height <= 1e-4 {
		return 0.0;
	}
	((surface - feet_y) / height).clamp(0.0, 1.0)
}

/// Capsule-center rest pose for draft `d` (head out when `d ≈ 0.55`).
pub fn equilibrium_center(surface: f32, draft: f32, height: f32) -> f32 {
	surface - (draft - 0.5) * height
}

pub fn water_regime(column: Option<WaterColumn>, feet_y: f32, height: f32) -> WaterRegime {
	let Some(column) = column else {
		return WaterRegime::Dry;
	};
	let submersion = submersion_fraction(feet_y, height, column.surface);
	if submersion <= 0.0 {
		return WaterRegime::Dry;
	}
	let y_eq = equilibrium_center(column.surface, FLOAT_DRAFT, height);
	let feet_eq = y_eq - height * 0.5;
	let chest_deep = submersion + 1e-4 >= FLOAT_DRAFT;
	if column.depth() < WADE_MAX_DEPTH || !chest_deep || feet_eq < column.bed - 1e-4 {
		WaterRegime::Wade
	} else {
		WaterRegime::Float
	}
}

pub fn spring_vertical_accel(y: f32, vy: f32, y_eq: f32) -> f32 {
	SPRING_K * (y_eq - y) - SPRING_C * vy
}

pub(crate) fn swim_speed() -> f32 {
	MOVE_SPEED * SWIM_SPEED_SCALE
}

pub(crate) fn wade_speed() -> f32 {
	MOVE_SPEED * WADE_SPEED_SCALE
}

pub(crate) fn apply_buoyancy(
	mut commands: Commands,
	time: Res<Time>,
	surface: Option<DurhamSurface>,
	gravity: Option<Res<Gravity>>,
	mut controllers: Query<
		(
			Entity,
			&Transform,
			&LocomotionCapsule,
			&mut LinearVelocity,
			Has<Grounded>,
			Option<&GravityScale>,
			Option<&Jumping>,
		),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	let Some(surface) = surface else {
		clear_water_markers(&mut commands, &mut controllers);
		return;
	};
	let g = gravity.map(|g| g.0.y.abs()).unwrap_or(DEFAULT_GRAVITY);

	for (entity, transform, hull, mut velocity, grounded, gravity_scale, jumping) in
		&mut controllers
	{
		let height = hull.half_height() * 2.0;
		let feet_y = transform.translation.y - hull.half_height();
		let column = match surface.water_column_at(transform.translation.xz()) {
			Ok(column) => column,
			Err(_) => continue,
		};
		let submersion =
			column.map(|c| submersion_fraction(feet_y, height, c.surface)).unwrap_or(0.0);
		match water_regime(column, feet_y, height) {
			WaterRegime::Dry => {
				commands.entity(entity).remove::<(Buoyant, Wading)>();
			}
			WaterRegime::Wade => {
				commands.entity(entity).remove::<Buoyant>().insert(Wading);
				damp_xz(&mut velocity, WADE_XZ_DRAG, dt);
				let _ = grounded;
			}
			WaterRegime::Float => {
				let column = column.expect("float requires a wet column");
				if !jumping.is_some_and(Jumping::airborne) {
					let y_eq = equilibrium_center(column.surface, FLOAT_DRAFT, height);
					let scale = gravity_scale.map(|s| s.0).unwrap_or(1.25);
					let ay = spring_vertical_accel(transform.translation.y, velocity.y, y_eq)
						+ g * scale;
					velocity.y += ay * dt;
					damp_xz(&mut velocity, FLOAT_XZ_DRAG, dt);
				}
				commands
					.entity(entity)
					.remove::<(Grounded, Wading)>()
					.insert(Buoyant { submersion });
			}
		}
	}
}

fn clear_water_markers(
	commands: &mut Commands,
	controllers: &mut Query<
		(
			Entity,
			&Transform,
			&LocomotionCapsule,
			&mut LinearVelocity,
			Has<Grounded>,
			Option<&GravityScale>,
			Option<&Jumping>,
		),
		With<CharacterController>,
	>,
) {
	for (entity, ..) in controllers.iter() {
		commands.entity(entity).remove::<(Buoyant, Wading)>();
	}
}

fn damp_xz(velocity: &mut LinearVelocity, drag: f32, dt: f32) {
	let keep = (1.0 - drag * dt).max(0.0);
	velocity.x *= keep;
	velocity.z *= keep;
}

pub(crate) fn water_jump_impulse(base: f32, buoyant: Option<&Buoyant>) -> Option<f32> {
	let buoyant = buoyant?;
	buoyant.can_surface_jump().then_some(base * WATER_JUMP_SCALE)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn humanoid_height() -> f32 {
		LocomotionCapsule::HUMANOID.half_height() * 2.0
	}

	#[test]
	fn dry_column_is_dry() {
		let height = humanoid_height();
		assert_eq!(water_regime(None, 10.0, height), WaterRegime::Dry);
	}

	#[test]
	fn ankle_deep_stream_is_wade() {
		let height = humanoid_height();
		let column = WaterColumn { surface: 10.4, bed: 10.0 };
		assert!(column.depth() < WADE_MAX_DEPTH);
		assert_eq!(water_regime(Some(column), 10.0, height), WaterRegime::Wade);
	}

	#[test]
	fn puddle_below_bed_is_wade() {
		let height = humanoid_height();
		let bed = 10.0;
		let surface = bed + 0.2;
		let column = WaterColumn { surface, bed };
		let y_eq = equilibrium_center(surface, FLOAT_DRAFT, height);
		assert!(y_eq - height * 0.5 < bed);
		assert_eq!(water_regime(Some(column), bed, height), WaterRegime::Wade);
	}

	#[test]
	fn chest_deep_lake_is_float() {
		let height = humanoid_height();
		let bed = 10.0;
		let surface = bed + 3.0;
		let column = WaterColumn { surface, bed };
		let y_eq = equilibrium_center(surface, FLOAT_DRAFT, height);
		let feet = y_eq - height * 0.5;
		assert_eq!(water_regime(Some(column), feet, height), WaterRegime::Float);
	}

	#[test]
	fn equilibrium_tracks_graded_surface() {
		let height = humanoid_height();
		let west = equilibrium_center(30.0, FLOAT_DRAFT, height);
		let east = equilibrium_center(34.0, FLOAT_DRAFT, height);
		assert!((east - west - 4.0).abs() < 1e-4);
	}

	#[test]
	fn chest_deep_rest_settles_near_equilibrium() {
		let height = humanoid_height();
		let surface = 20.0;
		let y_eq = equilibrium_center(surface, FLOAT_DRAFT, height);
		let mut y = surface;
		let mut vy = 0.0;
		let dt = 1.0 / 60.0;
		for _ in 0..180 {
			vy += spring_vertical_accel(y, vy, y_eq) * dt;
			y += vy * dt;
		}
		assert!((y - y_eq).abs() < 0.05, "settled {y} vs {y_eq}");
	}

	#[test]
	fn drop_in_overshoots_then_settles() {
		let height = humanoid_height();
		let surface = 20.0;
		let y_eq = equilibrium_center(surface, FLOAT_DRAFT, height);
		let mut y = surface + 1.0;
		let mut vy = -8.0;
		let dt = 1.0 / 60.0;
		let mut went_below = false;
		for _ in 0..240 {
			vy += spring_vertical_accel(y, vy, y_eq) * dt;
			y += vy * dt;
			if y < y_eq {
				went_below = true;
			}
		}
		assert!(went_below, "underdamped entry must cross {y_eq}");
		assert!((y - y_eq).abs() < 0.05, "settled {y} vs {y_eq}");
	}

	#[test]
	fn surface_band_allows_a_water_jump() {
		assert!(Buoyant { submersion: 0.55 }.can_surface_jump());
		assert!(water_jump_impulse(8.0, Some(&Buoyant { submersion: 0.55 })).is_some());
		assert!(water_jump_impulse(8.0, Some(&Buoyant { submersion: 0.9 })).is_none());
		assert!(water_jump_impulse(8.0, None).is_none());
	}
}
