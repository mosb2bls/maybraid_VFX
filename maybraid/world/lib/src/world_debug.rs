//! Debug commands for testing (debug-commands feature only).
//!
//! Injects ordinary Hit, charge, and activation requests through standard gameplay paths.
//! Peaceful mode suppresses unwanted hits before damage application.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use damage::Hit;
use lubikans::{LubikanActivationRequest, LubikanChargeReason, LubikanChargeRequest};
use player::Npc;
use world_player::Player as WorldPlayer;

use crate::commands::{RequestActivate, RequestCamera, RequestGrantPoints, RequestPeaceful, RequestSpawnNpc, RequestTakeDamage, RequestTeleport, CameraMode};
use crate::map_view::WorldMapView;

/// Debug command plugin (behind debug-commands feature).
///
/// Provides grant-points, take-damage, spawn-npc, activate, peaceful, camera, and teleport
/// commands. All commands inject requests through standard gameplay systems (Hit for damage,
/// LubikanChargeRequest for points, etc.).
pub struct WorldDebugPlugin;

impl Plugin for WorldDebugPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<DebugPeacefulMode>();
		
		app.add_systems(
			Update,
			(
				log_player_lifecycle.before(apply_debug_take_damage),
				apply_debug_grant_points,
				apply_debug_take_damage,
				apply_debug_spawn_npc,
				apply_debug_activate,
				apply_debug_peaceful,
				apply_debug_camera,
				apply_debug_teleport,
			),
		);
		
		app.add_systems(
			PreUpdate,
			suppress_npc_hits_in_peaceful.before(damage::DamageSystems::Apply),
		);
	}
}

/// Peaceful mode: suppresses NPC damage to player by filtering Hit messages before application.
#[derive(Resource, Default)]
pub struct DebugPeacefulMode(pub bool);

fn apply_debug_grant_points(
	requests: Query<(Entity, &RequestGrantPoints)>,
	players: Query<Entity, (With<player::Player>, With<WorldPlayer>)>,
	mut lubikan_requests: MessageWriter<LubikanChargeRequest>,
	mut commands: Commands,
) {
	for (entity, request) in &requests {
		if let Some(player) = players.iter().next() {
			lubikan_requests.write(LubikanChargeRequest::equipped(
				player,
				request.amount,
				LubikanChargeReason::ADJUSTMENT,
				None,
			));
		}
		commands.entity(entity).despawn();
	}
}

fn apply_debug_take_damage(
	requests: Query<(Entity, &RequestTakeDamage)>,
	players: Query<Entity, With<WorldPlayer>>,
	mut hits: MessageWriter<Hit>,
	mut commands: Commands,
) {
	for (entity, request) in &requests {
		if let Some(player_entity) = players.iter().next() {
			hits.write(Hit {
				target: player_entity,
				source: None,
				amount: request.amount,
				point: Vec3::ZERO,
				allow_self: false,
			});
		}
		commands.entity(entity).despawn();
	}
}

fn apply_debug_activate(
	requests: Query<(Entity, &RequestActivate)>,
	players: Query<Entity, (With<player::Player>, With<WorldPlayer>)>,
	mut activation_requests: MessageWriter<LubikanActivationRequest>,
	mut commands: Commands,
) {
	for (entity, _request) in &requests {
		if let Some(player) = players.iter().next() {
			activation_requests.write(LubikanActivationRequest::equipped(player));
		}
		commands.entity(entity).despawn();
	}
}

fn apply_debug_peaceful(
	requests: Query<(Entity, &RequestPeaceful)>,
	mut peaceful: ResMut<DebugPeacefulMode>,
	mut commands: Commands,
) {
	for (entity, _request) in &requests {
		peaceful.0 = !peaceful.0;
		log::info!("Debug peaceful mode: {}", if peaceful.0 { "ON" } else { "OFF" });
		commands.entity(entity).despawn();
	}
}

fn suppress_npc_hits_in_peaceful(
	peaceful: Res<DebugPeacefulMode>,
	players: Query<Entity, With<player::Player>>,
	mut hits: MessageReader<Hit>,
	mut suppressed_count: Local<usize>,
) {
	if !peaceful.0 {
		return;
	}
	
	let Ok(player) = players.single() else {
		return;
	};
	
	let mut hits_vec: Vec<Hit> = hits.read().copied().collect();
	hits_vec.retain(|hit| {
		if hit.target == player && hit.source.is_some() {
			*suppressed_count += 1;
			log::info!("[PEACEFUL] Suppressed NPC hit #{}: {:.1} damage from {:?}", 
				*suppressed_count, hit.amount, hit.source);
			false
		} else {
			true
		}
	});
}

fn apply_debug_spawn_npc(
	requests: Query<(Entity, &RequestSpawnNpc)>,
	players: Query<&GlobalTransform, With<player::Player>>,
	mut commands: Commands,
) {
	for (entity, _) in &requests {
		if let Ok(player_transform) = players.single() {
			let spawn_pos = player_transform.translation() + Vec3::new(2.0, 0.0, 2.0);
			commands.spawn((
				Npc,
				damage::Health { current: 30.0, max: 30.0 },
				GlobalTransform::from_translation(spawn_pos),
				Transform::from_translation(spawn_pos),
				Name::new("Debug NPC"),
			));
		}
		commands.entity(entity).despawn();
	}
}

fn apply_debug_camera(
	requests: Query<(Entity, &RequestCamera)>,
	mut map_view: ResMut<WorldMapView>,
	mut commands: Commands,
) {
	for (entity, request) in &requests {
		match request.mode {
			CameraMode::ThirdPerson => {
				map_view.close();
			}
		}
		commands.entity(entity).despawn();
	}
}

fn apply_debug_teleport(
	requests: Query<(Entity, &RequestTeleport)>,
	mut players: Query<(&mut Transform, &mut LinearVelocity), (With<player::Player>, With<WorldPlayer>)>,
	mut commands: Commands,
) {
	for (entity, request) in &requests {
		if let Some((mut transform, mut velocity)) = players.iter_mut().next() {
			transform.translation = request.position;
			**velocity = Vec3::ZERO;
		}
		commands.entity(entity).despawn();
	}
}

fn log_player_lifecycle(
	frame: Res<Time<()>>,
	added: Query<Entity, Added<WorldPlayer>>,
	mut removed: RemovedComponents<WorldPlayer>,
	existing: Query<Entity, With<WorldPlayer>>,
) {
	// Logging disabled (auto-capture removed)
	
	let frame_count = frame.elapsed().as_secs_f32() * 60.0;
	if frame_count >= 100.0 && frame_count <= 180.0 {
		for entity in &added {
			eprintln!("[LIFECYCLE] frame~{:.0} Added<WorldPlayer>: {:?}", frame_count, entity);
		}
		for entity in removed.read() {
			eprintln!("[LIFECYCLE] frame~{:.0} RemovedComponents<WorldPlayer>: {:?}", frame_count, entity);
		}
		if existing.is_empty() {
			eprintln!("[LIFECYCLE] frame~{:.0} No WorldPlayer entities exist!", frame_count);
		}
	}
}
