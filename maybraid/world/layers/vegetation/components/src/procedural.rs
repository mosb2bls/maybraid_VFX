//! Procedural style backends for vegetation IR (unit stick / ball meshes).
//!
//! Named as style variants until GLBs under `maybraid/art/vegetation/` replace them.

use std::sync::OnceLock;

use bevy::mesh::primitives::{MeshBuilder, Meshable};
use bevy::mesh::VertexAttributeValues;
use bevy::mesh::{SphereKind, SphereMeshBuilder};
use bevy::prelude::*;

use crate::lod_host::inherit_not_shadow_caster_on_meshes;

/// Half-extent of stick kit on \(X/Z\) (\(X = Z \in [-\texttt{STICK\_KIT\_HALF}, \texttt{STICK\_KIT\_HALF}]\)).
pub const STICK_KIT_HALF: f32 = 0.2;

/// Half-extent of straight frond kit on \(X\) (\(X \in [-\texttt{FROND\_KIT\_HALF\_X}, \texttt{FROND\_KIT\_HALF\_X}]\)).
/// Kit length is \(Y \in [0, 1]\); \(Z\) is authored negligible (flat blade).
pub const FROND_KIT_HALF_X: f32 = 0.1;

/// Registers shared procedural meshes / materials for vegetation IR fallbacks.
pub struct VegetationProceduralPlugin;

impl Plugin for VegetationProceduralPlugin {
	fn build(&self, app: &mut App) {
		// Level updates are owned by lod refresh (region → level → sync). This plugin
		// only registers procedural assets.
		app.add_systems(Startup, init_procedural_assets)
			.add_systems(PostUpdate, inherit_not_shadow_caster_on_meshes);
	}
}

#[derive(Resource, Debug, Clone)]
pub struct VegetationProceduralAssets {
	pub stick_cylinder: Handle<Mesh>,
	pub foliage_ball: Handle<Mesh>,
	pub stick_material: Handle<StandardMaterial>,
	pub foliage_material: Handle<StandardMaterial>,
}

static ASSETS: OnceLock<VegetationProceduralAssets> = OnceLock::new();

impl VegetationProceduralAssets {
	pub fn get() -> &'static VegetationProceduralAssets {
		ASSETS
			.get()
			.expect("VegetationProceduralPlugin must run before vegetation scenes")
	}

	pub fn stick_cylinder() -> Handle<Mesh> {
		Self::get().stick_cylinder.clone()
	}

	pub fn foliage_ball() -> Handle<Mesh> {
		Self::get().foliage_ball.clone()
	}

	pub fn stick_material() -> Handle<StandardMaterial> {
		Self::get().stick_material.clone()
	}

	pub fn foliage_material() -> Handle<StandardMaterial> {
		Self::get().foliage_material.clone()
	}
}

fn init_procedural_assets(
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let stick_cylinder = meshes.add(stick_kit_cylinder_mesh());
	let foliage_ball =
		meshes.add(SphereMeshBuilder::new(1.0, SphereKind::Ico { subdivisions: 1 }).build());
	let stick_material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.45, 0.28, 0.14),
		perceptual_roughness: 0.9,
		..default()
	});
	let foliage_material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.22, 0.55, 0.18),
		perceptual_roughness: 0.85,
		cull_mode: None,
		..default()
	});
	let _ = ASSETS.set(VegetationProceduralAssets {
		stick_cylinder,
		foliage_ball,
		stick_material,
		foliage_material,
	});
}

/// Solid cylinder for stick kit space: \(Y \in [0, 1]\), radius [`STICK_KIT_HALF`].
fn stick_kit_cylinder_mesh() -> Mesh {
	let mut mesh = Cylinder::new(STICK_KIT_HALF, 1.0).mesh().build();
	// Bevy's cylinder is centered on Y; shift so base sits at Y = 0.
	if let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
	{
		for p in positions.iter_mut() {
			p[1] += 0.5;
		}
	}
	mesh
}
