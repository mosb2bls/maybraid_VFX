//! Tropical Tufts — well-known sparse tuft grove with palm companions
//! ([RFC-183 §3.4.4.5], [#305](https://github.com/ramate-io/maybraid/issues/305)).
//!
//! All authored data (cell footprint, placement ranges, bucket weights, constraints, palettes,
//! and item geometry) lives in this module as constants mirroring the RFC blocks.
//! Under `render`, tufts emit flattened lazy kits. Palm companions are quantized
//! (`grow_num` → `Arc<PalmBush>`) and posed as kits on the grove (not nested hosts).

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

/// Authored Tropical Tufts grove definition.
///
/// The offset range is signed and wider than the RFC's nominal `0.0..1.0` (± one cell) so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TropicalTuftsCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(3.25),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-3.25, 3.25),
		),
		distribution: TropicalTuftsCell::distribution(),
	}
}

/// Ordered tropical-tufts variants ([RFC-183 §3.4.2.2]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalTuftsCell {
	BrightTuft,
	DeepTuft,
	YellowGreenTuft,
	SmallPalmBush,
	JuvenilePalmBush,
	BrightTuftPatch,
	DeepTuftPatch,
	YellowGreenTuftPatch,
}

/// Typed authored geometry for one tropical-tufts variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalTuftsItem {
	Tuft(&'static TropicalTuftClump),
	PalmBush(&'static TropicalPalmBush),
	Patch(&'static GroveTuftPatch<TropicalTuftClump>),
}

/// Authored geometry ranges for one tropical tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalTuftClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**; absolute widths render far-too-thick
	/// blades (the RFC widths describe the clump footprint, not blade thickness).
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

// Modest per-clump shape variation; Braid Grass authors the widest bands of the tuft groves.
const BLADE_COUNT: RangeInclusive<u32> = 6..=12;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=6;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.35);

/// Authored geometry ranges for one ground-anchored palm bush companion.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalPalmBush {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

const BRIGHT_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.25, 0.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const DEEP_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.30, 0.8),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const YELLOW_GREEN_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.25, 0.45),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

// Patch varietals scatter each tuft's blades as loose mounds; they carry most of the tuft
// weight, so the single-anchor "cone" clump reads as the rarer silhouette.

const BRIGHT_TUFT_PATCH: GroveTuftPatch<TropicalTuftClump> = GroveTuftPatch {
	clump: BRIGHT_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.30),
};

const DEEP_TUFT_PATCH: GroveTuftPatch<TropicalTuftClump> = GroveTuftPatch {
	clump: DEEP_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.2, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const YELLOW_GREEN_TUFT_PATCH: GroveTuftPatch<TropicalTuftClump> = GroveTuftPatch {
	clump: YELLOW_GREEN_TUFT,
	clump_count: 2..=5,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.30),
};

const SMALL_PALM_BUSH: TropicalPalmBush = TropicalPalmBush {
	height: UnitRange::new(0.35, 0.80),
	frond_count: 4..=7,
	frond_length: UnitRange::new(0.18, 0.45),
	crown_spread: UnitRange::new(0.25, 0.55),
};

const JUVENILE_PALM_BUSH: TropicalPalmBush = TropicalPalmBush {
	height: UnitRange::new(0.50, 1.10),
	frond_count: 3..=5,
	frond_length: UnitRange::new(0.25, 0.60),
	crown_spread: UnitRange::new(0.30, 0.70),
};

impl TropicalTuftsCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Tuft weight (`5.5` total) leans on the patch varietals (`4.4`); single-anchor clumps
	/// share the remaining `1.1`. Palm companions keep their original weights.
	pub fn distribution() -> GroveDistribution<Self> {
		let bright =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.35));
		let lowland =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.75));
		let juvenile =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.75));
		GroveDistribution::new(vec![
			GroveBucket::none(10.0),
			GroveBucket::placed(0.5, bright, Self::BrightTuft),
			GroveBucket::placed(0.35, lowland, Self::DeepTuft),
			GroveBucket::placed(0.25, lowland, Self::YellowGreenTuft),
			GroveBucket::placed(0.75, lowland, Self::SmallPalmBush),
			GroveBucket::placed(0.35, juvenile, Self::JuvenilePalmBush),
			GroveBucket::placed(2.0, bright, Self::BrightTuftPatch),
			GroveBucket::placed(1.5, lowland, Self::DeepTuftPatch),
			GroveBucket::placed(0.9, lowland, Self::YellowGreenTuftPatch),
		])
	}

	/// Authored geometry for this variant.
	pub fn item(self) -> TropicalTuftsItem {
		match self {
			Self::BrightTuft => TropicalTuftsItem::Tuft(&BRIGHT_TUFT),
			Self::DeepTuft => TropicalTuftsItem::Tuft(&DEEP_TUFT),
			Self::YellowGreenTuft => TropicalTuftsItem::Tuft(&YELLOW_GREEN_TUFT),
			Self::SmallPalmBush => TropicalTuftsItem::PalmBush(&SMALL_PALM_BUSH),
			Self::JuvenilePalmBush => TropicalTuftsItem::PalmBush(&JUVENILE_PALM_BUSH),
			Self::BrightTuftPatch => TropicalTuftsItem::Patch(&BRIGHT_TUFT_PATCH),
			Self::DeepTuftPatch => TropicalTuftsItem::Patch(&DEEP_TUFT_PATCH),
			Self::YellowGreenTuftPatch => TropicalTuftsItem::Patch(&YELLOW_GREEN_TUFT_PATCH),
		}
	}

	/// Authored palette ranges for this variant.
	pub fn palette_mix(self) -> PaletteMix {
		const BRIGHT_TUFT_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("bright_green", "lime_green"),
			PaletteSlot::new("lush_green", "fresh_green"),
			PaletteSlot::new("yellow_green", "light_green"),
		]);
		const DEEP_TUFT_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "emerald_green"),
			PaletteSlot::new("dark_green", "wet_green"),
			PaletteSlot::new("blue_green", "bright_green"),
		]);
		const YELLOW_GREEN_TUFT_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "fresh_green"),
			PaletteSlot::new("lime_green", "light_green"),
			PaletteSlot::new("young_green", "bright_green"),
		]);
		const SMALL_PALM_BUSH_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("lush_green", "bright_green"),
			PaletteSlot::new("deep_green", "fresh_green"),
			PaletteSlot::new("wet_green", "lime_green"),
		]);
		const JUVENILE_PALM_BUSH_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("young_green", "lime_green"),
			PaletteSlot::new("fresh_green", "light_green"),
			PaletteSlot::new("bright_green", "yellow_green"),
		]);
		match self {
			Self::BrightTuft | Self::BrightTuftPatch => BRIGHT_TUFT_MIX,
			Self::DeepTuft | Self::DeepTuftPatch => DEEP_TUFT_MIX,
			Self::YellowGreenTuft | Self::YellowGreenTuftPatch => YELLOW_GREEN_TUFT_MIX,
			Self::SmallPalmBush => SMALL_PALM_BUSH_MIX,
			Self::JuvenilePalmBush => JUVENILE_PALM_BUSH_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::vc_tuft::{
	TUFT_GROVE_STRUCTURAL_HIGH_FACTOR, TUFT_GROVE_STRUCTURAL_LOW_FACTOR,
	TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(feature = "render")]
pub const TROPICAL_TUFTS_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
#[cfg(feature = "render")]
pub const TROPICAL_TUFTS_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
#[cfg(feature = "render")]
pub const TROPICAL_TUFTS_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

#[cfg(feature = "render")]
pub use vc::{TropicalTufts, TropicalTuftsParams};

#[cfg(test)]
mod tests;
