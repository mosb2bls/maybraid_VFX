//! Valley-train family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::{MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use crate::terrain::stamps::family_macro::define_stamp_family;
use terrain_stamps::ValleyTrain;

define_stamp_family! {
	layout: ValleyLowPassControllerLayout,
	controller: ValleyLowPassControllerCell,
	stamp: ValleyLowPassStampCell,
	family_salt: 77,
	cell_size: (TERRAIN_CELL_SIZE * 1.25, MACRO_CELL_SIZE * 1.5),
	controller_cell_size: MACRO_CELL_SIZE * 6.0,
	origin_offset: (MACRO_CELL_SIZE * 0.5, MACRO_CELL_SIZE * 0.5),
	likelihood: 0.85,
	spatial_correlation: MACRO_CELL_SIZE * 12.0,
	strength: (0.6, 1.1),
	config_family: valley,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		ValleyTrain::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}

define_stamp_family! {
	layout: ValleyHighPassControllerLayout,
	controller: ValleyHighPassControllerCell,
	stamp: ValleyHighPassStampCell,
	family_salt: 177,
	cell_size: (MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 8.0),
	controller_cell_size: MACRO_CELL_SIZE * 40.0,
	origin_offset: (MACRO_CELL_SIZE * 6.0, MACRO_CELL_SIZE * 4.0),
	likelihood: 0.28,
	spatial_correlation: MACRO_CELL_SIZE * 80.0,
	strength: (1.5, 3.0),
	config_family: valley,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		ValleyTrain::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
