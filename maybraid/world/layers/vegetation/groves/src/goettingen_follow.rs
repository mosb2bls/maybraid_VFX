//! Goettingen Follow — well-known low-density temperate lower-canopy follow grove
//! ([RFC-183 §3.4.6.4], [#325](https://github.com/ramate-io/maybraid/issues/325)).
//!
//! Sparse braid oaks and storybook forms beneath taller canopy.

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

/// Authored Goettingen Follow grove definition.
///
/// Cell footprint at `9.0` m (below the RFC midpoint for tighter follow-layer spacing). The offset
/// range is signed and ± one cell so placements break the underlying grid.
pub fn definition() -> GroveDefinition<GoettingenFollowCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(9.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-9.0, 9.0)),
		distribution: GoettingenFollowCell::distribution(),
	}
}

/// Ordered goettingen-follow varietals ([RFC-183 §3.4.6.4]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoettingenFollowCell {
	FollowBraidOak,
	RedBranchBraidOak,
	MossyTrailBraidOak,
	ParkEdgeBraidOak,
	TallFollowBraidOak,
	OldGrowthFollowBraidOak,
	FollowStorybook,
}

/// Typed authored geometry for one goettingen-follow varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GoettingenFollowItem {
	BraidOak(&'static GoettingenFollowBraidOak),
	Storybook(&'static GoettingenFollowStorybook),
}

/// Authored geometry ranges for one Braid Oak form (shared geometry; palette differs per cell).
#[derive(Debug, Clone, PartialEq)]
pub struct GoettingenFollowBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one follow Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct GoettingenFollowStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const FOLLOW_BRAID_OAK: GoettingenFollowBraidOak = GoettingenFollowBraidOak {
	height: UnitRange::new(4.0, 9.0),
	canopy_spread: UnitRange::new(1.6, 4.2),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const TALL_FOLLOW_BRAID_OAK: GoettingenFollowBraidOak = GoettingenFollowBraidOak {
	height: UnitRange::new(7.0, 11.0),
	canopy_spread: UnitRange::new(2.0, 4.8),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const OLD_GROWTH_FOLLOW_BRAID_OAK: GoettingenFollowBraidOak = GoettingenFollowBraidOak {
	height: UnitRange::new(8.0, 12.0),
	canopy_spread: UnitRange::new(2.2, 5.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const FOLLOW_STORYBOOK: GoettingenFollowStorybook = GoettingenFollowStorybook {
	height: UnitRange::new(4.0, 9.0),
	stalk_radius: UnitRange::new(0.18, 0.40),
	canopy_spread: UnitRange::new(1.6, 4.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const FOLLOW_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const FOLLOW_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
]);

const RED_BRANCH_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "copper_red"),
	PaletteSlot::new("dark_bark", "gray_brown"),
]);

const RED_BRANCH_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

const MOSSY_TRAIL_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_bark", "gnarled_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const MOSSY_TRAIL_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_green", "olive_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const PARK_EDGE_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ornamental_bark", "young_bark"),
	PaletteSlot::new("oak_bark", "gray_brown"),
]);

const PARK_EDGE_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("silver_green", "broadleaf_green"),
	PaletteSlot::new("light_green", "fresh_green"),
]);

const TALL_FOLLOW_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "gray_brown"),
]);

const TALL_FOLLOW_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("olive_green", "light_green"),
]);

const OLD_GROWTH_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gnarled_brown", "dark_bark"),
	PaletteSlot::new("moss_bark", "wet_bark"),
]);

const OLD_GROWTH_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "moss_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);

const FOLLOW_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const FOLLOW_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl GoettingenFollowCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.75` (RFC braid-oak and storybook proportions plus follow accents);
	/// the `None` weight of `9.7` puts the placed share at `3.75 / 13.45 ≈ 0.28`, upper RFC
	/// `DENSITY_RANGE` (`0.10..0.28`).
	/// Placement constraints are unconstrained until RFC elevation bands land ([#325](https://github.com/ramate-io/maybraid/issues/325)).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(9.7),
			GroveBucket::placed(1.0, PlacementConstraints::UNCONSTRAINED, Self::FollowBraidOak),
			GroveBucket::placed(0.35, PlacementConstraints::UNCONSTRAINED, Self::RedBranchBraidOak),
			GroveBucket::placed(
				0.40,
				PlacementConstraints::UNCONSTRAINED,
				Self::MossyTrailBraidOak,
			),
			GroveBucket::placed(0.30, PlacementConstraints::UNCONSTRAINED, Self::ParkEdgeBraidOak),
			GroveBucket::placed(
				0.45,
				PlacementConstraints::UNCONSTRAINED,
				Self::TallFollowBraidOak,
			),
			GroveBucket::placed(
				0.25,
				PlacementConstraints::UNCONSTRAINED,
				Self::OldGrowthFollowBraidOak,
			),
			GroveBucket::placed(1.0, PlacementConstraints::UNCONSTRAINED, Self::FollowStorybook),
		])
	}

	pub fn item(self) -> GoettingenFollowItem {
		match self {
			Self::FollowBraidOak
			| Self::RedBranchBraidOak
			| Self::MossyTrailBraidOak
			| Self::ParkEdgeBraidOak => GoettingenFollowItem::BraidOak(&FOLLOW_BRAID_OAK),
			Self::TallFollowBraidOak => GoettingenFollowItem::BraidOak(&TALL_FOLLOW_BRAID_OAK),
			Self::OldGrowthFollowBraidOak => {
				GoettingenFollowItem::BraidOak(&OLD_GROWTH_FOLLOW_BRAID_OAK)
			}
			Self::FollowStorybook => GoettingenFollowItem::Storybook(&FOLLOW_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FollowBraidOak => FOLLOW_BRAID_OAK_STICK_MIX,
			Self::RedBranchBraidOak => RED_BRANCH_BRAID_OAK_STICK_MIX,
			Self::MossyTrailBraidOak => MOSSY_TRAIL_BRAID_OAK_STICK_MIX,
			Self::ParkEdgeBraidOak => PARK_EDGE_BRAID_OAK_STICK_MIX,
			Self::TallFollowBraidOak => TALL_FOLLOW_BRAID_OAK_STICK_MIX,
			Self::OldGrowthFollowBraidOak => OLD_GROWTH_BRAID_OAK_STICK_MIX,
			Self::FollowStorybook => FOLLOW_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FollowBraidOak => FOLLOW_BRAID_OAK_CANOPY_MIX,
			Self::RedBranchBraidOak => RED_BRANCH_BRAID_OAK_CANOPY_MIX,
			Self::MossyTrailBraidOak => MOSSY_TRAIL_BRAID_OAK_CANOPY_MIX,
			Self::ParkEdgeBraidOak => PARK_EDGE_BRAID_OAK_CANOPY_MIX,
			Self::TallFollowBraidOak => TALL_FOLLOW_BRAID_OAK_CANOPY_MIX,
			Self::OldGrowthFollowBraidOak => OLD_GROWTH_BRAID_OAK_CANOPY_MIX,
			Self::FollowStorybook => FOLLOW_STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const GOETTINGEN_FOLLOW_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const GOETTINGEN_FOLLOW_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const GOETTINGEN_FOLLOW_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	GOETTINGEN_FOLLOW_STRUCTURAL_HIGH_FACTOR,
	GOETTINGEN_FOLLOW_STRUCTURAL_MEDIUM_FACTOR,
	GOETTINGEN_FOLLOW_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{GoettingenFollow, GoettingenFollowParams, GoettingenFollowPlant};

#[cfg(test)]
mod tests;
