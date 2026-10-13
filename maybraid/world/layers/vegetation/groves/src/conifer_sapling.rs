//! Conifer Sapling — well-known moderate-density young conifer lower-canopy grove
//! ([RFC-183 §3.4.6.5], [#326](https://github.com/ramate-io/maybraid/issues/326)).
//!
//! Mixed Friend's and Northern Conifer saplings beneath taller evergreen canopy.

use bevy_math::{Vec2, Vec3};
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveWorldSample,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Uniform terrain tuned for conifer sapling placement constraints (RFC elevation bands overlap).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Terrain"))]
pub struct SaplingFlatTerrain {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.50))]
	pub elevation: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.30))]
	pub steepness: f32,
}

impl Default for SaplingFlatTerrain {
	fn default() -> Self {
		Self { elevation: 0.50, steepness: 0.30 }
	}
}

impl GroveWorldSample for SaplingFlatTerrain {
	fn height_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// Standard sapling height band ([`1.0`, `4.0`] m).
const SAPLING_HEIGHT: UnitRange = UnitRange::new(1.0, 4.0);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse..moderate band for windswept northern accents.
const SPARSE_TO_MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.20, 0.55);

/// Authored Conifer Sapling grove definition.
///
/// Cell footprint at the RFC midpoint (`10.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid.
pub fn definition() -> GroveDefinition<ConiferSaplingCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(10.5),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-10.5, 10.5),
		),
		distribution: ConiferSaplingCell::distribution(),
	}
}

/// Ordered conifer-sapling varietals ([RFC-183 §3.4.6.5]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConiferSaplingCell {
	FriendSapling,
	NorthernSapling,
	MossyFriendSapling,
	ColdNorthernSapling,
	BrightFriendSapling,
	WindsweptNorthernSapling,
}

/// Typed authored geometry for one conifer-sapling varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConiferSaplingItem {
	FriendsConifer(&'static ConiferSaplingFriendsConifer),
	NorthernConifer(&'static ConiferSaplingNorthernConifer),
}

/// Authored geometry ranges for one Friend's Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferSaplingFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Northern Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferSaplingNorthernConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (Northern `0.032 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.20, 0.70),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const MOSSY_FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.15, 0.55),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const BRIGHT_FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.22, 0.75),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.20, 0.70),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const COLD_NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.18, 0.60),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WINDSWEPT_NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.12, 0.50),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const MOSSY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_bark", "conifer_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const MOSSY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_green", "deep_green"),
	PaletteSlot::new("olive_green", "needle_green"),
]);

const BRIGHT_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("young_bark", "conifer_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BRIGHT_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "yellow_green"),
	PaletteSlot::new("light_green", "spring_green"),
]);

const NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const COLD_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "gray_brown"),
	PaletteSlot::new("conifer_bark", "dark_bark"),
]);

const COLD_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "deep_green"),
	PaletteSlot::new("blue_green", "dark_green"),
]);

const WINDSWEPT_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gray_brown", "cold_bark"),
	PaletteSlot::new("conifer_bark", "dry_bark"),
]);

const WINDSWEPT_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "cold_green"),
	PaletteSlot::new("needle_green", "olive_green"),
]);

impl ConiferSaplingCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.4` (RFC pair plus sapling accents); the `None` weight of `5.2` puts
	/// the placed share at `3.4 / 8.6 ≈ 0.40`, mid RFC `DENSITY_RANGE` (`0.28..0.48`).
	pub fn distribution() -> GroveDistribution<Self> {
		let friend =
			PlacementConstraints::new(UnitRange::new(0.18, 0.82), UnitRange::new(0.0, 0.64));
		let northern =
			PlacementConstraints::new(UnitRange::new(0.22, 0.88), UnitRange::new(0.0, 0.72));
		GroveDistribution::new(vec![
			GroveBucket::none(5.2),
			GroveBucket::placed(1.0, friend, Self::FriendSapling),
			GroveBucket::placed(1.0, northern, Self::NorthernSapling),
			GroveBucket::placed(0.35, friend, Self::MossyFriendSapling),
			GroveBucket::placed(0.35, northern, Self::ColdNorthernSapling),
			GroveBucket::placed(0.30, friend, Self::BrightFriendSapling),
			GroveBucket::placed(0.40, northern, Self::WindsweptNorthernSapling),
		])
	}

	pub fn item(self) -> ConiferSaplingItem {
		match self {
			Self::FriendSapling => ConiferSaplingItem::FriendsConifer(&FRIEND_SAPLING),
			Self::BrightFriendSapling => ConiferSaplingItem::FriendsConifer(&BRIGHT_FRIEND_SAPLING),
			Self::MossyFriendSapling => ConiferSaplingItem::FriendsConifer(&MOSSY_FRIEND_SAPLING),
			Self::NorthernSapling => ConiferSaplingItem::NorthernConifer(&NORTHERN_SAPLING),
			Self::ColdNorthernSapling => {
				ConiferSaplingItem::NorthernConifer(&COLD_NORTHERN_SAPLING)
			}
			Self::WindsweptNorthernSapling => {
				ConiferSaplingItem::NorthernConifer(&WINDSWEPT_NORTHERN_SAPLING)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FriendSapling => FRIEND_SAPLING_STICK_MIX,
			Self::MossyFriendSapling => MOSSY_FRIEND_SAPLING_STICK_MIX,
			Self::BrightFriendSapling => BRIGHT_FRIEND_SAPLING_STICK_MIX,
			Self::NorthernSapling => NORTHERN_SAPLING_STICK_MIX,
			Self::ColdNorthernSapling => COLD_NORTHERN_SAPLING_STICK_MIX,
			Self::WindsweptNorthernSapling => WINDSWEPT_NORTHERN_SAPLING_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FriendSapling => FRIEND_SAPLING_CANOPY_MIX,
			Self::MossyFriendSapling => MOSSY_FRIEND_SAPLING_CANOPY_MIX,
			Self::BrightFriendSapling => BRIGHT_FRIEND_SAPLING_CANOPY_MIX,
			Self::NorthernSapling => NORTHERN_SAPLING_CANOPY_MIX,
			Self::ColdNorthernSapling => COLD_NORTHERN_SAPLING_CANOPY_MIX,
			Self::WindsweptNorthernSapling => WINDSWEPT_NORTHERN_SAPLING_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR,
	CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR,
	CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{ConiferSapling, ConiferSaplingParams, ConiferSaplingPlant};

#[cfg(test)]
mod tests;
