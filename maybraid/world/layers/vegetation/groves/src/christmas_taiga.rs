//! Christmas Taiga — moderate-density cold Northern Conifer upper-canopy grove
//! ([RFC-183 §3.4.7.18], [#341](https://github.com/ramate-io/maybraid/issues/341)).
//!
//! Dense cold-forest Northern Conifer forms with a colder high-band variant.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Christmas Taiga grove definition.
///
/// Cell footprint sits at the RFC midpoint (`16.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ChristmasTaigaCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(16.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-16.0, 16.0),
		),
		distribution: ChristmasTaigaCell::distribution(),
	}
}

/// Ordered christmas-taiga varietals ([RFC-183 §3.4.7.18]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChristmasTaigaCell {
	ChristmasNorthernConifer,
	HighBandNorthernConifer,
}

/// Typed authored geometry for one christmas-taiga varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChristmasTaigaItem {
	NorthernConifer(&'static ChristmasTaigaNorthernConifer),
}

/// Authored geometry ranges for one Northern Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ChristmasTaigaNorthernConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const CHRISTMAS_NORTHERN_CONIFER: ChristmasTaigaNorthernConifer = ChristmasTaigaNorthernConifer {
	height: UnitRange::new(8.0, 20.0),
	stalk_radius: UnitRange::new(0.22, 0.65),
	canopy_spread: UnitRange::new(2.0, 6.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const HIGH_BAND_NORTHERN_CONIFER: ChristmasTaigaNorthernConifer = ChristmasTaigaNorthernConifer {
	height: UnitRange::new(8.0, 20.0),
	stalk_radius: UnitRange::new(0.22, 0.65),
	canopy_spread: UnitRange::new(2.0, 6.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const CHRISTMAS_NORTHERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const CHRISTMAS_NORTHERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("christmas_green", "deep_green"),
	PaletteSlot::new("blue_green", "dark_green"),
]);

const HIGH_BAND_NORTHERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const HIGH_BAND_NORTHERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

impl ChristmasTaigaCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `1.5`; the `None` weight of `3.3` puts the placed share at
	/// `1.5 / 4.8 ≈ 0.31`, mid RFC `DENSITY_RANGE` (`0.20..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		let christmas_northern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.76));
		let high_band_northern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.82));
		GroveDistribution::new(vec![
			GroveBucket::none(3.3),
			GroveBucket::placed(1.0, christmas_northern, Self::ChristmasNorthernConifer),
			GroveBucket::placed(0.5, high_band_northern, Self::HighBandNorthernConifer),
		])
	}

	pub fn item(self) -> ChristmasTaigaItem {
		match self {
			Self::ChristmasNorthernConifer => {
				ChristmasTaigaItem::NorthernConifer(&CHRISTMAS_NORTHERN_CONIFER)
			}
			Self::HighBandNorthernConifer => {
				ChristmasTaigaItem::NorthernConifer(&HIGH_BAND_NORTHERN_CONIFER)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ChristmasNorthernConifer => CHRISTMAS_NORTHERN_STICK_MIX,
			Self::HighBandNorthernConifer => HIGH_BAND_NORTHERN_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ChristmasNorthernConifer => CHRISTMAS_NORTHERN_CANOPY_MIX,
			Self::HighBandNorthernConifer => HIGH_BAND_NORTHERN_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const CHRISTMAS_TAIGA_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const CHRISTMAS_TAIGA_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const CHRISTMAS_TAIGA_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	CHRISTMAS_TAIGA_STRUCTURAL_HIGH_FACTOR,
	CHRISTMAS_TAIGA_STRUCTURAL_MEDIUM_FACTOR,
	CHRISTMAS_TAIGA_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{ChristmasTaiga, ChristmasTaigaParams, ChristmasTaigaPlant};

#[cfg(test)]
mod tests;
