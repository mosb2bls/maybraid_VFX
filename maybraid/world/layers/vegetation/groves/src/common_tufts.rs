//! Common Tufts — well-known sparse-to-moderate grass-clump grove
//! ([RFC-183 §3.4.4.1](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/01-common-tufts/README.md),
//! [#301](https://github.com/ramate-io/maybraid/issues/301)).
//!
//! A lightweight volumetric layer over terrain and ground cover: low 10–50 cm tuft clumps in a
//! few material and shape varietals. All authored data (cell footprint, placement ranges, bucket
//! weights, constraints, palettes, and clump geometry) lives in this module as constants
//! mirroring the RFC blocks.
pub mod variants;

#[cfg(feature = "render")]
mod vc;

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Authored Common Tufts grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`1.0..3.0`). The offset range
/// is signed and wider than the RFC's nominal `0.0..1.0` (± one cell) so placements break the
/// underlying grid instead of clustering near cell centers; the usual slight deterministic
/// scale variation applies.
pub fn definition() -> GroveDefinition<CommonTuftsCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-2.0, 2.0)),
		distribution: CommonTuftsCell::distribution(),
	}
}

/// Ordered common-tufts varietals ([RFC-183 §3.4.4.1]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonTuftsCell {
	ShortGreen,
	DryScrub,
	TallWild,
	ShortGreenPatch,
	DryScrubPatch,
	TallWildPatch,
}

/// Typed authored geometry for one common-tufts varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommonTuftsItem {
	Clump(&'static CommonTuftClump),
	Patch(&'static GroveTuftPatch<CommonTuftClump>),
}

/// Authored geometry ranges for one common-tufts grass clump.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonTuftClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness — read literally they render far-too-thick blades.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

// Modest per-clump shape variation; Braid Grass authors the widest bands of the tuft groves.
const BLADE_COUNT: RangeInclusive<u32> = 6..=10;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=5;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.30);

const SHORT_GREEN: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.10, 0.40),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const DRY_SCRUB: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.15, 0.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const TALL_WILD: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.30, 1.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

// Patch varietals scatter each clump's blades as loose mounds; they carry most of the
// placed weight, so the single-anchor "cone" clump reads as the rarer silhouette.

const SHORT_GREEN_PATCH: GroveTuftPatch<CommonTuftClump> = GroveTuftPatch {
	clump: SHORT_GREEN,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.8, 1.6),
	base_spread: UnitRange::new(0.10, 0.25),
};

const DRY_SCRUB_PATCH: GroveTuftPatch<CommonTuftClump> = GroveTuftPatch {
	clump: DRY_SCRUB,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.9, 1.8),
	base_spread: UnitRange::new(0.10, 0.25),
};

const TALL_WILD_PATCH: GroveTuftPatch<CommonTuftClump> = GroveTuftPatch {
	clump: TALL_WILD,
	clump_count: 2..=5,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.35),
};

impl CommonTuftsCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.0`; the `None` weight of `13.78` puts the placed share at
	/// `5.0 / 18.78 ≈ 0.266`, inside the RFC's `DENSITY_RANGE` (`0.10..0.35`). Patches carry
	/// `4.0` of the placed weight; single-anchor clumps share the remaining `1.0`.
	pub fn distribution() -> GroveDistribution<Self> {
		let short_green =
			PlacementConstraints::new(UnitRange::new(0.0, 0.80), UnitRange::new(0.0, 0.70));
		let dry_scrub =
			PlacementConstraints::new(UnitRange::new(0.0, 0.90), UnitRange::new(0.0, 0.70));
		let tall_wild =
			PlacementConstraints::new(UnitRange::new(0.0, 0.60), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(13.78),
			GroveBucket::placed(0.5, short_green, Self::ShortGreen),
			GroveBucket::placed(0.25, dry_scrub, Self::DryScrub),
			GroveBucket::placed(0.25, tall_wild, Self::TallWild),
			GroveBucket::placed(2.0, short_green, Self::ShortGreenPatch),
			GroveBucket::placed(1.0, dry_scrub, Self::DryScrubPatch),
			GroveBucket::placed(1.0, tall_wild, Self::TallWildPatch),
		])
	}

	/// Authored geometry for this varietal.
	pub fn item(self) -> CommonTuftsItem {
		match self {
			Self::ShortGreen => CommonTuftsItem::Clump(&SHORT_GREEN),
			Self::DryScrub => CommonTuftsItem::Clump(&DRY_SCRUB),
			Self::TallWild => CommonTuftsItem::Clump(&TALL_WILD),
			Self::ShortGreenPatch => CommonTuftsItem::Patch(&SHORT_GREEN_PATCH),
			Self::DryScrubPatch => CommonTuftsItem::Patch(&DRY_SCRUB_PATCH),
			Self::TallWildPatch => CommonTuftsItem::Patch(&TALL_WILD_PATCH),
		}
	}

	/// Authored palette ranges for this varietal (one RFC slot each).
	pub fn palette_mix(self) -> PaletteMix {
		const SHORT_GREEN_MIX: PaletteMix =
			PaletteMix::new(&[PaletteSlot::new("dark_green", "light_green")]);
		const DRY_SCRUB_MIX: PaletteMix =
			PaletteMix::new(&[PaletteSlot::new("vibrant_yellow_green", "dry_yellow_green")]);
		const TALL_WILD_MIX: PaletteMix =
			PaletteMix::new(&[PaletteSlot::new("green", "pale_green")]);
		match self {
			Self::ShortGreen | Self::ShortGreenPatch => SHORT_GREEN_MIX,
			Self::DryScrub | Self::DryScrubPatch => DRY_SCRUB_MIX,
			Self::TallWild | Self::TallWildPatch => TALL_WILD_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::vc_tuft::{
	TUFT_GROVE_STRUCTURAL_HIGH_FACTOR, TUFT_GROVE_STRUCTURAL_LOW_FACTOR,
	TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(feature = "render")]
pub const COMMON_TUFTS_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
#[cfg(feature = "render")]
pub const COMMON_TUFTS_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
#[cfg(feature = "render")]
pub const COMMON_TUFTS_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

#[cfg(feature = "render")]
pub use vc::{CommonTufts, CommonTuftsParams};

#[cfg(test)]
mod tests;
