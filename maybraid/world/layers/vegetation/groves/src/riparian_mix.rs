//! Riparian Mix — mixed riparian upper-canopy grove with conifer accents
//! ([RFC-183 §3.4.7.11], [#333](https://github.com/ramate-io/maybraid/issues/333)).
//!
//! Braid oak and storybook bank/overbank forms with Friend's and Temperate Conifer on sheltered
//! margins.

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

/// Authored Riparian Mix grove definition.
///
/// Cell footprint sits at the RFC midpoint (`17.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<RiparianMixCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(17.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-17.0, 17.0),
		),
		distribution: RiparianMixCell::distribution(),
	}
}

/// Ordered riparian-mix varietals ([RFC-183 §3.4.7.11]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiparianMixCell {
	BankBraidOak,
	OverbankBraidOak,
	RoundRiparianStorybook,
	TallRiparianStorybook,
	BankFriendConifer,
	ShelteredTemperateConifer,
}

/// Typed authored geometry for one riparian-mix varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiparianMixItem {
	BraidOak(&'static RiparianMixBraidOak),
	Storybook(&'static RiparianMixStorybook),
	FriendsConifer(&'static RiparianMixFriendsConifer),
	TemperateConifer(&'static RiparianMixTemperateConifer),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixBraidOak {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixFriendsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Temperate Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixTemperateConifer {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

const BANK_BRAID_OAK: RiparianMixBraidOak =
	RiparianMixBraidOak { height: UnitRange::new(5.0, 12.0), canopy_density: DENSE_CANOPY_DENSITY };

const OVERBANK_BRAID_OAK: RiparianMixBraidOak = RiparianMixBraidOak {
	height: UnitRange::new(10.0, 18.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const ROUND_RIPARIAN_STORYBOOK: RiparianMixStorybook = RiparianMixStorybook {
	height: UnitRange::new(5.0, 15.0),
	stalk_radius: UnitRange::new(0.18, 0.40),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const TALL_RIPARIAN_STORYBOOK: RiparianMixStorybook = RiparianMixStorybook {
	height: UnitRange::new(12.0, 22.0),
	stalk_radius: UnitRange::new(0.26, 0.52),
	canopy_spread: UnitRange::new(3.5, 8.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const BANK_FRIEND_CONIFER: RiparianMixFriendsConifer = RiparianMixFriendsConifer {
	height: UnitRange::new(8.0, 16.0),
	stalk_radius: UnitRange::new(0.20, 0.40),
	canopy_spread: UnitRange::new(2.5, 6.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const SHELTERED_TEMPERATE_CONIFER: RiparianMixTemperateConifer = RiparianMixTemperateConifer {
	height: UnitRange::new(10.0, 20.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const BANK_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "gray_brown"),
]);

const BANK_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const OVERBANK_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const OVERBANK_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("river_green", "yellow_green"),
]);

const RIPARIAN_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const RIPARIAN_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "light_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const BANK_FRIEND_CONIFER_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BANK_FRIEND_CONIFER_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("river_green", "fresh_green"),
]);

const SHELTERED_TEMPERATE_CONIFER_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("temperate_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const SHELTERED_TEMPERATE_CONIFER_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "deep_green"),
	PaletteSlot::new("river_green", "fresh_green"),
]);

impl RiparianMixCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.45` (RFC relative proportions); the `None` weight of `10.9` puts
	/// the placed share at `4.45 / 15.35 ≈ 0.29`, mid RFC `DENSITY_RANGE` (`0.18..0.40`).
	pub fn distribution() -> GroveDistribution<Self> {
		let bank_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 0.38), UnitRange::new(0.0, 0.30));
		let overbank_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.48), UnitRange::new(0.0, 0.42));
		let round_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.46), UnitRange::new(0.0, 0.42));
		let tall_storybook =
			PlacementConstraints::new(UnitRange::new(0.00, 0.52), UnitRange::new(0.0, 0.48));
		let bank_friend =
			PlacementConstraints::new(UnitRange::new(0.00, 0.58), UnitRange::new(0.0, 0.50));
		let sheltered_temperate =
			PlacementConstraints::new(UnitRange::new(0.00, 0.62), UnitRange::new(0.0, 0.54));
		GroveDistribution::new(vec![
			GroveBucket::none(6.9),
			GroveBucket::placed(0.9, bank_braid_oak, Self::BankBraidOak),
			GroveBucket::placed(0.6, overbank_braid_oak, Self::OverbankBraidOak),
			GroveBucket::placed(0.9, round_storybook, Self::RoundRiparianStorybook),
			GroveBucket::placed(0.45, tall_storybook, Self::TallRiparianStorybook),
			GroveBucket::placed(0.8, bank_friend, Self::BankFriendConifer),
			GroveBucket::placed(0.8, sheltered_temperate, Self::ShelteredTemperateConifer),
		])
	}

	pub fn item(self) -> RiparianMixItem {
		match self {
			Self::BankBraidOak => RiparianMixItem::BraidOak(&BANK_BRAID_OAK),
			Self::OverbankBraidOak => RiparianMixItem::BraidOak(&OVERBANK_BRAID_OAK),
			Self::RoundRiparianStorybook => RiparianMixItem::Storybook(&ROUND_RIPARIAN_STORYBOOK),
			Self::TallRiparianStorybook => RiparianMixItem::Storybook(&TALL_RIPARIAN_STORYBOOK),
			Self::BankFriendConifer => RiparianMixItem::FriendsConifer(&BANK_FRIEND_CONIFER),
			Self::ShelteredTemperateConifer => {
				RiparianMixItem::TemperateConifer(&SHELTERED_TEMPERATE_CONIFER)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::BankBraidOak => BANK_BRAID_OAK_STICK_MIX,
			Self::OverbankBraidOak => OVERBANK_BRAID_OAK_STICK_MIX,
			Self::RoundRiparianStorybook | Self::TallRiparianStorybook => {
				RIPARIAN_STORYBOOK_STICK_MIX
			}
			Self::BankFriendConifer => BANK_FRIEND_CONIFER_STICK_MIX,
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_CONIFER_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::BankBraidOak => BANK_BRAID_OAK_CANOPY_MIX,
			Self::OverbankBraidOak => OVERBANK_BRAID_OAK_CANOPY_MIX,
			Self::RoundRiparianStorybook | Self::TallRiparianStorybook => {
				RIPARIAN_STORYBOOK_CANOPY_MIX
			}
			Self::BankFriendConifer => BANK_FRIEND_CONIFER_CANOPY_MIX,
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_CONIFER_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const RIPARIAN_MIX_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const RIPARIAN_MIX_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const RIPARIAN_MIX_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	RIPARIAN_MIX_STRUCTURAL_HIGH_FACTOR,
	RIPARIAN_MIX_STRUCTURAL_MEDIUM_FACTOR,
	RIPARIAN_MIX_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{RiparianMix, RiparianMixParams, RiparianMixPlant};

#[cfg(test)]
mod tests;
