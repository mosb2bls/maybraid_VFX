use bevy::prelude::*;

#[derive(Component)]
pub struct GroundPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mesh = meshes.add(Plane3d::default().mesh().size(40.0, 40.0));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.16, 0.17, 0.18),
		perceptual_roughness: 0.95,
		..default()
	});
	commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY, GroundPlane));
}
