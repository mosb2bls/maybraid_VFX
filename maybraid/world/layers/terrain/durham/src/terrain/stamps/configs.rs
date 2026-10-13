//! Universal per-family guillotine + stamp authoring knobs (dual band).

use crate::terrain::stamps::canyon::{
	CanyonHighPassControllerLayout, CanyonLowPassControllerLayout,
};
use crate::terrain::stamps::massif::{
	MassifHighPassControllerLayout, MassifLowPassControllerLayout,
};
use crate::terrain::stamps::plateau::{
	PlateauHighPassControllerLayout, PlateauLowPassControllerLayout,
};
use crate::terrain::stamps::pocket_water::{
	PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout,
};
use crate::terrain::stamps::rolling::{
	RollingHighPassControllerLayout, RollingLowPassControllerLayout,
};
use crate::terrain::stamps::valley::{
	ValleyHighPassControllerLayout, ValleyLowPassControllerLayout,
};
use bevy::prelude::*;
use comproc::guillotine::GuillotineConfig;
use terrain_stamps::{
	CanyonParams, PlateauCapParams, PocketWaterParams, RollingGroundParams, RuggedMassifParams,
	ValleyTrainParams,
};

/// Guillotine cut knobs + stamp params + occupancy for one family band.
#[derive(Debug, Clone)]
pub struct FamilyGuillotineConfig<P> {
	pub seed: u32,
	pub depth: u8,
	pub guillotine: GuillotineConfig,
	pub noise_frequency: f32,
	/// Occupancy value-noise lattice spacing (world units) — spatial correlation length.
	pub spatial_correlation: f32,
	/// Approximate leaf acceptance rate (`0.0..=1.0`) for value-noise occupancy.
	///
	/// Prefer setting defaults in `define_stamp_family!` (`likelihood:`); this
	/// field is the runtime override on the resource.
	pub likelihood: f32,
	/// Per-leaf stamp strength lower bound (`1.0` ≈ default vertical knobs).
	pub strength_min: f32,
	/// Per-leaf stamp strength upper bound.
	pub strength_max: f32,
	pub stamp: P,
}

impl<P: Default> FamilyGuillotineConfig<P> {
	fn low_pass(
		seed: u32,
		likelihood: f32,
		spatial_correlation: f32,
		strength_min: f32,
		strength_max: f32,
		cell_size_min: f32,
		cell_size_max: f32,
	) -> Self {
		Self {
			seed,
			depth: 6,
			guillotine: GuillotineConfig::new(cell_size_min, cell_size_max).with_snap_quantum(20.0),
			noise_frequency: 0.05,
			spatial_correlation,
			likelihood: likelihood.clamp(0.0, 1.0),
			strength_min: strength_min.max(0.0),
			strength_max: strength_max.max(0.0),
			stamp: P::default(),
		}
	}

	fn high_pass(
		seed: u32,
		likelihood: f32,
		spatial_correlation: f32,
		strength_min: f32,
		strength_max: f32,
		cell_size_min: f32,
		cell_size_max: f32,
	) -> Self {
		Self {
			seed,
			depth: 4,
			guillotine: GuillotineConfig::new(cell_size_min, cell_size_max).with_snap_quantum(40.0),
			noise_frequency: 0.02,
			spatial_correlation,
			likelihood: likelihood.clamp(0.0, 1.0),
			strength_min: strength_min.max(0.0),
			strength_max: strength_max.max(0.0),
			stamp: P::default(),
		}
	}
}

/// Low-pass (detail) + high-pass (regional) knobs for one stamp family.
#[derive(Debug, Clone)]
pub struct DualBandFamilyConfig<P> {
	pub low_pass: FamilyGuillotineConfig<P>,
	pub high_pass: FamilyGuillotineConfig<P>,
}

/// Universal configs for every jersey family's dual-band guillotine grids.
#[derive(Resource, Debug, Clone)]
pub struct TerrainStampConfigs {
	pub plateau: DualBandFamilyConfig<PlateauCapParams>,
	pub massif: DualBandFamilyConfig<RuggedMassifParams>,
	pub canyon: DualBandFamilyConfig<CanyonParams>,
	pub pocket_water: DualBandFamilyConfig<PocketWaterParams>,
	pub rolling: DualBandFamilyConfig<RollingGroundParams>,
	pub valley: DualBandFamilyConfig<ValleyTrainParams>,
}

macro_rules! band_from_layout {
	(low_pass, $seed:expr, $Layout:ty) => {
		FamilyGuillotineConfig::low_pass(
			$seed,
			<$Layout>::LIKELIHOOD,
			<$Layout>::SPATIAL_CORRELATION,
			<$Layout>::STRENGTH_MIN,
			<$Layout>::STRENGTH_MAX,
			<$Layout>::CELL_SIZE_MIN,
			<$Layout>::CELL_SIZE_MAX,
		)
	};
	(high_pass, $seed:expr, $Layout:ty) => {
		FamilyGuillotineConfig::high_pass(
			$seed,
			<$Layout>::LIKELIHOOD,
			<$Layout>::SPATIAL_CORRELATION,
			<$Layout>::STRENGTH_MIN,
			<$Layout>::STRENGTH_MAX,
			<$Layout>::CELL_SIZE_MIN,
			<$Layout>::CELL_SIZE_MAX,
		)
	};
}

impl TerrainStampConfigs {
	/// Per-family cut seeds derived from a world seed (default world seed is `42`).
	pub fn from_world_seed(seed: u32) -> Self {
		Self {
			plateau: DualBandFamilyConfig {
				low_pass: band_from_layout!(low_pass, seed, PlateauLowPassControllerLayout),
				high_pass: band_from_layout!(
					high_pass,
					seed.wrapping_add(1000),
					PlateauHighPassControllerLayout
				),
			},
			massif: DualBandFamilyConfig {
				low_pass: band_from_layout!(
					low_pass,
					seed.wrapping_add(1),
					MassifLowPassControllerLayout
				),
				high_pass: band_from_layout!(
					high_pass,
					seed.wrapping_add(1001),
					MassifHighPassControllerLayout
				),
			},
			canyon: DualBandFamilyConfig {
				low_pass: band_from_layout!(
					low_pass,
					seed.wrapping_add(2),
					CanyonLowPassControllerLayout
				),
				high_pass: band_from_layout!(
					high_pass,
					seed.wrapping_add(1002),
					CanyonHighPassControllerLayout
				),
			},
			pocket_water: DualBandFamilyConfig {
				low_pass: band_from_layout!(
					low_pass,
					seed.wrapping_add(3),
					PocketWaterLowPassControllerLayout
				),
				high_pass: band_from_layout!(
					high_pass,
					seed.wrapping_add(1003),
					PocketWaterHighPassControllerLayout
				),
			},
			rolling: DualBandFamilyConfig {
				low_pass: band_from_layout!(
					low_pass,
					seed.wrapping_add(4),
					RollingLowPassControllerLayout
				),
				high_pass: band_from_layout!(
					high_pass,
					seed.wrapping_add(1004),
					RollingHighPassControllerLayout
				),
			},
			valley: DualBandFamilyConfig {
				low_pass: band_from_layout!(
					low_pass,
					seed.wrapping_add(5),
					ValleyLowPassControllerLayout
				),
				high_pass: band_from_layout!(
					high_pass,
					seed.wrapping_add(1005),
					ValleyHighPassControllerLayout
				),
			},
		}
	}
}

impl Default for TerrainStampConfigs {
	fn default() -> Self {
		Self::from_world_seed(42)
	}
}

lod::seeded_root!(TerrainStampConfigs);
