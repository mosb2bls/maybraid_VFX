//! High Bush — well-known moderate-density tall shrub understory grove
//! ([RFC-183 §3.4.5.4](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/04-high-bush/README.md),
//! [#312](https://github.com/ramate-io/maybraid/issues/312)).
//!
//! Common High Bush forms at 1.0–2.5 m: substantial shrub masses that shape sightlines and
//! local movement. Each placement is a High Bush Shoots plant with dual stick and canopy palettes.

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

/// RFC `projection_count: Moderate` — all High Bush varietals.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// Authored High Bush grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`3.5..8.0`). The offset range
/// is signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<HighBushCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(5.75),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-5.75, 5.75),
		),
		distribution: HighBushCell::distribution(),
	}
}

/// Ordered high-bush varietals ([RFC-183 §3.4.5.4]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighBushCell {
	GreenHighBush,
	DenseHighBush,
	DryHighBush,
	BerryHighBush,
	CopperCaneHighBush,
}

/// Typed authored geometry for one high-bush varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighBushItem {
	Bush(&'static HighBushBush),
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct HighBushBush {
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

const GREEN_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.00, 3.20),
	shoot_count: 7..=10,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.06, 0.12),
};

const DENSE_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.40, 3.50),
	shoot_count: 8..=12,
	branch_depth: 3..=5,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.07, 0.14),
};

const DRY_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.00, 3.00),
	shoot_count: 6..=9,
	branch_depth: 2..=3,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.10),
};

const BERRY_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.20, 2.80),
	shoot_count: 7..=10,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.06, 0.12),
};

const COPPER_CANE_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.20, 2.50),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.06, 0.12),
};

const GREEN_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "green_brown"),
	PaletteSlot::new("dark_bark", "gray_brown"),
]);
const DENSE_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "wet_brown"),
	PaletteSlot::new("green_brown", "shrub_bark"),
]);
const DRY_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);
const BERRY_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);
const COPPER_CANE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "orange_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const GREEN_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);
const DENSE_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);
const DRY_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("tan_green", "pale_green"),
	PaletteSlot::new("straw_brown", "green"),
]);
const BERRY_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "leaf_green"),
	PaletteSlot::new("berry_red", "deep_green"),
	PaletteSlot::new("berry_blue", "fresh_green"),
]);
const COPPER_CANE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
	PaletteSlot::new("berry_red", "leaf_green"),
]);

impl HighBushCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.65` (RFC relative proportions); the `None` weight of `11.0` puts
	/// the placed share at `4.65 / 15.65 ≈ 0.30`, inside the RFC's `DENSITY_RANGE`
	/// (`0.16..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(11.0),
			GroveBucket::placed(
				2.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::GreenHighBush,
			),
			GroveBucket::placed(
				1.25,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::DenseHighBush,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::DryHighBush,
			),
			GroveBucket::placed(
				0.35,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::BerryHighBush,
			),
			GroveBucket::placed(
				0.30,
				PlacementConstraints::new(UnitRange::new(0.05, 0.45), UnitRange::new(0.0, 0.58)),
				Self::CopperCaneHighBush,
			),
		])
	}

	pub fn item(self) -> HighBushItem {
		match self {
			Self::GreenHighBush => HighBushItem::Bush(&GREEN_HIGH_BUSH),
			Self::DenseHighBush => HighBushItem::Bush(&DENSE_HIGH_BUSH),
			Self::DryHighBush => HighBushItem::Bush(&DRY_HIGH_BUSH),
			Self::BerryHighBush => HighBushItem::Bush(&BERRY_HIGH_BUSH),
			Self::CopperCaneHighBush => HighBushItem::Bush(&COPPER_CANE_HIGH_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenHighBush => GREEN_HIGH_STICK_MIX,
			Self::DenseHighBush => DENSE_HIGH_STICK_MIX,
			Self::DryHighBush => DRY_HIGH_STICK_MIX,
			Self::BerryHighBush => BERRY_HIGH_STICK_MIX,
			Self::CopperCaneHighBush => COPPER_CANE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenHighBush => GREEN_HIGH_CANOPY_MIX,
			Self::DenseHighBush => DENSE_HIGH_CANOPY_MIX,
			Self::DryHighBush => DRY_HIGH_CANOPY_MIX,
			Self::BerryHighBush => BERRY_HIGH_CANOPY_MIX,
			Self::CopperCaneHighBush => COPPER_CANE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const HIGH_BUSH_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const HIGH_BUSH_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	HIGH_BUSH_STRUCTURAL_HIGH_FACTOR,
	HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR,
	HIGH_BUSH_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{HighBush, HighBushParams, HighBushPlant};

#[cfg(test)]
mod tests;
