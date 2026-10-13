use bevy::prelude::*;

#[derive(Component)]
pub struct GroundPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let size = 40.0;
	let mesh = meshes.add(Plane3d::default().mesh().size(size, size));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.55, 0.58, 0.52),
		perceptual_roughness: 0.95,
		..default()
	});

	commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY, GroundPlane));
}
