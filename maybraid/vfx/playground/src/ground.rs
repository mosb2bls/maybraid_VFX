use bevy::prelude::*;

#[derive(Component)]
pub struct GroundPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let ground = meshes.add(Plane3d::default().mesh().size(40.0, 40.0));
	let dirt = materials.add(StandardMaterial {
		base_color: Color::srgb(0.28, 0.24, 0.18),
		perceptual_roughness: 0.96,
		..default()
	});
	commands.spawn((Mesh3d(ground), MeshMaterial3d(dirt), Transform::IDENTITY, GroundPlane));

	let band = meshes.add(Plane3d::default().mesh().size(40.0, 8.0));
	let cool = materials.add(StandardMaterial {
		base_color: Color::srgb(0.18, 0.22, 0.26),
		perceptual_roughness: 0.95,
		..default()
	});
	commands.spawn((
		Mesh3d(band),
		MeshMaterial3d(cool),
		Transform::from_xyz(0.0, 0.005, -8.0),
		GroundPlane,
	));

	let wall = meshes.add(Cuboid::new(0.22, 2.6, 3.4));
	let plaster = materials.add(StandardMaterial {
		base_color: Color::srgb(0.42, 0.36, 0.30),
		perceptual_roughness: 0.9,
		..default()
	});
	commands.spawn((
		Mesh3d(wall),
		MeshMaterial3d(plaster.clone()),
		Transform::from_xyz(-1.15, 1.3, 0.15),
		Name::new("vfx-occluder-wall"),
	));

	let crate_mesh = meshes.add(Cuboid::new(0.7, 0.7, 0.7));
	commands.spawn((
		Mesh3d(crate_mesh),
		MeshMaterial3d(plaster),
		Transform::from_xyz(1.35, 0.35, 0.8),
		Name::new("vfx-occluder-box"),
	));
}
