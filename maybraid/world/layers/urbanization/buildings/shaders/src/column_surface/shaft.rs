//! Placeholder Doric shaft mesh that satisfies the column POM UV contract.
//!
//! The shaft is a circular cylinder of revolution: fluting is POM, not
//! silhouette. Caps are omitted so capital / pedestal stay real kit parts.

use std::f32::consts::{PI, TAU};

use bevy::{
	asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

/// Metres. ~1:6 Doric proportion on a 2.5 m preview shaft.
pub const PLACEHOLDER_SHAFT_RADIUS_M: f32 = 0.42;
pub const PLACEHOLDER_SHAFT_HEIGHT_M: f32 = 2.52;
pub const PLACEHOLDER_SHAFT_LONGITUDES: u32 = 48;
pub const PLACEHOLDER_SHAFT_RINGS: u32 = 12;

/// Target authored texels per flute around U (one wrap = [`crate::DEFAULT_FLUTE_COUNT`]).
pub const TEXELS_PER_FLUTE: u32 = 96;
/// Target authored texels per metre along V.
pub const TEXELS_PER_METRE_V: u32 = 384;

/// Open circular shaft with duplicated U-seam vertices, outward normals, and
/// a right-handed tangent frame: **+U** circumferential, **+V** up, **N** out.
pub fn placeholder_shaft_mesh() -> Mesh {
	PlaceholderShaft::default().build()
}

/// Authored dimensions for the placeholder (and the UV contract it encodes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceholderShaft {
	pub radius_m: f32,
	pub height_m: f32,
	pub longitudes: u32,
	pub rings: u32,
}

impl Default for PlaceholderShaft {
	fn default() -> Self {
		Self {
			radius_m: PLACEHOLDER_SHAFT_RADIUS_M,
			height_m: PLACEHOLDER_SHAFT_HEIGHT_M,
			longitudes: PLACEHOLDER_SHAFT_LONGITUDES,
			rings: PLACEHOLDER_SHAFT_RINGS,
		}
	}
}

impl PlaceholderShaft {
	/// Recommended height-map width for one U wrap at [`TEXELS_PER_FLUTE`].
	pub fn recommended_height_map_width(self, flute_count: u32) -> u32 {
		flute_count.saturating_mul(TEXELS_PER_FLUTE)
	}

	/// Recommended height-map height for this shaft at [`TEXELS_PER_METRE_V`].
	pub fn recommended_height_map_height(self) -> u32 {
		(self.height_m * TEXELS_PER_METRE_V as f32).round() as u32
	}

	pub fn build(self) -> Mesh {
		let cols = self.longitudes + 1;
		let rows = self.rings + 1;
		let vertex_count = (cols * rows) as usize;
		let mut positions = Vec::with_capacity(vertex_count);
		let mut normals = Vec::with_capacity(vertex_count);
		let mut uvs = Vec::with_capacity(vertex_count);
		let mut tangents = Vec::with_capacity(vertex_count);

		for row in 0..rows {
			let v = row as f32 / self.rings as f32;
			let y = (v - 0.5) * self.height_m;
			for col in 0..cols {
				let u = col as f32 / self.longitudes as f32;
				// U = 0/1 on local -Z so the seam sits opposite a +Z approach.
				let theta = TAU * u - PI;
				let (sin_t, cos_t) = theta.sin_cos();
				let x = self.radius_m * sin_t;
				let z = self.radius_m * cos_t;
				positions.push([x, y, z]);
				normals.push([sin_t, 0.0, cos_t]);
				uvs.push([u, v]);
				// dP/dU = R * TAU * (cos θ, 0, −sin θ); handedness +1 so T × B = N.
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
				indices.extend([i0, i2, i1, i1, i2, i3]);
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
			assert!((radial - spec.radius_m).abs() < 1e-5);
			assert!(position[1].abs() <= spec.height_m * 0.5 + 1e-5);
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
		assert!((positions[0][2] + spec.radius_m).abs() < 1e-4);
		assert!((positions[spec.longitudes as usize][2] + spec.radius_m).abs() < 1e-4);

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
	fn recommended_maps_meet_texel_density() -> anyhow::Result<()> {
		let spec = PlaceholderShaft::default();
		assert_eq!(spec.recommended_height_map_width(20), 1920);
		assert_eq!(spec.recommended_height_map_height(), 968);
		assert!(TEXELS_PER_FLUTE >= 64);
		assert!(TEXELS_PER_METRE_V >= 256);
		Ok(())
	}
}
