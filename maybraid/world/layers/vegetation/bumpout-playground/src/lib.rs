//! Interactive viewer for Chico terrain-mesh bump outs.

pub mod camera;
pub mod checkerboard_material;
pub mod commands;
mod ground;
mod scene;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use camera_controls::look::CameraLookPlugin;
use commands::NeighborhoodValues;
use game_commands::command::GameCommandPlugin;
use ground::setup_ground;
use material_ref::MaterialRefRoot;
use scene::{setup_tiles, PlaygroundTerrainBuilder, TILE_RADIUS};
use terrain_chunk_ref::{TerrainChunkRef, TerrainChunkRefPlugin};
use vegetation_bumpout::{BumpOut, BumpOutMaterialRefPlugin, BumpOutPlugin};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PresenterLayer {
	Terrain,
	GroundCover,
	CanopyProxy,
}

impl PresenterLayer {
	pub(crate) fn label(self) -> &'static str {
		match self {
			Self::Terrain => "Terrain",
			Self::GroundCover => "GroundCover",
			Self::CanopyProxy => "CanopyProxy",
		}
	}
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileCoordinate(IVec2);

#[derive(Resource, Default)]
struct SharedHandleReport(bool);

#[derive(Resource)]
pub(crate) struct NeighborhoodControls {
	pub(crate) layer: PresenterLayer,
	row: usize,
	column: usize,
}

impl NeighborhoodControls {
	pub(crate) fn sample_index(&self) -> usize {
		self.row * 3 + self.column
	}

	pub(crate) fn selected_coordinate(&self) -> IVec2 {
		IVec2::new(self.column as i32 - 1, self.row as i32 - 1)
	}
}

impl Default for NeighborhoodControls {
	fn default() -> Self {
		Self { layer: PresenterLayer::GroundCover, row: 1, column: 1 }
	}
}

pub struct BumpOutPlaygroundPlugin;

impl Plugin for BumpOutPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<SharedHandleReport>()
			.init_resource::<NeighborhoodControls>()
			.add_plugins(CameraLookPlugin::default())
			.add_plugins(BumpOutPlugin)
			.add_plugins(TerrainChunkRefPlugin::<PlaygroundTerrainBuilder>::default())
			.add_plugins(BumpOutMaterialRefPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_plugins(MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default())
			.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground, setup_tiles))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					report_shared_mesh_handle,
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight { brightness: 450.0, ..default() });
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 3500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

type EditableBumpOut<'a> = (
	&'a PresenterLayer,
	&'a TileCoordinate,
	&'a TerrainChunkRef<PlaygroundTerrainBuilder>,
	&'a mut BumpOut,
	&'a mut MaterialRefRoot,
	&'a mut Aabb,
);

pub(crate) fn apply_neighborhood_edit(
	world: &mut World,
	values: &NeighborhoodValues,
	adjust: bool,
) {
	let (layer, selected_coordinate) = {
		let controls = world.resource::<NeighborhoodControls>();
		(controls.layer, controls.selected_coordinate())
	};
	let mut layers = world.query::<EditableBumpOut>();
	for (candidate, tile, terrain_ref, mut bump_out, mut material_root, mut aabb) in
		layers.iter_mut(world)
	{
		if *candidate != layer {
			continue;
		}
		let relative = selected_coordinate - tile.0;
		if relative.x.abs() > 1 || relative.y.abs() > 1 {
			continue;
		}

		let sample = ((relative.y + 1) * 3 + relative.x + 1) as usize;
		let mut neighborhood = bump_out.neighborhood();
		if let Some(value) = values.density {
			neighborhood
				.set_density(sample, edited_value(neighborhood.densities[sample], value, adjust));
		}
		if let Some(value) = values.bite_size {
			neighborhood.set_bite_size(
				sample,
				edited_value(neighborhood.bite_sizes[sample], value, adjust),
			);
		}
		if let Some(value) = values.bite_size_deviation {
			neighborhood.set_bite_size_deviation(
				sample,
				edited_value(neighborhood.bite_size_deviations[sample], value, adjust),
			);
		}
		if let Some(value) = values.average_height {
			neighborhood.set_average_height(
				sample,
				edited_value(neighborhood.average_heights[sample], value, adjust),
			);
		}
		if let Some(value) = values.height_deviation {
			neighborhood.set_height_deviation(
				sample,
				edited_value(neighborhood.height_deviations[sample], value, adjust),
			);
		}

		bump_out.set_neighborhood(neighborhood);
		material_root.0 = bump_out.material.clone();
		*aabb = bump_out.aabb(terrain_ref);
	}
}

pub(crate) fn change_layer_visibility(
	world: &mut World,
	selected: PresenterLayer,
	visible: Option<bool>,
) {
	let mut layers = world.query::<(&PresenterLayer, &mut Visibility)>();
	for (layer, mut visibility) in layers.iter_mut(world) {
		if *layer != selected {
			continue;
		}
		let show = visible.unwrap_or(matches!(*visibility, Visibility::Hidden));
		*visibility = if show { Visibility::Visible } else { Visibility::Hidden };
	}
}

fn edited_value(current: f32, requested: f32, adjust: bool) -> f32 {
	if adjust {
		current + requested
	} else {
		requested
	}
}

fn report_shared_mesh_handle(
	mut report: ResMut<SharedHandleReport>,
	layers: Query<(&TileCoordinate, &PresenterLayer, &Mesh3d)>,
) {
	if report.0 {
		return;
	}
	let mut handles = Vec::new();
	for (tile, layer, mesh) in &layers {
		handles.push((tile.0, *layer, mesh.0.id()));
	}
	let diameter = (TILE_RADIUS * 2 + 1) as usize;
	if handles.len() != diameter * diameter * 3 {
		return;
	}

	let mut shared = true;
	for z in -TILE_RADIUS..=TILE_RADIUS {
		for x in -TILE_RADIUS..=TILE_RADIUS {
			let coordinate = IVec2::new(x, z);
			let terrain_handle = handles
				.iter()
				.find(|(tile, layer, _)| *tile == coordinate && *layer == PresenterLayer::Terrain)
				.map(|(_, _, handle)| *handle);
			for layer in [PresenterLayer::GroundCover, PresenterLayer::CanopyProxy] {
				let bump_handle = handles
					.iter()
					.find(|(tile, candidate, _)| *tile == coordinate && *candidate == layer)
					.map(|(_, _, handle)| *handle);
				shared &= terrain_handle.is_some() && terrain_handle == bump_handle;
			}
		}
	}
	info!("Every tile shares one mesh handle across its three presenters: {shared}");
	report.0 = true;
}
