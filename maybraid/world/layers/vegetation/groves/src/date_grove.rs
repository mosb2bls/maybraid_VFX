//! Date Grove — high-density cultivated Date Palm upper-canopy grove
//! ([RFC-183 §3.4.7.9], [#357](https://github.com/ramate-io/maybraid/issues/357)).
//!
//! Single moderate-crown date palm form with tight cell offset on warm flat terrain.
//!

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Moderate sampled crown-density band ([`0.35`, `0.65`]).
const MODERATE_CROWN_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Date Grove definition.
///
/// Cell footprint sits at the RFC midpoint (`12.0` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter for regular palm rows.
pub fn definition() -> GroveDefinition<DateGroveCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(12.0),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: DateGroveCell::distribution(),
	}
}

/// Ordered date-grove varietals ([RFC-183 §3.4.7.9]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateGroveCell {
	FruitingDatePalm,
}

/// Typed authored geometry for one date-grove varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DateGroveItem {
	DatePalm(&'static DateGroveDatePalm),
}

/// Authored geometry ranges for one Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct DateGroveDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const FRUITING_DATE_PALM: DateGroveDatePalm =
	DateGroveDatePalm { height: UnitRange::new(5.0, 8.0), crown_density: MODERATE_CROWN_DENSITY };

const DATE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("date_trunk", "dry_brown"),
]);

const DATE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_green", "olive_green"),
	PaletteSlot::new("fresh_green", "yellow_green"),
]);

/// Explicit `None` weight so ~`95%` of cells receive a palm (`0.05` empty vs `0.95` placed).
const CULTIVATED_EMPTY_WEIGHT: f32 = 0.05;
const CULTIVATED_PLACED_WEIGHT: f32 = 0.95;

impl DateGroveCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// `None` weight `0.05` against placed weight `0.95` yields a `0.95` placed share for
	/// regular grove planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let fruiting_date =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.30));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(CULTIVATED_PLACED_WEIGHT, fruiting_date, Self::FruitingDatePalm),
		])
	}

	pub fn item(self) -> DateGroveItem {
		match self {
			Self::FruitingDatePalm => DateGroveItem::DatePalm(&FRUITING_DATE_PALM),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		DATE_PALM_STICK_MIX
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		DATE_PALM_CANOPY_MIX
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const DATE_GROVE_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const DATE_GROVE_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const DATE_GROVE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::keep_low_plants(
	DATE_GROVE_STRUCTURAL_HIGH_FACTOR,
	DATE_GROVE_STRUCTURAL_MEDIUM_FACTOR,
	DATE_GROVE_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{DateGrove, DateGroveParams, DateGrovePlant};

#[cfg(test)]
mod tests;
