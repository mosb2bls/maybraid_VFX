//! Rockadder block shockwave and Cosimo launch flare.

use std::f32::consts::TAU;

use avian3d::prelude::{
	Collider, Gravity, GravityScale, LinearVelocity, SpatialQuery, SpatialQueryFilter,
};
use damage::HitPayload;
use lod_avian::PhysicsInteractionLayer;
use player::{JumpPhase, Jumping};
use projectiles::{ProjectileContact, ProjectileSource};

use bevy::prelude::*;

use crate::user::SkillMapUser;
use crate::{skill_map_dispatch_ready, SkillKind, SkillMapEnabled, SkillMapEvent};

pub const ROCKADDER_RADIUS: f32 = 20.0;
pub const ROCKADDER_DAMAGE: f32 = 32.0;
pub const ROCKADDER_SECS: f32 = 0.7;
pub const COSIMO_LAUNCH_HEIGHT: f32 = 40.0;
pub const COSIMO_BURST_RADIUS: f32 = 10.0;
pub const COSIMO_BURST_DAMAGE: f32 = 28.0;
pub const COSIMO_BURST_SECS: f32 = 0.45;
const DEFAULT_GRAVITY: f32 = 9.81;
const DEFAULT_GRAVITY_SCALE: f32 = 1.25;
const RING_STEP: f32 = 2.0;
const SHARD_LIFE: f32 = 0.42;
const ROCKADDER_SHARD_LIFE: f32 = SHARD_LIFE * 2.0;
const COSIMO_SHARD_LIFE: f32 = SHARD_LIFE * 5.0;
const FEET_DROP: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BurstStyle {
	Rockadder,
	Cosimo,
}

#[derive(Component)]
pub(crate) struct ExpandingBurst {
	style: BurstStyle,
	age: f32,
	max_age: f32,
	max_radius: f32,
	last_ring: i32,
	contacted: Vec<Entity>,
}

#[derive(Component)]
pub(crate) struct BurstShard {
	age: f32,
	max_age: f32,
	start_scale: Vec3,
}

#[derive(Resource)]
pub(crate) struct BurstAssets {
	cube: Handle<Mesh>,
	rock: [Handle<StandardMaterial>; 4],
	cosmos: [Handle<StandardMaterial>; 5],
}

pub(crate) fn setup_burst_assets(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.insert_resource(BurstAssets {
		cube: meshes.add(Cuboid::from_length(0.5)),
		rock: [
			unlit(
				materials.as_mut(),
				Color::srgb(0.62, 0.28, 0.16),
				LinearRgba::new(0.4, 0.14, 0.05, 1.0),
			),
			unlit(
				materials.as_mut(),
				Color::srgb(0.12, 0.38, 0.36),
				LinearRgba::new(0.06, 0.22, 0.2, 1.0),
			),
			unlit(
				materials.as_mut(),
				Color::srgb(0.78, 0.58, 0.22),
				LinearRgba::new(0.7, 0.45, 0.08, 1.0),
			),
			unlit(
				materials.as_mut(),
				Color::srgb(0.86, 0.78, 0.62),
				LinearRgba::new(0.35, 0.28, 0.16, 1.0),
			),
		],
		cosmos: [
			unlit(
				materials.as_mut(),
				Color::srgb(0.72, 0.22, 1.0),
				LinearRgba::new(1.4, 0.25, 2.2, 1.0),
			),
			unlit(
				materials.as_mut(),
				Color::srgb(1.0, 0.45, 0.2),
				LinearRgba::new(2.0, 0.55, 0.12, 1.0),
			),
			unlit(
				materials.as_mut(),
				Color::srgb(0.2, 0.85, 1.0),
				LinearRgba::new(0.2, 1.1, 1.8, 1.0),
			),
			unlit(
				materials.as_mut(),
				Color::srgb(1.0, 0.85, 0.35),
				LinearRgba::new(1.8, 1.2, 0.25, 1.0),
			),
			unlit(
				materials.as_mut(),
				Color::srgb(0.95, 0.4, 0.7),
				LinearRgba::new(1.5, 0.35, 0.9, 1.0),
			),
		],
	});
}

fn unlit(
	materials: &mut Assets<StandardMaterial>,
	base: Color,
	emissive: LinearRgba,
) -> Handle<StandardMaterial> {
	materials.add(StandardMaterial {
		base_color: base,
		emissive,
		unlit: true,
		alpha_mode: AlphaMode::Opaque,
		..default()
	})
}

/// Upward speed that reaches `height` under constant gravity.
pub fn launch_vy(height: f32, gravity_scale: f32) -> f32 {
	(2.0 * DEFAULT_GRAVITY * gravity_scale.max(0.05) * height).sqrt()
}

fn gravity_scale(scale: Option<&GravityScale>, gravity: Option<&Gravity>) -> f32 {
	let g = gravity.map(|gravity| gravity.0.length()).unwrap_or(DEFAULT_GRAVITY);
	let scale = scale.map(|scale| scale.0).unwrap_or(DEFAULT_GRAVITY_SCALE);
	scale * (g / DEFAULT_GRAVITY)
}

pub fn dispatch_rockadders(
	mut commands: Commands,
	enabled: Res<SkillMapEnabled>,
	mut events: MessageReader<SkillMapEvent>,
	users: Query<(Entity, &GlobalTransform), With<SkillMapUser>>,
) {
	if !skill_map_dispatch_ready(&enabled, &mut events) {
		return;
	}
	for event in events.read() {
		let SkillMapEvent::Claim { user, kind: SkillKind::Rockadder } = *event else {
			continue;
		};
		let Ok((player, transform)) = users.get(user) else {
			continue;
		};
		let origin = transform.translation() - Vec3::Y * FEET_DROP;
		spawn_burst(
			&mut commands,
			player,
			origin,
			BurstStyle::Rockadder,
			ROCKADDER_RADIUS,
			ROCKADDER_SECS,
			ROCKADDER_DAMAGE,
		);
	}
}

pub fn dispatch_cosmos(
	mut commands: Commands,
	enabled: Res<SkillMapEnabled>,
	gravity: Option<Res<Gravity>>,
	mut events: MessageReader<SkillMapEvent>,
	users: Query<(Entity, &GlobalTransform, Option<&GravityScale>), With<SkillMapUser>>,
	mut velocities: Query<&mut LinearVelocity>,
) {
	if !skill_map_dispatch_ready(&enabled, &mut events) {
		return;
	}
	let gravity = gravity.as_deref();
	for event in events.read() {
		let SkillMapEvent::Claim { user, kind: SkillKind::Cosimo } = *event else {
			continue;
		};
		let Ok((player, transform, scale)) = users.get(user) else {
			continue;
		};
		let vy = launch_vy(COSIMO_LAUNCH_HEIGHT, gravity_scale(scale, gravity));
		if let Ok(mut velocity) = velocities.get_mut(player) {
			velocity.y = vy;
		}
		commands.entity(player).insert(Jumping {
			phase: JumpPhase::Air,
			left_ground: false,
			leaping: true,
			phase_elapsed: 0.0,
			launch_vy: vy,
		});
		let origin = transform.translation() - Vec3::Y * FEET_DROP;
		spawn_burst(
			&mut commands,
			player,
			origin,
			BurstStyle::Cosimo,
			COSIMO_BURST_RADIUS,
			COSIMO_BURST_SECS,
			COSIMO_BURST_DAMAGE,
		);
	}
}

fn spawn_burst(
	commands: &mut Commands,
	source: Entity,
	origin: Vec3,
	style: BurstStyle,
	max_radius: f32,
	max_age: f32,
	damage: f32,
) {
	let name = match style {
		BurstStyle::Rockadder => "rockadder-burst",
		BurstStyle::Cosimo => "cosimo-flare",
	};
	commands.spawn((
		Name::new(name),
		ExpandingBurst {
			style,
			age: 0.0,
			max_age,
			max_radius,
			last_ring: -1,
			contacted: Vec::new(),
		},
		Transform::from_translation(origin),
		Visibility::default(),
		ProjectileSource(source),
		HitPayload { amount: damage },
	));
}

pub fn tick_bursts(
	time: Res<Time>,
	mut commands: Commands,
	assets: Option<Res<BurstAssets>>,
	mut bursts: Query<(&mut ExpandingBurst, &Transform)>,
) {
	let Some(assets) = assets else {
		return;
	};
	let dt = time.delta_secs();
	for (mut burst, transform) in &mut bursts {
		burst.age += dt;
		let t = (burst.age / burst.max_age).clamp(0.0, 1.0);
		let radius = burst.max_radius * t;
		let ring = (radius / RING_STEP).floor() as i32;
		for next in (burst.last_ring + 1)..=ring {
			spawn_ring(&mut commands, &assets, burst.style, transform.translation, next);
		}
		burst.last_ring = ring;
	}
}

fn spawn_ring(
	commands: &mut Commands,
	assets: &BurstAssets,
	style: BurstStyle,
	origin: Vec3,
	ring: i32,
) {
	if ring < 0 {
		return;
	}
	let radius = ring as f32 * RING_STEP;
	if radius < 0.05 {
		return;
	}
	match style {
		BurstStyle::Rockadder => spawn_rock_ring(commands, assets, origin, radius, ring),
		BurstStyle::Cosimo => spawn_flare_ring(commands, assets, origin, radius, ring),
	}
}

fn spawn_rock_ring(
	commands: &mut Commands,
	assets: &BurstAssets,
	origin: Vec3,
	radius: f32,
	ring: i32,
) {
	let count = (10 + ring * 6).min(28) as u32;
	for i in 0..count {
		let turn = i as f32 / count as f32 * TAU + hash(ring, i) * 0.35;
		let lift = 0.22 + hash(ring.wrapping_add(3), i.wrapping_add(7)) * 0.55;
		let pos = origin + Vec3::new(turn.cos(), 0.0, turn.sin()) * radius + Vec3::Y * lift;
		let yaw = hash(i, ring as u32) * TAU;
		let scale = 0.7 + hash(ring.wrapping_add(11), i.wrapping_add(2)) * 0.7;
		spawn_shard(
			commands,
			assets.cube.clone(),
			assets.rock[(i as usize + ring as usize) % assets.rock.len()].clone(),
			pos,
			Quat::from_rotation_y(yaw),
			Vec3::splat(scale),
			ROCKADDER_SHARD_LIFE,
		);
	}
}

fn spawn_flare_ring(
	commands: &mut Commands,
	assets: &BurstAssets,
	origin: Vec3,
	radius: f32,
	ring: i32,
) {
	let count = (12 + ring * 8).min(32) as u32;
	for i in 0..count {
		let dir = flare_dir(i, count, ring);
		let pos = origin + dir * radius;
		let scale = Vec3::new(0.28, 0.55 + hash(i, ring as u32) * 0.35, 0.28);
		spawn_shard(
			commands,
			assets.cube.clone(),
			assets.cosmos[(i as usize + ring as usize) % assets.cosmos.len()].clone(),
			pos,
			Quat::from_rotation_arc(Vec3::Y, dir),
			scale,
			COSIMO_SHARD_LIFE,
		);
	}
}

fn flare_dir(i: u32, n: u32, ring: i32) -> Vec3 {
	let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
	let y = 1.0 - (i as f32 + 0.5) / n as f32 * 2.0;
	let r = (1.0 - y * y).max(0.0).sqrt();
	let theta = golden * i as f32 + ring as f32 * 0.37;
	let dir = Vec3::new(theta.cos() * r, y, theta.sin() * r);
	(dir + Vec3::NEG_Y * 0.7).normalize_or(Vec3::NEG_Y)
}

fn spawn_shard(
	commands: &mut Commands,
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
	translation: Vec3,
	rotation: Quat,
	scale: Vec3,
	max_age: f32,
) {
	commands.spawn((
		Name::new("skill-burst-shard"),
		BurstShard { age: 0.0, max_age, start_scale: scale },
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform { translation, rotation, scale },
		Visibility::default(),
	));
}

pub fn tick_shards(
	time: Res<Time>,
	mut commands: Commands,
	mut shards: Query<(Entity, &mut BurstShard, &mut Transform)>,
) {
	let dt = time.delta_secs();
	for (entity, mut shard, mut transform) in &mut shards {
		shard.age += dt;
		let life = (shard.age / shard.max_age.max(1e-3)).clamp(0.0, 1.0);
		let fade = ((life - 0.7) / 0.3).clamp(0.0, 1.0);
		transform.scale = shard.start_scale * (1.0 - fade * 0.9);
		if life >= 1.0 {
			commands.entity(entity).despawn();
		}
	}
}

pub fn contact_bursts(
	mut commands: Commands,
	spatial: SpatialQuery,
	mut contacts: MessageWriter<ProjectileContact>,
	mut bursts: Query<(Entity, &Transform, &mut ExpandingBurst, &ProjectileSource)>,
) {
	for (entity, transform, mut burst, source) in &mut bursts {
		let t = (burst.age / burst.max_age.max(1e-3)).clamp(0.0, 1.0);
		let radius = (burst.max_radius * t).max(0.2);
		let filter = SpatialQueryFilter::from_mask([
			PhysicsInteractionLayer::Fixed,
			PhysicsInteractionLayer::Animated,
		])
		.with_excluded_entities([entity, source.0]);
		let hits = spatial.shape_intersections(
			&Collider::sphere(radius),
			transform.translation,
			transform.rotation,
			&filter,
		);
		for target in hits {
			if target == entity || target == source.0 {
				continue;
			}
			if burst.contacted.contains(&target) {
				continue;
			}
			burst.contacted.push(target);
			contacts.write(ProjectileContact {
				projectile: entity,
				source: Some(source.0),
				target,
				point: transform.translation,
				normal: Vec3::Y,
			});
		}
		if burst.age >= burst.max_age {
			commands.entity(entity).despawn();
		}
	}
}

fn hash(a: impl Into<i64>, b: u32) -> f32 {
	let mixed = (a.into() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ u64::from(b);
	let hashed = mixed.wrapping_mul(0x0100_0000_01B3);
	((hashed >> 33) as u32 as f32) / (u32::MAX as f32)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn launch_clears_forty_meters_under_player_gravity() {
		let vy = launch_vy(COSIMO_LAUNCH_HEIGHT, DEFAULT_GRAVITY_SCALE);
		let height = vy * vy / (2.0 * DEFAULT_GRAVITY * DEFAULT_GRAVITY_SCALE);
		assert!((height - COSIMO_LAUNCH_HEIGHT).abs() < 1e-3);
		assert!(vy > 30.0);
	}

	#[test]
	fn burst_radii_match_the_brief() {
		assert!((ROCKADDER_RADIUS - 20.0).abs() < 1e-5);
		assert!((COSIMO_BURST_RADIUS - 10.0).abs() < 1e-5);
		assert!(ROCKADDER_DAMAGE > 0.0 && COSIMO_BURST_DAMAGE > 0.0);
	}

	#[test]
	fn shards_hold_for_the_requested_multiples() {
		assert!((ROCKADDER_SHARD_LIFE - SHARD_LIFE * 2.0).abs() < 1e-5);
		assert!((COSIMO_SHARD_LIFE - SHARD_LIFE * 5.0).abs() < 1e-5);
	}
}
