//! Rolling Oaks — low-density open oak-country upper-canopy grove
//! ([RFC-183 §3.4.7.5], [#349](https://github.com/ramate-io/maybraid/issues/349)).
//!
//! Common dry Braid Oak forms with rare Storybook accents across rolling open woodland.

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

/// Authored Rolling Oaks grove definition.
///
/// Cell footprint sits at the RFC midpoint (`22` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<RollingOaksCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(22.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-22.0, 22.0),
		),
		distribution: RollingOaksCell::distribution(),
	}
}

/// Ordered rolling-oaks varietals ([RFC-183 §3.4.7.5]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingOaksCell {
	RollingBraidOak,
	RareTallRollingBraidOak,
	RareSentinelRollingBraidOak,
	RareRollingStorybook,
}

/// Typed authored geometry for one rolling-oaks varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RollingOaksItem {
	BraidOak(&'static RollingOaksBraidOak),
	Storybook(&'static RollingOaksStorybook),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingOaksBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingOaksStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const ROLLING_BRAID_OAK: RollingOaksBraidOak = RollingOaksBraidOak {
	height: UnitRange::new(5.0, 20.0),
	canopy_spread: UnitRange::new(2.0, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_TALL_ROLLING_BRAID_OAK: RollingOaksBraidOak = RollingOaksBraidOak {
	height: UnitRange::new(20.0, 32.0),
	canopy_spread: UnitRange::new(5.0, 11.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_SENTINEL_ROLLING_BRAID_OAK: RollingOaksBraidOak = RollingOaksBraidOak {
	height: UnitRange::new(28.0, 40.0),
	canopy_spread: UnitRange::new(7.0, 14.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_ROLLING_STORYBOOK: RollingOaksStorybook = RollingOaksStorybook {
	height: UnitRange::new(5.0, 20.0),
	stalk_radius: UnitRange::new(0.20, 0.48),
	canopy_spread: UnitRange::new(2.0, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const ROLLING_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dry_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const ROLLING_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const RARE_TALL_ROLLING_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gnarled_brown", "oak_bark"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const RARE_TALL_ROLLING_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("olive_green", "light_green"),
]);

const RARE_SENTINEL_ROLLING_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_bark", "gnarled_brown"),
	PaletteSlot::new("dark_bark", "moss_bark"),
]);

const RARE_SENTINEL_ROLLING_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("emerald_green", "deep_green"),
	PaletteSlot::new("moss_green", "olive_green"),
]);

const ROLLING_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "dry_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const ROLLING_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl RollingOaksCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.55`; the `None` weight of `12.4` puts the placed share at
	/// `2.55 / 14.95 ≈ 0.17`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let tall_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let sentinel_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.44));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.54));
		GroveDistribution::new(vec![
			GroveBucket::none(12.4),
			GroveBucket::placed(2.0, braid_oak, Self::RollingBraidOak),
			GroveBucket::placed(0.15, tall_braid_oak, Self::RareTallRollingBraidOak),
			GroveBucket::placed(0.05, sentinel_braid_oak, Self::RareSentinelRollingBraidOak),
			GroveBucket::placed(0.35, storybook, Self::RareRollingStorybook),
		])
	}

	pub fn item(self) -> RollingOaksItem {
		match self {
			Self::RollingBraidOak => RollingOaksItem::BraidOak(&ROLLING_BRAID_OAK),
			Self::RareTallRollingBraidOak => {
				RollingOaksItem::BraidOak(&RARE_TALL_ROLLING_BRAID_OAK)
			}
			Self::RareSentinelRollingBraidOak => {
				RollingOaksItem::BraidOak(&RARE_SENTINEL_ROLLING_BRAID_OAK)
			}
			Self::RareRollingStorybook => RollingOaksItem::Storybook(&RARE_ROLLING_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::RollingBraidOak => ROLLING_BRAID_OAK_STICK_MIX,
			Self::RareTallRollingBraidOak => RARE_TALL_ROLLING_BRAID_OAK_STICK_MIX,
			Self::RareSentinelRollingBraidOak => RARE_SENTINEL_ROLLING_BRAID_OAK_STICK_MIX,
			Self::RareRollingStorybook => ROLLING_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::RollingBraidOak => ROLLING_BRAID_OAK_CANOPY_MIX,
			Self::RareTallRollingBraidOak => RARE_TALL_ROLLING_BRAID_OAK_CANOPY_MIX,
			Self::RareSentinelRollingBraidOak => RARE_SENTINEL_ROLLING_BRAID_OAK_CANOPY_MIX,
			Self::RareRollingStorybook => ROLLING_STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Typical sentinels ~36 m. `grove_bands_for_typical_height(36)`.
pub const ROLLING_OAKS_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const ROLLING_OAKS_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const ROLLING_OAKS_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	ROLLING_OAKS_STRUCTURAL_HIGH_FACTOR,
	ROLLING_OAKS_STRUCTURAL_MEDIUM_FACTOR,
	ROLLING_OAKS_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{RollingOaks, RollingOaksParams, RollingOaksPlant};

#[cfg(test)]
mod tests;
