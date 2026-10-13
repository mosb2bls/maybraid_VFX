//! Playground-local repeating shots so energy shaders can be judged in motion.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use firearms::laser_hex_ref;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};

use crate::shape::{PlaygroundMeshes, PreviewShape};

pub const SHOT_INTERVAL: f32 = 0.35;
pub const SHOT_SPEED: f32 = 18.0;
pub const SHOT_MAX_AGE: f32 = 2.2;

#[derive(Resource, Clone, Debug)]
pub struct ShootConfig {
	pub enabled: bool,
	pub material: MaterialRef,
	pub shape: PreviewShape,
	pub interval: f32,
	pub accumulator: f32,
}

impl Default for ShootConfig {
	fn default() -> Self {
		Self {
			enabled: false,
			material: laser_hex_ref(),
			shape: PreviewShape::Capsule,
			interval: SHOT_INTERVAL,
			accumulator: 0.0,
		}
	}
}

#[derive(Component)]
pub struct Shot {
	pub age: f32,
}

pub fn tick_shoot(
	mut commands: Commands,
	time: Res<Time>,
	mut shoot: ResMut<ShootConfig>,
	meshes: Option<Res<PlaygroundMeshes>>,
) {
	if !shoot.enabled {
		shoot.accumulator = 0.0;
		return;
	}
	let Some(meshes) = meshes else {
		return;
	};

	shoot.accumulator += time.delta_secs();
	while shoot.accumulator >= shoot.interval {
		shoot.accumulator -= shoot.interval;
		spawn_shot(&mut commands, &meshes, shoot.shape, shoot.material.clone());
	}
}

fn spawn_shot(
	commands: &mut Commands,
	meshes: &PlaygroundMeshes,
	shape: PreviewShape,
	material: MaterialRef,
) {
	commands
		.spawn((
			Name::new(format!("material-shot-{}", shape.label())),
			Shot { age: 0.0 },
			Transform {
				translation: Vec3::new(0.9, 1.0, 0.0),
				rotation: shape.shot_rotation(),
				scale: Vec3::ONE,
			},
			Visibility::default(),
			MaterialRefRoot(material),
			PropagateToDescendants,
			NotShadowCaster,
		))
		.with_children(|parent| {
			for (mesh, transform) in meshes.parts(shape) {
				parent.spawn((Mesh3d(mesh), transform, NotShadowCaster, Visibility::default()));
			}
		});
}

pub fn fly_shots(
	mut commands: Commands,
	time: Res<Time>,
	mut shots: Query<(Entity, &mut Transform, &mut Shot)>,
) {
	let dt = time.delta_secs();
	for (entity, mut transform, mut shot) in &mut shots {
		shot.age += dt;
		transform.translation.x += SHOT_SPEED * dt;
		if shot.age >= SHOT_MAX_AGE {
			commands.entity(entity).try_despawn();
		}
	}
}
