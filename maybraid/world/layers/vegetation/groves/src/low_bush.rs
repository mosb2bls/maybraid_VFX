//! Low Bush — well-known moderate-density low shrub understory grove
//! ([RFC-183 §3.4.5.3](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/03-low-bush/README.md),
//! [#310](https://github.com/ramate-io/maybraid/issues/310)).
//!
//! Common High Bush forms at 50 cm–1.5 m: structured but permeable woody filler above ground
//! cover. Each placement is a High Bush Shoots plant with dual stick and canopy palettes.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// RFC `projection_count: Low` — upright rounded low shrubs.
const LOW_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.20, 0.38);
const LOW_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.68, 0.88);

/// RFC `projection_count: Moderate` — LeafyLowBush only.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// Authored Low Bush grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`2.5..6.0`). The offset range
/// is signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<LowBushCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(4.25),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-4.25, 4.25),
		),
		distribution: LowBushCell::distribution(),
	}
}

/// Ordered low-bush varietals ([RFC-183 §3.4.5.3]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowBushCell {
	GreenLowBush,
	DryLowBush,
	LeafyLowBush,
	FloweringLowBush,
	RedStemLowBush,
}

/// Typed authored geometry for one low-bush varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LowBushItem {
	Bush(&'static LowBushBush),
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct LowBushBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	/// RFC `projection_count` — horizontal splay in shoot direction mix.
	pub radial_strength: UnitRange,
	/// RFC `projection_count` — upward bias in shoot direction mix.
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

const GREEN_LOW_BUSH: LowBushBush = LowBushBush {
	height: UnitRange::new(0.50, 1.20),
	shoot_count: 5..=8,
	branch_depth: 1..=2,
	radial_strength: LOW_PROJECTION_RADIAL,
	vertical_bias: LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.04, 0.08),
};

const DRY_LOW_BUSH: LowBushBush = LowBushBush {
	height: UnitRange::new(0.50, 1.10),
	shoot_count: 4..=7,
	branch_depth: 1..=2,
	radial_strength: LOW_PROJECTION_RADIAL,
	vertical_bias: LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.03, 0.07),
};

const LEAFY_LOW_BUSH: LowBushBush = LowBushBush {
	height: UnitRange::new(0.80, 1.50),
	shoot_count: 7..=10,
	branch_depth: 2..=3,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.10),
};

const FLOWERING_LOW_BUSH: LowBushBush = LowBushBush {
	height: UnitRange::new(0.60, 1.20),
	shoot_count: 5..=8,
	branch_depth: 1..=2,
	radial_strength: LOW_PROJECTION_RADIAL,
	vertical_bias: LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.04, 0.08),
};

const RED_STEM_LOW_BUSH: LowBushBush = LowBushBush {
	height: UnitRange::new(0.60, 1.30),
	shoot_count: 5..=9,
	branch_depth: 1..=3,
	radial_strength: LOW_PROJECTION_RADIAL,
	vertical_bias: LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.04, 0.09),
};

const GREEN_LOW_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "green_brown"),
	PaletteSlot::new("dark_bark", "gray_brown"),
]);
const DRY_LOW_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);
const LEAFY_LOW_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);
const FLOWERING_LOW_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "tan_brown"),
	PaletteSlot::new("green_brown", "dark_bark"),
]);
const RED_STEM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_bark", "copper_red"),
	PaletteSlot::new("burgundy_brown", "dark_bark"),
]);

const GREEN_LOW_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "light_green"),
	PaletteSlot::new("scrub_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);
const DRY_LOW_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_green", "straw_brown"),
	PaletteSlot::new("olive_green", "tan_green"),
	PaletteSlot::new("pale_green", "dry_yellow_green"),
]);
const LEAFY_LOW_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("blue_green", "light_green"),
]);
const FLOWERING_LOW_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("green", "light_green"),
	PaletteSlot::new("flower_pink", "leaf_green"),
	PaletteSlot::new("flower_white", "fresh_green"),
]);
const RED_STEM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
	PaletteSlot::new("flower_pink", "leaf_green"),
]);

impl LowBushCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.6` (RFC relative proportions); the `None` weight of `10.0` puts
	/// the placed share at `4.6 / 14.6 ≈ 0.32`, inside the RFC's `DENSITY_RANGE`
	/// (`0.18..0.45`).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(10.0),
			GroveBucket::placed(
				2.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.65)),
				Self::GreenLowBush,
			),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.85), UnitRange::new(0.0, 0.65)),
				Self::DryLowBush,
			),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.35), UnitRange::new(0.0, 0.35)),
				Self::LeafyLowBush,
			),
			GroveBucket::placed(
				0.35,
				PlacementConstraints::new(UnitRange::new(0.0, 0.35), UnitRange::new(0.0, 0.65)),
				Self::FloweringLowBush,
			),
			GroveBucket::placed(
				0.25,
				PlacementConstraints::new(UnitRange::new(0.05, 0.45), UnitRange::new(0.0, 0.70)),
				Self::RedStemLowBush,
			),
		])
	}

	pub fn item(self) -> LowBushItem {
		match self {
			Self::GreenLowBush => LowBushItem::Bush(&GREEN_LOW_BUSH),
			Self::DryLowBush => LowBushItem::Bush(&DRY_LOW_BUSH),
			Self::LeafyLowBush => LowBushItem::Bush(&LEAFY_LOW_BUSH),
			Self::FloweringLowBush => LowBushItem::Bush(&FLOWERING_LOW_BUSH),
			Self::RedStemLowBush => LowBushItem::Bush(&RED_STEM_LOW_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenLowBush => GREEN_LOW_STICK_MIX,
			Self::DryLowBush => DRY_LOW_STICK_MIX,
			Self::LeafyLowBush => LEAFY_LOW_STICK_MIX,
			Self::FloweringLowBush => FLOWERING_LOW_STICK_MIX,
			Self::RedStemLowBush => RED_STEM_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenLowBush => GREEN_LOW_CANOPY_MIX,
			Self::DryLowBush => DRY_LOW_CANOPY_MIX,
			Self::LeafyLowBush => LEAFY_LOW_CANOPY_MIX,
			Self::FloweringLowBush => FLOWERING_LOW_CANOPY_MIX,
			Self::RedStemLowBush => RED_STEM_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const LOW_BUSH_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const LOW_BUSH_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const LOW_BUSH_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	LOW_BUSH_STRUCTURAL_HIGH_FACTOR,
	LOW_BUSH_STRUCTURAL_MEDIUM_FACTOR,
	LOW_BUSH_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{LowBush, LowBushParams, LowBushPlant};

#[cfg(test)]
mod tests;
