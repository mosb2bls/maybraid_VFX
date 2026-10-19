//! Isolated preview for Doric column kit placeholders, anchors, and cover colliders.

mod camera;
mod gizmos;
mod ground;
mod preview;

use avian3d::prelude::PhysicsPlugins;
use bevy::prelude::*;
use bevy::window::{PresentMode, Window, WindowResolution};
use doric_column::DoricColumnPlugin;
use material_ref::{MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache};

pub fn run() {
	let mut app = App::new();
	app.add_plugins(DefaultPlugins.set(WindowPlugin {
		primary_window: Some(Window {
			title: "doric-column-playground".into(),
			resolution: WindowResolution::new(1280, 720),
			present_mode: PresentMode::AutoNoVsync,
			decorations: false,
			..default()
		}),
		..default()
	}))
	.add_plugins((
		PhysicsPlugins::default(),
		DoricColumnPlugin,
		MaterialRefPlugin::<StandardMaterialLib<'_>>::default(),
	))
	.init_asset::<StandardMaterial>()
	.init_resource::<StandardMaterialRefCache>()
	.insert_resource(ClearColor(Color::srgb(0.52, 0.58, 0.64)))
	.insert_resource(GlobalAmbientLight {
		color: Color::srgb(0.85, 0.88, 0.92),
		brightness: 280.0,
		affects_lightmapped_meshes: true,
	})
	.insert_resource(gizmos::DrawDoricColumnDebug::default())
	.add_systems(Startup, (camera::setup_camera, ground::setup_ground, preview::spawn_preview))
	.add_systems(Update, (camera::camera_controller, gizmos::draw_debug));
	app.run();
}
