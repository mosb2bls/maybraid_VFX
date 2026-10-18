//! Canyon family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::{MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use crate::terrain::stamps::family_macro::define_stamp_family;
use terrain_stamps::Canyon;

define_stamp_family! {
	layout: CanyonLowPassControllerLayout,
	controller: CanyonLowPassControllerCell,
	stamp: CanyonLowPassStampCell,
	family_salt: 44,
	cell_size: (TERRAIN_CELL_SIZE * 1.25, MACRO_CELL_SIZE * 1.5),
	controller_cell_size: MACRO_CELL_SIZE * 3.0,
	origin_offset: (MACRO_CELL_SIZE * 0.25, 0.0),
	likelihood: 0.78,
	spatial_correlation: MACRO_CELL_SIZE * 12.0,
	strength: (0.6, 1.1),
	config_family: canyon,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		Canyon::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}

define_stamp_family! {
	layout: CanyonHighPassControllerLayout,
	controller: CanyonHighPassControllerCell,
	stamp: CanyonHighPassStampCell,
	family_salt: 144,
	cell_size: (MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 8.0),
	controller_cell_size: MACRO_CELL_SIZE * 20.0,
	origin_offset: (MACRO_CELL_SIZE * 4.0, MACRO_CELL_SIZE * 1.0),
	likelihood: 0.24,
	spatial_correlation: MACRO_CELL_SIZE * 80.0,
	strength: (1.5, 3.0),
	config_family: canyon,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		Canyon::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
