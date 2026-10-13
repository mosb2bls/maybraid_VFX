//! Heightfield SDF terrain with elevation modulations.

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType};
use render_item::mesh::{IdentifiedMesh, MeshId};
use render_item::NormalizeChunk;
use sdf::{Sdf, Sign, SignBoundary, SignUniformIntervals};
use std::fmt::Debug;
use std::sync::{Arc, OnceLock};

/// Trait for elevation modulations that modify terrain height in 2.5D.
pub trait ElevationModulation: Send + Sync + Debug {
	fn modify_elevation(
		&self,
		terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		index: usize,
	) -> f32;

	/// Deterministic CpuShot identity fragment.
	///
	/// Override when [`Debug`] includes `HashMap` buckets or other unstable
	/// order — those would miss the disk mesh cache every process.
	fn mesh_identity(&self) -> String {
		format!("{self:?}")
	}
}

/// SDF representation of noise-based terrain:
/// `f(p) = p.y - height(p.x, p.z)` intersected with bedrock.
#[derive(Clone)]
pub struct TerrainSdf {
	noise: NoiseConfig,
	height_scale: f32,
	elevation_modulations: Vec<Arc<dyn ElevationModulation>>,
	bounds: Option<[Vec2; 4]>,
}

impl Debug for TerrainSdf {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let modulations: Vec<String> =
			self.elevation_modulations.iter().map(|m| m.mesh_identity()).collect();
		f.debug_struct("TerrainSdf")
			.field("noise_params", self.noise.params())
			.field("height_scale", &self.height_scale)
			.field("modulations", &modulations)
			.field("bounds", &self.bounds)
			.finish()
	}
}

impl TerrainSdf {
	pub fn new(seed: u32, height_scale: f32) -> Self {
		let noise = NoiseConfig::new(NoiseParams {
			seed: seed as i32,
			frequency: 0.0005,
			amplitude: 1.0,
			octaves: 4,
			noise_type: NoiseType::Perlin,
		});
		Self { noise, height_scale, elevation_modulations: Vec::new(), bounds: None }
	}

	pub fn with_bounds(mut self, bounds: [Vec2; 4]) -> Self {
		self.bounds = Some(bounds);
		self
	}

	pub fn add_elevation_modulation(&mut self, modulation: Box<dyn ElevationModulation>) {
		self.elevation_modulations.push(Arc::from(modulation));
	}

	fn height_at(&self, world_x: f32, world_z: f32) -> f32 {
		if let Some(bounds) = &self.bounds {
			if world_x < bounds[0].x
				|| world_x > bounds[1].x
				|| world_z < bounds[0].y
				|| world_z > bounds[1].y
			{
				return 0.0;
			}
		}

		// Manual octave ladder matching the legacy terrain shaping (amplitude/frequency
		// halve / double per octave) before contrast + height scale.
		let mut height = 0.0;
		let mut amplitude = 1.0;
		let mut frequency = self.noise.params().frequency;
		let seed = self.noise.params().seed;
		for _ in 0..self.noise.params().octaves.max(1) {
			let octave = NoiseConfig::new(NoiseParams {
				seed,
				frequency,
				amplitude: 1.0,
				octaves: 1,
				noise_type: NoiseType::Perlin,
			});
			height += octave.sample_2d(Vec2::new(world_x, world_z)) * amplitude;
			amplitude *= 0.5;
			frequency *= 2.0;
		}

		let exponent = 1.1;
		let sign = height.signum();
		let height = sign * height.abs().powf(exponent);
		height * self.height_scale
	}

	pub fn height_at_with_all_modulations(&self, world_x: f32, world_z: f32) -> f32 {
		let mut terrain_height = self.height_at(world_x, world_z);
		for modulation in &self.elevation_modulations {
			terrain_height = modulation.modify_elevation(self, terrain_height, world_x, world_z, 0);
		}
		terrain_height
	}
}

impl Sdf for TerrainSdf {
	fn distance(&self, p: Vec3) -> f32 {
		let terrain_height = self.height_at_with_all_modulations(p.x, p.z);
		let bedrock_level = -self.height_scale * 4.0;
		let d_surface = p.y - terrain_height;
		let d_bedrock = bedrock_level - p.y;
		d_surface.max(d_bedrock)
	}

	fn sign_uniform_on_y(&self, x: f32, z: f32) -> SignUniformIntervals {
		let mut intervals = SignUniformIntervals::default();
		intervals.insert_boundary(SignBoundary { min: f32::NEG_INFINITY, sign: Sign::Positive });
		let bedrock_level = -self.height_scale * 4.0;
		intervals.insert_boundary(SignBoundary { min: bedrock_level, sign: Sign::Negative });
		let height = self.height_at_with_all_modulations(x, z);
		intervals.insert_boundary(SignBoundary { min: height, sign: Sign::Positive });
		intervals
	}
}

impl NormalizeChunk for TerrainSdf {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		cascade_chunk.clone()
	}
}

impl IdentifiedMesh for TerrainSdf {
	fn id(&self) -> MeshId {
		MeshId::new(format!("{:?}", self))
	}
}

/// World SDF: heightfield terrain with an optional tube carve (difference).
#[derive(Clone)]
pub struct ComposedTerrain {
	terrain: TerrainSdf,
	tube: Option<sdf::TubeSdf>,
	mesh_id: OnceLock<MeshId>,
}

impl Debug for ComposedTerrain {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ComposedTerrain")
			.field("terrain", &self.terrain)
			.field("tube", &self.tube)
			.finish()
	}
}

impl ComposedTerrain {
	pub fn from_terrain(terrain: TerrainSdf) -> Self {
		Self { terrain, tube: None, mesh_id: OnceLock::new() }
	}

	pub fn terrain(&self) -> &TerrainSdf {
		&self.terrain
	}

	pub fn with_tube(mut self, tube: sdf::TubeSdf) -> Self {
		self.tube = Some(tube);
		self.mesh_id = OnceLock::new();
		self
	}
}

impl Sdf for ComposedTerrain {
	fn distance(&self, p: Vec3) -> f32 {
		let d = self.terrain.distance(p);
		match &self.tube {
			Some(tube) => d.max(-tube.distance(p)),
			None => d,
		}
	}

	fn sign_uniform_on_y(&self, x: f32, z: f32) -> SignUniformIntervals {
		match &self.tube {
			Some(tube) => self
				.terrain
				.sign_uniform_on_y(x, z)
				.interval_mapping(&tube.sign_uniform_on_y(x, z))
				.difference()
				.normalize(),
			None => self.terrain.sign_uniform_on_y(x, z),
		}
	}
}

impl NormalizeChunk for ComposedTerrain {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		cascade_chunk.clone()
	}
}

impl IdentifiedMesh for ComposedTerrain {
	fn id(&self) -> MeshId {
		self.mesh_id.get_or_init(|| MeshId::new(format!("{self:?}"))).clone()
	}
}
