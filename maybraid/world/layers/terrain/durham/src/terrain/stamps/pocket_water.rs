//! Pocket-water family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::{MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use crate::terrain::stamps::family_macro::define_stamp_family;
use terrain_stamps::PocketWater;

define_stamp_family! {
	layout: PocketWaterLowPassControllerLayout,
	controller: PocketWaterLowPassControllerCell,
	stamp: PocketWaterLowPassStampCell,
	family_salt: 55,
	cell_size: (TERRAIN_CELL_SIZE * 1.25, MACRO_CELL_SIZE * 1.5),
	controller_cell_size: MACRO_CELL_SIZE * 3.0,
	origin_offset: (0.0, MACRO_CELL_SIZE * 0.25),
	likelihood: 0.88,
	spatial_correlation: MACRO_CELL_SIZE * 12.0,
	strength: (0.6, 1.1),
	config_family: pocket_water,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		PocketWater::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}

define_stamp_family! {
	layout: PocketWaterHighPassControllerLayout,
	controller: PocketWaterHighPassControllerCell,
	stamp: PocketWaterHighPassStampCell,
	family_salt: 155,
	cell_size: (MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 8.0),
	controller_cell_size: MACRO_CELL_SIZE * 20.0,
	origin_offset: (MACRO_CELL_SIZE * 3.0, MACRO_CELL_SIZE * 5.0),
	likelihood: 0.2,
	spatial_correlation: MACRO_CELL_SIZE * 80.0,
	strength: (1.0, 2.0),
	config_family: pocket_water,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		PocketWater::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
