use bevy::prelude::*;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mesh = meshes.add(Plane3d::default().mesh().size(80.0, 80.0));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.35, 0.42, 0.32),
		perceptual_roughness: 1.0,
		..default()
	});
	commands.spawn((
		Name::new("ground"),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::IDENTITY,
	));
}
