//! Riverine Green — well-known sparse wet shrub understory grove
//! ([RFC-183 §3.4.5.10](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/10-riverine-green/README.md),
//! [#307](https://github.com/ramate-io/maybraid/issues/307)).
//!
//! Moderate-density Common High Bush punctuation along riparian edges. Each placement is a
//! single High Bush Shoots plant with dual stick and canopy palettes.

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

/// Authored Riverine Green grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`4.0..10.0`). The offset range
/// is signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<RiverineGreenCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(7.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-7.0, 7.0)),
		distribution: RiverineGreenCell::distribution(),
	}
}

/// Ordered riverine-green varietals ([RFC-183 §3.4.5.10]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiverineGreenCell {
	WetGreenBush,
	BrightBankBush,
	DeepShadeBush,
	PaleRiparianBush,
	RedTwigRiverBush,
}

/// Typed authored geometry for one riverine-green bush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiverineGreenItem {
	Bush(&'static RiverineGreenBush),
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct RiverineGreenBush {
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

const WET_GREEN_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(1.00, 2.20),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: UnitRange::new(0.38, 0.52),
	vertical_bias: UnitRange::new(0.18, 0.82),
	leaf_radius: UnitRange::new(0.06, 0.13),
};

const BRIGHT_BANK_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(0.80, 1.70),
	shoot_count: 6..=10,
	branch_depth: 2..=3,
	radial_strength: UnitRange::new(0.42, 0.58),
	vertical_bias: UnitRange::new(0.22, 0.78),
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const DEEP_SHADE_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(1.20, 2.40),
	shoot_count: 8..=12,
	branch_depth: 3..=5,
	radial_strength: UnitRange::new(0.30, 0.45),
	vertical_bias: UnitRange::new(0.72, 0.90),
	leaf_radius: UnitRange::new(0.07, 0.14),
};

const PALE_RIPARIAN_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(0.90, 1.80),
	shoot_count: 6..=10,
	branch_depth: 2..=4,
	radial_strength: UnitRange::new(0.35, 0.50),
	vertical_bias: UnitRange::new(0.18, 0.80),
	leaf_radius: UnitRange::new(0.05, 0.12),
};

const RED_TWIG_RIVER_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(0.90, 1.90),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: UnitRange::new(0.38, 0.55),
	vertical_bias: UnitRange::new(0.18, 0.82),
	leaf_radius: UnitRange::new(0.05, 0.12),
};

const WET_GREEN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);
const BRIGHT_BANK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("young_bark", "green_brown"),
	PaletteSlot::new("wet_brown", "tan_bark"),
]);
const DEEP_SHADE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "wet_brown"),
	PaletteSlot::new("green_brown", "gray_brown"),
]);
const PALE_RIPARIAN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_bark", "gray_brown"),
	PaletteSlot::new("green_brown", "tan_bark"),
]);
const RED_TWIG_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_twig", "copper_red"),
	PaletteSlot::new("wet_burgundy", "dark_bark"),
]);

const WET_GREEN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("deep_green", "light_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);
const BRIGHT_BANK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("bright_green", "light_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
	PaletteSlot::new("lush_green", "lime_green"),
]);
const DEEP_SHADE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("blue_green", "wet_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);
const PALE_RIPARIAN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("pale_green", "fresh_green"),
	PaletteSlot::new("silver_green", "light_green"),
	PaletteSlot::new("yellow_green", "wet_green"),
]);
const RED_TWIG_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("bright_green", "yellow_green"),
	PaletteSlot::new("silver_green", "light_green"),
]);

impl RiverineGreenCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.45` (RFC relative proportions); the `None` weight of `11.0` puts
	/// the placed share at `4.45 / 15.45 ≈ 0.29` — denser than the RFC midpoint while keeping
	/// shorelines readable.
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(11.0),
			GroveBucket::placed(
				2.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.42)),
				Self::WetGreenBush,
			),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.65)),
				Self::BrightBankBush,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.45)),
				Self::DeepShadeBush,
			),
			GroveBucket::placed(
				0.45,
				PlacementConstraints::new(UnitRange::new(0.0, 0.42), UnitRange::new(0.0, 0.60)),
				Self::PaleRiparianBush,
			),
			GroveBucket::placed(
				0.25,
				PlacementConstraints::new(UnitRange::new(0.0, 0.38), UnitRange::new(0.0, 0.55)),
				Self::RedTwigRiverBush,
			),
		])
	}

	/// Authored geometry for this varietal.
	pub fn item(self) -> RiverineGreenItem {
		match self {
			Self::WetGreenBush => RiverineGreenItem::Bush(&WET_GREEN_BUSH),
			Self::BrightBankBush => RiverineGreenItem::Bush(&BRIGHT_BANK_BUSH),
			Self::DeepShadeBush => RiverineGreenItem::Bush(&DEEP_SHADE_BUSH),
			Self::PaleRiparianBush => RiverineGreenItem::Bush(&PALE_RIPARIAN_BUSH),
			Self::RedTwigRiverBush => RiverineGreenItem::Bush(&RED_TWIG_RIVER_BUSH),
		}
	}

	/// Authored stick palette for this varietal.
	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::WetGreenBush => WET_GREEN_STICK_MIX,
			Self::BrightBankBush => BRIGHT_BANK_STICK_MIX,
			Self::DeepShadeBush => DEEP_SHADE_STICK_MIX,
			Self::PaleRiparianBush => PALE_RIPARIAN_STICK_MIX,
			Self::RedTwigRiverBush => RED_TWIG_STICK_MIX,
		}
	}

	/// Authored canopy palette for this varietal.
	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::WetGreenBush => WET_GREEN_CANOPY_MIX,
			Self::BrightBankBush => BRIGHT_BANK_CANOPY_MIX,
			Self::DeepShadeBush => DEEP_SHADE_CANOPY_MIX,
			Self::PaleRiparianBush => PALE_RIPARIAN_CANOPY_MIX,
			Self::RedTwigRiverBush => RED_TWIG_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const RIVERINE_GREEN_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const RIVERINE_GREEN_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const RIVERINE_GREEN_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	RIVERINE_GREEN_STRUCTURAL_HIGH_FACTOR,
	RIVERINE_GREEN_STRUCTURAL_MEDIUM_FACTOR,
	RIVERINE_GREEN_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{RiverineGreen, RiverineGreenParams, RiverineGreenPlant};

#[cfg(test)]
mod tests;
