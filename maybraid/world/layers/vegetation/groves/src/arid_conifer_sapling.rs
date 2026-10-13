//! Arid Conifer Sapling — well-known low-density dry young conifer lower-canopy grove
//! ([RFC-183 §3.4.6.6], [#327](https://github.com/ramate-io/maybraid/issues/327)).
//!
//! Sparse Friend's, Northern, and rare Liam's Conifer saplings on dry exposed terrain.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Standard arid sapling height band ([`2.0`, `4.0`] m).
const ARID_SAPLING_HEIGHT: UnitRange = UnitRange::new(2.0, 4.0);
/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.04, 0.15);
/// Ultra-sparse sampled canopy-density band ([`0.0`, `0.15`]).
const ULTRA_SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.01, 0.18);

/// Authored Arid Conifer Sapling grove definition.
///
/// Cell footprint at the RFC midpoint (`13.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid.
pub fn definition() -> GroveDefinition<AridConiferSaplingCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(13.5),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-13.5, 13.5),
		),
		distribution: AridConiferSaplingCell::distribution(),
	}
}

/// Ordered arid-conifer-sapling varietals ([RFC-183 §3.4.6.6]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AridConiferSaplingCell {
	DryFriendSapling,
	DryNorthernSapling,
	WispyDryFriendSapling,
	WispyDryNorthernSapling,
	BareDryFriendSapling,
	BareDryNorthernSapling,
	DryLiamsConiferSapling,
}

/// Typed authored geometry for one arid-conifer-sapling varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AridConiferSaplingItem {
	FriendsConifer(&'static AridConiferSaplingFriendsConifer),
	NorthernConifer(&'static AridConiferSaplingNorthernConifer),
	LiamsConifer(&'static AridConiferSaplingLiamsConifer),
}

/// Authored geometry ranges for one dry Friend's Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct AridConiferSaplingFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Northern Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct AridConiferSaplingNorthernConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (Northern `0.032 × H`).
	pub stalk_radius: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Liam's Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct AridConiferSaplingLiamsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse tuft density at render time.
	pub canopy_density: UnitRange,
}

const DRY_FRIEND_SAPLING: AridConiferSaplingFriendsConifer = AridConiferSaplingFriendsConifer {
	height: ARID_SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.05, 0.10),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRY_NORTHERN_SAPLING: AridConiferSaplingNorthernConifer = AridConiferSaplingNorthernConifer {
	height: ARID_SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.064, 0.128),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WISPY_DRY_FRIEND_SAPLING: AridConiferSaplingFriendsConifer =
	AridConiferSaplingFriendsConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.05, 0.10),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const WISPY_DRY_NORTHERN_SAPLING: AridConiferSaplingNorthernConifer =
	AridConiferSaplingNorthernConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.064, 0.128),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const BARE_DRY_FRIEND_SAPLING: AridConiferSaplingFriendsConifer =
	AridConiferSaplingFriendsConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.05, 0.09),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const BARE_DRY_NORTHERN_SAPLING: AridConiferSaplingNorthernConifer =
	AridConiferSaplingNorthernConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.064, 0.115),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const DRY_LIAMS_SAPLING: AridConiferSaplingLiamsConifer = AridConiferSaplingLiamsConifer {
	height: ARID_SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.05, 0.10),
	canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
};

const DRY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_conifer_bark", "tan_bark"),
	PaletteSlot::new("gray_brown", "sun_baked_bark"),
]);

const DRY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sage_green", "dusty_green"),
	PaletteSlot::new("deep_green", "olive_green"),
]);

const DRY_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_gray_bark", "dark_bark"),
	PaletteSlot::new("tan_bark", "conifer_bark"),
]);

const DRY_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_sage", "dusty_green"),
	PaletteSlot::new("dark_green", "olive_green"),
]);

const WISPY_DRY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sun_baked_bark", "dry_conifer_bark"),
	PaletteSlot::new("gray_brown", "tan_bark"),
]);

const WISPY_DRY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "sage_green"),
	PaletteSlot::new("olive_green", "deep_green"),
]);

const WISPY_DRY_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_gray_bark", "sun_baked_bark"),
	PaletteSlot::new("conifer_bark", "tan_bark"),
]);

const WISPY_DRY_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_sage", "sage_green"),
	PaletteSlot::new("olive_green", "dusty_green"),
]);

const BARE_DRY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tan_bark", "sun_baked_bark"),
	PaletteSlot::new("dry_conifer_bark", "gray_brown"),
]);

const BARE_DRY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sage_green", "olive_green"),
	PaletteSlot::new("dusty_green", "blue_sage"),
]);

const BARE_DRY_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dry_gray_bark"),
	PaletteSlot::new("sun_baked_bark", "gray_brown"),
]);

const BARE_DRY_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "blue_sage"),
	PaletteSlot::new("dark_green", "olive_green"),
]);

const DRY_LIAMS_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_gray_bark", "sun_baked_bark"),
	PaletteSlot::new("tan_bark", "dry_conifer_bark"),
]);

const DRY_LIAMS_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_sage", "sage_green"),
	PaletteSlot::new("dusty_green", "olive_green"),
]);

impl AridConiferSaplingCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.7` (two sparse pair, four ultra-sparse accents, rare Liam's); the
	/// `None` weight of `24.0` puts the placed share at `4.7 / 28.7 ≈ 0.16`, mid RFC
	/// `DENSITY_RANGE` (`0.08..0.24`).
	/// Placement constraints are unconstrained until RFC elevation bands land ([#327](https://github.com/ramate-io/maybraid/issues/327)).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(24.0),
			GroveBucket::placed(0.5, PlacementConstraints::UNCONSTRAINED, Self::DryFriendSapling),
			GroveBucket::placed(0.5, PlacementConstraints::UNCONSTRAINED, Self::DryNorthernSapling),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::UNCONSTRAINED,
				Self::WispyDryFriendSapling,
			),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::UNCONSTRAINED,
				Self::WispyDryNorthernSapling,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::UNCONSTRAINED,
				Self::BareDryFriendSapling,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::UNCONSTRAINED,
				Self::BareDryNorthernSapling,
			),
			GroveBucket::placed(
				0.2,
				PlacementConstraints::UNCONSTRAINED,
				Self::DryLiamsConiferSapling,
			),
		])
	}

	pub fn item(self) -> AridConiferSaplingItem {
		match self {
			Self::DryFriendSapling => AridConiferSaplingItem::FriendsConifer(&DRY_FRIEND_SAPLING),
			Self::WispyDryFriendSapling => {
				AridConiferSaplingItem::FriendsConifer(&WISPY_DRY_FRIEND_SAPLING)
			}
			Self::BareDryFriendSapling => {
				AridConiferSaplingItem::FriendsConifer(&BARE_DRY_FRIEND_SAPLING)
			}
			Self::DryNorthernSapling => {
				AridConiferSaplingItem::NorthernConifer(&DRY_NORTHERN_SAPLING)
			}
			Self::WispyDryNorthernSapling => {
				AridConiferSaplingItem::NorthernConifer(&WISPY_DRY_NORTHERN_SAPLING)
			}
			Self::BareDryNorthernSapling => {
				AridConiferSaplingItem::NorthernConifer(&BARE_DRY_NORTHERN_SAPLING)
			}
			Self::DryLiamsConiferSapling => {
				AridConiferSaplingItem::LiamsConifer(&DRY_LIAMS_SAPLING)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryFriendSapling => DRY_FRIEND_SAPLING_STICK_MIX,
			Self::WispyDryFriendSapling => WISPY_DRY_FRIEND_SAPLING_STICK_MIX,
			Self::BareDryFriendSapling => BARE_DRY_FRIEND_SAPLING_STICK_MIX,
			Self::DryNorthernSapling => DRY_NORTHERN_SAPLING_STICK_MIX,
			Self::WispyDryNorthernSapling => WISPY_DRY_NORTHERN_SAPLING_STICK_MIX,
			Self::BareDryNorthernSapling => BARE_DRY_NORTHERN_SAPLING_STICK_MIX,
			Self::DryLiamsConiferSapling => DRY_LIAMS_SAPLING_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryFriendSapling => DRY_FRIEND_SAPLING_CANOPY_MIX,
			Self::WispyDryFriendSapling => WISPY_DRY_FRIEND_SAPLING_CANOPY_MIX,
			Self::BareDryFriendSapling => BARE_DRY_FRIEND_SAPLING_CANOPY_MIX,
			Self::DryNorthernSapling => DRY_NORTHERN_SAPLING_CANOPY_MIX,
			Self::WispyDryNorthernSapling => WISPY_DRY_NORTHERN_SAPLING_CANOPY_MIX,
			Self::BareDryNorthernSapling => BARE_DRY_NORTHERN_SAPLING_CANOPY_MIX,
			Self::DryLiamsConiferSapling => DRY_LIAMS_SAPLING_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const ARID_CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const ARID_CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const ARID_CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	ARID_CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR,
	ARID_CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR,
	ARID_CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{AridConiferSapling, AridConiferSaplingParams, AridConiferSaplingPlant};

#[cfg(test)]
mod tests;
