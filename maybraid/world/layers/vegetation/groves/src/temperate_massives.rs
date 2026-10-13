//! Temperate Massives — low-density giant broadleaf upper-canopy grove
//! ([RFC-183 §3.4.7.3], [#345](https://github.com/ramate-io/maybraid/issues/345)).
//!
//! Enormous Braid Oak, Storybook Tree, and rare Rory's Head-trained skyline forms above temperate
//! lower massives.

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
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Temperate Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`49` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TemperateMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(49.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-49.0, 49.0),
		),
		distribution: TemperateMassivesCell::distribution(),
	}
}

/// Ordered temperate-massive varietals ([RFC-183 §3.4.7.3]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperateMassivesCell {
	MassiveBraidOak,
	MassiveStorybook,
	RareMassiveRory,
}

/// Typed authored geometry for one temperate-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemperateMassivesItem {
	BraidOak(&'static TemperateMassivesBraidOak),
	Storybook(&'static TemperateMassivesStorybook),
	Rory(&'static TemperateMassivesRory),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one rare Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const MASSIVE_BRAID_OAK: TemperateMassivesBraidOak = TemperateMassivesBraidOak {
	height: UnitRange::new(28.0, 80.0),
	canopy_spread: UnitRange::new(8.0, 20.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_STORYBOOK: TemperateMassivesStorybook = TemperateMassivesStorybook {
	height: UnitRange::new(35.0, 170.0),
	stalk_radius: UnitRange::new(3.0, 9.0),
	canopy_spread: UnitRange::new(12.0, 35.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const RARE_MASSIVE_RORY: TemperateMassivesRory = TemperateMassivesRory {
	height: UnitRange::new(50.0, 200.0),
	stalk_radius: UnitRange::new(0.45, 1.80),
	canopy_spread: UnitRange::new(6.0, 14.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
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

impl TemperateMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.35`; the `None` weight of `24.6` puts the placed share at
	/// `4.35 / 28.95 ≈ 0.15`, mid RFC `DENSITY_RANGE` (`0.08..0.22`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.44));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(24.6),
			GroveBucket::placed(2.0, braid_oak, Self::MassiveBraidOak),
			GroveBucket::placed(2.0, storybook, Self::MassiveStorybook),
			GroveBucket::placed(0.35, rory, Self::RareMassiveRory),
		])
	}

	pub fn item(self) -> TemperateMassivesItem {
		match self {
			Self::MassiveBraidOak => TemperateMassivesItem::BraidOak(&MASSIVE_BRAID_OAK),
			Self::MassiveStorybook => TemperateMassivesItem::Storybook(&MASSIVE_STORYBOOK),
			Self::RareMassiveRory => TemperateMassivesItem::Rory(&RARE_MASSIVE_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveBraidOak => BRAID_OAK_STICK_MIX,
			Self::MassiveStorybook => STORYBOOK_STICK_MIX,
			Self::RareMassiveRory => RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveBraidOak => BRAID_OAK_CANOPY_MIX,
			Self::MassiveStorybook => STORYBOOK_CANOPY_MIX,
			Self::RareMassiveRory => RORY_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Typical large types ~170 m (storybook / rory). `grove_bands_for_typical_height(170)`.
pub const TEMPERATE_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const TEMPERATE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 12.0;
#[cfg(feature = "render")]
pub const TEMPERATE_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 24.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
	TEMPERATE_MASSIVES_STRUCTURAL_HIGH_FACTOR,
	TEMPERATE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
	TEMPERATE_MASSIVES_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{TemperateMassives, TemperateMassivesParams, TemperateMassivesPlant};

#[cfg(test)]
mod tests;
