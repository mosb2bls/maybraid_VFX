//! Storyteller's — colorful whimsical Storybook and Braid Oak upper-canopy grove
//! ([RFC-183 §3.4.7.14], [#336](https://github.com/ramate-io/maybraid/issues/336)).
//!
//! Moderate-density color-pop canopy with common storybook, braid-oak, and torch forms.

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

/// Authored Storyteller's grove definition.
///
/// Cell footprint sits at the RFC midpoint (`22.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<StorytellersCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(22.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-22.0, 22.0),
		),
		distribution: StorytellersCell::distribution(),
	}
}

/// Ordered storyteller varietals ([RFC-183 §3.4.7.14]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorytellersCell {
	ColorfulStorybook,
	ColorfulBraidOak,
	BrightCanopyStorybook,
	PinkLanternStorybook,
	RedFestivalBraidOak,
	PurpleCrownStorybook,
	BlueMoonStorybook,
	GoldenLanternPenmarch,
	BlueFlameKamakura,
	FestivalTorchTree,
	VioletCanopyBraidOak,
	GoldLeafBraidOak,
	CopperFlameBraidOak,
}

/// Typed authored geometry for one storyteller varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorytellersItem {
	Storybook(&'static StorytellersStorybook),
	BraidOak(&'static StorytellersBraidOak),
	PenmarchTorch(&'static StorytellersTorch),
	KamakuraTorch(&'static StorytellersTorch),
	TorchTree(&'static StorytellersTorch),
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersBraidOak {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one upper-canopy torch form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

const COLORFUL_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(10.0, 30.0),
	stalk_radius: UnitRange::new(0.24, 0.55),
	canopy_spread: UnitRange::new(3.0, 8.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const COLORFUL_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(10.0, 30.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const BRIGHT_CANOPY_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(10.0, 26.0),
	stalk_radius: UnitRange::new(0.22, 0.50),
	canopy_spread: UnitRange::new(2.5, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PINK_LANTERN_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(8.0, 18.0),
	stalk_radius: UnitRange::new(0.20, 0.44),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const RED_FESTIVAL_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(12.0, 24.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PURPLE_CROWN_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(14.0, 30.0),
	stalk_radius: UnitRange::new(0.28, 0.58),
	canopy_spread: UnitRange::new(3.5, 9.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const BLUE_MOON_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(12.0, 22.0),
	stalk_radius: UnitRange::new(0.22, 0.48),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const GOLDEN_LANTERN_PENMARCH: StorytellersTorch = StorytellersTorch {
	height: UnitRange::new(10.0, 26.0),
	stalk_radius: UnitRange::new(0.16, 0.38),
	canopy_spread: UnitRange::new(2.5, 7.0),
};

const BLUE_FLAME_KAMAKURA: StorytellersTorch = StorytellersTorch {
	height: UnitRange::new(12.0, 28.0),
	stalk_radius: UnitRange::new(0.14, 0.36),
	canopy_spread: UnitRange::new(2.0, 6.5),
};

const FESTIVAL_TORCH_TREE: StorytellersTorch = StorytellersTorch {
	height: UnitRange::new(10.0, 24.0),
	stalk_radius: UnitRange::new(0.16, 0.40),
	canopy_spread: UnitRange::new(2.5, 6.0),
};

const VIOLET_CANOPY_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(14.0, 28.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const GOLD_LEAF_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(10.0, 22.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const COPPER_FLAME_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(12.0, 26.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const COLORFUL_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "red_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const COLORFUL_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("storybook_green", "gold_green"),
	PaletteSlot::new("rose_leaf", "fresh_green"),
]);

const COLORFUL_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "copper_red"),
	PaletteSlot::new("oak_bark", "dark_bark"),
]);

const COLORFUL_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "gold_green"),
	PaletteSlot::new("copper_leaf", "fresh_green"),
]);

const BRIGHT_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "red_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BRIGHT_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gold_leaf", "fresh_green"),
	PaletteSlot::new("rose_leaf", "light_green"),
]);

const PINK_LANTERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "purple_brown"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const PINK_LANTERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("hot_pink", "rose_leaf"),
	PaletteSlot::new("fresh_green", "pink_bloom"),
]);

const RED_FESTIVAL_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "bright_red_bark"),
	PaletteSlot::new("copper_red", "dark_bark"),
]);

const RED_FESTIVAL_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_leaf", "copper_leaf"),
	PaletteSlot::new("gold_leaf", "fresh_green"),
]);

const PURPLE_CROWN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const PURPLE_CROWN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("violet_leaf", "purple_leaf"),
	PaletteSlot::new("deep_green", "rose_leaf"),
]);

const BLUE_MOON_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_gray_bark", "purple_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BLUE_MOON_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_leaf", "cyan_leaf"),
	PaletteSlot::new("deep_blue_green", "fresh_green"),
]);

const GOLDEN_LANTERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "gold"),
	PaletteSlot::new("ornamental_bark", "red_brown"),
]);

const GOLDEN_LANTERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gold_leaf", "flower_yellow"),
	PaletteSlot::new("warm_yellow", "fresh_green"),
]);

const BLUE_FLAME_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_gray_bark", "purple_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BLUE_FLAME_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_leaf", "cyan_leaf"),
	PaletteSlot::new("deep_blue_green", "violet_leaf"),
]);

const FESTIVAL_TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ritual_red_bark", "copper_red"),
	PaletteSlot::new("bright_red_bark", "dark_bark"),
]);

const FESTIVAL_TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("hot_pink", "gold_leaf"),
	PaletteSlot::new("flower_yellow", "rose_leaf"),
]);

const VIOLET_CANOPY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "blue_gray_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const VIOLET_CANOPY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("violet_leaf", "purple_leaf"),
	PaletteSlot::new("deep_green", "rose_leaf"),
]);

const GOLD_LEAF_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "warm_bark"),
	PaletteSlot::new("ornamental_bark", "gray_brown"),
]);

const GOLD_LEAF_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gold_leaf", "gold_green"),
	PaletteSlot::new("warm_yellow", "fresh_green"),
]);

const COPPER_FLAME_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "bright_red_bark"),
	PaletteSlot::new("red_oak_bark", "dark_bark"),
]);

const COPPER_FLAME_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_leaf", "red_leaf"),
	PaletteSlot::new("gold_leaf", "orange_brown"),
]);

impl StorytellersCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `6.28`; the `None` weight of `16.2` puts the placed share at
	/// `6.28 / 22.48 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let colorful_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let colorful_braid =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let bright_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.56));
		let pink_lantern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let red_festival =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.46));
		let purple_crown =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.54));
		let blue_moon =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let golden_lantern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let blue_flame =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.54));
		let festival_torch =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let violet_canopy =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let gold_leaf =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let copper_flame =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		GroveDistribution::new(vec![
			GroveBucket::none(16.2),
			GroveBucket::placed(1.5, colorful_storybook, Self::ColorfulStorybook),
			GroveBucket::placed(1.5, colorful_braid, Self::ColorfulBraidOak),
			GroveBucket::placed(0.75, bright_storybook, Self::BrightCanopyStorybook),
			GroveBucket::placed(0.35, pink_lantern, Self::PinkLanternStorybook),
			GroveBucket::placed(0.30, red_festival, Self::RedFestivalBraidOak),
			GroveBucket::placed(0.25, purple_crown, Self::PurpleCrownStorybook),
			GroveBucket::placed(0.25, blue_moon, Self::BlueMoonStorybook),
			GroveBucket::placed(0.22, golden_lantern, Self::GoldenLanternPenmarch),
			GroveBucket::placed(0.20, blue_flame, Self::BlueFlameKamakura),
			GroveBucket::placed(0.18, festival_torch, Self::FestivalTorchTree),
			GroveBucket::placed(0.28, violet_canopy, Self::VioletCanopyBraidOak),
			GroveBucket::placed(0.26, gold_leaf, Self::GoldLeafBraidOak),
			GroveBucket::placed(0.24, copper_flame, Self::CopperFlameBraidOak),
		])
	}

	pub fn item(self) -> StorytellersItem {
		match self {
			Self::ColorfulStorybook => StorytellersItem::Storybook(&COLORFUL_STORYBOOK),
			Self::ColorfulBraidOak => StorytellersItem::BraidOak(&COLORFUL_BRAID_OAK),
			Self::BrightCanopyStorybook => StorytellersItem::Storybook(&BRIGHT_CANOPY_STORYBOOK),
			Self::PinkLanternStorybook => StorytellersItem::Storybook(&PINK_LANTERN_STORYBOOK),
			Self::RedFestivalBraidOak => StorytellersItem::BraidOak(&RED_FESTIVAL_BRAID_OAK),
			Self::PurpleCrownStorybook => StorytellersItem::Storybook(&PURPLE_CROWN_STORYBOOK),
			Self::BlueMoonStorybook => StorytellersItem::Storybook(&BLUE_MOON_STORYBOOK),
			Self::GoldenLanternPenmarch => {
				StorytellersItem::PenmarchTorch(&GOLDEN_LANTERN_PENMARCH)
			}
			Self::BlueFlameKamakura => StorytellersItem::KamakuraTorch(&BLUE_FLAME_KAMAKURA),
			Self::FestivalTorchTree => StorytellersItem::TorchTree(&FESTIVAL_TORCH_TREE),
			Self::VioletCanopyBraidOak => StorytellersItem::BraidOak(&VIOLET_CANOPY_BRAID_OAK),
			Self::GoldLeafBraidOak => StorytellersItem::BraidOak(&GOLD_LEAF_BRAID_OAK),
			Self::CopperFlameBraidOak => StorytellersItem::BraidOak(&COPPER_FLAME_BRAID_OAK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ColorfulStorybook => COLORFUL_STORYBOOK_STICK_MIX,
			Self::ColorfulBraidOak => COLORFUL_BRAID_OAK_STICK_MIX,
			Self::BrightCanopyStorybook => BRIGHT_STORYBOOK_STICK_MIX,
			Self::PinkLanternStorybook => PINK_LANTERN_STICK_MIX,
			Self::RedFestivalBraidOak => RED_FESTIVAL_STICK_MIX,
			Self::PurpleCrownStorybook => PURPLE_CROWN_STICK_MIX,
			Self::BlueMoonStorybook => BLUE_MOON_STICK_MIX,
			Self::GoldenLanternPenmarch => GOLDEN_LANTERN_STICK_MIX,
			Self::BlueFlameKamakura => BLUE_FLAME_STICK_MIX,
			Self::FestivalTorchTree => FESTIVAL_TORCH_STICK_MIX,
			Self::VioletCanopyBraidOak => VIOLET_CANOPY_STICK_MIX,
			Self::GoldLeafBraidOak => GOLD_LEAF_STICK_MIX,
			Self::CopperFlameBraidOak => COPPER_FLAME_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ColorfulStorybook => COLORFUL_STORYBOOK_CANOPY_MIX,
			Self::ColorfulBraidOak => COLORFUL_BRAID_OAK_CANOPY_MIX,
			Self::BrightCanopyStorybook => BRIGHT_STORYBOOK_CANOPY_MIX,
			Self::PinkLanternStorybook => PINK_LANTERN_CANOPY_MIX,
			Self::RedFestivalBraidOak => RED_FESTIVAL_CANOPY_MIX,
			Self::PurpleCrownStorybook => PURPLE_CROWN_CANOPY_MIX,
			Self::BlueMoonStorybook => BLUE_MOON_CANOPY_MIX,
			Self::GoldenLanternPenmarch => GOLDEN_LANTERN_CANOPY_MIX,
			Self::BlueFlameKamakura => BLUE_FLAME_CANOPY_MIX,
			Self::FestivalTorchTree => FESTIVAL_TORCH_CANOPY_MIX,
			Self::VioletCanopyBraidOak => VIOLET_CANOPY_CANOPY_MIX,
			Self::GoldLeafBraidOak => GOLD_LEAF_CANOPY_MIX,
			Self::CopperFlameBraidOak => COPPER_FLAME_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const STORYTELLERS_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const STORYTELLERS_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const STORYTELLERS_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	STORYTELLERS_STRUCTURAL_HIGH_FACTOR,
	STORYTELLERS_STRUCTURAL_MEDIUM_FACTOR,
	STORYTELLERS_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{Storytellers, StorytellersParams, StorytellersPlant};

#[cfg(test)]
mod tests;
