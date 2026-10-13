//! Spotty Bushes — well-known very sparse High Bush punctuation grove
//! ([RFC-183 §3.4.5.9](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/09-spotty-bushes/README.md),
//! [#321](https://github.com/ramate-io/maybraid/issues/321)).
//!
//! Isolated Common High Bush forms for open and transitional terrain. Each placement is a
//! High Bush Shoots plant with dual stick and canopy palettes.

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

/// RFC `projection_count: Moderate`.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.78);

/// RFC `projection_count: Low..Moderate` — spans low and moderate bands.
const LOW_MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.20, 0.48);
const LOW_MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.91);

/// Authored Spotty Bushes grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`5.0..12.0`). The offset range
/// is signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<SpottyBushesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(8.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-8.5, 8.5)),
		distribution: SpottyBushesCell::distribution(),
	}
}

/// Ordered spot-bush varietals ([RFC-183 §3.4.5.9]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpottyBushesCell {
	GreenSpotBush,
	DrySpotBush,
	DenseSpotBush,
	FloweringSpotBush,
}

/// Typed authored geometry for one spot-bush varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpottyBushesItem {
	Bush(&'static SpottyBushesBush),
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct SpottyBushesBush {
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

const GREEN_SPOT_BUSH: SpottyBushesBush = SpottyBushesBush {
	height: UnitRange::new(1.00, 2.10),
	shoot_count: 6..=10,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.12),
};

const DRY_SPOT_BUSH: SpottyBushesBush = SpottyBushesBush {
	height: UnitRange::new(0.80, 1.80),
	shoot_count: 5..=9,
	branch_depth: 1..=3,
	radial_strength: LOW_MODERATE_PROJECTION_RADIAL,
	vertical_bias: LOW_MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.04, 0.09),
};

const DENSE_SPOT_BUSH: SpottyBushesBush = SpottyBushesBush {
	height: UnitRange::new(1.40, 2.50),
	shoot_count: 8..=12,
	branch_depth: 3..=5,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.07, 0.14),
};

const FLOWERING_SPOT_BUSH: SpottyBushesBush = SpottyBushesBush {
	height: UnitRange::new(0.90, 1.80),
	shoot_count: 6..=10,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const GREEN_SPOT_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "green_brown"),
	PaletteSlot::new("dark_bark", "gray_brown"),
]);
const DRY_SPOT_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);
const DENSE_SPOT_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);
const FLOWERING_SPOT_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "tan_brown"),
	PaletteSlot::new("green_brown", "dark_bark"),
]);

const GREEN_SPOT_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
	PaletteSlot::new("scrub_green", "yellow_green"),
]);
const DRY_SPOT_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_green", "olive_green"),
	PaletteSlot::new("tan_green", "pale_green"),
	PaletteSlot::new("straw_brown", "green"),
]);
const DENSE_SPOT_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("blue_green", "light_green"),
]);
const FLOWERING_SPOT_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "leaf_green"),
	PaletteSlot::new("flower_white", "fresh_green"),
	PaletteSlot::new("flower_pink", "light_green"),
]);

impl SpottyBushesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.35` (RFC relative proportions); the `None` weight of `20.0` puts
	/// the placed share at `3.35 / 23.35 ≈ 0.14`, inside the RFC's `DENSITY_RANGE`
	/// (`0.04..0.16`).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(10.0),
			GroveBucket::placed(
				1.5,
				PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.48)),
				Self::GreenSpotBush,
			),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::new(UnitRange::new(0.05, 0.70), UnitRange::new(0.0, 0.55)),
				Self::DrySpotBush,
			),
			GroveBucket::placed(
				0.60,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.42)),
				Self::DenseSpotBush,
			),
			GroveBucket::placed(
				0.25,
				PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.38)),
				Self::FloweringSpotBush,
			),
		])
	}

	pub fn item(self) -> SpottyBushesItem {
		match self {
			Self::GreenSpotBush => SpottyBushesItem::Bush(&GREEN_SPOT_BUSH),
			Self::DrySpotBush => SpottyBushesItem::Bush(&DRY_SPOT_BUSH),
			Self::DenseSpotBush => SpottyBushesItem::Bush(&DENSE_SPOT_BUSH),
			Self::FloweringSpotBush => SpottyBushesItem::Bush(&FLOWERING_SPOT_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenSpotBush => GREEN_SPOT_STICK_MIX,
			Self::DrySpotBush => DRY_SPOT_STICK_MIX,
			Self::DenseSpotBush => DENSE_SPOT_STICK_MIX,
			Self::FloweringSpotBush => FLOWERING_SPOT_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenSpotBush => GREEN_SPOT_CANOPY_MIX,
			Self::DrySpotBush => DRY_SPOT_CANOPY_MIX,
			Self::DenseSpotBush => DENSE_SPOT_CANOPY_MIX,
			Self::FloweringSpotBush => FLOWERING_SPOT_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const SPOTTY_BUSHES_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const SPOTTY_BUSHES_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const SPOTTY_BUSHES_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	SPOTTY_BUSHES_STRUCTURAL_HIGH_FACTOR,
	SPOTTY_BUSHES_STRUCTURAL_MEDIUM_FACTOR,
	SPOTTY_BUSHES_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{SpottyBushes, SpottyBushesParams, SpottyBushesPlant};

#[cfg(test)]
mod tests;
