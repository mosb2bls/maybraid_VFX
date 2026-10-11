//! Isolated preview for Doric column kit placeholders, anchors, and cover colliders.

mod camera;
mod gizmos;
mod ground;
mod preview;

use bevy::prelude::*;
use doric_column::DoricColumnPlugin;
use avian3d::prelude::PhysicsPlugins;
use material_ref::{MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache};

pub fn run() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			PhysicsPlugins::default(),
			DoricColumnPlugin,
			MaterialRefPlugin::<StandardMaterialLib<'_>>::default(),
		))
		.init_asset::<StandardMaterial>()
		.init_resource::<StandardMaterialRefCache>()
		.insert_resource(gizmos::DrawDoricColumnDebug::default())
		.add_systems(Startup, (camera::setup_camera, ground::setup_ground, preview::spawn_preview))
		.add_systems(Update, (camera::camera_controller, gizmos::draw_debug))
		.run();
}
