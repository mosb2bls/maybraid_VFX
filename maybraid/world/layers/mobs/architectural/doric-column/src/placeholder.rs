//! Procedural placeholder meshes until authored GLBs replace them.
//!
//! D-normalized meshes ([`crate::authoring`]); runtime applies uniform **D** via [`part_placements`](crate::layout::part_placements).

use std::f32::consts::{PI, TAU};

use crate::authoring::authored_extents;
use crate::spec::DoricColumnRatios;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use material_ref::MaterialRefApplied;

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
	let r = DoricColumnRatios::default();
	let (plinth, shaft, capital) = authored_extents(r);
	let pedestal = meshes.add(solid_square_box_mesh(plinth.half_side_d, plinth.height_d));
	let shaft = meshes.add(solid_shaft_mesh(
		shaft.radius_bottom_d,
		shaft.radius_top_d,
		shaft.height_d,
		32,
		16,
	));
	let capital = meshes.add(solid_capital_placeholder_mesh(
		capital.echinus_radius_bottom_d,
		capital.echinus_radius_top_d,
		capital.echinus_height_d,
		capital.abacus_half_side_d,
		capital.abacus_height_d,
		32,
	));
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
		commands.entity(entity).remove::<MaterialRefApplied>();
		commands.entity(entity).insert(Mesh3d(mesh));
	}
}

/// Triangle soup for one closed frustum/cylinder (Y-up, y ∈ [0, height]).
#[derive(Clone, Debug)]
pub(crate) struct SolidMesh {
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	pub indices: Vec<u32>,
}

impl SolidMesh {
	pub(crate) fn into_bevy_mesh(self) -> Mesh {
		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
		mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
		mesh.insert_indices(Indices::U32(self.indices));
		mesh
	}
}

/// Closed frustum or cylinder: side quads + bottom cap + top cap, outward normals.
#[allow(dead_code)]
pub(crate) fn solid_frustum_mesh(
	bottom_radius: f32,
	top_radius: f32,
	height: f32,
	segments: u32,
) -> Mesh {
	solid_frustum_data(bottom_radius, top_radius, height, segments).into_bevy_mesh()
}

pub(crate) fn solid_frustum_data(
	bottom_radius: f32,
	top_radius: f32,
	height: f32,
	segments: u32,
) -> SolidMesh {
	let segments = segments.max(3);
	let n = segments as usize;
	let mut bottom_ring = Vec::with_capacity(n);
	let mut top_ring = Vec::with_capacity(n);
	for i in 0..n {
		let t = i as f32 / segments as f32 * TAU;
		let (sin, cos) = t.sin_cos();
		bottom_ring.push(Vec3::new(cos * bottom_radius, 0.0, sin * bottom_radius));
		top_ring.push(Vec3::new(cos * top_radius, height, sin * top_radius));
	}

	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut indices = Vec::new();

	let mut push_tri = |a: Vec3, b: Vec3, c: Vec3| {
		let normal = (b - a).cross(c - a).normalize_or_zero();
		let base = positions.len() as u32;
		positions.extend([a, b, c]);
		normals.extend([normal, normal, normal]);
		indices.extend([base, base + 1, base + 2]);
	};

	for i in 0..n {
		let i1 = (i + 1) % n;
		let b0 = bottom_ring[i];
		let b1 = bottom_ring[i1];
		let t0 = top_ring[i];
		let t1 = top_ring[i1];
		push_tri(b0, t1, b1);
		push_tri(b0, t0, t1);
	}

	let bottom_center = Vec3::ZERO;
	for i in 0..n {
		let i1 = (i + 1) % n;
		push_tri(bottom_center, bottom_ring[i], bottom_ring[i1]);
	}

	let top_center = Vec3::new(0.0, height, 0.0);
	for i in 0..n {
		let i1 = (i + 1) % n;
		push_tri(top_center, top_ring[i1], top_ring[i]);
	}

	SolidMesh { positions, normals, indices }
}

/// Closed axis-aligned box: **±X** / **±Z** faces, bottom at **Y = 0**, centered on the axis.
pub(crate) fn solid_square_box_mesh(half_side: f32, height: f32) -> Mesh {
	solid_square_box_data(half_side, height).into_bevy_mesh()
}

pub(crate) fn solid_square_box_data(half_side: f32, height: f32) -> SolidMesh {
	let hx = half_side;
	let hz = half_side;
	let y0 = 0.0;
	let y1 = height;
	let corners = [
		Vec3::new(-hx, y0, -hz),
		Vec3::new(hx, y0, -hz),
		Vec3::new(hx, y0, hz),
		Vec3::new(-hx, y0, hz),
		Vec3::new(-hx, y1, -hz),
		Vec3::new(hx, y1, -hz),
		Vec3::new(hx, y1, hz),
		Vec3::new(-hx, y1, hz),
	];

	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut indices = Vec::new();
	let mut push_tri = |a: Vec3, b: Vec3, c: Vec3| {
		let normal = (b - a).cross(c - a).normalize_or_zero();
		let base = positions.len() as u32;
		positions.extend([a, b, c]);
		normals.extend([normal, normal, normal]);
		indices.extend([base, base + 1, base + 2]);
	};

	// Sides (+X, +Z, -X, -Z outward).
	push_tri(corners[1], corners[5], corners[6]);
	push_tri(corners[1], corners[6], corners[2]);
	push_tri(corners[2], corners[6], corners[7]);
	push_tri(corners[2], corners[7], corners[3]);
	push_tri(corners[3], corners[7], corners[4]);
	push_tri(corners[3], corners[4], corners[0]);
	push_tri(corners[0], corners[4], corners[5]);
	push_tri(corners[0], corners[5], corners[1]);

	let bottom_center = Vec3::new(0.0, y0, 0.0);
	push_tri(bottom_center, corners[0], corners[1]);
	push_tri(bottom_center, corners[1], corners[2]);
	push_tri(bottom_center, corners[2], corners[3]);
	push_tri(bottom_center, corners[3], corners[0]);

	let top_center = Vec3::new(0.0, y1, 0.0);
	push_tri(top_center, corners[5], corners[4]);
	push_tri(top_center, corners[6], corners[5]);
	push_tri(top_center, corners[7], corners[6]);
	push_tri(top_center, corners[4], corners[7]);

	SolidMesh { positions, normals, indices }
}

/// Round echinus + square abacus in one mesh (heights in **D** units).
pub(crate) fn solid_capital_placeholder_mesh(
	echinus_radius_bottom: f32,
	echinus_radius_top: f32,
	echinus_height_d: f32,
	abacus_half_side_d: f32,
	abacus_height_d: f32,
	segments: u32,
) -> Mesh {
	let echinus_h = echinus_height_d.max(1e-3);
	let echinus = solid_frustum_data(echinus_radius_bottom, echinus_radius_top, echinus_h, segments);
	let abacus = solid_square_box_data(abacus_half_side_d, abacus_height_d);
	let offset_y = echinus_h;
	let mut positions = echinus.positions;
	let mut normals = echinus.normals;
	let mut indices = echinus.indices;
	let base = positions.len() as u32;
	for p in abacus.positions {
		positions.push(p + Vec3::new(0.0, offset_y, 0.0));
	}
	normals.extend(abacus.normals);
	for i in abacus.indices {
		indices.push(i + base);
	}
	SolidMesh { positions, normals, indices }.into_bevy_mesh()
}

/// Closed tapered shaft with column POM UV/tangent contract on the side wall.
///
/// Side: **U** wraps once with seam duplicated on local **−Z**; **V** is 0 at bottom, 1 at top.
/// Top/bottom caps are separate UV islands (`U = 0.5`) so fluting UVs do not cover caps.
pub(crate) fn solid_shaft_mesh(
	radius_bottom: f32,
	radius_top: f32,
	height: f32,
	longitudes: u32,
	rings: u32,
) -> Mesh {
	let longitudes = longitudes.max(3);
	let rings = rings.max(1);
	let cols = longitudes + 1;
	let rows = rings + 1;

	let mut positions: Vec<[f32; 3]> = Vec::with_capacity((cols * rows + 2) as usize);
	let mut normals: Vec<[f32; 3]> = Vec::with_capacity(positions.capacity());
	let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(positions.capacity());
	let mut tangents: Vec<[f32; 4]> = Vec::with_capacity(positions.capacity());

	for row in 0..rows {
		let v = row as f32 / rings as f32;
		let y = v * height;
		let radius = radius_bottom + (radius_top - radius_bottom) * v;
		for col in 0..cols {
			let u = col as f32 / longitudes as f32;
			let theta = TAU * u - PI;
			let (sin_t, cos_t) = theta.sin_cos();
			positions.push([radius * sin_t, y, radius * cos_t]);
			normals.push([sin_t, 0.0, cos_t]);
			uvs.push([u, v]);
			tangents.push([cos_t, 0.0, -sin_t, 1.0]);
		}
	}

	let bottom_center = positions.len() as u32;
	positions.push([0.0, 0.0, 0.0]);
	normals.push([0.0, -1.0, 0.0]);
	uvs.push([0.5, 0.0]);
	tangents.push([1.0, 0.0, 0.0, 1.0]);

	let top_center = positions.len() as u32;
	positions.push([0.0, height, 0.0]);
	normals.push([0.0, 1.0, 0.0]);
	uvs.push([0.5, 1.0]);
	tangents.push([1.0, 0.0, 0.0, 1.0]);

	let mut indices = Vec::with_capacity((longitudes * rings * 6 + longitudes * 6) as usize);
	for row in 0..rings {
		for col in 0..longitudes {
			let i0 = row * cols + col;
			let i1 = i0 + 1;
			let i2 = i0 + cols;
			let i3 = i2 + 1;
			indices.extend([i0, i1, i2, i1, i3, i2]);
		}
	}

	for col in 0..longitudes {
		let i0 = col;
		let i1 = col + 1;
		indices.extend([bottom_center, i1, i0]);
	}

	let top_row = rings * cols;
	for col in 0..longitudes {
		let i0 = top_row + col;
		let i1 = i0 + 1;
		indices.extend([top_center, i0, i1]);
	}

	Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
		.with_inserted_indices(Indices::U32(indices))
		.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
		.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
		.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
		.with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, tangents)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::mesh::VertexAttributeValues;
	use std::collections::HashMap;

	fn mesh_positions(mesh: &Mesh) -> Option<Vec<Vec3>> {
		mesh.attribute(Mesh::ATTRIBUTE_POSITION)
			.and_then(|values| values.as_float3())
			.map(|slice| slice.iter().map(|p| Vec3::from_array(*p)).collect())
	}

	fn mesh_indices(mesh: &Mesh) -> Option<Vec<u32>> {
		match mesh.indices() {
			Some(Indices::U32(indices)) => Some(indices.to_vec()),
			_ => None,
		}
	}

	fn weld_key(position: Vec3) -> (i64, i64, i64) {
		let scale = 100_000.0;
		(
			(position.x * scale).round() as i64,
			(position.y * scale).round() as i64,
			(position.z * scale).round() as i64,
		)
	}

	/// Every geometric edge appears in exactly two triangles (welds duplicated soup vertices).
	fn mesh_is_closed(positions: &[Vec3], indices: &[u32]) -> bool {
		let mut edges: HashMap<((i64, i64, i64), (i64, i64, i64)), u32> = HashMap::new();
		for tri in indices.chunks_exact(3) {
			let keys = [
				weld_key(positions[tri[0] as usize]),
				weld_key(positions[tri[1] as usize]),
				weld_key(positions[tri[2] as usize]),
			];
			for edge in [(keys[0], keys[1]), (keys[1], keys[2]), (keys[2], keys[0])] {
				let key = if edge.0 <= edge.1 { edge } else { (edge.1, edge.0) };
				edges.entry(key).and_modify(|c| *c += 1).or_insert(1);
			}
		}
		edges.values().all(|&count| count == 2)
	}

	fn mesh_normals_point_outward(positions: &[Vec3], indices: &[u32], height: f32) -> bool {
		for tri in indices.chunks_exact(3) {
			let a = positions[tri[0] as usize];
			let b = positions[tri[1] as usize];
			let c = positions[tri[2] as usize];
			let geom_n = (b - a).cross(c - a);
			if geom_n.length_squared() < 1e-12 {
				return false;
			}
			let centroid = (a + b + c) / 3.0;
			let outward = if centroid.y <= height * 0.02 {
				Vec3::NEG_Y
			} else if centroid.y >= height * 0.98 {
				Vec3::Y
			} else {
				let radial = Vec3::new(centroid.x, 0.0, centroid.z);
				if radial.length_squared() < 1e-10 {
					continue;
				}
				radial.normalize()
			};
			if geom_n.dot(outward) <= 0.0 {
				return false;
			}
		}
		true
	}

	fn check_solid(mesh: &SolidMesh, height: f32) {
		assert!(mesh_is_closed(&mesh.positions, &mesh.indices), "mesh should be closed");
		assert!(
			mesh_normals_point_outward(&mesh.positions, &mesh.indices, height),
			"normals should point outward"
		);
	}

	fn square_faces_align_to_axes(mesh: &SolidMesh, half: f32) {
		for p in &mesh.positions {
			if p.x.abs() < 1e-4 && p.z.abs() < 1e-4 {
				continue;
			}
			let on_x = (p.x.abs() - half).abs() < 1e-4;
			let on_z = (p.z.abs() - half).abs() < 1e-4;
			assert!(on_x || on_z, "vertex {:?} should lie on ±X or ±Z square faces", p);
		}
	}

	fn check_shaft_closed(mesh: &Mesh, height_d: f32) {
		let positions = mesh_positions(mesh).expect("positions");
		let indices = mesh_indices(mesh).expect("indices");
		assert!(
			mesh_is_closed(&positions, &indices),
			"shaft should be closed"
		);
		assert!(
			mesh_normals_point_outward(&positions, &indices, height_d),
			"shaft normals should point outward"
		);
	}

	#[test]
	fn plinth_placeholder_is_closed_square_and_axis_aligned() {
		let r = DoricColumnRatios::default();
		let (plinth, _, _) = authored_extents(r);
		let mesh = solid_square_box_data(plinth.half_side_d, plinth.height_d);
		check_solid(&mesh, plinth.height_d);
		square_faces_align_to_axes(&mesh, plinth.half_side_d);
	}

	#[test]
	fn capital_placeholder_is_closed_echinus_and_abacus() {
		let r = DoricColumnRatios::default();
		let (_, _, capital) = authored_extents(r);
		let echinus = solid_frustum_data(
			capital.echinus_radius_bottom_d,
			capital.echinus_radius_top_d,
			capital.echinus_height_d,
			24,
		);
		check_solid(&echinus, capital.echinus_height_d);
		let abacus = solid_square_box_data(capital.abacus_half_side_d, capital.abacus_height_d);
		check_solid(&abacus, capital.abacus_height_d);
		square_faces_align_to_axes(&abacus, capital.abacus_half_side_d);
	}

	#[test]
	fn shaft_placeholder_is_closed_and_outward() {
		let r = DoricColumnRatios::default();
		let (_, shaft, _) = authored_extents(r);
		check_shaft_closed(
			&solid_shaft_mesh(shaft.radius_bottom_d, shaft.radius_top_d, shaft.height_d, 24, 8),
			shaft.height_d,
		);
	}

	#[test]
	fn shaft_uv_seam_is_duplicated_on_local_neg_z() {
		let r = DoricColumnRatios::default();
		let (_, shaft, _) = authored_extents(r);
		let mesh = solid_shaft_mesh(shaft.radius_bottom_d, shaft.radius_top_d, shaft.height_d, 24, 4);
		let positions = mesh_positions(&mesh).expect("positions");
		let uvs = mesh
			.attribute(Mesh::ATTRIBUTE_UV_0)
			.and_then(|values| match values {
				VertexAttributeValues::Float32x2(uv) => Some(uv.clone()),
				_ => None,
			})
			.expect("uvs");
		let longitudes = 24;
		let cols = longitudes + 1;

		assert!((uvs[0][0] - 0.0).abs() < 1e-5);
		assert!((uvs[longitudes as usize][0] - 1.0).abs() < 1e-5);
		assert!(positions[0].x.abs() < 1e-5);
		assert!((positions[0].z + shaft.radius_bottom_d).abs() < 1e-4);
		assert!((positions[longitudes as usize].z + shaft.radius_bottom_d).abs() < 1e-4);

		for row in 0..5 {
			let i0 = (row * cols) as usize;
			let i1 = i0 + longitudes as usize;
			assert!((uvs[i0][0] - 0.0).abs() < 1e-5);
			assert!((uvs[i1][0] - 1.0).abs() < 1e-5);
			assert!((uvs[i0][1] - uvs[i1][1]).abs() < 1e-5);
		}
	}

	#[test]
	fn shaft_tangents_are_right_handed_and_circumferential() {
		let r = DoricColumnRatios::default();
		let (_, shaft, _) = authored_extents(r);
		let mesh = solid_shaft_mesh(shaft.radius_bottom_d, shaft.radius_top_d, shaft.height_d, 16, 4);
		let normals = mesh
			.attribute(Mesh::ATTRIBUTE_NORMAL)
			.and_then(|v| v.as_float3())
			.expect("normals");
		let tangents = mesh
			.attribute(Mesh::ATTRIBUTE_TANGENT)
			.and_then(|values| match values {
				VertexAttributeValues::Float32x4(t) => Some(t.clone()),
				_ => None,
			})
			.expect("tangents");
		assert_eq!(normals.len(), tangents.len());

		for (normal, tangent) in normals.iter().zip(tangents.iter()) {
			let t = Vec3::new(tangent[0], tangent[1], tangent[2]);
			let n = Vec3::from_array(*normal);
			if n.y.abs() > 0.99 {
				continue;
			}
			assert!((t.length() - 1.0).abs() < 1e-4);
			assert!(t.dot(n).abs() < 1e-4);
			assert!((tangent[3] - 1.0).abs() < 1e-5);
			let bitangent = n.cross(t);
			assert!(bitangent.y > 0.9);
		}
	}
}
