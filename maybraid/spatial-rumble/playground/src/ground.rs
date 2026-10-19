//! Ground channel visualization. The sample is presentation, not mesh displace.

use bevy::prelude::*;
use maybraid_spatial_rumble::SpatialRumbleOutput;

#[derive(Component)]
pub struct GroundVisual;

#[derive(Component)]
pub struct GroundRing;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::default().mesh().size(48.0, 48.0))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: Color::srgb(0.18, 0.2, 0.16),
			perceptual_roughness: 0.9,
			..default()
		})),
		Transform::from_xyz(0.0, 0.0, 0.0),
		GroundVisual,
	));
	for radius in [3.0, 7.0, 12.0] {
		commands.spawn((
			Mesh3d(meshes.add(Cylinder::new(radius, 0.03))),
			MeshMaterial3d(materials.add(StandardMaterial {
				base_color: Color::srgba(0.72, 0.58, 0.32, 0.35),
				alpha_mode: AlphaMode::Blend,
				unlit: true,
				..default()
			})),
			Transform::from_xyz(0.0, 0.02, 0.0),
			GroundRing,
		));
	}
	commands.spawn((
		Mesh3d(meshes.add(Cuboid::new(0.4, 1.2, 0.4))),
		MeshMaterial3d(
			materials
				.add(StandardMaterial { base_color: Color::srgb(0.55, 0.22, 0.18), ..default() }),
		),
		Transform::from_xyz(0.0, 0.6, 0.0),
	));
}

pub fn sync_ground_presentation(
	output: Res<SpatialRumbleOutput>,
	mut rings: Query<&mut Transform, With<GroundRing>>,
	mut planes: Query<&mut Transform, (With<GroundVisual>, Without<GroundRing>)>,
) {
	let amplitude = output.ground.amplitude;
	let lift = amplitude * 0.08;
	for mut transform in &mut planes {
		transform.translation.y = lift;
	}
	let pulse = 1.0 + amplitude * 0.18;
	for mut transform in &mut rings {
		transform.scale = Vec3::new(pulse, 1.0, pulse);
		transform.translation = output.ground.origin + Vec3::Y * (0.04 + lift);
		if output.ground.direction.length_squared() > 1e-6 {
			let yaw = output.ground.direction.x.atan2(output.ground.direction.z);
			transform.rotation = Quat::from_rotation_y(yaw);
		}
	}
}
