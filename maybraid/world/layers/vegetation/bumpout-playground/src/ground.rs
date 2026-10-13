use bevy::prelude::*;

use crate::checkerboard_material::CheckerboardMaterial;

#[derive(Component)]
pub struct GroundPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<CheckerboardMaterial>>,
) {
	let size = 2000.0;
	let mesh = meshes.add(Plane3d::default().mesh().size(size, size));
	let material = materials.add(CheckerboardMaterial {
		checker_size_m: 10.0,
		color1: Color::srgb(0.9, 0.9, 0.9).into(),
		color2: Color::srgb(0.7, 0.7, 0.7).into(),
	});

	commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY, GroundPlane));
}
