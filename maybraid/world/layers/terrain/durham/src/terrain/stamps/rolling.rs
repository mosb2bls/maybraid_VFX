//! Rolling-ground family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::{MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use crate::terrain::stamps::family_macro::define_stamp_family;
use terrain_stamps::RollingGround;

define_stamp_family! {
	layout: RollingLowPassControllerLayout,
	controller: RollingLowPassControllerCell,
	stamp: RollingLowPassStampCell,
	family_salt: 66,
	cell_size: (TERRAIN_CELL_SIZE, MACRO_CELL_SIZE * 0.75),
	controller_cell_size: MACRO_CELL_SIZE * 1.5,
	origin_offset: (MACRO_CELL_SIZE * 0.5, 0.0),
	likelihood: 0.92,
	spatial_correlation: MACRO_CELL_SIZE * 12.0,
	strength: (0.4, 0.9),
	config_family: rolling,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RollingGround::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}

define_stamp_family! {
	layout: RollingHighPassControllerLayout,
	controller: RollingHighPassControllerCell,
	stamp: RollingHighPassStampCell,
	family_salt: 166,
	cell_size: (MACRO_CELL_SIZE, MACRO_CELL_SIZE * 4.0),
	controller_cell_size: MACRO_CELL_SIZE * 10.0,
	origin_offset: (MACRO_CELL_SIZE * 2.5, MACRO_CELL_SIZE * 1.5),
	likelihood: 0.35,
	spatial_correlation: MACRO_CELL_SIZE * 80.0,
	strength: (0.8, 1.5),
	config_family: rolling,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RollingGround::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}
