//! Braid Grass — well-known understory grove ([RFC-183 §3.4.5.1], [#306](https://github.com/ramate-io/maybraid/issues/306)).
//!
//! All authored data (cell footprint, placement ranges, bucket weights, constraints, palettes,
//! and clump geometry) lives in this module as constants mirroring the RFC blocks.

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

/// Authored Braid Grass grove definition.
///
/// The cell footprint is denser than the RFC's nominal grid to keep preview groves visually
/// populated; forest gridding may override it per grove. The offset range is wider than the
/// RFC's nominal ±1 m so biased sampling plus noise still reaches meaningful horizontal
/// variety; [`crate::grove::GroveExtent`] validation keeps the grove LOD unit bounded.
pub fn definition() -> GroveDefinition<BraidGrassCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.125),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-3.0, 3.0)),
		distribution: BraidGrassCell::distribution(),
	}
}

/// Ordered braid-grass variants ([RFC-183 §3.4.2.2]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraidGrassCell {
	DeepGreenBlade,
	PaleReedBlade,
	JungleBlade,
	RedEdgeBlade,
	GreenSpear,
	FountainSpear,
	DeepGreenPatch,
	PaleReedPatch,
	JunglePatch,
}

/// Typed authored geometry for one braid-grass variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BraidGrassItem {
	Blade(&'static BraidGrassClump),
	Spear(&'static BraidSpearClump),
	Patch(&'static GroveTuftPatch<BraidGrassClump>),
}

/// Authored geometry ranges for one braid-grass blade clump.
#[derive(Debug, Clone, PartialEq)]
pub struct BraidGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**; absolute widths render far-too-thick
	/// blades (the RFC widths describe the clump footprint, not blade thickness).
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	/// Max polar blade tilt — the RFC's "braid twist".
	pub max_tilt_radians: UnitRange,
}

/// Authored geometry ranges for one braid-grass spear clump (flat belly→tip ribbons).
#[derive(Debug, Clone, PartialEq)]
pub struct BraidSpearClump {
	pub height: UnitRange,
	/// Belly half-width as a **fraction of spear length** (same proportional contract as
	/// [`BraidGrassClump::width_factor`]); the base tapers from the belly in the render build.
	pub belly_factor: UnitRange,
	pub spear_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–3 % of blade length — braid blades run long (1–3 m),
/// so the proportional band is tighter than the short-tuft groves.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.022, 0.034);

/// Braid Grass takes the widest shape variation of the tuft groves: kink counts span
/// near-straight reeds through heavily braided blades.
const BEND_SEGMENTS: RangeInclusive<u32> = 2..=10;

// Most tilt bands sit in a moderate regime (uprightish PaleReed/GreenSpear through leaning
// DeepGreen/RedEdge); Jungle and FountainSpear instead take *wide* bands, so individual
// clumps of those varietals range anywhere from upright to fully splayed.

const DEEP_GREEN_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.0, 2.2),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=28,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.20, 0.45),
};

const PALE_REED_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.2, 2.6),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 6..=22,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.10, 0.25),
};

const JUNGLE_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.6, 3.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 6..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.15, 0.90),
};

const RED_EDGE_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.0, 2.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=18,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.25, 0.55),
};

const GREEN_SPEAR: BraidSpearClump = BraidSpearClump {
	height: UnitRange::new(1.2, 2.4),
	belly_factor: UnitRange::new(0.008, 0.015),
	spear_count: 10..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.12, 0.30),
};

const FOUNTAIN_SPEAR: BraidSpearClump = BraidSpearClump {
	height: UnitRange::new(1.0, 2.0),
	belly_factor: UnitRange::new(0.010, 0.018),
	spear_count: 14..=30,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.20, 1.10),
};

// Patch varietals scatter each blade clump as loose mounds; they carry most of the blade
// weight, so the single-anchor "cone" clump reads as the rarer silhouette. Spears keep
// their own buckets — their ribbon profile already breaks the cone read.

const DEEP_GREEN_PATCH: GroveTuftPatch<BraidGrassClump> = GroveTuftPatch {
	clump: DEEP_GREEN_BLADE,
	clump_count: 3..=5,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.20, 0.45),
};

const PALE_REED_PATCH: GroveTuftPatch<BraidGrassClump> = GroveTuftPatch {
	clump: PALE_REED_BLADE,
	clump_count: 2..=4,
	patch_extent_xz: UnitRange::new(1.2, 2.2),
	base_spread: UnitRange::new(0.15, 0.35),
};

const JUNGLE_PATCH: GroveTuftPatch<BraidGrassClump> = GroveTuftPatch {
	clump: JUNGLE_BLADE,
	clump_count: 2..=4,
	patch_extent_xz: UnitRange::new(1.4, 2.4),
	base_spread: UnitRange::new(0.25, 0.50),
};

impl BraidGrassCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Blade weight (`5.25` total) leans on the patch varietals (`4.0`); single-anchor blade
	/// clumps share the remaining `1.25`. Spears keep their original weights.
	pub fn distribution() -> GroveDistribution<Self> {
		let low_ground =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.60));
		let jungle_floor =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.30));
		let red_edge_ground =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(2.5),
			GroveBucket::placed(0.5, low_ground, Self::DeepGreenBlade),
			GroveBucket::placed(0.25, low_ground, Self::PaleReedBlade),
			GroveBucket::placed(0.25, jungle_floor, Self::JungleBlade),
			GroveBucket::placed(0.25, red_edge_ground, Self::RedEdgeBlade),
			GroveBucket::placed(1.0, low_ground, Self::GreenSpear),
			GroveBucket::placed(0.75, jungle_floor, Self::FountainSpear),
			GroveBucket::placed(2.0, low_ground, Self::DeepGreenPatch),
			GroveBucket::placed(1.0, low_ground, Self::PaleReedPatch),
			GroveBucket::placed(1.0, jungle_floor, Self::JunglePatch),
		])
	}

	/// Authored geometry for this variant.
	pub fn item(self) -> BraidGrassItem {
		match self {
			Self::DeepGreenBlade => BraidGrassItem::Blade(&DEEP_GREEN_BLADE),
			Self::PaleReedBlade => BraidGrassItem::Blade(&PALE_REED_BLADE),
			Self::JungleBlade => BraidGrassItem::Blade(&JUNGLE_BLADE),
			Self::RedEdgeBlade => BraidGrassItem::Blade(&RED_EDGE_BLADE),
			Self::GreenSpear => BraidGrassItem::Spear(&GREEN_SPEAR),
			Self::FountainSpear => BraidGrassItem::Spear(&FOUNTAIN_SPEAR),
			Self::DeepGreenPatch => BraidGrassItem::Patch(&DEEP_GREEN_PATCH),
			Self::PaleReedPatch => BraidGrassItem::Patch(&PALE_REED_PATCH),
			Self::JunglePatch => BraidGrassItem::Patch(&JUNGLE_PATCH),
		}
	}

	/// Authored palette ranges for this variant.
	pub fn palette_mix(self) -> PaletteMix {
		const DEEP_GREEN_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "wet_green"),
			PaletteSlot::new("dark_green", "emerald_green"),
			PaletteSlot::new("blue_green", "fresh_green"),
		]);
		const PALE_REED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "pale_straw"),
			PaletteSlot::new("dry_green", "light_green"),
			PaletteSlot::new("tan_green", "fresh_green"),
		]);
		const JUNGLE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("lush_green", "bright_green"),
			PaletteSlot::new("wet_green", "lime_green"),
			PaletteSlot::new("blue_green", "deep_green"),
		]);
		const RED_EDGE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("red_green", "deep_green"),
			PaletteSlot::new("copper_red", "yellow_green"),
			PaletteSlot::new("dark_red", "wet_green"),
		]);
		const GREEN_SPEAR_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("emerald_green", "fresh_green"),
			PaletteSlot::new("deep_green", "lime_green"),
			PaletteSlot::new("wet_green", "bright_green"),
		]);
		const FOUNTAIN_SPEAR_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("bright_green", "lime_green"),
			PaletteSlot::new("yellow_green", "fresh_green"),
			PaletteSlot::new("lush_green", "light_green"),
		]);
		match self {
			Self::DeepGreenBlade | Self::DeepGreenPatch => DEEP_GREEN_MIX,
			Self::PaleReedBlade | Self::PaleReedPatch => PALE_REED_MIX,
			Self::JungleBlade | Self::JunglePatch => JUNGLE_MIX,
			Self::RedEdgeBlade => RED_EDGE_MIX,
			Self::GreenSpear => GREEN_SPEAR_MIX,
			Self::FountainSpear => FOUNTAIN_SPEAR_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::vc_tuft::{
	TUFT_GROVE_STRUCTURAL_HIGH_FACTOR, TUFT_GROVE_STRUCTURAL_LOW_FACTOR,
	TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(feature = "render")]
pub const BRAID_GRASS_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
#[cfg(feature = "render")]
pub const BRAID_GRASS_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
#[cfg(feature = "render")]
pub const BRAID_GRASS_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

#[cfg(feature = "render")]
pub use vc::{BraidGrass, BraidGrassParams};

#[cfg(test)]
mod tests;
