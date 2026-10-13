//! Tall Grass — well-known dense mid-height tuft grove
//! ([RFC-183 §3.4.4.2](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/02-tall-grass/README.md),
//! [#302](https://github.com/ramate-io/maybraid/issues/302)).
//!
//! Dense blade-tuft clumps at 50–100 cm for wet meadows, river edges, and tropical grasslands.
//! Patch varietals scatter each clump's blades as loose mounds and carry most of the placed
//! weight so single-anchor clumps read as the rarer silhouette.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Authored Tall Grass grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`1.0..2.5`). The offset range
/// is signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<TallGrassCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(1.75),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-1.75, 1.75),
		),
		distribution: TallGrassCell::distribution(),
	}
}

/// Ordered tall-grass varietals ([RFC-183 §3.4.4.2]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallGrassCell {
	RiverGreen,
	PaleReed,
	TropicalBlade,
	HawaiianRed,
	RiverGreenPatch,
	PaleReedPatch,
	TropicalBladePatch,
	HawaiianRedPatch,
}

/// Typed authored geometry for one tall-grass varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TallGrassItem {
	Clump(&'static TallGrassClump),
	Patch(&'static GroveTuftPatch<TallGrassClump>),
}

/// Authored geometry ranges for one tall-grass blade clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TallGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.022, 0.04);

const BLADE_COUNT: RangeInclusive<u32> = 8..=12;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=6;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.35);

const RIVER_GREEN: TallGrassClump = TallGrassClump {
	height: UnitRange::new(0.40, 1.20),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const PALE_REED: TallGrassClump = TallGrassClump {
	height: UnitRange::new(0.60, 1.10),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const TROPICAL_BLADE: TallGrassClump = TallGrassClump {
	height: UnitRange::new(0.70, 1.40),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const HAWAIIAN_RED: TallGrassClump = TallGrassClump {
	height: UnitRange::new(0.70, 1.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

// Patch varietals scatter each clump's blades as loose mounds; they carry most of the placed
// weight so the single-anchor "cone" clump reads as the rarer silhouette.

const RIVER_GREEN_PATCH: GroveTuftPatch<TallGrassClump> = GroveTuftPatch {
	clump: RIVER_GREEN,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.30),
};

const PALE_REED_PATCH: GroveTuftPatch<TallGrassClump> = GroveTuftPatch {
	clump: PALE_REED,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.12, 0.28),
};

const TROPICAL_BLADE_PATCH: GroveTuftPatch<TallGrassClump> = GroveTuftPatch {
	clump: TROPICAL_BLADE,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.2),
	base_spread: UnitRange::new(0.18, 0.35),
};

const HAWAIIAN_RED_PATCH: GroveTuftPatch<TallGrassClump> = GroveTuftPatch {
	clump: HAWAIIAN_RED,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.2),
	base_spread: UnitRange::new(0.18, 0.35),
};

const RIVER_GREEN_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("deep_green", "light_green")]);
const PALE_REED_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("yellow_green", "pale_straw")]);
const TROPICAL_BLADE_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("blue_green", "bright_green")]);
const HAWAIIAN_RED_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_brown", "deep_rust"),
	PaletteSlot::new("light_brown", "dark_brown"),
	PaletteSlot::new("yellow_green", "dark_green"),
]);

impl TallGrassCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.0` (RFC relative proportions); the `None` weight of `2.14` puts
	/// the placed share at `5.0 / 7.14 ≈ 0.70`, inside the RFC's `DENSITY_RANGE`
	/// (`0.55..0.85`). Patches carry `4.0` of the placed weight; single-anchor clumps share
	/// the remaining `1.0`.
	pub fn distribution() -> GroveDistribution<Self> {
		let river_green =
			PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.30));
		let pale_reed =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.30));
		let tropical_blade =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.30));
		let hawaiian_red =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(2.14),
			GroveBucket::placed(0.4, river_green, Self::RiverGreen),
			GroveBucket::placed(0.2, pale_reed, Self::PaleReed),
			GroveBucket::placed(0.2, tropical_blade, Self::TropicalBlade),
			GroveBucket::placed(0.2, hawaiian_red, Self::HawaiianRed),
			GroveBucket::placed(1.6, river_green, Self::RiverGreenPatch),
			GroveBucket::placed(0.8, pale_reed, Self::PaleReedPatch),
			GroveBucket::placed(0.8, tropical_blade, Self::TropicalBladePatch),
			GroveBucket::placed(0.8, hawaiian_red, Self::HawaiianRedPatch),
		])
	}

	pub fn item(self) -> TallGrassItem {
		match self {
			Self::RiverGreen => TallGrassItem::Clump(&RIVER_GREEN),
			Self::PaleReed => TallGrassItem::Clump(&PALE_REED),
			Self::TropicalBlade => TallGrassItem::Clump(&TROPICAL_BLADE),
			Self::HawaiianRed => TallGrassItem::Clump(&HAWAIIAN_RED),
			Self::RiverGreenPatch => TallGrassItem::Patch(&RIVER_GREEN_PATCH),
			Self::PaleReedPatch => TallGrassItem::Patch(&PALE_REED_PATCH),
			Self::TropicalBladePatch => TallGrassItem::Patch(&TROPICAL_BLADE_PATCH),
			Self::HawaiianRedPatch => TallGrassItem::Patch(&HAWAIIAN_RED_PATCH),
		}
	}

	pub fn palette_mix(self) -> PaletteMix {
		match self {
			Self::RiverGreen | Self::RiverGreenPatch => RIVER_GREEN_MIX,
			Self::PaleReed | Self::PaleReedPatch => PALE_REED_MIX,
			Self::TropicalBlade | Self::TropicalBladePatch => TROPICAL_BLADE_MIX,
			Self::HawaiianRed | Self::HawaiianRedPatch => HAWAIIAN_RED_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::vc_tuft::{
	TUFT_GROVE_STRUCTURAL_HIGH_FACTOR, TUFT_GROVE_STRUCTURAL_LOW_FACTOR,
	TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(feature = "render")]
pub const TALL_GRASS_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
#[cfg(feature = "render")]
pub const TALL_GRASS_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
#[cfg(feature = "render")]
pub const TALL_GRASS_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

#[cfg(feature = "render")]
pub use vc::{TallGrass, TallGrassParams};

#[cfg(test)]
mod tests;
