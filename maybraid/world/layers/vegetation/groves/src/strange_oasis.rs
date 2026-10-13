//! Strange Oasis — well-known sparse oasis lower-canopy grove
//! ([RFC-183 §3.4.6.2], [#323](https://github.com/ramate-io/maybraid/issues/323)).
//!
//! Compact date palms with rare Penmarch torch and Storybook accents in wet desert pockets.
//!

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
/// Sparse..moderate sampled canopy-density band.
const SPARSE_TO_MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.65);

/// Authored Strange Oasis grove definition.
///
/// Cell footprint sits at the RFC midpoint (`12.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<StrangeOasisCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(8.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-12.0, 12.0),
		),
		distribution: StrangeOasisCell::distribution(),
	}
}

/// Ordered strange-oasis varietals ([RFC-183 §3.4.6.2]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrangeOasisCell {
	CompactDatePalm,
	TorchAccent,
	RedTorchAccent,
	OasisStorybook,
}

/// Typed authored geometry for one strange-oasis varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrangeOasisItem {
	DatePalm(&'static StrangeOasisDatePalm),
	Torch(&'static StrangeOasisTorch),
	Storybook(&'static StrangeOasisStorybook),
}

/// Authored geometry ranges for one compact Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct StrangeOasisDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one Penmarch Torch accent (standard or red-stick palette).
#[derive(Debug, Clone, PartialEq)]
pub struct StrangeOasisTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one oasis Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct StrangeOasisStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const COMPACT_DATE_PALM: StrangeOasisDatePalm = StrangeOasisDatePalm {
	height: UnitRange::new(3.0, 5.0),
	crown_density: MODERATE_CANOPY_DENSITY,
};

const TORCH_ACCENT: StrangeOasisTorch = StrangeOasisTorch {
	height: UnitRange::new(3.0, 7.0),
	stalk_radius: UnitRange::new(0.12, 0.24),
	canopy_spread: UnitRange::new(1.2, 3.5),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const RED_TORCH_ACCENT: StrangeOasisTorch = StrangeOasisTorch {
	height: UnitRange::new(3.0, 6.5),
	stalk_radius: UnitRange::new(0.12, 0.22),
	canopy_spread: UnitRange::new(1.2, 3.2),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const OASIS_STORYBOOK: StrangeOasisStorybook = StrangeOasisStorybook {
	height: UnitRange::new(4.0, 6.0),
	stalk_radius: UnitRange::new(0.20, 0.32),
	canopy_spread: UnitRange::new(1.6, 3.6),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const DATE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("dry_brown", "gray_brown"),
]);

const DATE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "date_green"),
]);

const TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "ornamental_bark"),
	PaletteSlot::new("gray_brown", "tan_brown"),
]);

const TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "olive_green"),
	PaletteSlot::new("flower_yellow", "fresh_green"),
]);

const RED_TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("desert_red_bark", "copper_red"),
	PaletteSlot::new("orange_bark", "dark_bark"),
]);

const RED_TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "fresh_green"),
	PaletteSlot::new("flower_yellow", "light_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "tan_brown"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("green", "light_green"),
	PaletteSlot::new("olive_green", "fresh_green"),
]);

impl StrangeOasisCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.23` (RFC relative proportions); the `None` weight of `14.0` puts
	/// the placed share at `3.23 / 17.23 ≈ 0.19`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let date_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.38), UnitRange::new(0.0, 0.28));
		let torch = PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.34));
		let red_torch =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.40));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.42), UnitRange::new(0.0, 0.32));
		GroveDistribution::new(vec![
			GroveBucket::none(10.0),
			GroveBucket::placed(2.0, date_palm, Self::CompactDatePalm),
			GroveBucket::placed(0.30, torch, Self::TorchAccent),
			GroveBucket::placed(0.18, red_torch, Self::RedTorchAccent),
			GroveBucket::placed(0.75, storybook, Self::OasisStorybook),
		])
	}

	pub fn item(self) -> StrangeOasisItem {
		match self {
			Self::CompactDatePalm => StrangeOasisItem::DatePalm(&COMPACT_DATE_PALM),
			Self::TorchAccent => StrangeOasisItem::Torch(&TORCH_ACCENT),
			Self::RedTorchAccent => StrangeOasisItem::Torch(&RED_TORCH_ACCENT),
			Self::OasisStorybook => StrangeOasisItem::Storybook(&OASIS_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::CompactDatePalm => DATE_PALM_STICK_MIX,
			Self::TorchAccent => TORCH_STICK_MIX,
			Self::RedTorchAccent => RED_TORCH_STICK_MIX,
			Self::OasisStorybook => STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::CompactDatePalm => DATE_PALM_CANOPY_MIX,
			Self::TorchAccent => TORCH_CANOPY_MIX,
			Self::RedTorchAccent => RED_TORCH_CANOPY_MIX,
			Self::OasisStorybook => STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const STRANGE_OASIS_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const STRANGE_OASIS_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const STRANGE_OASIS_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	STRANGE_OASIS_STRUCTURAL_HIGH_FACTOR,
	STRANGE_OASIS_STRUCTURAL_MEDIUM_FACTOR,
	STRANGE_OASIS_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{OasisDatePalm, StrangeOasis, StrangeOasisParams, StrangeOasisPlant};

#[cfg(test)]
mod tests;
