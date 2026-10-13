//! Alpine — cold upland conifer upper-canopy grove
//! ([RFC-183 §3.4.7.12], [#334](https://github.com/ramate-io/maybraid/issues/334)).
//!
//! Tall Friend's Conifer with less common Liam's Conifer on high, steep terrain.

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
/// Moderate sampled canopy-density band ([`0.25`, `0.45`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.25, 0.45);
/// Dense sampled canopy-density band ([`0.35`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.85);

/// Authored Alpine grove definition.
///
/// Cell footprint sits at the RFC midpoint (`27.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<AlpineCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(27.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-27.0, 27.0),
		),
		distribution: AlpineCell::distribution(),
	}
}

/// Ordered alpine varietals ([RFC-183 §3.4.7.12]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpineCell {
	TallAlpineFriendsConifer,
	WindlineFriendsConifer,
	AlpineLiamsConifer,
	NeedleSpireLiamsConifer,
}

/// Typed authored geometry for one alpine varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlpineItem {
	FriendsConifer(&'static AlpineFriendsConifer),
	LiamsConifer(&'static AlpineLiamsConifer),
}

/// Authored geometry ranges for one Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct AlpineFriendsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct AlpineLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

const TALL_ALPINE_FRIENDS: AlpineFriendsConifer = AlpineFriendsConifer {
	height: UnitRange::new(12.0, 40.0),
	stalk_radius: UnitRange::new(0.32, 0.72),
	canopy_spread: UnitRange::new(4.0, 7.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const WINDLINE_FRIENDS: AlpineFriendsConifer = AlpineFriendsConifer {
	height: UnitRange::new(6.0, 22.0),
	stalk_radius: UnitRange::new(0.18, 0.42),
	canopy_spread: UnitRange::new(1.5, 5.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ALPINE_LIAMS: AlpineLiamsConifer = AlpineLiamsConifer {
	height: UnitRange::new(8.0, 40.0),
	stalk_radius: UnitRange::new(0.25, 0.85),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const NEEDLE_SPIRE_LIAMS: AlpineLiamsConifer = AlpineLiamsConifer {
	height: UnitRange::new(6.0, 32.0),
	stalk_radius: UnitRange::new(0.30, 0.55),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const TALL_FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const TALL_FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const WINDLINE_FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wind_barked", "cold_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const WINDLINE_FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("dark_green", "deep_green"),
]);

const ALPINE_LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const ALPINE_LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const NEEDLE_SPIRE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("stone_gray", "conifer_bark"),
]);

const NEEDLE_SPIRE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "dark_green"),
	PaletteSlot::new("cold_green", "deep_green"),
]);

impl AlpineCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.7`; the `None` weight of `9.5` puts the placed share at
	/// `3.7 / 13.2 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let tall_friends =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.68));
		let windline_friends =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.86));
		let alpine_liams =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.86));
		let needle_spire =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.92));
		GroveDistribution::new(vec![
			GroveBucket::none(9.5),
			GroveBucket::placed(1.5, tall_friends, Self::TallAlpineFriendsConifer),
			GroveBucket::placed(0.75, windline_friends, Self::WindlineFriendsConifer),
			GroveBucket::placed(1.0, alpine_liams, Self::AlpineLiamsConifer),
			GroveBucket::placed(0.45, needle_spire, Self::NeedleSpireLiamsConifer),
		])
	}

	pub fn item(self) -> AlpineItem {
		match self {
			Self::TallAlpineFriendsConifer | Self::WindlineFriendsConifer => match self {
				Self::TallAlpineFriendsConifer => AlpineItem::FriendsConifer(&TALL_ALPINE_FRIENDS),
				Self::WindlineFriendsConifer => AlpineItem::FriendsConifer(&WINDLINE_FRIENDS),
				_ => unreachable!(),
			},
			Self::AlpineLiamsConifer => AlpineItem::LiamsConifer(&ALPINE_LIAMS),
			Self::NeedleSpireLiamsConifer => AlpineItem::LiamsConifer(&NEEDLE_SPIRE_LIAMS),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::TallAlpineFriendsConifer => TALL_FRIENDS_STICK_MIX,
			Self::WindlineFriendsConifer => WINDLINE_FRIENDS_STICK_MIX,
			Self::AlpineLiamsConifer => ALPINE_LIAMS_STICK_MIX,
			Self::NeedleSpireLiamsConifer => NEEDLE_SPIRE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::TallAlpineFriendsConifer => TALL_FRIENDS_CANOPY_MIX,
			Self::WindlineFriendsConifer => WINDLINE_FRIENDS_CANOPY_MIX,
			Self::AlpineLiamsConifer => ALPINE_LIAMS_CANOPY_MIX,
			Self::NeedleSpireLiamsConifer => NEEDLE_SPIRE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Typical large conifers ~36 m. `grove_bands_for_typical_height(36)`.
pub const ALPINE_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const ALPINE_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const ALPINE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	ALPINE_STRUCTURAL_HIGH_FACTOR,
	ALPINE_STRUCTURAL_MEDIUM_FACTOR,
	ALPINE_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{Alpine, AlpineParams, AlpinePlant};

#[cfg(test)]
mod tests;
