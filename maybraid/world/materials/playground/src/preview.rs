//! Persistent preview host. `/show` restamps the recipe; `/mesh` rebuilds the hull.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use firearms::laser_hex_ref;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};

use crate::commands::preview_label;
use crate::shape::{PlaygroundMeshes, PreviewShape};

#[derive(Component)]
pub struct PreviewRoot;

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct PreviewConfig {
	pub material: MaterialRef,
	pub shape: PreviewShape,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { material: laser_hex_ref(), shape: PreviewShape::Sphere }
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		format!("preview: {} · {}", preview_label(&self.material), self.shape.label())
	}
}

pub fn sync_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	meshes: Option<Res<PlaygroundMeshes>>,
	roots: Query<Entity, With<PreviewRoot>>,
	mut last: Local<Option<PreviewConfig>>,
) {
	let Some(meshes) = meshes else {
		return;
	};
	if last.as_ref() == Some(&*config) && roots.iter().next().is_some() {
		return;
	}

	for entity in &roots {
		commands.entity(entity).try_despawn();
	}
	spawn_preview(&mut commands, &meshes, &config);
	*last = Some(config.clone());
}

fn spawn_preview(commands: &mut Commands, meshes: &PlaygroundMeshes, config: &PreviewConfig) {
	commands
		.spawn((
			PreviewRoot,
			Name::new(format!("preview-{}", config.shape.label())),
			Transform::from_xyz(0.0, 1.0, 0.0),
			Visibility::default(),
			MaterialRefRoot(config.material.clone()),
			PropagateToDescendants,
			NotShadowCaster,
		))
		.with_children(|parent| {
			for (mesh, transform) in meshes.parts(config.shape) {
				parent.spawn((Mesh3d(mesh), transform, NotShadowCaster, Visibility::default()));
			}
		});
}
