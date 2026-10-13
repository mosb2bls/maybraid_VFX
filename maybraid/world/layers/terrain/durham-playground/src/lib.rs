//! Interactive Durham terrain models playground.

pub mod camera;
pub mod character;
pub mod commands;
mod debug_bounds;
mod pitch;
mod player;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use player::PlaygroundMode;

use avian3d::prelude::LinearVelocity;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use camera::{camera_controller, refocus_camera_on_layout, setup_camera};
use character::{apply_set_character, drive_player_locomotion};
use characters::{CharacterHostsPlugin, CharacterMotionSystems};
use commands::{
	PendingCellLayoutPatch, RequestCellShow, RequestMeshStats, RequestModeCharacter,
	RequestModeFree, RequestSeed,
};
use debug_bounds::{setup_cell_location_hud, update_cell_location_hud, PlaygroundDebugOverlay};
use durham::{
	terrain_collider_covers_xz, BaseTerrainNoise, CascadeChunk, ComposedWater,
	DurhamPresentationPlugin, DurhamRoots, DurhamTerrainModelsPlugin, DurhamWindow, OuterCellRing,
	SharedTerrainStorage, Terrain, TerrainCellLayout, TerrainConfig, TerrainMeshAssets,
	TerrainMeshBuilder, TerrainMeshLodBand, TerrainStampConfigs, TerrainTrimeshCollider,
	WaterMeshAssets, WatershedConfigs, TERRAIN_CELL_SIZE,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use lod::hcsg::{
	HcsgBoundsPlugin, HcsgNode, HcsgRestartRequest, HcsgSessionRestarted, HcsgStorage,
};
use pitch::{apply_avian_terrain_pitch, sync_suspend_terrain_pitch};
use player::{respawn_player_on_layout, Player, PlayerControlSystems, PlayerPlugin};
use render_item::mesh::handle::EnforceCachingPlugin;
use std::f32::consts::PI;
use std::sync::Arc;
use terrain_shaders::{RefractionWater, TerrainShader, TerrainShaderPlugin};

/// Fine-grid half-extent in base cells (covers rings through base-sized `res_2 = 2`).
/// World footprint `[-R·s, R·s)` so it abuts the 2× outer-ring tiles.
///
/// Cumulative fine bands: 4 + 2 + 2 + 8 + 8 = 24.
const PLAYGROUND_FINE_HALF_EXTENT_CELLS: i32 = 24;

/// Nested macro rings beyond the fine footprint (world half-extent → 32s, then 40s).
const PLAYGROUND_OUTER_2X_ROWS: i32 = 4; // 4 × 2s = 8s
const PLAYGROUND_OUTER_4X_ROWS: i32 = 2; // 2 × 4s = 8s

/// Chebyshev LOD bands on the **fine** (base-sized) grid:
/// - `r ≤ 4` → 5
/// - `r ≤ 6` (+2) → 4
/// - `r ≤ 8` (+2) → 3
/// - `r ≤ 16` (+8) → 2
/// - `r ≤ 24` (+8) → 2
/// Then 2× cells to world radius 32s, then 4× cells to 40s (both `res_2 = 2`).
fn playground_lod_bands() -> Vec<TerrainMeshLodBand> {
	vec![
		TerrainMeshLodBand { max_radius_cells: 4, res_2: 5 },
		TerrainMeshLodBand { max_radius_cells: 6, res_2: 4 },
		TerrainMeshLodBand { max_radius_cells: 8, res_2: 3 },
		TerrainMeshLodBand { max_radius_cells: 16, res_2: 2 },
		TerrainMeshLodBand { max_radius_cells: 24, res_2: 2 },
	]
}

/// Base noise used for camera / player height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

fn playground_cell_layout() -> TerrainCellLayout {
	let mut layout = TerrainCellLayout::default();
	layout.origin =
		IVec2::new(-PLAYGROUND_FINE_HALF_EXTENT_CELLS, -PLAYGROUND_FINE_HALF_EXTENT_CELLS);
	let n = (2 * PLAYGROUND_FINE_HALF_EXTENT_CELLS) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings = vec![
		OuterCellRing { cell_size: 2.0 * TERRAIN_CELL_SIZE, rows: PLAYGROUND_OUTER_2X_ROWS },
		OuterCellRing { cell_size: 4.0 * TERRAIN_CELL_SIZE, rows: PLAYGROUND_OUTER_4X_ROWS },
	];
	layout
}

/// The seed or layout changed: start a new session.
#[derive(Resource)]
struct TerrainPresentationDirty(bool);

/// The player lands once the new session's ground under the region center
/// is solid.
#[derive(Resource, Default)]
struct PlayerRespawnPending(bool);

pub struct TerrainModelsPlaygroundPlugin;

impl Plugin for TerrainModelsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);

		app.add_plugins(DurhamTerrainModelsPlugin)
			.add_plugins((
				HcsgBoundsPlugin::<DurhamWindow>::default(),
				DurhamPresentationPlugin::<DurhamWindow>::default(),
			))
			.add_plugins(TerrainShaderPlugin)
			.add_plugins(EnforceCachingPlugin::<TerrainMeshBuilder, TerrainShader>::default())
			.add_plugins(EnforceCachingPlugin::<ComposedWater, RefractionWater>::default())
			.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			)
			.add_plugins(PlayerPlugin)
			.add_plugins(CharacterHostsPlugin)
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			// After `DurhamTerrainModelsPlugin` so this replaces the default layout.
			.insert_resource(playground_cell_layout())
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<PlayerRespawnPending>()
			.init_resource::<PlaygroundRestartFollowup>()
			.init_resource::<PlaygroundDebugOverlay>()
			.add_systems(
				Startup,
				(setup_camera, setup_lighting, setup_presentation_assets, setup_cell_location_hud),
			)
			.add_systems(
				Update,
				(
					camera_controller,
					apply_cell_commands.after(capture_command_line_input::<PlaygroundCommand>),
					apply_seed.after(apply_cell_commands),
					apply_set_character.after(apply_seed),
					apply_mode_commands.after(apply_set_character),
					apply_mesh_stats.after(apply_mode_commands),
					queue_playground_restart.after(apply_mesh_stats),
					playground_after_hcsg_restart.after(HcsgSessionRestarted),
					respawn_player_on_ground,
					drive_player_locomotion
						.after(PlayerControlSystems)
						.before(CharacterMotionSystems::Anim),
					sync_suspend_terrain_pitch.after(PlayerControlSystems),
					apply_avian_terrain_pitch
						.in_set(CharacterMotionSystems::Elevation)
						.after(drive_player_locomotion)
						.after(sync_suspend_terrain_pitch),
					update_cell_location_hud,
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	commands.spawn((
		DirectionalLight { illuminance: 12_000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 2_500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

pub(crate) fn setup_presentation_assets(
	mut commands: Commands,
	mut terrain_materials: ResMut<Assets<TerrainShader>>,
	mut water_materials: ResMut<Assets<RefractionWater>>,
	config: Res<TerrainConfig>,
) {
	let material = terrain_materials.add(TerrainShader::default());
	let s = TERRAIN_CELL_SIZE;
	let fine_half = PLAYGROUND_FINE_HALF_EXTENT_CELLS as f32 * s;
	let mid_half = fine_half + PLAYGROUND_OUTER_2X_ROWS as f32 * 2.0 * s; // 32s
	commands.insert_resource(TerrainMeshAssets {
		config: config.clone(),
		material,
		lod_bands: playground_lod_bands(),
		outer_add_walls: true,
		fine_grid_max_radius: Some(PLAYGROUND_FINE_HALF_EXTENT_CELLS),
		macro_seam_half_extents: vec![fine_half, mid_half],
		macro_cell_min_size: Some(2.0 * s),
		macro_res_2: Some(2),
	});
	commands.insert_resource(WaterMeshAssets {
		material: water_materials.add(RefractionWater::default()),
	});
}

fn apply_cell_commands(
	mut commands: Commands,
	mut layout: ResMut<TerrainCellLayout>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut status: ResMut<GameCommandStatusText>,
	patches: Query<(Entity, &PendingCellLayoutPatch)>,
	shows: Query<Entity, With<RequestCellShow>>,
) {
	for entity in &shows {
		status.0 = format!(
			"cells size={:.1} origin=({}, {}) extents={}×{}",
			layout.cell_size, layout.origin.x, layout.origin.y, layout.extents.x, layout.extents.y
		);
		commands.entity(entity).despawn();
	}

	for (entity, patch) in &patches {
		if let Some(size) = patch.size {
			if size > 0.0 {
				layout.cell_size = size;
			}
		}
		if let Some(x) = patch.origin_x {
			layout.origin.x = x;
		}
		if let Some(z) = patch.origin_z {
			layout.origin.y = z;
		}
		if let Some(x) = patch.extent_x {
			layout.extents.x = x.max(1);
		}
		if let Some(z) = patch.extent_z {
			layout.extents.y = z.max(1);
		}
		dirty.0 = true;
		status.0 = format!(
			"cells size={:.1} origin=({}, {}) extents={}×{} (regen)",
			layout.cell_size, layout.origin.x, layout.origin.y, layout.extents.x, layout.extents.y
		);
		commands.entity(entity).despawn();
	}
}

fn apply_seed(
	mut commands: Commands,
	mut config: ResMut<TerrainConfig>,
	mut assets: ResMut<TerrainMeshAssets>,
	mut jersey: ResMut<TerrainStampConfigs>,
	mut marazion: ResMut<WatershedConfigs>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut status: ResMut<GameCommandStatusText>,
	requests: Query<(Entity, &RequestSeed)>,
) {
	for (entity, request) in &requests {
		config.seed = request.0;
		assets.config.seed = request.0;
		*jersey = TerrainStampConfigs::from_world_seed(request.0);
		*marazion = WatershedConfigs::default().with_seed(request.0);
		world_base.0 = BaseTerrainNoise::from_config(&config);
		dirty.0 = true;
		status.0 = format!("seed {} (regen)", request.0);
		info!("seed {} — regenerating terrain and water", request.0);
		commands.entity(entity).despawn();
	}
}

fn apply_mode_commands(
	mut commands: Commands,
	mut mode: ResMut<PlaygroundMode>,
	mut status: ResMut<GameCommandStatusText>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	store: Res<HcsgStorage>,
	free: Query<Entity, With<RequestModeFree>>,
	character: Query<Entity, With<RequestModeCharacter>>,
	mut players: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
) {
	for entity in &free {
		*mode = PlaygroundMode::Free;
		status.0 = "mode free".into();
		if let Ok((mut cam_t, mut controller)) = cameras.single_mut() {
			refocus_camera_on_layout(&layout, &base.0, &mut cam_t, &mut controller);
		}
		commands.entity(entity).despawn();
	}

	for entity in &character {
		*mode = PlaygroundMode::Character;
		status.0 = "mode character — WASD move, mouse look, Space jump".into();
		if let Ok((mut transform, mut velocity)) = players.single_mut() {
			let center = layout.region_center_xz();
			let elevation = store
				.try_composed_height_at(&layout, center.x, center.z)
				.ok()
				.flatten()
				.unwrap_or_else(|| base.0.height_at(center.x, center.z));
			respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
		}
		commands.entity(entity).despawn();
	}
}

fn apply_mesh_stats(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<&Mesh3d, Without<Player>>,
) {
	for entity in &requests {
		let mut mesh_count = 0usize;
		let mut missing = 0usize;
		let mut vertices = 0usize;
		let mut indices = 0usize;
		let mut triangles = 0usize;
		let mut unique_handles = std::collections::HashSet::new();

		for mesh3d in &mesh_entities {
			mesh_count += 1;
			unique_handles.insert(mesh3d.0.id());
			let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
				missing += 1;
				continue;
			};
			let verts = mesh.count_vertices();
			let index_count = mesh.indices().map(|i| i.len()).unwrap_or(verts);
			vertices += verts;
			indices += index_count;
			triangles += index_count / 3;
		}

		status.0 = format!(
			"stats mesh: entities={mesh_count} unique_handles={} missing={missing} verts={vertices} indices={indices} tris={triangles}",
			unique_handles.len()
		);
		info!("{}", status.0);
		commands.entity(entity).despawn();
	}
}

#[derive(Resource, Default)]
struct PlaygroundRestartFollowup(bool);

fn queue_playground_restart(
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut request: ResMut<HcsgRestartRequest>,
	mut followup: ResMut<PlaygroundRestartFollowup>,
) {
	if !dirty.0 {
		return;
	}
	request.request();
	dirty.0 = false;
	followup.0 = true;
}

/// The worker regenerates in the background; hosts swap to the new session's
/// values as they land.
fn playground_after_hcsg_restart(
	roots: DurhamRoots,
	mut followup: ResMut<PlaygroundRestartFollowup>,
	mut respawn: ResMut<PlayerRespawnPending>,
	mode: Res<PlaygroundMode>,
	world_base: Res<WorldBaseTerrain>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
) {
	if !followup.0 {
		return;
	}
	followup.0 = false;
	if *mode == PlaygroundMode::Free {
		if let Ok((mut transform, mut controller)) = cameras.single_mut() {
			refocus_camera_on_layout(
				roots.layout(),
				&world_base.0,
				&mut transform,
				&mut controller,
			);
		}
	}
	respawn.0 = true;
}

/// Waits for the center cell's trimesh from the new session's value, so the
/// player neither drops through missing ground nor lands on the old surface.
fn respawn_player_on_ground(
	storage: Res<HcsgStorage>,
	layout: Res<TerrainCellLayout>,
	mut respawn: ResMut<PlayerRespawnPending>,
	colliders: Query<(Entity, &CascadeChunk), With<TerrainTrimeshCollider>>,
	parents: Query<&ChildOf>,
	hosts: Query<&HcsgNode<Terrain>>,
	mut players: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
) {
	if !respawn.0 {
		return;
	}
	let center = layout.region_center_xz();
	let Ok(Some(terrain)) = storage.try_terrain_at(&layout, center.x, center.z) else {
		return;
	};
	let solid = colliders.iter().any(|(fill, chunk)| {
		terrain_collider_covers_xz(center, [chunk])
			&& parents
				.iter_ancestors(fill)
				.any(|host| hosts.get(host).is_ok_and(|node| Arc::ptr_eq(&node.value, &terrain)))
	});
	if !solid {
		return;
	}
	if let Ok((mut transform, mut velocity)) = players.single_mut() {
		let elevation = terrain.sdf.terrain().height_at_with_all_modulations(center.x, center.z);
		respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
	}
	respawn.0 = false;
}

#[cfg(test)]
mod tests {
	use super::*;
	use durham::CellTiling;

	#[test]
	fn playground_fine_grid_is_twenty_four_cells() {
		assert_eq!(PLAYGROUND_FINE_HALF_EXTENT_CELLS, 24);
		assert!(playground_lod_bands().iter().any(|band| band.max_radius_cells == 24));
	}

	#[test]
	fn playground_origin_cells_cover_fine_disk_plus_macro_rings() {
		let layout = playground_cell_layout();
		let ids = layout.cell_ids(layout.request_region());
		assert_eq!(ids.len(), 48 * 48 + 448 + 144);
	}
}
