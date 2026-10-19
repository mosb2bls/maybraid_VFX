//! Unlit mesh presenter. VFX lanes replace this without touching gameplay.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
	AttackInstanceId, AttackPresentationBinding, AttackPresentationBindingKind,
	AttackPresentationEvent, AttackPresentationKind, AttackSystems, HazardPose, HazardShape,
	PresentationPhase,
};

#[derive(Resource, Default)]
struct PlaceholderAssets {
	sphere: Option<Handle<Mesh>>,
	cylinder: Option<Handle<Mesh>>,
	cube: Option<Handle<Mesh>>,
	telegraph: Option<Handle<StandardMaterial>>,
	hazard: Option<Handle<StandardMaterial>>,
}

#[derive(Resource, Default)]
struct PlaceholderBindings {
	entities: HashMap<(AttackInstanceId, AttackPresentationBindingKind), Entity>,
}

/// Draws placeholder telegraph and hazard volumes from presentation events.
pub struct AttackPlaceholderPresentationPlugin;

impl Plugin for AttackPlaceholderPresentationPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PlaceholderAssets>()
			.init_resource::<PlaceholderBindings>()
			.add_systems(Startup, seed_placeholder_assets)
			.add_systems(Update, present_placeholders.after(AttackSystems::Advance));
	}
}

fn seed_placeholder_assets(
	mut assets: ResMut<PlaceholderAssets>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	assets.sphere = Some(meshes.add(Sphere::new(1.0)));
	assets.cylinder = Some(meshes.add(Cylinder::new(1.0, 1.0)));
	assets.cube = Some(meshes.add(Cuboid::new(1.0, 1.0, 1.0)));
	assets.telegraph = Some(materials.add(StandardMaterial {
		base_color: Color::srgba(1.0, 0.82, 0.16, 0.38),
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		..default()
	}));
	assets.hazard = Some(materials.add(StandardMaterial {
		base_color: Color::srgba(0.92, 0.18, 0.14, 0.46),
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		..default()
	}));
}

fn present_placeholders(
	mut events: MessageReader<AttackPresentationEvent>,
	assets: Res<PlaceholderAssets>,
	mut bindings: ResMut<PlaceholderBindings>,
	mut transforms: Query<&mut Transform>,
	mut commands: Commands,
) {
	for event in events.read() {
		match event.kind {
			AttackPresentationKind::Telegraph { phase, pose, shape } => {
				present_volume(
					&mut commands,
					&assets,
					&mut bindings,
					&mut transforms,
					event.instance,
					AttackPresentationBindingKind::Telegraph,
					phase,
					pose,
					shape,
					true,
				);
			}
			AttackPresentationKind::Hazard { index, phase, pose, shape, wave } => {
				present_volume(
					&mut commands,
					&assets,
					&mut bindings,
					&mut transforms,
					event.instance,
					AttackPresentationBindingKind::Hazard {
						index,
						wave: wave.map(|wave| wave.wave_index).unwrap_or(0),
					},
					phase,
					pose,
					shape,
					false,
				);
			}
			AttackPresentationKind::Ended { .. } => {
				clear_instance(&mut commands, &mut bindings, event.instance);
			}
			AttackPresentationKind::Stage { .. } => {}
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn present_volume(
	commands: &mut Commands,
	assets: &PlaceholderAssets,
	bindings: &mut PlaceholderBindings,
	transforms: &mut Query<&mut Transform>,
	instance: AttackInstanceId,
	kind: AttackPresentationBindingKind,
	phase: PresentationPhase,
	pose: HazardPose,
	shape: HazardShape,
	telegraph: bool,
) {
	let key = (instance, kind);
	match phase {
		PresentationPhase::Despawned => {
			if let Some(entity) = bindings.entities.remove(&key) {
				commands.entity(entity).try_despawn();
			}
		}
		PresentationPhase::Spawned | PresentationPhase::Updated => {
			let transform = volume_transform(pose, shape);
			if let Some(entity) = bindings.entities.get(&key).copied() {
				if let Ok(mut current) = transforms.get_mut(entity) {
					*current = transform;
					return;
				}
			}
			let Some(mesh) = mesh_for(assets, shape) else {
				return;
			};
			let material = if telegraph { assets.telegraph.clone() } else { assets.hazard.clone() };
			let Some(material) = material else {
				return;
			};
			let entity = commands
				.spawn((
					Name::new(if telegraph { "attack-telegraph" } else { "attack-hazard" }),
					transform,
					Visibility::default(),
					Mesh3d(mesh),
					MeshMaterial3d(material),
					AttackPresentationBinding { instance, kind },
				))
				.id();
			bindings.entities.insert(key, entity);
		}
	}
}

fn clear_instance(
	commands: &mut Commands,
	bindings: &mut PlaceholderBindings,
	instance: AttackInstanceId,
) {
	bindings.entities.retain(|(bound, _), entity| {
		if *bound == instance {
			commands.entity(*entity).try_despawn();
			false
		} else {
			true
		}
	});
}

fn mesh_for(assets: &PlaceholderAssets, shape: HazardShape) -> Option<Handle<Mesh>> {
	match shape {
		HazardShape::Sphere { .. } => assets.sphere.clone(),
		HazardShape::Wavefront(_) | HazardShape::Cylinder { .. } => assets.cylinder.clone(),
		HazardShape::Box { .. } | HazardShape::Line { .. } => assets.cube.clone(),
	}
}

pub fn volume_transform(pose: HazardPose, shape: HazardShape) -> Transform {
	match shape {
		HazardShape::Sphere { radius } => Transform {
			translation: pose.origin,
			rotation: pose.rotation,
			scale: Vec3::splat(radius.max(0.05) * 2.0),
		},
		HazardShape::Wavefront(wave) => Transform {
			translation: pose.origin,
			rotation: pose.rotation * Quat::from_rotation_arc(Vec3::Y, Vec3::Z),
			scale: Vec3::new(
				wave.radius.max(0.05),
				wave.half_depth.max(0.05) * 2.0,
				wave.radius.max(0.05),
			),
		},
		HazardShape::Cylinder { radius, height } => Transform {
			translation: pose.origin + pose.rotation * Vec3::Y * (height.max(0.05) * 0.5),
			rotation: pose.rotation,
			scale: Vec3::new(radius.max(0.05), height.max(0.05), radius.max(0.05)),
		},
		HazardShape::Box { half_extents } => Transform {
			translation: pose.origin,
			rotation: pose.rotation,
			scale: half_extents.max(Vec3::splat(0.05)) * 2.0,
		},
		HazardShape::Line { length, half_width } => Transform {
			translation: pose.origin + pose.rotation * Vec3::Z * (length.max(0.05) * 0.5),
			rotation: pose.rotation,
			scale: Vec3::new(
				half_width.max(0.05) * 2.0,
				half_width.max(0.05) * 2.0,
				length.max(0.05),
			),
		},
	}
}
