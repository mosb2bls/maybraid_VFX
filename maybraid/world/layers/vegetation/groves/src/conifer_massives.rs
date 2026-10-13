//! Conifer Massives — low-density giant evergreen upper-canopy grove
//! ([RFC-183 §3.4.7.2], [#343](https://github.com/ramate-io/maybraid/issues/343)).
//!
//! Towering Northern, Friend's, Liam's, and Temperate Conifer skyline forms above conifer lower
//! massives.

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
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Conifer Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`50.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ConiferMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(50.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-50.0, 50.0),
		),
		distribution: ConiferMassivesCell::distribution(),
	}
}

/// Ordered conifer-massive varietals ([RFC-183 §3.4.7.2]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConiferMassivesCell {
	MassiveNorthernConifer,
	MassiveFriendsConifer,
	MassiveLiamsConifer,
	MassiveTemperateConifer,
}

/// Typed authored geometry for one conifer-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConiferMassivesItem {
	NorthernConifer(&'static ConiferMassivesNorthernConifer),
	FriendsConifer(&'static ConiferMassivesFriendsConifer),
	LiamsConifer(&'static ConiferMassivesLiamsConifer),
	TemperateConifer(&'static ConiferMassivesTemperateConifer),
}

/// Authored geometry ranges for one Northern Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesNorthernConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesFriendsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Temperate Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesTemperateConifer {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

const MASSIVE_NORTHERN_CONIFER: ConiferMassivesNorthernConifer = ConiferMassivesNorthernConifer {
	height: UnitRange::new(70.0, 200.0),
	stalk_radius: UnitRange::new(2.0, 6.5),
	canopy_spread: UnitRange::new(15.0, 45.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_FRIENDS_CONIFER: ConiferMassivesFriendsConifer = ConiferMassivesFriendsConifer {
	height: UnitRange::new(100.0, 130.0),
	stalk_radius: UnitRange::new(2.5, 5.5),
	canopy_spread: UnitRange::new(18.0, 35.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_LIAMS_CONIFER: ConiferMassivesLiamsConifer = ConiferMassivesLiamsConifer {
	height: UnitRange::new(25.0, 130.0),
	stalk_radius: UnitRange::new(0.5, 4.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const MASSIVE_TEMPERATE_CONIFER: ConiferMassivesTemperateConifer =
	ConiferMassivesTemperateConifer {
		height: UnitRange::new(40.0, 120.0),
		canopy_density: MODERATE_CANOPY_DENSITY,
	};

const NORTHERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const NORTHERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("temperate_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "deep_green"),
	PaletteSlot::new("blue_green", "fresh_green"),
]);

impl ConiferMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.5`; the `None` weight of `23.0` puts the placed share at
	/// `3.5 / 26.5 ≈ 0.132`, mid RFC `DENSITY_RANGE` (`0.06..0.20`).
	pub fn distribution() -> GroveDistribution<Self> {
		let northern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.70));
		let friends =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		let liams = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.76));
		let temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		GroveDistribution::new(vec![
			GroveBucket::none(23.0),
			GroveBucket::placed(1.25, northern, Self::MassiveNorthernConifer),
			GroveBucket::placed(1.25, friends, Self::MassiveFriendsConifer),
			GroveBucket::placed(0.75, liams, Self::MassiveLiamsConifer),
			GroveBucket::placed(0.25, temperate, Self::MassiveTemperateConifer),
		])
	}

	pub fn item(self) -> ConiferMassivesItem {
		match self {
			Self::MassiveNorthernConifer => {
				ConiferMassivesItem::NorthernConifer(&MASSIVE_NORTHERN_CONIFER)
			}
			Self::MassiveFriendsConifer => {
				ConiferMassivesItem::FriendsConifer(&MASSIVE_FRIENDS_CONIFER)
			}
			Self::MassiveLiamsConifer => ConiferMassivesItem::LiamsConifer(&MASSIVE_LIAMS_CONIFER),
			Self::MassiveTemperateConifer => {
				ConiferMassivesItem::TemperateConifer(&MASSIVE_TEMPERATE_CONIFER)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveNorthernConifer => NORTHERN_STICK_MIX,
			Self::MassiveFriendsConifer => FRIENDS_STICK_MIX,
			Self::MassiveLiamsConifer => LIAMS_STICK_MIX,
			Self::MassiveTemperateConifer => TEMPERATE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveNorthernConifer => NORTHERN_CANOPY_MIX,
			Self::MassiveFriendsConifer => FRIENDS_CANOPY_MIX,
			Self::MassiveLiamsConifer => LIAMS_CANOPY_MIX,
			Self::MassiveTemperateConifer => TEMPERATE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Typical large types ~160 m (northern / friends firs). `grove_bands_for_typical_height(160)`.
pub const CONIFER_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const CONIFER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 12.0;
#[cfg(feature = "render")]
pub const CONIFER_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 24.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	CONIFER_MASSIVES_STRUCTURAL_HIGH_FACTOR,
	CONIFER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
	CONIFER_MASSIVES_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{ConiferMassives, ConiferMassivesParams, ConiferMassivesPlant};

#[cfg(test)]
mod tests;
