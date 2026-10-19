//! Isolated preview for Doric column kit placeholders, anchors, and cover colliders.

mod camera;
mod capture;
mod gizmos;
mod ground;
mod preview;

use bevy::prelude::*;
use bevy::window::{PresentMode, Window, WindowResolution};
use building_shaders::{
	BuildingShadersPlugin, ColumnSurfaceMaterial, UrbanMaterialRefPlugin, UrbanSurfaceMaterial,
};
use doric_column::DoricColumnPlugin;
use avian3d::prelude::PhysicsPlugins;
use material_ref::MaterialRefPlugin;

pub fn run() {
	let capturing = capture::CaptureSession::from_env();
	let mut app = App::new();
	app.add_plugins(
		DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "doric-column-playground".into(),
				resolution: WindowResolution::new(1280, 720),
				present_mode: PresentMode::AutoNoVsync,
				decorations: false,
				..default()
			}),
			..default()
		}),
	)
		.add_plugins((
			PhysicsPlugins::default(),
			BuildingShadersPlugin,
			DoricColumnPlugin,
			UrbanMaterialRefPlugin,
			MaterialRefPlugin::<building_shaders::UrbanSurfaceMaterialLib<'_>>::default(),
		))
		.init_asset::<StandardMaterial>()
		.init_asset::<UrbanSurfaceMaterial>()
		.init_asset::<ColumnSurfaceMaterial>();
	app.insert_resource(if capturing.is_some() {
		gizmos::DrawDoricColumnDebug(false)
	} else {
		gizmos::DrawDoricColumnDebug::default()
	})
		.insert_resource(ClearColor(Color::srgb(0.52, 0.58, 0.64)))
		.add_systems(
			Startup,
			(setup_lighting, camera::setup_camera, ground::setup_ground, preview::spawn_preview),
		)
		.add_systems(
			Update,
			(
				camera::camera_controller.run_if(not(resource_exists::<capture::CaptureSession>)),
				gizmos::draw_debug,
			),
		);
	if let Some(session) = capturing {
		app.insert_resource(session)
			.add_systems(Update, capture::tick_capture.after(preview::spawn_preview));
	}
	app.run();
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		color: Color::srgb(0.85, 0.88, 0.92),
		brightness: 280.0,
		affects_lightmapped_meshes: true,
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 9800.0,
			color: Color::srgb(1.0, 0.92, 0.82),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
		capture::KeySun,
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 2400.0,
			color: Color::srgb(0.55, 0.68, 1.0),
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
		capture::FillSky,
	));
}
