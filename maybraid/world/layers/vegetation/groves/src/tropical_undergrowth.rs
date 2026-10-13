//! Tropical Undergrowth — well-known moderate-to-dense hybrid understory grove
//! ([RFC-183 §3.4.5.5], [#315](https://github.com/ramate-io/maybraid/issues/315)).
//!
//! Mixes bright/deep tufts (mostly as patches), small palm bushes, and rare mini SBS-tree forms.
//!

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Authored Tropical Undergrowth grove definition.
///
/// Cell footprint sits at the RFC midpoint (`5.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TropicalUndergrowthCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(5.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-5.0, 5.0)),
		distribution: TropicalUndergrowthCell::distribution(),
	}
}

/// Ordered tropical-undergrowth varietals ([RFC-183 §3.4.5.5]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalUndergrowthCell {
	BrightTuft,
	DeepTuft,
	SmallPalmBush,
	MiniRoryHeadTrained,
	MiniVaseTree,
	MiniSparseStorybook,
	MiniPenmarchTorch,
	MiniKamakuraTorch,
	MiniTorchTree,
	BrightTuftPatch,
	DeepTuftPatch,
}

/// Typed authored geometry for one tropical-undergrowth varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalUndergrowthItem {
	Tuft(&'static TropicalUndergrowthTuft),
	Patch(&'static GroveTuftPatch<TropicalUndergrowthTuft>),
	PalmBush(&'static TropicalUndergrowthPalm),
	RoryHead(&'static TropicalUndergrowthRoryHead),
	VaseTree(&'static TropicalUndergrowthVaseTree),
	Storybook(&'static TropicalUndergrowthStorybook),
	PenmarchTorch(&'static TropicalUndergrowthTorch),
	KamakuraTorch(&'static TropicalUndergrowthTorch),
	TorchTree(&'static TropicalUndergrowthTorch),
}

/// Authored geometry ranges for one tropical-undergrowth tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthTuft {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Authored geometry ranges for one ground-anchored palm bush companion.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthPalm {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

/// Authored geometry ranges for one mini Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthRoryHead {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one mini Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Authored geometry ranges for one mini torch form (Penmarch, Kamakura, or generic torch tree).
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Authored geometry ranges for one mini Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

const BLADE_COUNT: RangeInclusive<u32> = 6..=12;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=6;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.35);

const BRIGHT_TUFT: TropicalUndergrowthTuft = TropicalUndergrowthTuft {
	height: UnitRange::new(0.30, 1.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const DEEP_TUFT: TropicalUndergrowthTuft = TropicalUndergrowthTuft {
	height: UnitRange::new(0.40, 0.90),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const BRIGHT_TUFT_PATCH: GroveTuftPatch<TropicalUndergrowthTuft> = GroveTuftPatch {
	clump: BRIGHT_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const DEEP_TUFT_PATCH: GroveTuftPatch<TropicalUndergrowthTuft> = GroveTuftPatch {
	clump: DEEP_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const SMALL_PALM_BUSH: TropicalUndergrowthPalm = TropicalUndergrowthPalm {
	height: UnitRange::new(1.00, 2.80),
	frond_count: 5..=9,
	frond_length: UnitRange::new(0.50, 1.40),
	crown_spread: UnitRange::new(0.70, 1.80),
};

const MINI_RORY_HEAD: TropicalUndergrowthRoryHead = TropicalUndergrowthRoryHead {
	height: UnitRange::new(0.80, 1.80),
	stalk_radius: UnitRange::new(0.037, 0.055),
	canopy_spread: UnitRange::new(0.70, 1.68),
	canopy_density: UnitRange::new(0.0, 1.0),
};

const MINI_VASE_TREE: TropicalUndergrowthVaseTree = TropicalUndergrowthVaseTree {
	height: UnitRange::new(1.00, 2.30),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.98, 2.10),
};

const MINI_STORYBOOK: TropicalUndergrowthStorybook = TropicalUndergrowthStorybook {
	height: UnitRange::new(1.20, 2.50),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.84, 1.96),
};

const MINI_TORCH_TREE: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.00, 2.20),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.77, 1.68),
};

const MINI_PENMARCH_TORCH: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.20, 2.50),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.91, 1.96),
};

const MINI_KAMAKURA_TORCH: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.00, 2.30),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.84, 1.82),
};

const BRIGHT_TUFT_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("bright_green", "lime_green"),
	PaletteSlot::new("lush_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

const DEEP_TUFT_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "emerald_green"),
	PaletteSlot::new("dark_green", "wet_green"),
	PaletteSlot::new("blue_green", "bright_green"),
]);

const PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("green_stem", "wet_brown"),
]);

const PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("blue_green", "wet_green"),
	PaletteSlot::new("yellow_green", "lime_green"),
]);

const VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "tropical_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("dark_green", "emerald_green"),
	PaletteSlot::new("flower_white", "fresh_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "light_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("blue_green", "yellow_green"),
]);

const TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("yellow_green", "warm_yellow"),
	PaletteSlot::new("lime_green", "fresh_green"),
]);

impl TropicalUndergrowthCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `8.44` (RFC relative proportions plus torch companions); the
	/// `None` weight of `12.0` puts the placed share at `8.44 / 20.44 ≈ 0.41`, mid RFC
	/// `DENSITY_RANGE` (`0.22..0.58`).
	pub fn distribution() -> GroveDistribution<Self> {
		let lowland =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.70));
		let palm = PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.60));
		let mini_tree =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.50));
		GroveDistribution::new(vec![
			GroveBucket::none(12.0),
			GroveBucket::placed(0.4, lowland, Self::BrightTuft),
			GroveBucket::placed(0.3, lowland, Self::DeepTuft),
			GroveBucket::placed(1.0, palm, Self::SmallPalmBush),
			GroveBucket::placed(0.85, lowland, Self::MiniRoryHeadTrained),
			GroveBucket::placed(0.20, mini_tree, Self::MiniVaseTree),
			GroveBucket::placed(0.15, mini_tree, Self::MiniSparseStorybook),
			GroveBucket::placed(1.30, mini_tree, Self::MiniPenmarchTorch),
			GroveBucket::placed(1.22, mini_tree, Self::MiniKamakuraTorch),
			GroveBucket::placed(0.22, mini_tree, Self::MiniTorchTree),
			GroveBucket::placed(1.6, lowland, Self::BrightTuftPatch),
			GroveBucket::placed(1.2, lowland, Self::DeepTuftPatch),
		])
	}

	pub fn item(self) -> TropicalUndergrowthItem {
		match self {
			Self::BrightTuft => TropicalUndergrowthItem::Tuft(&BRIGHT_TUFT),
			Self::DeepTuft => TropicalUndergrowthItem::Tuft(&DEEP_TUFT),
			Self::SmallPalmBush => TropicalUndergrowthItem::PalmBush(&SMALL_PALM_BUSH),
			Self::MiniRoryHeadTrained => TropicalUndergrowthItem::RoryHead(&MINI_RORY_HEAD),
			Self::MiniVaseTree => TropicalUndergrowthItem::VaseTree(&MINI_VASE_TREE),
			Self::MiniSparseStorybook => TropicalUndergrowthItem::Storybook(&MINI_STORYBOOK),
			Self::MiniPenmarchTorch => TropicalUndergrowthItem::PenmarchTorch(&MINI_PENMARCH_TORCH),
			Self::MiniKamakuraTorch => TropicalUndergrowthItem::KamakuraTorch(&MINI_KAMAKURA_TORCH),
			Self::MiniTorchTree => TropicalUndergrowthItem::TorchTree(&MINI_TORCH_TREE),
			Self::BrightTuftPatch => TropicalUndergrowthItem::Patch(&BRIGHT_TUFT_PATCH),
			Self::DeepTuftPatch => TropicalUndergrowthItem::Patch(&DEEP_TUFT_PATCH),
		}
	}

	pub fn palette_mix(self) -> PaletteMix {
		match self {
			Self::BrightTuft | Self::BrightTuftPatch => BRIGHT_TUFT_MIX,
			Self::DeepTuft | Self::DeepTuftPatch => DEEP_TUFT_MIX,
			Self::SmallPalmBush => PALM_CANOPY_MIX,
			Self::MiniRoryHeadTrained => RORY_CANOPY_MIX,
			Self::MiniVaseTree => VASE_CANOPY_MIX,
			Self::MiniSparseStorybook => STORYBOOK_CANOPY_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_CANOPY_MIX
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallPalmBush => PALM_STICK_MIX,
			Self::MiniRoryHeadTrained => RORY_STICK_MIX,
			Self::MiniVaseTree => VASE_STICK_MIX,
			Self::MiniSparseStorybook => STORYBOOK_STICK_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_STICK_MIX
			}
			_ => PALM_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallPalmBush => PALM_CANOPY_MIX,
			Self::MiniRoryHeadTrained => RORY_CANOPY_MIX,
			Self::MiniVaseTree => VASE_CANOPY_MIX,
			Self::MiniSparseStorybook => STORYBOOK_CANOPY_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_CANOPY_MIX
			}
			_ => BRIGHT_TUFT_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
use crate::grove::vc_tuft::{
	TUFT_GROVE_STRUCTURAL_HIGH_FACTOR, TUFT_GROVE_STRUCTURAL_LOW_FACTOR,
	TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(feature = "render")]
pub const TROPICAL_UNDERGROWTH_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
#[cfg(feature = "render")]
pub const TROPICAL_UNDERGROWTH_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
#[cfg(feature = "render")]
pub const TROPICAL_UNDERGROWTH_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
	TROPICAL_UNDERGROWTH_STRUCTURAL_HIGH_FACTOR,
	TROPICAL_UNDERGROWTH_STRUCTURAL_MEDIUM_FACTOR,
	TROPICAL_UNDERGROWTH_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{TropicalUndergrowth, TropicalUndergrowthParams};

#[cfg(test)]
mod tests;
