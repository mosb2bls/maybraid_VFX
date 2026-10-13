//! Orchard — high-density cultivated Storybook Tree upper-canopy grove
//! ([RFC-183 §3.4.7.7], [#353](https://github.com/ramate-io/maybraid/issues/353)).
//!
//! Compact fruiting and pale-bloom storybook forms on low-slope terrain with tight cell offset.
//!
//! Under `render`, High/Medium pose one Storybook kit group per plant under the
//! grove host (no per-tree [`lod::LodSceneHost`]). Plants unitize through
//! [`StorybookTree::unit_from_num`](sbs_trees::StorybookTree::unit_from_num)
//! (`tree_variants`, default `100`) so merged stick/ball collections share archetypal
//! meshes. Low ≈ one canopy ball per tree; UltraLow bins those sites at
//! [`ULTRA_LOW_CANOPY_BIN_METERS`].

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

/// Authored Orchard grove definition.
///
/// Cell footprint sits at the RFC midpoint (`11.0` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter so the grove reads as regular tended rows.
pub fn definition() -> GroveDefinition<OrchardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(11.0),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: OrchardCell::distribution(),
	}
}

/// Ordered orchard varietals ([RFC-183 §3.4.7.7]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchardCell {
	FruitingStorybook,
	PaleBloomStorybook,
}

/// Typed authored geometry for one orchard varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrchardItem {
	Storybook(&'static OrchardStorybook),
}

/// Authored geometry ranges for one cultivated Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct OrchardStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const FRUITING_STORYBOOK: OrchardStorybook = OrchardStorybook {
	height: UnitRange::new(5.0, 10.0),
	stalk_radius: UnitRange::new(0.22, 0.44),
	canopy_spread: UnitRange::new(1.8, 4.2),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PALE_BLOOM_STORYBOOK: OrchardStorybook = OrchardStorybook {
	height: UnitRange::new(5.0, 9.0),
	stalk_radius: UnitRange::new(0.20, 0.38),
	canopy_spread: UnitRange::new(1.6, 3.8),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const FRUITING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("orchard_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const FRUITING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const PALE_BLOOM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("orchard_bark", "gray_brown"),
	PaletteSlot::new("tan_bark", "brown_bark"),
]);

const PALE_BLOOM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("pale_blossom", "fresh_green"),
	PaletteSlot::new("light_green", "yellow_green"),
]);

/// Explicit `None` weight paired with placed weights so ~`95%` of cells receive a tree.
const CULTIVATED_EMPTY_WEIGHT: f32 = 2.25 / 19.0;

impl OrchardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.25`; the `None` weight of `2.25 / 19` yields a `~0.95` placed share
	/// for regular tended-row planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let fruiting =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.30));
		let pale_bloom =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.28));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(1.5, fruiting, Self::FruitingStorybook),
			GroveBucket::placed(0.75, pale_bloom, Self::PaleBloomStorybook),
		])
	}

	pub fn item(self) -> OrchardItem {
		match self {
			Self::FruitingStorybook => OrchardItem::Storybook(&FRUITING_STORYBOOK),
			Self::PaleBloomStorybook => OrchardItem::Storybook(&PALE_BLOOM_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FruitingStorybook => FRUITING_STICK_MIX,
			Self::PaleBloomStorybook => PALE_BLOOM_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FruitingStorybook => FRUITING_CANOPY_MIX,
			Self::PaleBloomStorybook => PALE_BLOOM_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const ORCHARD_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const ORCHARD_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const ORCHARD_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	ORCHARD_STRUCTURAL_HIGH_FACTOR,
	ORCHARD_STRUCTURAL_MEDIUM_FACTOR,
	ORCHARD_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{Orchard, OrchardParams, OrchardPlant};

#[cfg(test)]
mod tests;
