//! Fireball flights and the Dumbwave forget-pulse.

use combat_targeting::CombatTargeting;
use damage::HitPayload;
use evasion_intelligence::EvasionIntelligenceUser;
use player::PlayerLook;
use projectiles::{spawn_flight, ProjectileSource, ProjectileVisualCache};
use threat_management_intelligence::{
	SkillDaze, ThreatManagementIntelligence, ThreatTactic, ThreatTacticChanged,
};

use bevy::prelude::*;

use crate::fireball_embers::FireballEffects;
use crate::fireball_material::FireballMaterial;
use crate::user::SkillMapUser;
use crate::{skill_map_dispatch_ready, SkillKind, SkillMapEnabled, SkillMapEvent};

pub const FIREBALL_LENGTH: f32 = 0.12;
pub const FIREBALL_RADIUS: f32 = 1.0;
pub const FIREBALL_SPEED: f32 = 30.0;
pub const FIREBALL_MAX_RANGE: f32 = 160.0;
pub const FIREBALL_MAX_THROUGH: f32 = 1.6;
pub const FIREBALL_MAX_AGE: f32 = 5.0;
pub const FIREBALL_GRAVITY: f32 = 1.0;
pub const FIREBALL_COLOR: Color = Color::srgb(1.0, 0.28, 0.08);
pub const FIREBALL_DAMAGE: f32 = 35.0;
const FIREBALL_MUZZLE_HEIGHT: f32 = 1.35;
const FIREBALL_MUZZLE_FORWARD: f32 = 0.55;

pub const DUMBWAVE_RADIUS: f32 = 18.0;
pub const DUMBWAVE_DAZE_SECS: f32 = 4.0;
pub const DUMBWAVE_PULSE_SECS: f32 = 0.55;

#[derive(Component)]
pub(crate) struct DumbwavePulse {
	age: f32,
	max_age: f32,
}

/// Distance-weighted chance a nearby combatant forgets the player.
pub fn forget_chance(distance: f32, radius: f32) -> f32 {
	if radius <= 1e-5 {
		return 0.0;
	}
	let t = (distance / radius).clamp(0.0, 1.0);
	(0.75 * (1.0 - t)).max(0.18)
}

pub fn look_forward(look: PlayerLook) -> Vec3 {
	let rotation =
		Quat::from_axis_angle(Vec3::Y, look.yaw) * Quat::from_axis_angle(Vec3::X, look.pitch);
	rotation * -Vec3::Z
}

pub fn dispatch_fireballs(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut lit: ResMut<Assets<StandardMaterial>>,
	mut fire: ResMut<Assets<FireballMaterial>>,
	mut visuals: ResMut<ProjectileVisualCache>,
	effects: Option<Res<FireballEffects>>,
	enabled: Res<SkillMapEnabled>,
	mut events: MessageReader<SkillMapEvent>,
	users: Query<(Entity, &GlobalTransform, Option<&PlayerLook>), With<SkillMapUser>>,
) {
	if !skill_map_dispatch_ready(&enabled, &mut events) {
		return;
	}
	for event in events.read() {
		let SkillMapEvent::Claim { user, kind: SkillKind::Fireball } = *event else {
			continue;
		};
		let Ok((player, transform, look)) = users.get(user) else {
			continue;
		};
		let direction = look
			.map(|look| look_forward(*look))
			.unwrap_or(Vec3::NEG_Z)
			.normalize_or(-Vec3::Z);
		let muzzle = transform.translation()
			+ Vec3::Y * FIREBALL_MUZZLE_HEIGHT
			+ direction * FIREBALL_MUZZLE_FORWARD;
		let projectile = spawn_flight(
			&mut commands,
			&mut meshes,
			&mut lit,
			&mut visuals,
			muzzle,
			direction,
			FIREBALL_LENGTH,
			FIREBALL_RADIUS,
			FIREBALL_SPEED,
			FIREBALL_MAX_RANGE,
			FIREBALL_MAX_THROUGH,
			FIREBALL_MAX_AGE,
			FIREBALL_COLOR,
			FIREBALL_GRAVITY,
		);
		dress_fireball(
			&mut commands,
			&mut meshes,
			&mut fire,
			effects.as_deref(),
			projectile,
			muzzle,
		);
		commands.entity(projectile).insert((
			ProjectileSource(player),
			HitPayload { amount: FIREBALL_DAMAGE },
			Name::new("skill-fireball"),
		));
	}
}

fn dress_fireball(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<FireballMaterial>,
	effects: Option<&FireballEffects>,
	projectile: Entity,
	origin: Vec3,
) {
	let (mesh, material) = match effects {
		Some(effects) => (effects.mesh.clone(), effects.material.clone()),
		None => (
			meshes.add(crate::fireball_material::fireball_visual_mesh()),
			materials.add(FireballMaterial::new(1.0)),
		),
	};
	commands.entity(projectile).insert((Mesh3d(mesh), MeshMaterial3d(material)));
	commands.entity(projectile).remove::<MeshMaterial3d<StandardMaterial>>();
	crate::fireball_trail::dress_trail(commands, projectile, origin);
}

type DumbwaveManagers<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static GlobalTransform,
		&'static mut ThreatManagementIntelligence,
		Option<&'static mut CombatTargeting>,
		Option<&'static mut EvasionIntelligenceUser>,
	),
>;

#[allow(clippy::too_many_arguments)]
pub fn dispatch_dumbwaves(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	time: Res<Time>,
	enabled: Res<SkillMapEnabled>,
	mut events: MessageReader<SkillMapEvent>,
	mut changed: MessageWriter<ThreatTacticChanged>,
	users: Query<(Entity, &GlobalTransform), With<SkillMapUser>>,
	mut managers: DumbwaveManagers,
) {
	if !skill_map_dispatch_ready(&enabled, &mut events) {
		return;
	}
	let now = time.elapsed_secs();
	for event in events.read() {
		let SkillMapEvent::Claim { user: player, kind: SkillKind::Dumbwave } = *event else {
			continue;
		};
		let Ok((_, origin)) = users.get(player) else {
			continue;
		};
		let origin = origin.translation();
		spawn_pulse(&mut commands, &mut meshes, &mut materials, origin);
		for (entity, transform, mut management, mut targeting, mut evasion) in &mut managers {
			if entity == player {
				continue;
			}
			let offset = transform.translation() - origin;
			let distance = Vec2::new(offset.x, offset.z).length();
			if distance > DUMBWAVE_RADIUS {
				continue;
			}
			if !tracks_player(player, targeting.as_deref(), evasion.as_deref(), management.tactic) {
				continue;
			}
			if roll(entity, now, distance) >= forget_chance(distance, DUMBWAVE_RADIUS) {
				continue;
			}
			commands.entity(entity).insert(SkillDaze::for_secs(now, DUMBWAVE_DAZE_SECS));
			if management.tactic != ThreatTactic::Ignore {
				let from = management.tactic;
				management.generation = management.generation.wrapping_add(1).max(1);
				management.tactic = ThreatTactic::Ignore;
				changed.write(ThreatTacticChanged {
					entity,
					from,
					to: ThreatTactic::Ignore,
					generation: management.generation,
				});
			}
			if let Some(targeting) = targeting.as_deref_mut() {
				targeting.enabled = false;
				targeting.clear_source(combat_targeting::TargetSource::ENEMYSHIP);
				targeting.clear_source(combat_targeting::TargetSource::SPOTTING);
				targeting.ranked.clear();
				targeting.clear_engagement();
			}
			if let Some(evasion) = evasion.as_deref_mut() {
				evasion.enabled = false;
				evasion.clear_source(evasion_intelligence::AssailantSource::ENEMYSHIP);
				evasion.clear_source(evasion_intelligence::AssailantSource::SPOTTING);
				evasion.ranked.clear();
				evasion.signal = evasion_intelligence::EvasionSignal::idle();
			}
		}
	}
}

fn tracks_player(
	player: Entity,
	targeting: Option<&CombatTargeting>,
	evasion: Option<&EvasionIntelligenceUser>,
	tactic: ThreatTactic,
) -> bool {
	let combat = targeting
		.is_some_and(|targeting| targeting.enabled && targeting.active_target(player).is_some());
	let evade = evasion
		.is_some_and(|evasion| evasion.enabled && evasion.active_assailant(player).is_some());
	combat || evade || matches!(tactic, ThreatTactic::Combat | ThreatTactic::Evade)
}

fn roll(entity: Entity, now: f32, distance: f32) -> f32 {
	let mixed = entity.to_bits() ^ now.to_bits() as u64 ^ distance.to_bits() as u64;
	let hashed = mixed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
	((hashed >> 33) as u32 as f32) / (u32::MAX as f32)
}

fn spawn_pulse(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	origin: Vec3,
) {
	commands.spawn((
		Name::new("dumbwave-pulse"),
		DumbwavePulse { age: 0.0, max_age: DUMBWAVE_PULSE_SECS },
		Mesh3d(meshes.add(Annulus::new(0.35, 0.55))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: Color::srgba(0.35, 0.92, 1.0, 0.82),
			emissive: LinearRgba::new(0.45, 1.3, 1.5, 1.0),
			alpha_mode: AlphaMode::Blend,
			unlit: true,
			cull_mode: None,
			..default()
		})),
		Transform {
			translation: origin + Vec3::Y * 0.12,
			rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
			scale: Vec3::splat(0.6),
		},
		Visibility::default(),
	));
}

pub fn tick_pulses(
	time: Res<Time>,
	mut commands: Commands,
	mut pulses: Query<(
		Entity,
		&mut DumbwavePulse,
		&mut Transform,
		&mut MeshMaterial3d<StandardMaterial>,
	)>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let dt = time.delta_secs();
	for (entity, mut pulse, mut transform, material) in &mut pulses {
		pulse.age += dt;
		let t = (pulse.age / pulse.max_age).clamp(0.0, 1.0);
		transform.scale = Vec3::splat(0.6 + t * DUMBWAVE_RADIUS * 2.0);
		if let Some(mut material) = materials.get_mut(&material.0) {
			let alpha = 0.82 * (1.0 - t);
			material.base_color = Color::srgba(0.35, 0.92, 1.0, alpha);
		}
		if pulse.age >= pulse.max_age {
			commands.entity(entity).despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forget_chance_falls_with_distance() {
		assert!((forget_chance(0.0, 18.0) - 0.75).abs() < 1e-5);
		assert!(forget_chance(9.0, 18.0) < forget_chance(0.0, 18.0));
		assert!((forget_chance(18.0, 18.0) - 0.18).abs() < 1e-5);
	}

	#[test]
	fn look_forward_matches_camera_basis() {
		let look = PlayerLook { yaw: 0.0, pitch: 0.0, ..default() };
		let forward = look_forward(look);
		assert!((forward - Vec3::NEG_Z).length() < 1e-5);
	}

	#[test]
	fn fireball_material_is_shared_fill() {
		use crate::fireball_material::FireballMaterial;

		let material = FireballMaterial::new(1.0);
		assert_eq!(material.base_color.x, 1.0);
	}
}
