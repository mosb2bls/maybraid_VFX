//! Massif family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::{MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use crate::terrain::stamps::family_macro::define_stamp_family;
use terrain_stamps::RuggedMassif;

define_stamp_family! {
	layout: MassifLowPassControllerLayout,
	controller: MassifLowPassControllerCell,
	stamp: MassifLowPassStampCell,
	family_salt: 33,
	cell_size: (TERRAIN_CELL_SIZE * 1.25, MACRO_CELL_SIZE * 1.5),
	controller_cell_size: MACRO_CELL_SIZE * 6.0,
	origin_offset: (MACRO_CELL_SIZE * 0.5, MACRO_CELL_SIZE * 0.5),
	likelihood: 0.92,
	spatial_correlation: MACRO_CELL_SIZE * 12.0,
	strength: (0.6, 1.1),
	config_family: massif,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RuggedMassif::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}

define_stamp_family! {
	layout: MassifHighPassControllerLayout,
	controller: MassifHighPassControllerCell,
	stamp: MassifHighPassStampCell,
	family_salt: 133,
	cell_size: (MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 8.0),
	controller_cell_size: MACRO_CELL_SIZE * 40.0,
	origin_offset: (MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 3.0),
	likelihood: 0.24,
	spatial_correlation: MACRO_CELL_SIZE * 80.0,
	strength: (0.8, 1.8),
	config_family: massif,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RuggedMassif::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}
