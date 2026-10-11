//! Procedural placeholder meshes until authored GLBs replace them.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use crate::parts::DoricColumnPartSlot;

/// Marks entities that use [`DoricColumnPlaceholderPlugin`] meshes.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlaceholderVisual;

/// Selects which procedural mesh to instance.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoricColumnPartMesh(pub DoricColumnPartSlot);

#[derive(Resource, Clone)]
pub struct DoricColumnPlaceholderMeshes {
	pub pedestal: Handle<Mesh>,
	pub shaft: Handle<Mesh>,
	pub capital: Handle<Mesh>,
}

impl DoricColumnPlaceholderMeshes {
	pub fn mesh_for(&self, slot: DoricColumnPartSlot) -> Handle<Mesh> {
		match slot {
			DoricColumnPartSlot::Pedestal => self.pedestal.clone(),
			DoricColumnPartSlot::Shaft => self.shaft.clone(),
			DoricColumnPartSlot::Capital => self.capital.clone(),
		}
	}
}

/// Builds unit-height frustum/cylinder meshes with bottom center at origin (+Y up).
pub struct DoricColumnPlaceholderPlugin;

impl Plugin for DoricColumnPlaceholderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_placeholder_meshes)
			.add_systems(Update, attach_placeholder_meshes);
	}
}

fn init_placeholder_meshes(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands) {
	let pedestal = meshes.add(unit_frustum_mesh(1.0, 1.2, 32));
	let shaft = meshes.add(unit_cylinder_mesh(32));
	let capital = meshes.add(unit_frustum_mesh(1.15, 1.0, 32));
	commands.insert_resource(DoricColumnPlaceholderMeshes { pedestal, shaft, capital });
}

#[allow(clippy::type_complexity)]
fn attach_placeholder_meshes(
	mut commands: Commands,
	meshes: Res<DoricColumnPlaceholderMeshes>,
	pending: Query<(Entity, &DoricColumnPartMesh), (With<PlaceholderVisual>, Without<Mesh3d>)>,
) {
	for (entity, part) in &pending {
		let mesh = meshes.mesh_for(part.0);
		commands.entity(entity).insert(Mesh3d(mesh));
	}
}

/// Unit height along +Y, radius 0.5 at bottom (y=0) and `top_radius` at y=1, centered on XZ.
fn unit_cylinder_mesh(segments: u32) -> Mesh {
	unit_frustum_mesh(0.5, 0.5, segments)
}

fn unit_frustum_mesh(bottom_radius: f32, top_radius: f32, segments: u32) -> Mesh {
	let segments = segments.max(3);
	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut uvs = Vec::new();
	let mut indices = Vec::new();

	for i in 0..=segments {
		let t = i as f32 / segments as f32;
		let angle = t * std::f32::consts::TAU;
		let (sin, cos) = angle.sin_cos();
		for (y, r) in [(0.0, bottom_radius), (1.0, top_radius)] {
			let pos = Vec3::new(cos * r, y, sin * r);
			positions.push(pos);
			let normal = Vec3::new(cos, 0.0, sin).normalize();
			normals.push(normal);
			uvs.push([t, y]);
		}
	}

	for i in 0..segments {
		let i0 = i * 2;
		let i1 = i0 + 1;
		let i2 = i0 + 2;
		let i3 = i0 + 3;
		indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
	}

	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
	mesh.insert_indices(Indices::U32(indices));
	mesh
}
