//! Riparian General — moderate-density mixed river-corridor upper-canopy grove
//! ([RFC-183 §3.4.7.4], [#347](https://github.com/ramate-io/maybraid/issues/347)).
//!
//! Common Braid Oak and Storybook Tree forms with rare willow-like High Bush accents.

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

const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Flat sparse crown projection for willow-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Riparian General grove definition.
///
/// Cell footprint sits at the RFC midpoint (`16` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<RiparianGeneralCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(16.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-16.0, 16.0),
		),
		distribution: RiparianGeneralCell::distribution(),
	}
}

/// Ordered riparian-general varietals ([RFC-183 §3.4.7.4]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiparianGeneralCell {
	RiparianBraidOak,
	RiparianStorybook,
	RareRiparianHighBush,
}

/// Typed authored geometry for one riparian-general varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiparianGeneralItem {
	BraidOak(&'static RiparianGeneralBraidOak),
	Storybook(&'static RiparianGeneralStorybook),
	HighBush(&'static RiparianGeneralHighBush),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianGeneralBraidOak {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianGeneralStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one willow-like Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianGeneralHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

const RIPARIAN_BRAID_OAK: RiparianGeneralBraidOak = RiparianGeneralBraidOak {
	height: UnitRange::new(5.0, 15.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RIPARIAN_STORYBOOK: RiparianGeneralStorybook = RiparianGeneralStorybook {
	height: UnitRange::new(5.0, 15.0),
	stalk_radius: UnitRange::new(0.20, 0.42),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_RIPARIAN_HIGH_BUSH: RiparianGeneralHighBush = RiparianGeneralHighBush {
	height: UnitRange::new(5.0, 15.0),
	shoot_count: 5..=14,
	branch_depth: 2..=4,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.12, 0.28),
};

const RIPARIAN_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "gray_brown"),
]);

const RIPARIAN_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const RIPARIAN_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const RIPARIAN_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "light_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const RIPARIAN_HIGH_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("willow_bark", "wet_brown"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const RIPARIAN_HIGH_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "yellow_green"),
	PaletteSlot::new("river_green", "light_green"),
]);

impl RiparianGeneralCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.35`; the `None` weight of `7.4` puts the placed share at
	/// `3.35 / 10.75 ≈ 0.31`, mid RFC `DENSITY_RANGE` (`0.20..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.36));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.44));
		let high_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		GroveDistribution::new(vec![
			GroveBucket::none(7.4),
			GroveBucket::placed(1.5, braid_oak, Self::RiparianBraidOak),
			GroveBucket::placed(1.5, storybook, Self::RiparianStorybook),
			GroveBucket::placed(0.35, high_bush, Self::RareRiparianHighBush),
		])
	}

	pub fn item(self) -> RiparianGeneralItem {
		match self {
			Self::RiparianBraidOak => RiparianGeneralItem::BraidOak(&RIPARIAN_BRAID_OAK),
			Self::RiparianStorybook => RiparianGeneralItem::Storybook(&RIPARIAN_STORYBOOK),
			Self::RareRiparianHighBush => RiparianGeneralItem::HighBush(&RARE_RIPARIAN_HIGH_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::RiparianBraidOak => RIPARIAN_BRAID_OAK_STICK_MIX,
			Self::RiparianStorybook => RIPARIAN_STORYBOOK_STICK_MIX,
			Self::RareRiparianHighBush => RIPARIAN_HIGH_BUSH_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::RiparianBraidOak => RIPARIAN_BRAID_OAK_CANOPY_MIX,
			Self::RiparianStorybook => RIPARIAN_STORYBOOK_CANOPY_MIX,
			Self::RareRiparianHighBush => RIPARIAN_HIGH_BUSH_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const RIPARIAN_GENERAL_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const RIPARIAN_GENERAL_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const RIPARIAN_GENERAL_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	RIPARIAN_GENERAL_STRUCTURAL_HIGH_FACTOR,
	RIPARIAN_GENERAL_STRUCTURAL_MEDIUM_FACTOR,
	RIPARIAN_GENERAL_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{RiparianGeneral, RiparianGeneralParams, RiparianGeneralPlant};

#[cfg(test)]
mod tests;
