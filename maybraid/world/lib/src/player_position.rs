//! Player position log and retained Discovery waypoints keyed by [`CharacterId`].

use std::fs;
use std::time::Duration;

use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use character_persist::{CharacterId, PersistError, SaveRoot};
use durham::{Durham, TerrainCellLayout, WorldBaseTerrain};
use player_camera::FollowCamera;
use serde::{Deserialize, Serialize};
use terrain_layer_model::{terrain_streaming, TerrainStreaming};
use world_player::player::{holding_elevation, player_spawn_point_at};
use world_player::{AwaitingTerrainSurface, ModePlayerPolicies, Player};

use layer_stack::ActiveGenerationMode;

use crate::{PlayerSpawnXz, WorldPlayerLoadout};

const LOG_INTERVAL: Duration = Duration::from_secs(10);
const WAYPOINT_GAP_M: f32 = 500.0;
const WAYPOINT_CAP: usize = 5;
const FILE_VERSION: u32 = 1;
/// Treat two samples as the same place. The layout-center holding pose is exact;
/// this only absorbs float noise.
const SAME_PLACE_M: f32 = 1.0;

/// Last [`WAYPOINT_CAP`] player positions that differ by more than [`WAYPOINT_GAP_M`].
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PlayerPositionWaypoints {
	pub owner: Option<CharacterId>,
	pub positions: Vec<Vec3>,
	/// True after a character's file is read. The leftover body from the
	/// previous character must not be written back onto this trail.
	pub(crate) just_loaded: bool,
	/// True while a loaded trail is waiting for the leftover body to arrive.
	/// Retain and the first-life picker honor this even if `just_loaded` was
	/// cleared by a same-frame write.
	pub(crate) resume_pending: bool,
}

impl PlayerPositionWaypoints {
	/// True when the file has a sample that is not the layout-center spawn.
	/// Those characters resume; they must not get a first-life picker.
	pub fn has_resume_trail(&self, default_spawn: Vec3) -> bool {
		resume_for_new_character(default_spawn, &self.positions)
			.is_some_and(|at| at.distance(default_spawn) > SAME_PLACE_M)
	}
}

#[derive(Serialize, Deserialize)]
struct WaypointFile {
	version: u32,
	id: CharacterId,
	waypoints: Vec<[f32; 3]>,
}

pub struct PlayerPositionPlugin;

impl Plugin for PlayerPositionPlugin {
	fn build(&self, app: &mut App) {
		if !app.world().contains_resource::<SaveRoot>() {
			app.insert_resource(SaveRoot::workspace());
		}
		app.init_resource::<PlayerPositionWaypoints>().add_systems(
			Update,
			(
				sync_waypoints_for_character,
				retain_player_waypoints,
				log_player_position
					.run_if(terrain_streaming::<Durham>)
					.run_if(on_timer(LOG_INTERVAL)),
			)
				.chain(),
		);
	}
}

fn current_character_id(loadout: Option<&WorldPlayerLoadout>) -> Option<CharacterId> {
	loadout.and_then(|loadout| CharacterId::from_hex(&loadout.key))
}

fn sync_waypoints_for_character(
	save_root: Res<SaveRoot>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	mut waypoints: ResMut<PlayerPositionWaypoints>,
) {
	let Some(id) = current_character_id(loadout.as_deref()) else {
		return;
	};
	if waypoints.owner == Some(id) {
		return;
	}
	*waypoints = load_waypoints(&save_root, id);
	if !waypoints.positions.is_empty() {
		info!(
			target: "world.player",
			"retained {} waypoint(s) for {}: {}",
			waypoints.positions.len(),
			id.to_hex(),
			format_waypoints(&waypoints.positions)
		);
	}
}

/// Place Discovery on the character's saved trail before the first streaming frame.
///
/// [`WorldPlayerLoadout`] arrives on the loading-screen transition, which is after
/// [`PreUpdate`] and before [`Update`]. Terrain fill reads the camera that same
/// [`Update`], so this has to run from `OnEnter` — the waypoint sync in [`Update`]
/// is too late, and [`retain_player_waypoints`] would otherwise store the layout-center
/// spawn ahead of the file.
pub fn resume_discovery_from_saved_waypoints(
	spawn: Res<PlayerSpawnXz>,
	save_root: Res<SaveRoot>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	mode: Option<Res<State<ActiveGenerationMode>>>,
	policies: Option<Res<ModePlayerPolicies>>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut commands: Commands,
	mut waypoints: ResMut<PlayerPositionWaypoints>,
	mut players: Query<
		(
			Entity,
			&mut Transform,
			Option<&mut GlobalTransform>,
			Option<&mut Position>,
			Option<&mut LinearVelocity>,
		),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	if spawn.0.is_some() {
		return;
	}
	if !mode_keeps_waypoints(mode.as_deref(), policies.as_deref()) {
		return;
	}
	let Some(id) = current_character_id(loadout.as_deref()) else {
		return;
	};
	let character_changed = waypoints.owner != Some(id);
	if character_changed {
		*waypoints = load_waypoints(&save_root, id);
		if !waypoints.positions.is_empty() {
			info!(
				target: "world.player",
				"loaded {} waypoint(s) for {}: {}",
				waypoints.positions.len(),
				id.to_hex(),
				format_waypoints(&waypoints.positions)
			);
		}
	}

	let player_count = players.iter().len();
	if player_count != 1 {
		warn!(
			target: "world.player",
			"resume expected one player body, found {player_count}"
		);
	}
	let Some((player, mut player_tf, mut player_global, mut body, mut velocity)) =
		players.iter_mut().next()
	else {
		return;
	};
	let center = layout.region_center_xz().xz();
	let default_spawn =
		player_spawn_point_at(center, holding_elevation(&base.0, center.x, center.y));
	let at_default = player_tf.translation.distance(default_spawn) <= SAME_PLACE_M;
	let trimmed =
		at_default && trim_trailing_default_spawn(&mut waypoints.positions, default_spawn);
	let resume_at = resume_translation(
		player_tf.translation,
		default_spawn,
		&waypoints.positions,
		character_changed,
	);
	if trimmed {
		if let Err(error) = save_waypoints(&save_root, id, &waypoints.positions) {
			warn!(target: "world.player", "failed to drop default-spawn waypoint: {error}");
		}
	}
	info!(
		target: "world.player",
		"resume character_changed={character_changed} player=({:.1},{:.1},{:.1}) default=({:.1},{:.1},{:.1}) trail={} -> {resume_at:?}",
		player_tf.translation.x,
		player_tf.translation.y,
		player_tf.translation.z,
		default_spawn.x,
		default_spawn.y,
		default_spawn.z,
		waypoints.positions.len()
	);
	if let Some(at) = resume_at {
		info!(
			target: "world.player",
			"resumed discovery at ({:.1}, {:.1}, {:.1})",
			at.x,
			at.y,
			at.z
		);
		player_tf.translation = at;
		if let Some(global) = player_global.as_deref_mut() {
			*global = GlobalTransform::from(*player_tf);
		} else {
			commands.entity(player).insert(GlobalTransform::from(*player_tf));
		}
		if let Some(position) = body.as_deref_mut() {
			position.0 = at;
		}
		if let Some(velocity) = velocity.as_deref_mut() {
			velocity.0 = Vec3::ZERO;
		}
		// Leftover bodies and persisted underground samples need a first-drop
		// snap. Same-character resume from the default spawn already has this
		// marker from startup; a character switch does not.
		commands.entity(player).insert(AwaitingTerrainSurface);
	}
	let player_at = player_tf.translation;
	drop((player_tf, player_global, body, velocity));

	let Ok((mut camera_tf, mut camera_global, follow)) = cameras.single_mut() else {
		return;
	};
	let parked = park_camera_on_player(player_at, follow);
	*camera_tf = parked;
	*camera_global = GlobalTransform::from(parked);
}

/// Where to put a body that is still on the layout-center spawn, or any leftover
/// body after the active character changed in the same process.
fn resume_translation(
	player: Vec3,
	default_spawn: Vec3,
	saved: &[Vec3],
	character_changed: bool,
) -> Option<Vec3> {
	if character_changed {
		return resume_for_new_character(default_spawn, saved);
	}
	if player.distance(default_spawn) > SAME_PLACE_M {
		return None;
	}
	let at = saved.last().copied()?;
	if at.distance(player) <= SAME_PLACE_M {
		return None;
	}
	Some(at)
}

/// Leave + pick another character leaves the previous body in the world. Always
/// move that leftover pose onto this character's trail (or back to layout center).
fn resume_for_new_character(default_spawn: Vec3, saved: &[Vec3]) -> Option<Vec3> {
	let mut positions = saved.to_vec();
	trim_trailing_default_spawn(&mut positions, default_spawn);
	match positions.last().copied() {
		Some(at) if at.distance(default_spawn) > SAME_PLACE_M => Some(at),
		_ => Some(default_spawn),
	}
}

/// Drop a trailing layout-center sample. Discovery used to record that pose before
/// the file was read, so it sits at the end of an otherwise real trail.
fn trim_trailing_default_spawn(positions: &mut Vec<Vec3>, default_spawn: Vec3) -> bool {
	let mut trimmed = false;
	while positions.len() > 1
		&& positions
			.last()
			.is_some_and(|point| point.distance(default_spawn) <= SAME_PLACE_M)
	{
		positions.pop();
		trimmed = true;
	}
	trimmed
}

fn park_camera_on_player(player: Vec3, follow: &FollowCamera) -> Transform {
	let look = player + Vec3::Y * follow.look_height;
	let eye = look + Vec3::new(-follow.distance, follow.height, 0.0);
	Transform::from_translation(eye).looking_at(look, Vec3::Y)
}

fn log_player_position(players: Query<&Transform, With<Player>>) {
	let Ok(transform) = players.single() else {
		return;
	};
	let p = transform.translation;
	info!(target: "world.player", "position ({:.1}, {:.1}, {:.1})", p.x, p.y, p.z);
}

fn mode_keeps_waypoints(
	mode: Option<&State<ActiveGenerationMode>>,
	policies: Option<&ModePlayerPolicies>,
) -> bool {
	let Some(policies) = policies else {
		return true;
	};
	let mode = mode.and_then(|mode| mode.get().mode_id());
	policies.keeps_waypoints(mode)
}

fn retain_player_waypoints(
	streaming: Res<TerrainStreaming<Durham>>,
	mode: Option<Res<State<ActiveGenerationMode>>>,
	policies: Option<Res<ModePlayerPolicies>>,
	save_root: Res<SaveRoot>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	players: Query<&Transform, With<Player>>,
	mut waypoints: ResMut<PlayerPositionWaypoints>,
) {
	if !streaming.enabled || !mode_keeps_waypoints(mode.as_deref(), policies.as_deref()) {
		return;
	}
	let Ok(transform) = players.single() else {
		return;
	};
	if waypoints.just_loaded || waypoints.resume_pending {
		if waypoints
			.positions
			.last()
			.is_some_and(|last| transform.translation.xz().distance(last.xz()) > SAME_PLACE_M)
		{
			return;
		}
		waypoints.just_loaded = false;
		waypoints.resume_pending = false;
	}
	if !record_waypoint(&mut waypoints.positions, transform.translation) {
		return;
	}
	let Some(id) = current_character_id(loadout.as_deref()) else {
		return;
	};
	waypoints.owner = Some(id);
	if let Err(error) = save_waypoints(&save_root, id, &waypoints.positions) {
		warn!(target: "world.player", "failed to persist player waypoints: {error}");
		return;
	}
	info!(
		target: "world.player",
		"retained waypoint ({:.1}, {:.1}, {:.1}) ({}/{})",
		transform.translation.x,
		transform.translation.y,
		transform.translation.z,
		waypoints.positions.len(),
		WAYPOINT_CAP
	);
}

/// Push `at` when it is the first sample or more than [`WAYPOINT_GAP_M`] from the last.
/// Returns whether the list changed.
pub fn record_waypoint(positions: &mut Vec<Vec3>, at: Vec3) -> bool {
	if let Some(last) = positions.last() {
		if last.xz().distance(at.xz()) <= WAYPOINT_GAP_M {
			return false;
		}
	}
	positions.push(at);
	while positions.len() > WAYPOINT_CAP {
		positions.remove(0);
	}
	true
}

fn format_waypoints(positions: &[Vec3]) -> String {
	positions
		.iter()
		.map(|p| format!("({:.1},{:.1},{:.1})", p.x, p.y, p.z))
		.collect::<Vec<_>>()
		.join(" ")
}

fn load_waypoints(root: &SaveRoot, id: CharacterId) -> PlayerPositionWaypoints {
	let path = root.position_path(id);
	let Ok(json) = fs::read_to_string(&path) else {
		return PlayerPositionWaypoints { owner: Some(id), just_loaded: true, ..default() };
	};
	let Ok(file) = serde_json::from_str::<WaypointFile>(&json) else {
		warn!(target: "world.player", "ignoring unreadable {}", path.display());
		return PlayerPositionWaypoints { owner: Some(id), just_loaded: true, ..default() };
	};
	if file.version != FILE_VERSION {
		return PlayerPositionWaypoints { owner: Some(id), just_loaded: true, ..default() };
	}
	let positions: Vec<Vec3> =
		file.waypoints.into_iter().map(|[x, y, z]| Vec3::new(x, y, z)).collect();
	let resume_pending = !positions.is_empty();
	PlayerPositionWaypoints { owner: Some(id), positions, just_loaded: true, resume_pending }
}

fn save_waypoints(
	root: &SaveRoot,
	id: CharacterId,
	positions: &[Vec3],
) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let file = WaypointFile {
		version: FILE_VERSION,
		id,
		waypoints: positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
	};
	fs::write(root.position_path(id), serde_json::to_string_pretty(&file)?)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use layer_stack::GenerationMode;

	use super::*;

	struct QuietMode;

	impl GenerationMode for QuietMode {}

	#[test]
	fn waypoints_keep_five_far_samples() {
		let mut positions = Vec::new();
		assert!(record_waypoint(&mut positions, Vec3::ZERO));
		assert!(!record_waypoint(&mut positions, Vec3::new(100.0, 0.0, 0.0)));
		assert!(record_waypoint(&mut positions, Vec3::new(600.0, 1.0, 0.0)));
		for i in 2..6 {
			assert!(record_waypoint(&mut positions, Vec3::new(600.0 * i as f32, 0.0, 0.0)));
		}
		assert_eq!(positions.len(), 5);
		assert_eq!(positions[0].x, 600.0);
		assert_eq!(positions[4].x, 3000.0);
	}

	#[test]
	fn a_mode_that_drops_waypoints_does_not_persist_a_pose() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		let mut world = World::new();
		world.insert_resource(root.clone());
		world.insert_resource(TerrainStreaming::<Durham>::new(true));
		let mut policies = ModePlayerPolicies::default();
		policies.register(
			std::any::TypeId::of::<QuietMode>(),
			world_player::ModePlayerPolicy {
				home: Vec2::ZERO,
				keep_waypoints: false,
				respawn_ends_life: false,
				pick_first_spawn: false,
			},
		);
		world.insert_resource(policies);
		world.insert_resource(State::new(ActiveGenerationMode::of::<QuietMode>()));
		world.insert_resource(PlayerPositionWaypoints::default());
		world.insert_resource(crate::WorldPlayerLoadout::new(
			id.to_hex(),
			characters::CharacterAppearance::default(),
			character_items::Inventory::default(),
		));
		world.spawn((world_player::Player, Transform::from_xyz(3.0, 4.0, 5.0)));
		world
			.run_system_once(retain_player_waypoints)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!root.position_path(id).exists());
		assert!(world.resource::<PlayerPositionWaypoints>().positions.is_empty());
		Ok(())
	}

	#[test]
	fn streaming_off_does_not_persist_a_pose() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(9);
		let mut world = World::new();
		world.insert_resource(root.clone());
		world.insert_resource(TerrainStreaming::<Durham>::new(false));
		world.insert_resource(PlayerPositionWaypoints::default());
		world.insert_resource(crate::WorldPlayerLoadout::new(
			id.to_hex(),
			characters::CharacterAppearance::default(),
			character_items::Inventory::default(),
		));
		world.spawn((world_player::Player, Transform::from_xyz(3.0, 4.0, 5.0)));
		world
			.run_system_once(retain_player_waypoints)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!root.position_path(id).exists());
		assert!(world.resource::<PlayerPositionWaypoints>().positions.is_empty());
		Ok(())
	}

	#[test]
	fn resume_uses_the_last_saved_point_from_the_default_spawn() {
		let default_spawn = Vec3::new(0.0, 176.4, 0.0);
		let saved = Vec3::new(-1369.0, 147.9, -568.0);
		let positions = vec![saved];
		assert_eq!(
			resume_translation(default_spawn, default_spawn, &positions, false),
			Some(saved)
		);
	}

	#[test]
	fn resume_steps_back_over_a_trailing_default_spawn_sample() {
		let default_spawn = Vec3::new(0.0, 176.4, 0.0);
		let earlier = Vec3::new(900.0, 92.1, 1400.0);
		let mut positions = vec![earlier, default_spawn];
		assert!(trim_trailing_default_spawn(&mut positions, default_spawn));
		assert_eq!(positions, vec![earlier]);
		assert_eq!(
			resume_translation(default_spawn, default_spawn, &positions, false),
			Some(earlier)
		);
	}

	#[test]
	fn resume_leaves_a_body_that_already_left_the_default_spawn() {
		let default_spawn = Vec3::ZERO;
		let positions = vec![Vec3::new(800.0, 1.0, 0.0), default_spawn];
		let player = Vec3::new(40.0, 1.0, 0.0);
		assert!(resume_translation(player, default_spawn, &positions, false).is_none());
	}

	#[test]
	fn a_character_switch_moves_off_the_previous_body() {
		let default_spawn = Vec3::new(0.0, 176.4, 0.0);
		let leftover = Vec3::new(2178.0, 36.6, 1383.0);
		let next = Vec3::new(695.0, 73.0, 959.0);
		assert_eq!(
			resume_translation(leftover, default_spawn, &[default_spawn, next], true),
			Some(next)
		);
	}

	#[test]
	fn a_saved_trail_is_a_resume_trail() {
		let default_spawn = Vec3::new(0.0, 176.4, 0.0);
		let trail = PlayerPositionWaypoints {
			positions: vec![Vec3::new(695.6, 73.0, 959.5)],
			just_loaded: true,
			..default()
		};
		assert!(trail.has_resume_trail(default_spawn));
		let only_default = PlayerPositionWaypoints {
			positions: vec![default_spawn],
			just_loaded: true,
			..default()
		};
		assert!(!only_default.has_resume_trail(default_spawn));
		assert!(!PlayerPositionWaypoints::default().has_resume_trail(default_spawn));
	}

	#[test]
	fn a_character_switch_resets_a_fresh_character_to_the_default_spawn() {
		let default_spawn = Vec3::new(0.0, 176.4, 0.0);
		let leftover = Vec3::new(2178.0, 36.6, 1383.0);
		assert_eq!(
			resume_translation(leftover, default_spawn, &[default_spawn], true),
			Some(default_spawn)
		);
		assert_eq!(resume_translation(leftover, default_spawn, &[], true), Some(default_spawn));
	}

	#[test]
	fn resume_system_moves_a_leftover_body_onto_the_next_character() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let previous = CharacterId(1);
		let next = CharacterId(2);
		let leftover = Vec3::new(2178.0, 36.6, 1383.0);
		let saved = Vec3::new(695.6, 73.0, 959.5);
		save_waypoints(&root, next, &[saved])?;

		let mut world = World::new();
		world.insert_resource(PlayerSpawnXz(None));
		world.insert_resource(root);
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(durham::BaseTerrainNoise::from_config(
			&durham::TerrainConfig::new(42),
		)));
		world.insert_resource(crate::WorldPlayerLoadout::new(
			next.to_hex(),
			characters::CharacterAppearance::default(),
			character_items::Inventory::default(),
		));
		world.insert_resource(PlayerPositionWaypoints {
			owner: Some(previous),
			positions: vec![leftover],
			just_loaded: false,
			..default()
		});
		let player = world
			.spawn((
				world_player::Player,
				Transform::from_translation(leftover),
				GlobalTransform::from_translation(leftover),
			))
			.id();
		world.spawn((
			Camera3d::default(),
			FollowCamera::default(),
			Transform::default(),
			GlobalTransform::default(),
		));
		world
			.run_system_once(resume_discovery_from_saved_waypoints)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let transform = world
			.get::<Transform>(player)
			.ok_or_else(|| anyhow::anyhow!("player transform"))?;
		assert_eq!(transform.translation.xz(), saved.xz());
		assert!(world.get::<AwaitingTerrainSurface>(player).is_some());
		assert_eq!(world.resource::<PlayerPositionWaypoints>().owner, Some(next));
		Ok(())
	}

	#[test]
	fn waypoint_file_round_trips_beside_the_character() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(42);
		let positions = vec![Vec3::new(1.0, 2.0, 3.0), Vec3::new(800.0, 4.0, 9.0)];
		save_waypoints(&root, id, &positions)?;
		let loaded = load_waypoints(&root, id);
		assert_eq!(loaded.owner, Some(id));
		assert_eq!(loaded.positions, positions);
		assert!(loaded.just_loaded);
		assert_eq!(
			root.position_path(id),
			dir.path().join("positions").join(format!("{}.json", id.to_hex()))
		);
		Ok(())
	}

	fn retain_world(
		root: SaveRoot,
		id: CharacterId,
		waypoints: PlayerPositionWaypoints,
		at: Vec3,
	) -> World {
		let mut world = World::new();
		world.insert_resource(root);
		world.insert_resource(TerrainStreaming::<Durham>::new(true));
		world.insert_resource(waypoints);
		world.insert_resource(crate::WorldPlayerLoadout::new(
			id.to_hex(),
			characters::CharacterAppearance::default(),
			character_items::Inventory::default(),
		));
		world.spawn((world_player::Player, Transform::from_translation(at)));
		world
	}

	#[test]
	fn retain_does_not_write_a_leftover_body_onto_a_just_loaded_trail() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(3);
		let saved = Vec3::new(2178.3, 36.6, 1383.6);
		let leftover = Vec3::new(-49.7, 78.3, 50.0);
		save_waypoints(&root, id, &[saved])?;
		let mut world = retain_world(
			root.clone(),
			id,
			PlayerPositionWaypoints {
				owner: Some(id),
				positions: vec![saved],
				just_loaded: true,
				resume_pending: true,
				..default()
			},
			leftover,
		);
		world
			.run_system_once(retain_player_waypoints)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(retain_player_waypoints)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let waypoints = world.resource::<PlayerPositionWaypoints>();
		assert_eq!(waypoints.positions, vec![saved]);
		assert!(waypoints.just_loaded);
		assert!(waypoints.resume_pending);
		assert_eq!(load_waypoints(&root, id).positions, vec![saved]);
		Ok(())
	}

	#[test]
	fn retain_clears_just_loaded_once_the_body_is_on_the_trail() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(4);
		let saved = Vec3::new(695.6, 73.0, 959.5);
		save_waypoints(&root, id, &[saved])?;
		let mut world = retain_world(
			root,
			id,
			PlayerPositionWaypoints {
				owner: Some(id),
				positions: vec![saved],
				just_loaded: true,
				resume_pending: true,
				..default()
			},
			saved,
		);
		world
			.run_system_once(retain_player_waypoints)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let waypoints = world.resource::<PlayerPositionWaypoints>();
		assert!(!waypoints.just_loaded);
		assert!(!waypoints.resume_pending);
		assert_eq!(waypoints.positions, vec![saved]);
		Ok(())
	}

	#[test]
	fn resume_moves_a_leftover_body_that_has_no_global_transform() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let previous = CharacterId(5);
		let next = CharacterId(6);
		let leftover = Vec3::new(2178.0, 36.6, 1383.0);
		let saved = Vec3::new(695.6, 73.0, 959.5);
		save_waypoints(&root, next, &[saved])?;

		let mut world = World::new();
		world.insert_resource(PlayerSpawnXz(None));
		world.insert_resource(root);
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(durham::BaseTerrainNoise::from_config(
			&durham::TerrainConfig::new(42),
		)));
		world.insert_resource(crate::WorldPlayerLoadout::new(
			next.to_hex(),
			characters::CharacterAppearance::default(),
			character_items::Inventory::default(),
		));
		world.insert_resource(PlayerPositionWaypoints {
			owner: Some(previous),
			positions: vec![leftover],
			just_loaded: false,
			..default()
		});
		let player =
			world.spawn((world_player::Player, Transform::from_translation(leftover))).id();
		world.spawn((
			Camera3d::default(),
			FollowCamera::default(),
			Transform::default(),
			GlobalTransform::default(),
		));
		world
			.run_system_once(resume_discovery_from_saved_waypoints)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let transform = world
			.get::<Transform>(player)
			.ok_or_else(|| anyhow::anyhow!("player transform"))?;
		assert_eq!(transform.translation.xz(), saved.xz());
		assert!(world.get::<GlobalTransform>(player).is_some());
		assert!(world.get::<AwaitingTerrainSurface>(player).is_some());
		Ok(())
	}
}
