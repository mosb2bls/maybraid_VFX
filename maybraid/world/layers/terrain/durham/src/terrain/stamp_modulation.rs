//! Adapter from jersey / marazion hydro height ops onto durham [`ElevationModulation`].

use crate::terrain::sdf::{ElevationModulation, TerrainSdf};
use std::sync::Arc;
use terrain_stamps::StampModulation;
use terrain_watersheds::HydroComplex;

/// One elevation op in the final terrain stack.
#[derive(Debug, Clone)]
pub enum ComposedElevationOp {
	Stamp(StampModulation),
	/// Shared indexed hydrology complex (internally carve → rim → apron).
	Hydro(Arc<HydroComplex>),
}

impl ComposedElevationOp {
	pub fn modify_elevation_xz(&self, elevation: f32, x: f32, z: f32) -> f32 {
		match self {
			Self::Stamp(m) => StampModulation::modify_elevation(m, elevation, x, z),
			Self::Hydro(h) => HydroComplex::modify_elevation(h, elevation, x, z),
		}
	}
}

impl ElevationModulation for StampModulation {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		StampModulation::modify_elevation(self, elevation, x, z)
	}
}

impl ElevationModulation for ComposedElevationOp {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		self.modify_elevation_xz(elevation, x, z)
	}

	fn mesh_identity(&self) -> String {
		match self {
			Self::Stamp(m) => format!("Stamp({m:?})"),
			Self::Hydro(h) => format!("Hydro({})", ElevationModulation::mesh_identity(h.as_ref())),
		}
	}
}

impl ElevationModulation for HydroComplex {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		HydroComplex::modify_elevation(self, elevation, x, z)
	}

	fn mesh_identity(&self) -> String {
		// Skip `index` (HashMap buckets) so CpuShot ids stay process-stable.
		format!(
			"HydroComplex {{ bounds: {:?}, seed: {}, hydrology: {:?} }}",
			self.bounds, self.seed, self.hydrology
		)
	}
}
