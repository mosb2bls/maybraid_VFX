//! Vineyard — high-density cultivated Rory-trained vine upper-canopy grove
//! ([RFC-183 §3.4.7.8], [#355](https://github.com/ramate-io/maybraid/issues/355)).
//!
//! Low trained-vine rows with very tight cell offset and grape-like palettes.

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

/// Authored Vineyard grove definition.
///
/// Cell footprint sits at the RFC midpoint (`4.5` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter for regular vine rows.
pub fn definition() -> GroveDefinition<VineyardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(4.5),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: VineyardCell::distribution(),
	}
}

/// Ordered vineyard varietals ([RFC-183 §3.4.7.8]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VineyardCell {
	TrainedVineRory,
}

/// Typed authored geometry for one vineyard varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VineyardItem {
	Rory(&'static VineyardRory),
}

/// Authored geometry ranges for one trained-vine Rory form.
#[derive(Debug, Clone, PartialEq)]
pub struct VineyardRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const TRAINED_VINE_RORY: VineyardRory = VineyardRory {
	height: UnitRange::new(1.5, 3.0),
	stalk_radius: UnitRange::new(0.045, 0.090),
	canopy_spread: UnitRange::new(1.0, 2.4),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const VINE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("vine_bark", "red_brown"),
	PaletteSlot::new("weathered_bark", "gray_brown"),
]);

const VINE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("grape_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

/// Explicit `None` weight so ~`95%` of cells receive a vine (`0.05` empty vs `0.95` placed).
const CULTIVATED_EMPTY_WEIGHT: f32 = 0.05;
const CULTIVATED_PLACED_WEIGHT: f32 = 0.95;

impl VineyardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// `None` weight `0.05` against placed weight `0.95` yields a `0.95` placed share for
	/// regular row planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let trained_vine =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.34));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(CULTIVATED_PLACED_WEIGHT, trained_vine, Self::TrainedVineRory),
		])
	}

	pub fn item(self) -> VineyardItem {
		match self {
			Self::TrainedVineRory => VineyardItem::Rory(&TRAINED_VINE_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		VINE_STICK_MIX
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		VINE_CANOPY_MIX
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const VINEYARD_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const VINEYARD_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const VINEYARD_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
	VINEYARD_STRUCTURAL_HIGH_FACTOR,
	VINEYARD_STRUCTURAL_MEDIUM_FACTOR,
	VINEYARD_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{Vineyard, VineyardParams, VineyardPlant};

#[cfg(test)]
mod tests;
