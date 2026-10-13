//! Quiet world-space scale reference for the concepts playground.
//!
//! A thin pair of 2 m axes (up + toward-camera) meet at a corner offset from
//! the character so body framing can compare proportions without cluttering
//! head/eye close-ups or the right-side UI.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

/// Marks the root of the scale-reference prop.
#[derive(Component)]
pub struct ScaleReferenceRoot;

const LENGTH_M: f32 = 2.0;
const ROD_RADIUS: f32 = 0.012;
/// Corner where the axes meet — right of the character; UI is further right
/// but body framing usually keeps this in peripheral view.
const ORIGIN: Vec3 = Vec3::new(2.0, -1.0, 0.0);

pub fn setup_scale_reference(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let shaft_mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.55, 0.55, 0.58),
		unlit: true,
		..default()
	});
	let tick_mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.72, 0.72, 0.75),
		unlit: true,
		..default()
	});
	let major_mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.88, 0.88, 0.90),
		unlit: true,
		..default()
	});

	let shaft = meshes.add(Cylinder { radius: ROD_RADIUS, half_height: LENGTH_M * 0.5 });
	// Up-axis ticks extend toward the camera (+Z); ground-axis ticks stick up (+Y).
	let minor_tick_z = meshes.add(Cuboid::new(0.008, 0.008, 0.08));
	let major_tick_z = meshes.add(Cuboid::new(0.010, 0.010, 0.14));
	let end_tick_z = meshes.add(Cuboid::new(0.012, 0.012, 0.20));
	let minor_tick_y = meshes.add(Cuboid::new(0.008, 0.08, 0.008));
	let major_tick_y = meshes.add(Cuboid::new(0.010, 0.14, 0.010));
	let end_tick_y = meshes.add(Cuboid::new(0.012, 0.20, 0.012));

	commands
		.spawn((
			Transform::from_translation(ORIGIN),
			Visibility::default(),
			ScaleReferenceRoot,
			Name::new("ScaleReference"),
		))
		.with_children(|root| {
			// Vertical axis (+Y). Cylinder is Y-centered; lift so the base is at the corner.
			root.spawn((
				Mesh3d(shaft.clone()),
				MeshMaterial3d(shaft_mat.clone()),
				Transform::from_translation(Vec3::new(0.0, LENGTH_M * 0.5, 0.0)),
				Name::new("ScaleReferenceAxis_Y"),
			));
			spawn_ticks(
				root,
				"Y",
				|t, half_len| Vec3::new(0.0, t, ROD_RADIUS + half_len),
				minor_tick_z.clone(),
				major_tick_z.clone(),
				end_tick_z,
				tick_mat.clone(),
				major_mat.clone(),
			);

			// Ground axis (+Z, toward the default body camera). Meet at the same corner.
			root.spawn((
				Mesh3d(shaft),
				MeshMaterial3d(shaft_mat),
				Transform::from_translation(Vec3::new(0.0, 0.0, LENGTH_M * 0.5))
					.with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
				Name::new("ScaleReferenceAxis_Z"),
			));
			spawn_ticks(
				root,
				"Z",
				|t, half_len| Vec3::new(0.0, ROD_RADIUS + half_len, t),
				minor_tick_y,
				major_tick_y,
				end_tick_y,
				tick_mat,
				major_mat,
			);
		});
}

fn spawn_ticks(
	root: &mut ChildSpawnerCommands,
	axis: &str,
	position: impl Fn(f32, f32) -> Vec3,
	minor_tick: Handle<Mesh>,
	major_tick: Handle<Mesh>,
	end_tick: Handle<Mesh>,
	tick_mat: Handle<StandardMaterial>,
	major_mat: Handle<StandardMaterial>,
) {
	for i in 1..=4 {
		let t = i as f32 * 0.5;
		let major = i % 2 == 0;
		let (mesh, material, half_len) = if t >= LENGTH_M - f32::EPSILON {
			(end_tick.clone(), major_mat.clone(), 0.10)
		} else if major {
			(major_tick.clone(), major_mat.clone(), 0.07)
		} else {
			(minor_tick.clone(), tick_mat.clone(), 0.04)
		};
		root.spawn((
			Mesh3d(mesh),
			MeshMaterial3d(material),
			Transform::from_translation(position(t, half_len)),
			Name::new(format!("ScaleReferenceTick_{axis}_{t:.1}m")),
		));
	}
}
