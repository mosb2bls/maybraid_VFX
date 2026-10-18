//! Watershed **low-pass** band: small pocket water (leaf sides ≈200–600m).

use crate::terrain::watersheds::band_macro::define_marazion_band;

define_marazion_band! {
	layout: PrePocketLowPassLayout,
	layout_from_configs: pre_pocket_low_pass_layout,
	pre_cell: PrePocketLowPassCell,
	pocket: PocketLowPassCell,
	pocket_waters: PocketWatersLowPass,
	band_field: low_pass,
	band_pass: Low,
	family_salt: 0x1270_0001,
	cell_size: (135.0, 737.0),
	pre_pocket_pitch: 10_000.0,
	// Must divide `pre_pocket_pitch` (debug_assert in PrePocket::containing).
	pocket_pitches: [2_000.0, 1_250.0, 1_250.0, 2_500.0],
	origin_offset: (187.0, 93.0),
	likelihood: 0.24,
	spatial_correlation: 1.0,
}

#[cfg(test)]
mod tests {
	use super::PrePocketLowPassLayout;

	#[test]
	fn pocket_pitches_divide_pre_pitch() {
		let pre = PrePocketLowPassLayout::PRE_POCKET_PITCH;
		for pitch in PrePocketLowPassLayout::POCKET_PITCHES {
			let n = pre / pitch;
			assert!((n - n.round()).abs() < 1e-3, "{pitch} does not divide {pre}");
		}
	}
}
