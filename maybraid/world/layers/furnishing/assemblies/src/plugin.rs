//! Shared unit-cube mesh for gallery abutment walls (kits are GLBs).

use bevy::prelude::*;

/// Handles for the unit cuboid stand-in (spans \([-0.5, 0.5]^3\)).
#[derive(Resource, Clone)]
pub struct FurnitureKitMeshes {
	pub unit_cube: Handle<Mesh>,
	pub wall: Handle<StandardMaterial>,
}

/// Registers [`FurnitureKitMeshes`].
pub struct FurnitureAssembliesPlugin;

impl Plugin for FurnitureAssembliesPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_kit_meshes);
	}
}

fn init_kit_meshes(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.insert_resource(FurnitureKitMeshes {
		unit_cube: meshes.add(Cuboid::from_length(1.0)),
		wall: materials.add(StandardMaterial {
			base_color: Color::srgba(0.78, 0.76, 0.72, 0.55),
			unlit: true,
			alpha_mode: AlphaMode::Blend,
			cull_mode: None,
			..default()
		}),
	});
}
