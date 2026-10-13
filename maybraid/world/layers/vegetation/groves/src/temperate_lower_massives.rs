//! Temperate Lower Massives — massive lower-canopy grove beneath very tall upper canopy
//! ([RFC-183 §3.4.6.9], [#330](https://github.com/ramate-io/maybraid/issues/330)).
//!
//! Common 10–20 m braid oak and storybook forms with rare Rory's Head-trained accents.
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

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);

/// Authored Temperate Lower Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`26` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TemperateLowerMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(18.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-26.0, 26.0),
		),
		distribution: TemperateLowerMassivesCell::distribution(),
	}
}

/// Ordered temperate lower-massive varietals ([RFC-183 §3.4.6.9]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperateLowerMassivesCell {
	LowerMassiveBraidOak,
	LowerMassiveStorybook,
	RareLowerMassiveRory,
}

/// Typed authored geometry for one temperate lower-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemperateLowerMassivesItem {
	BraidOak(&'static TemperateLowerMassivesBraidOak),
	Storybook(&'static TemperateLowerMassivesStorybook),
	Rory(&'static TemperateLowerMassivesRory),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateLowerMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateLowerMassivesStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one rare Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateLowerMassivesRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const LOWER_MASSIVE_BRAID_OAK: TemperateLowerMassivesBraidOak = TemperateLowerMassivesBraidOak {
	height: UnitRange::new(8.0, 24.0),
	canopy_spread: UnitRange::new(3.0, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_STORYBOOK: TemperateLowerMassivesStorybook = TemperateLowerMassivesStorybook {
	height: UnitRange::new(8.0, 20.0),
	stalk_radius: UnitRange::new(0.36, 0.72),
	canopy_spread: UnitRange::new(3.5, 8.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_LOWER_MASSIVE_RORY: TemperateLowerMassivesRory = TemperateLowerMassivesRory {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.12, 0.30),
	canopy_spread: UnitRange::new(2.5, 6.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("weathered_bark", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

impl TemperateLowerMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.35` (RFC relative proportions); the `None` weight of `19.0` puts
	/// the placed share at `4.35 / 23.35 ≈ 0.19`, mid RFC `DENSITY_RANGE` (`0.10..0.26`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.68), UnitRange::new(0.0, 0.50));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.00, 0.72), UnitRange::new(0.0, 0.56));
		let rory = PlacementConstraints::new(UnitRange::new(0.00, 0.64), UnitRange::new(0.0, 0.68));
		GroveDistribution::new(vec![
			GroveBucket::none(8.0),
			GroveBucket::placed(2.0, braid_oak, Self::LowerMassiveBraidOak),
			GroveBucket::placed(2.0, storybook, Self::LowerMassiveStorybook),
			GroveBucket::placed(0.35, rory, Self::RareLowerMassiveRory),
		])
	}

	pub fn item(self) -> TemperateLowerMassivesItem {
		match self {
			Self::LowerMassiveBraidOak => {
				TemperateLowerMassivesItem::BraidOak(&LOWER_MASSIVE_BRAID_OAK)
			}
			Self::LowerMassiveStorybook => {
				TemperateLowerMassivesItem::Storybook(&LOWER_MASSIVE_STORYBOOK)
			}
			Self::RareLowerMassiveRory => {
				TemperateLowerMassivesItem::Rory(&RARE_LOWER_MASSIVE_RORY)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveBraidOak => BRAID_OAK_STICK_MIX,
			Self::LowerMassiveStorybook => STORYBOOK_STICK_MIX,
			Self::RareLowerMassiveRory => RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveBraidOak => BRAID_OAK_CANOPY_MIX,
			Self::LowerMassiveStorybook => STORYBOOK_CANOPY_MIX,
			Self::RareLowerMassiveRory => RORY_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const TEMPERATE_LOWER_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const TEMPERATE_LOWER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const TEMPERATE_LOWER_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
	TEMPERATE_LOWER_MASSIVES_STRUCTURAL_HIGH_FACTOR,
	TEMPERATE_LOWER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
	TEMPERATE_LOWER_MASSIVES_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{TemperateLowerMassives, TemperateLowerMassivesParams, TemperateLowerMassivesPlant};

#[cfg(test)]
mod tests;
