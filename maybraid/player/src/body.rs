//! Dynamic capsule: grounded, wish accel, jump.

use avian3d::prelude::*;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use characters::LocomotionCapsule;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

pub(crate) const JOG_SPEED: f32 = 6.0; // ≤ LEAP_SPEED, walk / slow-run clip
pub(crate) const MOVE_SPEED: f32 = 10.5; // sprint; run clip
pub(crate) const MOVE_ACCEL: f32 = 40.0;
/// Grounded idle brake toward rest along the walk plane (matches vegetation).
pub(crate) const MOVE_BRAKE: f32 = 50.0;
const AIR_CONTROL: f32 = 0.25;
pub(crate) const JUMP_IMPULSE: f32 = 8.0;
pub(crate) const MAX_SLOPE_ANGLE: f32 = PI * 0.45;
pub(crate) const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;

/// Walkable grounded slope for FFA / NPC capsules. Insert before [`crate::PlayerPlugin`]
/// to override the default (~81°). World / Durham playgrounds use ~70° so cliffs
/// never count as floor. Uncontrolled bodies need static floor friction above
/// `tan(this)`; motor capsules idle-brake to rest instead.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CharacterLocomotion {
	/// Hits steeper than this (radians from up) are not grounded.
	pub max_slope_angle: f32,
}

impl Default for CharacterLocomotion {
	fn default() -> Self {
		Self { max_slope_angle: MAX_SLOPE_ANGLE }
	}
}

#[derive(Component, Default, Clone, Copy, PartialEq)]
pub struct MoveWish(pub Vec3);

/// Absolute ground-speed cap (m/s) that replaces jog/sprint while present.
///
/// Stick locomotion never inserts this. Tanz bursts set it so a dash can
/// exceed [`JOG_SPEED`] without changing ordinary [`wish_throttle`] semantics.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MoveSpeedCap(pub f32);

/// Last walkable contact plane. Default is world up (flat XZ heading).
///
/// Used only while [`Grounded`]: if the down-cast misses for a snap frame, drive
/// still follows this plane instead of ramming world XZ into the mesh. Off the
/// ground, gravity owns Y.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WalkableGround {
	pub normal: Vec3,
}

impl Default for WalkableGround {
	fn default() -> Self {
		Self { normal: Vec3::Y }
	}
}

/// One-shot jump request. Consumed in Body when grounded.
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(storage = "SparseSet")]
pub struct JumpWish;

/// Player-granted sprint hold. Sparse, same storage idea as [`JumpWish`].
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(storage = "SparseSet")]
pub struct Sprinting;

/// Stance-scaled wish cap. Sprint uses [`MOVE_SPEED`]; jog stays at [`JOG_SPEED`].
pub(crate) fn move_cap(sprinting: bool, stance_scale: f32) -> f32 {
	let base = if sprinting { MOVE_SPEED } else { JOG_SPEED };
	base * stance_scale.max(0.0)
}

/// Takeoff (grounded) → air → land recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpPhase {
	Takeoff,
	Air,
	Land,
}

/// In-flight jump. Impulse waits until takeoff ends; land holds until recovery.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct Jumping {
	pub phase: JumpPhase,
	pub left_ground: bool,
	pub leaping: bool,
	pub phase_elapsed: f32,
	pub launch_vy: f32,
}

/// Horizontal speed at which a jump is a running leap rather than a standing hop.
pub const LEAP_SPEED: f32 = 7.5;
const JUMP_TAKEOFF_DURATION: f32 = 0.14;
const JUMP_LAND_DURATION: f32 = 0.24;
const FAILED_HOP_SECONDS: f32 = 0.05;

impl Jumping {
	pub fn start(xz_speed: f32) -> Self {
		Self {
			phase: JumpPhase::Takeoff,
			left_ground: false,
			leaping: xz_speed > LEAP_SPEED,
			phase_elapsed: 0.0,
			launch_vy: 0.0,
		}
	}

	pub fn airborne(&self) -> bool {
		self.phase == JumpPhase::Air
	}

	/// Normalized 0..1 takeoff / air / land for the leap sampler.
	pub fn leap_progress(&self, vertical_velocity: f32) -> f32 {
		match self.phase {
			JumpPhase::Takeoff => {
				(self.phase_elapsed / JUMP_TAKEOFF_DURATION).clamp(0.0, 1.0) * LEAP_TAKEOFF_END
			}
			JumpPhase::Air => {
				let launch = self.launch_vy.max(1e-3);
				let frac = (1.0 - vertical_velocity / launch).clamp(0.0, 2.0) * 0.5;
				LEAP_TAKEOFF_END + frac.clamp(0.0, 1.0) * (LEAP_AIR_END - LEAP_TAKEOFF_END)
			}
			JumpPhase::Land => {
				LEAP_AIR_END
					+ (self.phase_elapsed / JUMP_LAND_DURATION).clamp(0.0, 1.0)
						* (1.0 - LEAP_AIR_END)
			}
		}
	}
}

/// Advance a jump. Returns true when the shot is done and [`Jumping`] should drop.
///
/// Takeoff stays on the ground; impulse fires when takeoff ends or the capsule
/// leaves the mesh. Land starts on first grounded frame after leaving.
pub fn tick_jump(
	jump: &mut Jumping,
	grounded: bool,
	velocity: &mut Vec3,
	impulse: f32,
	dt: f32,
) -> bool {
	match jump.phase {
		JumpPhase::Takeoff => {
			jump.phase_elapsed += dt;
			if jump.phase_elapsed >= JUMP_TAKEOFF_DURATION || !grounded {
				velocity.y = impulse;
				jump.launch_vy = impulse;
				jump.phase = JumpPhase::Air;
				jump.phase_elapsed = 0.0;
				if !grounded {
					jump.left_ground = true;
				}
			}
			false
		}
		JumpPhase::Air => {
			jump.phase_elapsed += dt;
			if grounded && jump.left_ground {
				jump.phase = JumpPhase::Land;
				jump.phase_elapsed = 0.0;
				false
			} else {
				grounded
					&& !jump.left_ground
					&& velocity.y <= 0.05
					&& jump.phase_elapsed > FAILED_HOP_SECONDS
			}
		}
		JumpPhase::Land => {
			jump.phase_elapsed += dt;
			jump.phase_elapsed >= JUMP_LAND_DURATION
		}
	}
}

/// Leap sampler windows (must match `character_animations::animations::leap`).
const LEAP_TAKEOFF_END: f32 = 0.18;
const LEAP_AIR_END: f32 = 0.72;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerControlSystems;

#[derive(Component)]
pub struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

#[derive(Component)]
pub(crate) struct MovementAcceleration(pub f32);

#[derive(Component)]
pub(crate) struct JumpImpulse(pub f32);

#[derive(Component)]
pub(crate) struct MaxSlopeAngle(pub f32);

pub fn apply_locomotion_capsule(commands: &mut Commands, body: Entity, hull: LocomotionCapsule) {
	let collider = Collider::capsule(hull.radius, hull.length);
	let mut caster_shape = collider.clone();
	caster_shape.set_scale(Vec3::splat(0.99), 10);
	commands.entity(body).insert((
		hull,
		collider,
		ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::IDENTITY, Dir3::NEG_Y)
			.with_max_distance(GROUND_CAST_DISTANCE)
			.with_query_filter(SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)),
	));
	crate::hit::apply_hit_capsule(commands, body, hull);
}

/// Stamp the dynamic character controller onto an existing scene plant.
///
/// The caller owns [`Transform`], so semantic LOD fulfillment can preserve the
/// transform authored by its scene recipe.
pub fn apply_character_controller(commands: &mut Commands, body: Entity, hull: LocomotionCapsule) {
	commands.entity(body).insert((
		CharacterController,
		Visibility::default(),
		RigidBody::Dynamic,
		PhysicsInteractionLayer::animated_layers(),
		LockedAxes::ROTATION_LOCKED,
		MovementAcceleration(MOVE_ACCEL),
		JumpImpulse(JUMP_IMPULSE),
		MaxSlopeAngle(MAX_SLOPE_ANGLE),
		MoveWish::default(),
		crate::motor::MotorIntent::default(),
		WalkableGround::default(),
		crate::stance::RestLocomotionCapsule(hull),
		Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
		Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
		GravityScale(1.25),
	));
	commands.entity(body).insert(crate::contact::motor_traction_bundle());
	// Respawn / re-stamp must not reset a live squat or prone.
	commands
		.entity(body)
		.insert_if_new(crate::stance::CharacterStance::settled(crate::stance::StanceKind::Stand));
	apply_locomotion_capsule(commands, body, hull);
}

/// Copy [`CharacterLocomotion`] onto newly spawned capsules. Runs in PostUpdate
/// so scene materialize in Update is visible the same frame.
pub(crate) fn sync_character_locomotion(
	locomotion: Res<CharacterLocomotion>,
	mut slopes: Query<&mut MaxSlopeAngle, Added<MaxSlopeAngle>>,
) {
	for mut slope in &mut slopes {
		slope.0 = locomotion.max_slope_angle;
	}
}

/// Scale run acceleration and jump impulse from character-sheet factors.
pub fn apply_character_mobility(
	commands: &mut Commands,
	body: Entity,
	running_factor: f32,
	jump_factor: f32,
) {
	commands.entity(body).insert((
		MovementAcceleration(MOVE_ACCEL * running_factor.max(0.01)),
		JumpImpulse(JUMP_IMPULSE * jump_factor.max(0.01)),
	));
}

pub(crate) fn spawn_character_controller(
	commands: &mut Commands,
	translation: Vec3,
	hull: LocomotionCapsule,
) -> Entity {
	let body = commands.spawn(Transform::from_translation(translation)).id();
	apply_character_controller(commands, body, hull);
	body
}

pub(crate) fn update_grounded(
	mut commands: Commands,
	mut query: Query<
		(
			Entity,
			&ShapeHits,
			&LinearVelocity,
			Option<&MaxSlopeAngle>,
			Has<Grounded>,
			Option<&mut Jumping>,
		),
		With<CharacterController>,
	>,
) {
	for (entity, hits, velocity, max_slope_angle, was_grounded, jumping) in &mut query {
		if let Some(normal) = walkable_contact_normal(hits, max_slope_angle.map(|angle| angle.0)) {
			commands.entity(entity).insert(WalkableGround { normal });
		}
		let mut is_grounded = hits.iter().any(|hit| {
			if let Some(angle) = max_slope_angle {
				(-hit.normal2).angle_between(Vec3::Y).abs() <= angle.0
			} else {
				true
			}
		});
		if !is_grounded
			&& was_grounded
			&& jumping.as_ref().is_none_or(|jump| !jump.airborne())
			&& velocity.y > -GROUND_SNAP_SPEED
			&& velocity.y < GROUND_SNAP_SPEED
		{
			is_grounded = true;
		}
		if is_grounded {
			commands.entity(entity).insert(Grounded);
		} else {
			commands.entity(entity).remove::<Grounded>();
			if let Some(mut jump) = jumping {
				jump.left_ground = true;
			}
		}
	}
}

/// Most upright contact within the walkable slope, if any.
pub fn walkable_contact_normal(hits: &ShapeHits, max_slope_angle: Option<f32>) -> Option<Vec3> {
	let mut best: Option<(f32, Vec3)> = None;
	for hit in hits.iter() {
		let normal = (-hit.normal2).normalize_or_zero();
		if normal.length_squared() < 1e-8 {
			continue;
		}
		let angle = normal.angle_between(Vec3::Y).abs();
		if let Some(max) = max_slope_angle {
			if angle > max {
				continue;
			}
		}
		if best.is_none_or(|(best_angle, _)| angle < best_angle) {
			best = Some((angle, normal));
		}
	}
	best.map(|(_, normal)| normal)
}

fn walkable_ground_normal(hits: &ShapeHits, max_slope: Option<&MaxSlopeAngle>) -> Option<Vec3> {
	walkable_contact_normal(hits, max_slope.map(|angle| angle.0))
}

/// Contact plane used to turn a wish into capsule drive.
///
/// Airborne jump (and true air) are XZ only (gravity owns Y). Takeoff and land
/// stay on the walkable plane. A walkable hit this frame is the plane. Last
/// plane is only for a [`Grounded`] snap when the caster missed — never after
/// walking off a ridge.
pub fn ground_plane_for_wish(
	contact: Option<Vec3>,
	last: Option<Vec3>,
	grounded: bool,
	airborne: bool,
) -> Option<Vec3> {
	if airborne {
		return None;
	}
	if let Some(normal) = contact {
		return Some(normal);
	}
	if grounded {
		return last;
	}
	None
}

/// Stick throttle carried by a wish's XZ length, capped at full deflection.
/// NPC path wishes are unit length, so NPCs drive at the full cap.
pub(crate) fn wish_throttle(wish: Vec3) -> f32 {
	Vec2::new(wish.x, wish.z).length().min(1.0)
}

/// Unit drive for a movement wish. Compass heading on the contact plane.
///
/// Wish Y is ignored so a raised waypoint cannot become a launch vector. No
/// plane → XZ only (does not fly).
pub fn wish_on_ground(wish: Vec3, ground_normal: Option<Vec3>) -> Vec3 {
	let heading = Vec3::new(wish.x, 0.0, wish.z);
	if heading.length_squared() < 1e-8 {
		return Vec3::ZERO;
	}
	let Some(normal) = ground_normal.map(|normal| normal.normalize_or_zero()) else {
		return heading.normalize();
	};
	if normal.length_squared() < 1e-8 {
		return heading.normalize();
	}
	let along = heading - normal * heading.dot(normal);
	if along.length_squared() > 1e-8 {
		return along.normalize();
	}
	Vec3::ZERO
}

pub(crate) fn move_toward(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
	let delta = target - current;
	if delta.length_squared() <= max_delta * max_delta {
		target
	} else {
		current + delta.normalize_or_zero() * max_delta
	}
}

/// Target-speed drive along the walk plane. Same loop as the vegetation capsule.
fn control_ground_velocity(
	velocity: &mut LinearVelocity,
	wish: Vec3,
	accel: f32,
	dt: f32,
	ground_normal: Vec3,
	max_speed: f32,
) {
	let normal = ground_normal.normalize_or_zero();
	if normal.length_squared() < 1e-8 {
		control_air_velocity(velocity, wish, accel, dt, max_speed);
		return;
	}
	let tangent = **velocity - normal * velocity.dot(normal);
	let target = wish_on_ground(wish, Some(normal)) * max_speed * wish_throttle(wish);
	// Idle uses at least MOVE_BRAKE. Burst dashes pass a larger accel so they
	// can also stop inside the clip's brake window instead of coasting at 50 m/s².
	let rate = if target.length_squared() > 1e-8 { accel } else { accel.max(MOVE_BRAKE) };
	**velocity = move_toward(tangent, target, rate * dt);
}

fn control_air_velocity(
	velocity: &mut LinearVelocity,
	wish: Vec3,
	accel: f32,
	dt: f32,
	max_speed: f32,
) {
	let throttle = wish_throttle(wish);
	let wish = Vec3::new(wish.x, 0.0, wish.z).normalize_or_zero();
	if wish.length_squared() < 1e-8 {
		return;
	}
	let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
	let next = move_toward(horizontal, wish * max_speed * throttle, accel * AIR_CONTROL * dt);
	velocity.x = next.x;
	velocity.z = next.z;
}

/// Reach `cap` (or brake from `current_speed`) in about two 60 Hz frames.
pub(crate) fn burst_accel(cap: f32, current_speed: f32) -> f32 {
	(cap.max(current_speed) * 30.0).max(MOVE_ACCEL)
}

fn horizontal_speed(velocity: Vec3) -> f32 {
	Vec3::new(velocity.x, 0.0, velocity.z).length()
}

/// Apply [`MoveWish`] for every capsule. [`crate::apply_motor`] is the writer.
pub(crate) fn apply_wish_movement(
	time: Res<Time>,
	mut controllers: Query<
		(
			&MoveWish,
			&ShapeHits,
			Option<&MaxSlopeAngle>,
			Option<&WalkableGround>,
			&MovementAcceleration,
			Option<&MoveSpeedCap>,
			&mut LinearVelocity,
			Has<Grounded>,
			Has<crate::buoyancy::Buoyant>,
			Has<crate::buoyancy::Wading>,
			Has<Sprinting>,
			Option<&crate::stance::CharacterStance>,
			Option<&Jumping>,
		),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	for (
		wish,
		hits,
		max_slope,
		walkable,
		accel,
		speed_cap,
		mut velocity,
		grounded,
		buoyant,
		wading,
		sprinting,
		stance,
		jumping,
	) in &mut controllers
	{
		let default_cap = if buoyant {
			crate::buoyancy::swim_speed()
		} else if wading {
			crate::buoyancy::wade_speed()
		} else {
			move_cap(sprinting, stance.map(|stance| stance.speed_scale()).unwrap_or(1.0))
		};
		let (max_speed, drive_accel) = match speed_cap {
			Some(MoveSpeedCap(cap)) => (*cap, burst_accel(*cap, horizontal_speed(velocity.0))),
			None => (default_cap, accel.0),
		};
		let contact = walkable_ground_normal(hits, max_slope);
		let airborne = jumping.is_some_and(Jumping::airborne) || buoyant;
		let ground =
			ground_plane_for_wish(contact, walkable.map(|plane| plane.normal), grounded, airborne);
		if let Some(normal) = ground {
			control_ground_velocity(&mut velocity, wish.0, drive_accel, dt, normal, max_speed);
		} else {
			control_air_velocity(&mut velocity, wish.0, drive_accel, dt, max_speed);
		}
	}
}

/// Start a jump. Impulse waits for [`advance_jump_phases`].
pub(crate) fn apply_wish_jump(
	mut commands: Commands,
	mut controllers: Query<
		(
			Entity,
			&LinearVelocity,
			Has<Grounded>,
			Option<&crate::buoyancy::Buoyant>,
			Option<&mut crate::stance::CharacterStance>,
		),
		(With<CharacterController>, With<JumpWish>, Without<Jumping>),
	>,
) {
	for (entity, velocity, grounded, buoyant, stance) in &mut controllers {
		commands.entity(entity).remove::<JumpWish>();
		if let Some(mut stance) = stance {
			if stance.is_prone() {
				stance.stand();
				continue;
			}
		}
		if grounded || buoyant.is_some_and(crate::buoyancy::Buoyant::can_surface_jump) {
			let xz = Vec3::new(velocity.x, 0.0, velocity.z).length();
			commands.entity(entity).insert(Jumping::start(xz));
		}
	}
}

pub(crate) fn advance_jump_phases(
	mut commands: Commands,
	time: Res<Time>,
	mut controllers: Query<
		(
			Entity,
			&JumpImpulse,
			&mut LinearVelocity,
			&mut Jumping,
			Has<Grounded>,
			Option<&crate::buoyancy::Buoyant>,
		),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	for (entity, impulse, mut velocity, mut jumping, grounded, buoyant) in &mut controllers {
		let strength = crate::buoyancy::water_jump_impulse(impulse.0, buoyant).unwrap_or(impulse.0);
		if tick_jump(&mut jumping, grounded, &mut velocity, strength, dt) {
			commands.entity(entity).remove::<Jumping>();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_locomotion_keeps_legacy_slope() {
		assert!((CharacterLocomotion::default().max_slope_angle - MAX_SLOPE_ANGLE).abs() < 1e-6);
	}

	#[test]
	fn move_cap_is_jog_or_sprint() {
		assert!((move_cap(false, 1.0) - JOG_SPEED).abs() < 1e-6);
		assert!(JOG_SPEED <= LEAP_SPEED);
		assert!((move_cap(true, 1.0) - MOVE_SPEED).abs() < 1e-6);
		assert!((move_cap(true, 0.5) - MOVE_SPEED * 0.5).abs() < 1e-6);
	}

	#[test]
	fn burst_accel_reaches_dash_cruise_in_two_sixtieths() {
		let cap = 45.0;
		let accel = burst_accel(cap, 0.0);
		assert!(accel * (2.0 / 60.0) >= cap - 1e-4, "accel={accel}");
		let mut velocity = LinearVelocity(Vec3::ZERO);
		control_ground_velocity(&mut velocity, Vec3::Z, accel, 1.0 / 60.0, Vec3::Y, cap);
		control_ground_velocity(&mut velocity, Vec3::Z, accel, 1.0 / 60.0, Vec3::Y, cap);
		assert!((velocity.z - cap).abs() < 1e-3, "{velocity:?}");
	}

	#[test]
	fn burst_accel_brakes_from_cruise_when_the_cap_drops() {
		let mut velocity = LinearVelocity(Vec3::Z * 45.0);
		let cap = 0.0;
		let accel = burst_accel(cap, 45.0);
		control_ground_velocity(&mut velocity, Vec3::ZERO, accel, 1.0 / 60.0, Vec3::Y, cap);
		control_ground_velocity(&mut velocity, Vec3::ZERO, accel, 1.0 / 60.0, Vec3::Y, cap);
		assert!(velocity.z.abs() < 1e-3, "{velocity:?}");
	}

	#[test]
	fn jog_ground_drive_saturates_below_leap() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		control_ground_velocity(&mut velocity, Vec3::X, MOVE_ACCEL, 1.0, Vec3::Y, JOG_SPEED);
		assert!((velocity.x - JOG_SPEED).abs() < 1e-4, "{velocity:?}");
		assert!(velocity.x <= LEAP_SPEED);
	}

	#[test]
	fn half_stick_jogs_at_half_the_cap() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		control_ground_velocity(&mut velocity, Vec3::X * 0.5, MOVE_ACCEL, 1.0, Vec3::Y, JOG_SPEED);
		assert!((velocity.x - JOG_SPEED * 0.5).abs() < 1e-4, "{velocity:?}");
		let mut velocity = LinearVelocity(Vec3::ZERO);
		control_ground_velocity(&mut velocity, Vec3::X * 3.0, MOVE_ACCEL, 1.0, Vec3::Y, JOG_SPEED);
		assert!((velocity.x - JOG_SPEED).abs() < 1e-4, "over-unit wishes stay at the cap");
	}

	#[test]
	fn sprint_ground_drive_saturates_at_move_speed() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		control_ground_velocity(&mut velocity, Vec3::X, MOVE_ACCEL, 1.0, Vec3::Y, MOVE_SPEED);
		assert!((velocity.x - MOVE_SPEED).abs() < 1e-4, "{velocity:?}");
	}

	#[test]
	fn jump_from_squat_starts_and_prone_stands() -> anyhow::Result<()> {
		use crate::stance::{CharacterStance, StanceKind};
		use anyhow::anyhow;

		let mut app = App::new();
		app.add_systems(Update, apply_wish_jump);
		let squat = app
			.world_mut()
			.spawn((
				CharacterController,
				CharacterStance::settled(StanceKind::Squat),
				JumpWish,
				Grounded,
				LinearVelocity(Vec3::ZERO),
			))
			.id();
		let prone = app
			.world_mut()
			.spawn((
				CharacterController,
				CharacterStance::settled(StanceKind::Prone),
				JumpWish,
				Grounded,
				LinearVelocity(Vec3::ZERO),
			))
			.id();
		app.update();
		if app.world().get::<Jumping>(squat).is_none() {
			return Err(anyhow!("grounded squat + JumpWish should start Jumping"));
		}
		if app.world().get::<Jumping>(prone).is_some() {
			return Err(anyhow!("prone + JumpWish must not impulse"));
		}
		let stance = app.world().get::<CharacterStance>(prone).ok_or_else(|| anyhow!("stance"))?;
		if stance.kind != StanceKind::Stand {
			return Err(anyhow!("prone jump should stand first"));
		}
		Ok(())
	}

	#[test]
	fn jump_from_sprint_speed_is_a_leap() -> anyhow::Result<()> {
		use anyhow::anyhow;

		let mut app = App::new();
		app.add_systems(Update, apply_wish_jump);
		let entity = app
			.world_mut()
			.spawn((
				CharacterController,
				Sprinting,
				JumpWish,
				Grounded,
				LinearVelocity(Vec3::new(MOVE_SPEED, 0.0, 0.0)),
			))
			.id();
		app.update();
		let jumping = app
			.world()
			.get::<Jumping>(entity)
			.ok_or_else(|| anyhow!("grounded JumpWish at sprint speed should start Jumping"))?;
		if !jumping.leaping {
			return Err(anyhow!("sprint takeoff must be a leap"));
		}
		Ok(())
	}

	#[test]
	fn grounded_wish_climbs_along_the_slope() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		let slope = 70.0_f32.to_radians();
		// Hill rises in +X, so the normal tilts downhill (−X).
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		control_ground_velocity(&mut velocity, Vec3::X, MOVE_ACCEL, 1.0, normal, MOVE_SPEED);
		assert!(velocity.y > 0.0, "grounded drive must add uphill Y, got {}", velocity.y);
		assert!(velocity.x > 0.0);
		let tangent = velocity.0 - normal * velocity.0.dot(normal);
		assert!((tangent.length() - MOVE_SPEED).abs() < 1e-4, "{tangent:?}");
	}

	#[test]
	fn slope_drive_matches_target_speed_uphill_and_downhill() {
		let slope = 60.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let mut uphill = LinearVelocity(Vec3::ZERO);
		let mut downhill = LinearVelocity(Vec3::ZERO);

		control_ground_velocity(&mut uphill, Vec3::X, MOVE_ACCEL, 1.0, normal, MOVE_SPEED);
		control_ground_velocity(&mut downhill, Vec3::NEG_X, MOVE_ACCEL, 1.0, normal, MOVE_SPEED);

		assert!((uphill.length() - MOVE_SPEED).abs() < 1e-4, "{uphill:?}");
		assert!((downhill.length() - MOVE_SPEED).abs() < 1e-4, "{downhill:?}");
		assert!((uphill.x + downhill.x).abs() < 1e-4, "{uphill:?} vs {downhill:?}");
	}

	#[test]
	fn uphill_point_wish_climbs_along_the_slope() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		control_ground_velocity(
			&mut velocity,
			Vec3::new(1.0, 1.0, 0.0),
			MOVE_ACCEL,
			1.0,
			normal,
			MOVE_SPEED,
		);
		assert!(velocity.y > 0.0, "XZ heading on the plane must add uphill Y, got {}", velocity.y);
		let tangent = velocity.0 - normal * velocity.0.dot(normal);
		assert!((tangent.length() - MOVE_SPEED).abs() < 1e-4, "{tangent:?}");
		assert!(velocity.0.dot(normal).abs() < 1e-4, "{velocity:?}");
	}

	#[test]
	fn last_plane_is_only_for_grounded_snap() {
		let slope = Vec3::new(-0.2, 0.98, 0.0);
		assert_eq!(ground_plane_for_wish(None, Some(slope), true, false), Some(slope));
		assert_eq!(ground_plane_for_wish(None, Some(slope), false, false), None);
		assert_eq!(ground_plane_for_wish(None, Some(Vec3::Y), true, true), None);
	}

	#[test]
	fn takeoff_delays_impulse_until_the_window_ends() {
		let mut jump = Jumping::start(0.0);
		let mut velocity = Vec3::ZERO;
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.05));
		assert_eq!(jump.phase, JumpPhase::Takeoff);
		assert!(velocity.y.abs() < 1e-4);
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.12));
		assert_eq!(jump.phase, JumpPhase::Air);
		assert!((velocity.y - 8.0).abs() < 1e-4);
	}

	#[test]
	fn leaving_the_ground_during_takeoff_launches() {
		let mut jump = Jumping::start(MOVE_SPEED);
		let mut velocity = Vec3::ZERO;
		assert!(jump.leaping);
		assert!(!tick_jump(&mut jump, false, &mut velocity, 8.0, 0.016));
		assert_eq!(jump.phase, JumpPhase::Air);
		assert!(jump.left_ground);
		assert!((velocity.y - 8.0).abs() < 1e-4);
	}

	#[test]
	fn land_starts_on_contact_and_finishes_after_recovery() {
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Air;
		jump.left_ground = true;
		jump.launch_vy = 8.0;
		let mut velocity = Vec3::new(0.0, -2.0, 0.0);
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.016));
		assert_eq!(jump.phase, JumpPhase::Land);
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.1));
		assert!(tick_jump(&mut jump, true, &mut velocity, 8.0, 0.2));
	}

	#[test]
	fn leap_progress_covers_takeoff_air_and_land() {
		let takeoff = Jumping::start(0.0);
		assert!(takeoff.leap_progress(0.0) < 0.18);
		let mut air = Jumping::start(0.0);
		air.phase = JumpPhase::Air;
		air.launch_vy = 8.0;
		let apex = air.leap_progress(0.0);
		let descent = air.leap_progress(-8.0);
		assert!(apex > 0.18 && apex < 0.72, "{apex}");
		assert!(descent > apex && descent <= 0.72, "{descent}");
		let mut land = air;
		land.phase = JumpPhase::Land;
		land.phase_elapsed = JUMP_LAND_DURATION;
		assert!((land.leap_progress(0.0) - 1.0).abs() < 1e-4);
	}

	#[test]
	fn airborne_wish_stays_xz() {
		let mut velocity = LinearVelocity(Vec3::new(0.0, -5.0, 0.0));
		control_air_velocity(&mut velocity, Vec3::new(1.0, 4.0, 0.0), MOVE_ACCEL, 2.0, MOVE_SPEED);
		assert!((velocity.y + 5.0).abs() < 1e-4);
		assert!((velocity.x - MOVE_SPEED).abs() < 1e-4, "{velocity:?}");
	}

	#[test]
	fn wish_on_ground_does_not_fly() {
		let drive = wish_on_ground(Vec3::new(0.0, 1.0, 0.0), None);
		assert!(drive.length() < 1e-4, "{drive}");
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let along = wish_on_ground(Vec3::X, Some(normal));
		assert!(along.y > 0.0);
		assert!((along.dot(normal)).abs() < 1e-4, "{along} · {normal}");
		let from_3d = wish_on_ground(Vec3::new(1.0, 10.0, 0.0), Some(normal));
		assert!((from_3d - along).length() < 1e-4, "{from_3d} vs {along}");
	}

	#[test]
	fn slope_drive_is_symmetric_uphill_and_downhill() {
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let mut uphill = LinearVelocity(Vec3::ZERO);
		let mut downhill = LinearVelocity(Vec3::ZERO);
		control_ground_velocity(&mut uphill, Vec3::X, MOVE_ACCEL, 1.0, normal, MOVE_SPEED);
		control_ground_velocity(&mut downhill, Vec3::NEG_X, MOVE_ACCEL, 1.0, normal, MOVE_SPEED);

		assert!((uphill.length() - downhill.length()).abs() < 1e-5);
		assert!(uphill.dot(normal).abs() < 1e-5, "{uphill:?}");
		assert!(downhill.dot(normal).abs() < 1e-5, "{downhill:?}");
		assert!(uphill.y > 0.0);
		assert!(downhill.y < 0.0);
	}

	#[test]
	fn idle_brake_holds_on_a_steep_walkable_slope() {
		let gravity = Vec3::NEG_Y * 9.81 * 1.25;
		let slope = 70.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let g_tangent = gravity - normal * gravity.dot(normal);
		let dt = 1.0 / 60.0;
		let mut velocity = LinearVelocity(Vec3::ZERO);
		for _ in 0..120 {
			velocity.0 += g_tangent * dt;
			control_ground_velocity(&mut velocity, Vec3::ZERO, MOVE_ACCEL, dt, normal, MOVE_SPEED);
		}
		let tangent = velocity.0 - normal * velocity.0.dot(normal);
		assert!(
			tangent.length() < 0.05,
			"idle motor must cancel slope slide, leftover {tangent:?}"
		);
	}

	#[test]
	fn ground_speed_does_not_depend_on_frame_rate() {
		let mut slow = LinearVelocity(Vec3::ZERO);
		let mut fast = LinearVelocity(Vec3::ZERO);
		for _ in 0..60 {
			control_ground_velocity(
				&mut slow,
				Vec3::X,
				MOVE_ACCEL,
				1.0 / 60.0,
				Vec3::Y,
				MOVE_SPEED,
			);
		}
		for _ in 0..240 {
			control_ground_velocity(
				&mut fast,
				Vec3::X,
				MOVE_ACCEL,
				1.0 / 240.0,
				Vec3::Y,
				MOVE_SPEED,
			);
		}
		assert!((slow.x - MOVE_SPEED).abs() < 1e-3, "{slow:?}");
		assert!((fast.x - MOVE_SPEED).abs() < 1e-3, "{fast:?}");
	}
}
