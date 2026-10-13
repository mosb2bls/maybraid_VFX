//! Leeward — moderate-density sheltered upper-canopy grove
//! ([RFC-183 §3.4.7.17], [#339](https://github.com/ramate-io/maybraid/issues/339)).
//!
//! Temperate Conifer and Storybook Tree forms on mild lee slopes.

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
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Leeward grove definition.
///
/// Cell footprint sits at the RFC midpoint (`19.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<LeewardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(19.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-19.0, 19.0),
		),
		distribution: LeewardCell::distribution(),
	}
}

/// Ordered leeward varietals ([RFC-183 §3.4.7.17]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeewardCell {
	ShelteredTemperateConifer,
	WindbreakTemperateConifer,
	RoundedLeewardStorybook,
	HighLeewardStorybook,
}

/// Typed authored geometry for one leeward varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeewardItem {
	TemperateConifer(&'static LeewardTemperateConifer),
	Storybook(&'static LeewardStorybook),
}

/// Authored geometry ranges for one Temperate Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct LeewardTemperateConifer {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct LeewardStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const SHELTERED_TEMPERATE_CONIFER: LeewardTemperateConifer = LeewardTemperateConifer {
	height: UnitRange::new(10.0, 18.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WINDBREAK_TEMPERATE_CONIFER: LeewardTemperateConifer = LeewardTemperateConifer {
	height: UnitRange::new(16.0, 24.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ROUNDED_LEEWARD_STORYBOOK: LeewardStorybook = LeewardStorybook {
	height: UnitRange::new(10.0, 18.0),
	stalk_radius: UnitRange::new(0.22, 0.46),
	canopy_spread: UnitRange::new(2.5, 6.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const HIGH_LEEWARD_STORYBOOK: LeewardStorybook = LeewardStorybook {
	height: UnitRange::new(16.0, 24.0),
	stalk_radius: UnitRange::new(0.26, 0.52),
	canopy_spread: UnitRange::new(3.0, 7.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SHELTERED_TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("temperate_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const SHELTERED_TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "deep_green"),
	PaletteSlot::new("blue_green", "fresh_green"),
]);

const WINDBREAK_TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wind_barked", "temperate_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const WINDBREAK_TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "blue_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const LEEWARD_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const LEEWARD_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl LeewardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.65`; the `None` weight of `6.8` puts the placed share at
	/// `2.65 / 9.45 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let sheltered_temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let windbreak_temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.66));
		let rounded_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let high_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		GroveDistribution::new(vec![
			GroveBucket::none(4.0),
			GroveBucket::placed(1.8, sheltered_temperate, Self::ShelteredTemperateConifer),
			GroveBucket::placed(1.6, windbreak_temperate, Self::WindbreakTemperateConifer),
			GroveBucket::placed(2.4, rounded_storybook, Self::RoundedLeewardStorybook),
			GroveBucket::placed(0.45, high_storybook, Self::HighLeewardStorybook),
		])
	}

	pub fn item(self) -> LeewardItem {
		match self {
			Self::ShelteredTemperateConifer => {
				LeewardItem::TemperateConifer(&SHELTERED_TEMPERATE_CONIFER)
			}
			Self::WindbreakTemperateConifer => {
				LeewardItem::TemperateConifer(&WINDBREAK_TEMPERATE_CONIFER)
			}
			Self::RoundedLeewardStorybook => LeewardItem::Storybook(&ROUNDED_LEEWARD_STORYBOOK),
			Self::HighLeewardStorybook => LeewardItem::Storybook(&HIGH_LEEWARD_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_STICK_MIX,
			Self::WindbreakTemperateConifer => WINDBREAK_TEMPERATE_STICK_MIX,
			Self::RoundedLeewardStorybook | Self::HighLeewardStorybook => {
				LEEWARD_STORYBOOK_STICK_MIX
			}
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_CANOPY_MIX,
			Self::WindbreakTemperateConifer => WINDBREAK_TEMPERATE_CANOPY_MIX,
			Self::RoundedLeewardStorybook | Self::HighLeewardStorybook => {
				LEEWARD_STORYBOOK_CANOPY_MIX
			}
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const LEEWARD_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const LEEWARD_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const LEEWARD_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	LEEWARD_STRUCTURAL_HIGH_FACTOR,
	LEEWARD_STRUCTURAL_MEDIUM_FACTOR,
	LEEWARD_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{Leeward, LeewardParams, LeewardPlant};

#[cfg(test)]
mod tests;
