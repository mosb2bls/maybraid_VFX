//! Placeholder Doric shaft meshes that satisfy the column POM UV contract.
//!
//! Fluting is POM, not silhouette. Caps are omitted so capital / pedestal stay
//! real kit parts. [#1144](https://github.com/ramate-io/maybraid/pull/1144)
//! drives classical proportions from the **lower diameter D**: the shaft tapers
//! to about `0.8 D`, the full column (square plinth + shaft + square abacus)
//! is about `6–7 D` tall. U still wraps once while the circumference shrinks
//! up V.

use std::f32::consts::{PI, TAU};

use bevy::{
	asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

/// Preview **lower diameter D** (metres). #1144 picks the kit's final D;
/// texel targets below are written so they stay valid when that lands.
pub const PLACEHOLDER_LOWER_DIAMETER_M: f32 = 0.84;
/// Top diameter / D. Classical Doric necking (~0.8 D).
pub const PLACEHOLDER_SHAFT_TAPER: f32 = 0.8;
/// Authored shaft height in units of D (column is 6–7 D including plinth + abacus).
pub const PLACEHOLDER_TAPERED_SHAFT_HEIGHT_IN_D: f32 = 4.5;

/// Compact **cylinder** preview (same lower radius as [`PLACEHOLDER_LOWER_DIAMETER_M`]).
pub const PLACEHOLDER_SHAFT_RADIUS_M: f32 = PLACEHOLDER_LOWER_DIAMETER_M * 0.5;
pub const PLACEHOLDER_SHAFT_HEIGHT_M: f32 = 2.52;
pub const PLACEHOLDER_SHAFT_LONGITUDES: u32 = 48;
pub const PLACEHOLDER_SHAFT_RINGS: u32 = 12;

/// Target authored texels per flute around U (one wrap = [`crate::DEFAULT_FLUTE_COUNT`]).
/// Angular — independent of D.
pub const TEXELS_PER_FLUTE: u32 = 96;
/// Target authored texels per metre along V. Equivalently `× D` texels per D of height.
pub const TEXELS_PER_METRE_V: u32 = 384;

/// Open circular **cylinder** (bottom radius = top radius) with the UV / tangent contract.
pub fn placeholder_shaft_mesh() -> Mesh {
	PlaceholderShaft::default().build()
}

/// Open **tapered** shaft: lower diameter D, neck `0.8 D`, height `4.5 D`.
pub fn placeholder_tapered_shaft_mesh() -> Mesh {
	PlaceholderShaft::tapered_from_lower_diameter(PLACEHOLDER_LOWER_DIAMETER_M).build()
}

/// Authored dimensions for the placeholder (and the UV contract it encodes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceholderShaft {
	pub bottom_radius_m: f32,
	pub top_radius_m: f32,
	pub height_m: f32,
	pub longitudes: u32,
	pub rings: u32,
}

impl Default for PlaceholderShaft {
	fn default() -> Self {
		Self {
			bottom_radius_m: PLACEHOLDER_SHAFT_RADIUS_M,
			top_radius_m: PLACEHOLDER_SHAFT_RADIUS_M,
			height_m: PLACEHOLDER_SHAFT_HEIGHT_M,
			longitudes: PLACEHOLDER_SHAFT_LONGITUDES,
			rings: PLACEHOLDER_SHAFT_RINGS,
		}
	}
}

impl PlaceholderShaft {
	/// Classical preview taper driven by lower diameter D.
	pub fn tapered_from_lower_diameter(d: f32) -> Self {
		Self {
			bottom_radius_m: d * 0.5,
			top_radius_m: d * 0.5 * PLACEHOLDER_SHAFT_TAPER,
			height_m: d * PLACEHOLDER_TAPERED_SHAFT_HEIGHT_IN_D,
			longitudes: PLACEHOLDER_SHAFT_LONGITUDES,
			rings: PLACEHOLDER_SHAFT_RINGS,
		}
	}

	pub fn lower_diameter_m(self) -> f32 {
		self.bottom_radius_m * 2.0
	}

	pub fn radius_at_v(self, v: f32) -> f32 {
		self.bottom_radius_m.lerp(self.top_radius_m, v)
	}

	/// Recommended height-map width for one U wrap at [`TEXELS_PER_FLUTE`].
	/// D-invariant: U is angular (one wrap = flute count × texels/flute).
	pub fn recommended_height_map_width(self, flute_count: u32) -> u32 {
		flute_count.saturating_mul(TEXELS_PER_FLUTE)
	}

	/// Recommended height-map height at [`TEXELS_PER_METRE_V`].
	/// Scales with authored shaft height, which #1144 will set as a multiple of D.
	pub fn recommended_height_map_height(self) -> u32 {
		(self.height_m * TEXELS_PER_METRE_V as f32).round() as u32
	}

	/// V texel budget per D of shaft height (so the target survives a D change).
	pub fn texels_per_d_v(d: f32) -> f32 {
		TEXELS_PER_METRE_V as f32 * d
	}

	pub fn build(self) -> Mesh {
		let cols = self.longitudes + 1;
		let rows = self.rings + 1;
		let vertex_count = (cols * rows) as usize;
		let mut positions = Vec::with_capacity(vertex_count);
		let mut normals = Vec::with_capacity(vertex_count);
		let mut uvs = Vec::with_capacity(vertex_count);
		let mut tangents = Vec::with_capacity(vertex_count);
		let dr_dy = (self.top_radius_m - self.bottom_radius_m) / self.height_m.max(1e-6);

		for row in 0..rows {
			let v = row as f32 / self.rings as f32;
			// Bottom-center origin, Y ∈ [0, H] — same pivot as #1144 AUTHORING.md.
			let y = v * self.height_m;
			let radius = self.radius_at_v(v);
			// Outward cone normal: ∇(r_xz − r(y)) = (x/r, −dr/dy, z/r).
			let n_len = (1.0 + dr_dy * dr_dy).sqrt();
			let n_y = -dr_dy / n_len;
			let n_xz = 1.0 / n_len;
			for col in 0..cols {
				let u = col as f32 / self.longitudes as f32;
				// U = 0/1 on local -Z so the seam sits opposite a +Z approach.
				let theta = TAU * u - PI;
				let (sin_t, cos_t) = theta.sin_cos();
				let x = radius * sin_t;
				let z = radius * cos_t;
				positions.push([x, y, z]);
				normals.push([sin_t * n_xz, n_y, cos_t * n_xz]);
				uvs.push([u, v]);
				// dP/dU is circumferential even as r shrinks; T ⟂ N on a cone.
				let tx = cos_t;
				let tz = -sin_t;
				tangents.push([tx, 0.0, tz, 1.0]);
			}
		}

		let mut indices = Vec::with_capacity((self.longitudes * self.rings * 6) as usize);
		for row in 0..self.rings {
			for col in 0..self.longitudes {
				let i0 = row * cols + col;
				let i1 = i0 + 1;
				let i2 = i0 + cols;
				let i3 = i2 + 1;
				// Outward winding, same as #1144 `solid_shaft_mesh`: CCW when
				// looking against the authored normal (i0→i1→i2, i1→i3→i2).
				indices.extend([i0, i1, i2, i1, i3, i2]);
			}
		}

		Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
			.with_inserted_indices(Indices::U32(indices))
			.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
			.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
			.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
			.with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, tangents)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn positions(mesh: &Mesh) -> anyhow::Result<&[[f32; 3]]> {
		mesh.attribute(Mesh::ATTRIBUTE_POSITION)
			.and_then(|values| values.as_float3())
			.ok_or_else(|| anyhow::anyhow!("missing positions"))
	}

	fn normals(mesh: &Mesh) -> anyhow::Result<&[[f32; 3]]> {
		mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
			.and_then(|values| values.as_float3())
			.ok_or_else(|| anyhow::anyhow!("missing normals"))
	}

	fn uvs(mesh: &Mesh) -> anyhow::Result<&[[f32; 2]]> {
		let values = mesh
			.attribute(Mesh::ATTRIBUTE_UV_0)
			.ok_or_else(|| anyhow::anyhow!("missing uvs"))?;
		match values {
			bevy::mesh::VertexAttributeValues::Float32x2(uvs) => Ok(uvs.as_slice()),
			_ => Err(anyhow::anyhow!("uvs are not float32x2")),
		}
	}

	fn tangents(mesh: &Mesh) -> anyhow::Result<&[[f32; 4]]> {
		let values = mesh
			.attribute(Mesh::ATTRIBUTE_TANGENT)
			.ok_or_else(|| anyhow::anyhow!("missing tangents"))?;
		match values {
			bevy::mesh::VertexAttributeValues::Float32x4(tangents) => Ok(tangents.as_slice()),
			_ => Err(anyhow::anyhow!("tangents are not float32x4")),
		}
	}

	fn indices(mesh: &Mesh) -> anyhow::Result<&[u32]> {
		match mesh.indices() {
			Some(Indices::U32(indices)) => Ok(indices.as_slice()),
			_ => Err(anyhow::anyhow!("expected u32 indices")),
		}
	}

	/// Geometric winding must agree with authored vertex normals and point
	/// outward — same check as #1144 `mesh_normals_point_outward`.
	fn winding_matches_outward_normals(
		positions: &[[f32; 3]],
		normals: &[[f32; 3]],
		indices: &[u32],
	) -> Result<(), String> {
		for (tri_i, tri) in indices.chunks_exact(3).enumerate() {
			let a = Vec3::from_array(positions[tri[0] as usize]);
			let b = Vec3::from_array(positions[tri[1] as usize]);
			let c = Vec3::from_array(positions[tri[2] as usize]);
			let geom_n = (b - a).cross(c - a);
			if geom_n.length_squared() < 1e-12 {
				return Err(format!("triangle {tri_i} is degenerate"));
			}
			let vertex_n = Vec3::from_array(normals[tri[0] as usize])
				+ Vec3::from_array(normals[tri[1] as usize])
				+ Vec3::from_array(normals[tri[2] as usize]);
			if geom_n.dot(vertex_n) <= 0.0 {
				return Err(format!(
					"triangle {tri_i}: winding opposes vertex normals (dot {})",
					geom_n.normalize_or_zero().dot(vertex_n.normalize_or_zero())
				));
			}
			let centroid = (a + b + c) / 3.0;
			let outward = Vec3::new(centroid.x, 0.0, centroid.z);
			if outward.length_squared() < 1e-10 {
				continue;
			}
			if geom_n.dot(outward) <= 0.0 {
				return Err(format!("triangle {tri_i}: winding points inward"));
			}
		}
		Ok(())
	}

	#[test]
	fn placeholder_is_a_circular_open_shaft() -> anyhow::Result<()> {
		let spec = PlaceholderShaft::default();
		let mesh = spec.build();
		let positions = positions(&mesh)?;
		assert_eq!(positions.len(), ((spec.longitudes + 1) * (spec.rings + 1)) as usize);
		assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
		assert!(mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_some());

		for position in positions {
			let radial = (position[0] * position[0] + position[2] * position[2]).sqrt();
			assert!((radial - spec.bottom_radius_m).abs() < 1e-5);
			assert!(position[1] >= -1e-5);
			assert!(position[1] <= spec.height_m + 1e-5);
		}

		let Some(Indices::U32(indices)) = mesh.indices() else {
			return Err(anyhow::anyhow!("expected u32 indices"));
		};
		assert_eq!(indices.len(), (spec.longitudes * spec.rings * 6) as usize);
		Ok(())
	}

	#[test]
	fn uv_seam_is_duplicated_on_local_neg_z() -> anyhow::Result<()> {
		let spec = PlaceholderShaft::default();
		let mesh = spec.build();
		let positions = positions(&mesh)?;
		let uvs = uvs(&mesh)?;
		let cols = spec.longitudes + 1;

		let seam_u0 = uvs[0];
		let seam_u1 = uvs[spec.longitudes as usize];
		assert!((seam_u0[0] - 0.0).abs() < 1e-5);
		assert!((seam_u1[0] - 1.0).abs() < 1e-5);
		assert!((positions[0][0]).abs() < 1e-5);
		assert!((positions[0][2] + spec.bottom_radius_m).abs() < 1e-4);
		assert!((positions[spec.longitudes as usize][2] + spec.bottom_radius_m).abs() < 1e-4);

		for row in 0..spec.rings + 1 {
			let i0 = (row * cols) as usize;
			let i1 = i0 + spec.longitudes as usize;
			assert!((uvs[i0][0] - 0.0).abs() < 1e-5);
			assert!((uvs[i1][0] - 1.0).abs() < 1e-5);
			assert!((uvs[i0][1] - uvs[i1][1]).abs() < 1e-5);
		}
		Ok(())
	}

	#[test]
	fn tangents_are_right_handed_and_circumferential() -> anyhow::Result<()> {
		let mesh = placeholder_shaft_mesh();
		let normals = normals(&mesh)?;
		let tangents = tangents(&mesh)?;
		assert_eq!(normals.len(), tangents.len());

		for (normal, tangent) in normals.iter().zip(tangents) {
			let t = Vec3::new(tangent[0], tangent[1], tangent[2]);
			let n = Vec3::from_array(*normal);
			assert!((t.length() - 1.0).abs() < 1e-4);
			assert!(t.dot(n).abs() < 1e-4);
			assert!((tangent[3] - 1.0).abs() < 1e-5);
			let bitangent = n.cross(t);
			assert!(bitangent.y > 0.9);
			let reconstructed = t.cross(bitangent);
			assert!(reconstructed.dot(n) > 0.9);
		}
		Ok(())
	}

	#[test]
	fn winding_matches_vertex_normals_and_points_outward() -> anyhow::Result<()> {
		for mesh in [placeholder_shaft_mesh(), placeholder_tapered_shaft_mesh()] {
			winding_matches_outward_normals(positions(&mesh)?, normals(&mesh)?, indices(&mesh)?)
				.map_err(|error| anyhow::anyhow!(error))?;
		}
		Ok(())
	}

	#[test]
	fn recommended_maps_meet_texel_density() -> anyhow::Result<()> {
		let spec = PlaceholderShaft::default();
		assert_eq!(spec.recommended_height_map_width(20), 1920);
		assert_eq!(spec.recommended_height_map_height(), 968);
		assert!(TEXELS_PER_FLUTE >= 64);
		assert!(TEXELS_PER_METRE_V >= 256);
		Ok(())
	}

	#[test]
	fn tapered_shaft_shrinks_to_four_fifths_d_and_keeps_u_wrap() -> anyhow::Result<()> {
		let spec = PlaceholderShaft::tapered_from_lower_diameter(PLACEHOLDER_LOWER_DIAMETER_M);
		assert!((spec.lower_diameter_m() - PLACEHOLDER_LOWER_DIAMETER_M).abs() < 1e-5);
		assert!((spec.top_radius_m / spec.bottom_radius_m - PLACEHOLDER_SHAFT_TAPER).abs() < 1e-5);
		assert!(
			(spec.height_m - PLACEHOLDER_LOWER_DIAMETER_M * PLACEHOLDER_TAPERED_SHAFT_HEIGHT_IN_D)
				.abs() < 1e-5
		);

		let mesh = spec.build();
		let positions = positions(&mesh)?;
		let uvs = uvs(&mesh)?;
		let cols = spec.longitudes + 1;
		let top_row = spec.rings as usize;

		let bottom_r = (positions[0][0].hypot(positions[0][2])
			+ positions[spec.longitudes as usize][0].hypot(positions[spec.longitudes as usize][2]))
			* 0.5;
		let top_i = top_row * cols as usize;
		let top_r = (positions[top_i][0].hypot(positions[top_i][2])
			+ positions[top_i + spec.longitudes as usize][0]
				.hypot(positions[top_i + spec.longitudes as usize][2]))
			* 0.5;
		assert!((bottom_r - spec.bottom_radius_m).abs() < 1e-4);
		assert!((top_r - spec.top_radius_m).abs() < 1e-4);
		assert!((top_r / bottom_r - 0.8).abs() < 1e-3);

		assert!((uvs[0][0] - 0.0).abs() < 1e-5);
		assert!((uvs[spec.longitudes as usize][0] - 1.0).abs() < 1e-5);
		assert!((uvs[top_i][0] - 0.0).abs() < 1e-5);
		assert!((uvs[top_i + spec.longitudes as usize][0] - 1.0).abs() < 1e-5);
		assert!((uvs[top_i][1] - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn tapered_tangents_stay_perpendicular_to_cone_normals() -> anyhow::Result<()> {
		let mesh = placeholder_tapered_shaft_mesh();
		let normals = normals(&mesh)?;
		let tangents = tangents(&mesh)?;
		assert_eq!(normals.len(), tangents.len());

		for (normal, tangent) in normals.iter().zip(tangents) {
			let t = Vec3::new(tangent[0], tangent[1], tangent[2]);
			let n = Vec3::from_array(*normal);
			assert!((t.length() - 1.0).abs() < 1e-4);
			assert!((n.length() - 1.0).abs() < 1e-4);
			assert!(t.dot(n).abs() < 1e-4);
			assert!((tangent[3] - 1.0).abs() < 1e-5);
			let bitangent = n.cross(t);
			assert!(bitangent.y > 0.85);
			assert!(t.cross(bitangent).dot(n) > 0.9);
		}
		Ok(())
	}

	#[test]
	fn texel_targets_are_stated_in_d() -> anyhow::Result<()> {
		let d = PLACEHOLDER_LOWER_DIAMETER_M;
		let taper = PlaceholderShaft::tapered_from_lower_diameter(d);
		assert_eq!(taper.recommended_height_map_width(20), 1920);
		assert!((PlaceholderShaft::texels_per_d_v(d) - TEXELS_PER_METRE_V as f32 * d).abs() < 1e-4);
		assert_eq!(
			taper.recommended_height_map_height(),
			(PLACEHOLDER_TAPERED_SHAFT_HEIGHT_IN_D * PlaceholderShaft::texels_per_d_v(d)).round()
				as u32
		);
		Ok(())
	}
}
