//! Forlorn Savanna — low-density sparse dry upper-canopy grove
//! ([RFC-183 §3.4.7.6], [#351](https://github.com/ramate-io/maybraid/issues/351)).
//!
//! Wind-shaped Rory's Head-trained forms, acacia-impression High Bush, and rare dry Storybook
//! accents across open savanna. Low / UltraLow keep one canopy proxy per plant — the grove
//! is too sparse for UltraLow 8 m bins.

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

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Flat sparse crown projection for acacia-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Forlorn Savanna grove definition.
///
/// Cell footprint sits at the RFC midpoint (`30` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ForlornSavannaCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(30.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-30.0, 30.0),
		),
		distribution: ForlornSavannaCell::distribution(),
	}
}

/// Ordered forlorn-savanna varietals ([RFC-183 §3.4.7.6]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForlornSavannaCell {
	SavannaRory,
	AcaciaHighBush,
	RareSavannaStorybook,
}

/// Typed authored geometry for one forlorn-savanna varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForlornSavannaItem {
	Rory(&'static ForlornSavannaRory),
	HighBush(&'static ForlornSavannaHighBush),
	Storybook(&'static ForlornSavannaStorybook),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaRory {
	pub height: UnitRange,
	/// Stalk base radius as a **fraction of sampled height**. Large savanna
	/// umbrellas stay thick; leftover metres would stay spindly on a 30 m tree.
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one acacia-impression Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one dry Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const SAVANNA_RORY: ForlornSavannaRory = ForlornSavannaRory {
	height: UnitRange::new(5.0, 30.0),
	stalk_radius: UnitRange::new(0.15, 0.20),
	canopy_spread: UnitRange::new(3.0, 12.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ACACIA_HIGH_BUSH: ForlornSavannaHighBush = ForlornSavannaHighBush {
	height: UnitRange::new(5.0, 10.0),
	shoot_count: 4..=12,
	branch_depth: 2..=3,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.35, 0.55),
};

const RARE_SAVANNA_STORYBOOK: ForlornSavannaStorybook = ForlornSavannaStorybook {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.24, 0.52),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const SAVANNA_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("weathered_bark", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const SAVANNA_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("yellow_green", "dusty_green"),
]);

const ACACIA_HIGH_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const ACACIA_HIGH_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "olive_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const SAVANNA_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_brown", "dark_bark"),
	PaletteSlot::new("gray_brown", "tan_bark"),
]);

const SAVANNA_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "yellow_green"),
	PaletteSlot::new("dusty_green", "light_green"),
]);

impl ForlornSavannaCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.2`; the `None` weight of `30.0` puts the placed share at
	/// `5.2 / 35.2 ≈ 0.15`, mid RFC `DENSITY_RANGE` (`0.06..0.20`).
	pub fn distribution() -> GroveDistribution<Self> {
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let high_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		GroveDistribution::new(vec![
			GroveBucket::none(30.0),
			GroveBucket::placed(3.0, rory, Self::SavannaRory),
			GroveBucket::placed(2.0, high_bush, Self::AcaciaHighBush),
			GroveBucket::placed(0.2, storybook, Self::RareSavannaStorybook),
		])
	}

	pub fn item(self) -> ForlornSavannaItem {
		match self {
			Self::SavannaRory => ForlornSavannaItem::Rory(&SAVANNA_RORY),
			Self::AcaciaHighBush => ForlornSavannaItem::HighBush(&ACACIA_HIGH_BUSH),
			Self::RareSavannaStorybook => ForlornSavannaItem::Storybook(&RARE_SAVANNA_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_STICK_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_STICK_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_CANOPY_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_CANOPY_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Typical large types ~25 m. `grove_bands_for_typical_height(25)`.
pub const FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::skip_ultralow_bins(
	FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR,
	FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR,
	FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR,
)
.with_rory_trunks();

#[cfg(feature = "render")]
pub use vc::{ForlornSavanna, ForlornSavannaParams, ForlornSavannaPlant};

#[cfg(test)]
mod tests;
