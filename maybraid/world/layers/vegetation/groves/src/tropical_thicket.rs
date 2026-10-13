//! Tropical Thicket — well-known dense tropical understory grove
//! ([RFC-183 §3.4.5.6], [#317](https://github.com/ramate-io/maybraid/issues/317)).
//!
//! Mixes larger palm bushes, moderate Common High Bush forms, and rare mini Honu Banyan accents.
//!

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// RFC `projection_count: Moderate` with extended upper tails for occasional wide-span shrubs.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.56);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.50, 0.82);
/// Stick segment reach as a fraction of shoot height; upper tail exceeds the generic bush default.
const MODERATE_SEGMENT_LENGTH: UnitRange = UnitRange::new(0.08, 0.24);
const FLOWERING_SEGMENT_LENGTH: UnitRange = UnitRange::new(0.08, 0.22);

/// Authored Tropical Thicket grove definition.
///
/// Cell footprint sits at the RFC midpoint (`6.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TropicalThicketCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(6.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-6.5, 6.5)),
		distribution: TropicalThicketCell::distribution(),
	}
}

/// Ordered tropical-thicket varietals ([RFC-183 §3.4.5.6]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalThicketCell {
	LargePalmBush,
	BroadWetPalmBush,
	MiniHonuBanyan,
	ModerateHighBush,
	FloweringHighBush,
	RedStemPalmBush,
}

/// Typed authored geometry for one tropical-thicket varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalThicketItem {
	Palm(&'static TropicalThicketPalm),
	Banyan(&'static TropicalThicketBanyan),
	Bush(&'static TropicalThicketBush),
}

/// Authored geometry ranges for one ground-anchored palm bush.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalThicketPalm {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

/// Authored geometry ranges for one mini Honu Banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalThicketBanyan {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC mini form `0.2` m at mid height).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled descender probability band; lower values keep descenders sparse.
	pub descender_density: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalThicketBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Per-segment stick length sampled as a fraction of shoot height.
	pub segment_length_fraction: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

const LARGE_PALM_BUSH: TropicalThicketPalm = TropicalThicketPalm {
	height: UnitRange::new(3.00, 6.60),
	frond_count: 7..=12,
	frond_length: UnitRange::new(1.65, 4.50),
	crown_spread: UnitRange::new(2.40, 6.30),
};

const BROAD_WET_PALM_BUSH: TropicalThicketPalm = TropicalThicketPalm {
	height: UnitRange::new(3.60, 7.80),
	frond_count: 8..=14,
	frond_length: UnitRange::new(2.10, 5.25),
	crown_spread: UnitRange::new(3.00, 7.80),
};

const RED_STEM_PALM_BUSH: TropicalThicketPalm = TropicalThicketPalm {
	height: UnitRange::new(3.00, 6.90),
	frond_count: 6..=11,
	frond_length: UnitRange::new(1.65, 4.35),
	crown_spread: UnitRange::new(2.40, 6.30),
};

const MINI_HONU_BANYAN: TropicalThicketBanyan = TropicalThicketBanyan {
	height: UnitRange::new(1.80, 3.80),
	stalk_radius: UnitRange::new(0.14, 0.30),
	canopy_spread: UnitRange::new(1.20, 3.40),
	descender_density: UnitRange::new(0.02, 0.04),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const MODERATE_HIGH_BUSH: TropicalThicketBush = TropicalThicketBush {
	height: UnitRange::new(1.20, 2.40),
	shoot_count: 7..=11,
	branch_depth: 2..=5,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	segment_length_fraction: MODERATE_SEGMENT_LENGTH,
	leaf_radius: UnitRange::new(0.06, 0.15),
};

const FLOWERING_HIGH_BUSH: TropicalThicketBush = TropicalThicketBush {
	height: UnitRange::new(1.00, 2.20),
	shoot_count: 7..=10,
	branch_depth: 2..=5,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	segment_length_fraction: FLOWERING_SEGMENT_LENGTH,
	leaf_radius: UnitRange::new(0.06, 0.14),
};

const LARGE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("green_stem", "wet_brown"),
]);

const LARGE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const BROAD_WET_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const BROAD_WET_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "deep_green"),
	PaletteSlot::new("emerald_green", "wet_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

const RED_STEM_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_palm_stem", "copper_red"),
	PaletteSlot::new("wet_burgundy", "dark_bark"),
]);

const RED_STEM_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "bright_green"),
	PaletteSlot::new("lime_green", "fresh_green"),
	PaletteSlot::new("blue_green", "wet_green"),
]);

const HONU_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const HONU_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("wet_green", "blue_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);

const MODERATE_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "green_brown"),
	PaletteSlot::new("dark_bark", "wet_brown"),
]);

const MODERATE_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("blue_green", "light_green"),
]);

const FLOWERING_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const FLOWERING_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "leaf_green"),
	PaletteSlot::new("flower_white", "fresh_green"),
	PaletteSlot::new("flower_yellow", "lime_green"),
]);

impl TropicalThicketCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.25` (RFC relative proportions); the `None` weight of `7.0` puts
	/// the placed share at `5.25 / 12.25 ≈ 0.43`, mid RFC `DENSITY_RANGE` (`0.24..0.62`).
	pub fn distribution() -> GroveDistribution<Self> {
		let gentle =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.28));
		let wet_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.68));
		let flowering =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.78));
		let red_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.42), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(7.0),
			GroveBucket::placed(2.0, gentle, Self::LargePalmBush),
			GroveBucket::placed(1.25, wet_palm, Self::BroadWetPalmBush),
			GroveBucket::placed(0.45, gentle, Self::MiniHonuBanyan),
			GroveBucket::placed(1.0, gentle, Self::ModerateHighBush),
			GroveBucket::placed(0.30, flowering, Self::FloweringHighBush),
			GroveBucket::placed(0.25, red_palm, Self::RedStemPalmBush),
		])
	}

	pub fn item(self) -> TropicalThicketItem {
		match self {
			Self::LargePalmBush => TropicalThicketItem::Palm(&LARGE_PALM_BUSH),
			Self::BroadWetPalmBush => TropicalThicketItem::Palm(&BROAD_WET_PALM_BUSH),
			Self::MiniHonuBanyan => TropicalThicketItem::Banyan(&MINI_HONU_BANYAN),
			Self::ModerateHighBush => TropicalThicketItem::Bush(&MODERATE_HIGH_BUSH),
			Self::FloweringHighBush => TropicalThicketItem::Bush(&FLOWERING_HIGH_BUSH),
			Self::RedStemPalmBush => TropicalThicketItem::Palm(&RED_STEM_PALM_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::LargePalmBush => LARGE_PALM_STICK_MIX,
			Self::BroadWetPalmBush => BROAD_WET_PALM_STICK_MIX,
			Self::RedStemPalmBush => RED_STEM_PALM_STICK_MIX,
			Self::MiniHonuBanyan => HONU_STICK_MIX,
			Self::ModerateHighBush => MODERATE_BUSH_STICK_MIX,
			Self::FloweringHighBush => FLOWERING_BUSH_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::LargePalmBush => LARGE_PALM_CANOPY_MIX,
			Self::BroadWetPalmBush => BROAD_WET_PALM_CANOPY_MIX,
			Self::RedStemPalmBush => RED_STEM_PALM_CANOPY_MIX,
			Self::MiniHonuBanyan => HONU_CANOPY_MIX,
			Self::ModerateHighBush => MODERATE_BUSH_CANOPY_MIX,
			Self::FloweringHighBush => FLOWERING_BUSH_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const TROPICAL_THICKET_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const TROPICAL_THICKET_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const TROPICAL_THICKET_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	TROPICAL_THICKET_STRUCTURAL_HIGH_FACTOR,
	TROPICAL_THICKET_STRUCTURAL_MEDIUM_FACTOR,
	TROPICAL_THICKET_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{TropicalThicket, TropicalThicketParams, TropicalThicketPlant};

#[cfg(test)]
mod tests;
