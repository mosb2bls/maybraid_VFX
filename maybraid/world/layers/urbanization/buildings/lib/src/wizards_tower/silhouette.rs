//! Low-LOD cylinder silhouette for Wizard's Tower.

use std::sync::OnceLock;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

/// Shared unit cylinder silhouette (must be initialized by [`TowerSilhouettePlugin`]).
#[derive(Resource, Debug, Clone)]
pub struct TowerSilhouetteAssets {
	pub cylinder: Handle<Mesh>,
	pub material: Handle<StandardMaterial>,
}

static CYLINDER: OnceLock<Handle<Mesh>> = OnceLock::new();
static MATERIAL: OnceLock<Handle<StandardMaterial>> = OnceLock::new();

impl TowerSilhouetteAssets {
	pub fn cylinder() -> Handle<Mesh> {
		CYLINDER
			.get()
			.expect("TowerSilhouettePlugin must run before silhouette scenes")
			.clone()
	}

	pub fn material() -> Handle<StandardMaterial> {
		MATERIAL
			.get()
			.expect("TowerSilhouettePlugin must run before silhouette scenes")
			.clone()
	}

	fn init(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
		let _ = CYLINDER.set(meshes.add(Mesh::from(Cylinder { radius: 0.5, half_height: 0.5 })));
		let _ = MATERIAL.set(materials.add(StandardMaterial {
			base_color: Color::srgb(0.45, 0.42, 0.38),
			perceptual_roughness: 0.9,
			..default()
		}));
	}
}

/// Posed silhouette mesh (unit mesh scaled by `transform`).
pub fn silhouette_scene(
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
	transform: Transform,
) -> impl Scene + 'static {
	bsn! {
		Mesh3d({mesh})
		MeshMaterial3d::<StandardMaterial>({material})
		template_value(transform)
		Visibility::Inherited
	}
}

pub struct TowerSilhouettePlugin;

impl Plugin for TowerSilhouettePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_tower_silhouettes);
	}
}

fn init_tower_silhouettes(
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	TowerSilhouetteAssets::init(&mut meshes, &mut materials);
}
