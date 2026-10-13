//! Authoring knobs for development selection.

use bevy::prelude::*;
use procedural_common::NoiseParams;

use crate::cell::{DEFAULT_LIKELIHOOD, DEFAULT_SPATIAL_CORRELATION, DEVELOPMENT_CELL_SIZE};

/// Urbanization Hopscotch frequency (forest parallel).
pub const URBANIZATION_NOISE_FREQUENCY: f32 = 0.0005;

/// Where procedural development sites come from. Authored developments
/// are placed under every choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DevelopmentSites {
	/// None: only authored developments.
	Authored,
	/// The legacy dense 300 m occupancy lattice.
	Lattice,
	/// Urbanization guillotine leaves.
	#[default]
	Urbanization,
}

/// World-level development selection config (materialized as a Bevy resource).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct DevelopmentConfig {
	pub seed: u32,
	/// Fill probability before hydro skip (`0.0..=1.0`).
	pub likelihood: f32,
	/// Occupancy correlation length (world units).
	pub spatial_correlation: f32,
	pub cell_size: f32,
	pub sites: DevelopmentSites,
	/// Relative kind weight after a cell passes occupancy.
	pub les_halles_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub shepherds_village_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub shepherds_commune_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub ring_fort_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub temple_complex_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub single_highrise_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub suburban_homes_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub wizards_tower_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub skybridge_bazaar_weight: f32,
	/// Relative kind weight after a cell passes occupancy.
	pub old_city_market_weight: f32,
}

impl Default for DevelopmentConfig {
	fn default() -> Self {
		Self {
			seed: 42,
			likelihood: DEFAULT_LIKELIHOOD,
			spatial_correlation: DEFAULT_SPATIAL_CORRELATION,
			cell_size: DEVELOPMENT_CELL_SIZE,
			sites: DevelopmentSites::default(),
			les_halles_weight: 1.0,
			shepherds_village_weight: 1.0,
			shepherds_commune_weight: 1.0,
			ring_fort_weight: 1.0,
			temple_complex_weight: 1.0,
			single_highrise_weight: 1.0,
			suburban_homes_weight: 1.0,
			wizards_tower_weight: 1.0,
			skybridge_bazaar_weight: 1.0,
			old_city_market_weight: 1.0,
		}
	}
}

impl DevelopmentConfig {
	pub fn from_world_seed(seed: u32) -> Self {
		Self { seed, ..Self::default() }
	}

	pub fn occupancy_seed(&self) -> u32 {
		self.seed.wrapping_add(0xDE7E_10D0)
	}

	/// Noise params for urbanization Hopscotch / leaf throws.
	pub fn urbanization_noise(&self) -> NoiseParams {
		NoiseParams {
			seed: self.seed as i32,
			frequency: URBANIZATION_NOISE_FREQUENCY,
			amplitude: 1.0,
			octaves: 1,
			..Default::default()
		}
	}
}
