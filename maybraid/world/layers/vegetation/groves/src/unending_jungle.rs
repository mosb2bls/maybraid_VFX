//! Unending Jungle — well-known moderate lower-canopy grove
//! ([RFC-183 §3.4.6.1], [#322](https://github.com/ramate-io/maybraid/issues/322)).
//!
//! Mixes mini banyans, Storybook and Jungle Storybook forms, and rare torch, Rory, and palm accents
//! beneath taller forest layers.

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

/// Authored Unending Jungle grove definition.
///
/// Cell footprint sits at the RFC midpoint (`10.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<UnendingJungleCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(9.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-10.5, 10.5),
		),
		distribution: UnendingJungleCell::distribution(),
	}
}

/// Ordered unending-jungle varietals ([RFC-183 §3.4.6.1]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnendingJungleCell {
	SmallHonuBanyan,
	SmallSopeBanyan,
	LowerStorybook,
	SmallJungleStorybook,
	PenmarchAccent,
	RedJungleTorch,
	RoryAccent,
	WaialeaPalmAccent,
}

/// Typed authored geometry for one unending-jungle varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnendingJungleItem {
	Honu(&'static UnendingJungleBanyan),
	Sope(&'static UnendingJungleBanyan),
	Storybook(&'static UnendingJungleStorybook),
	JungleStorybook(&'static UnendingJungleJungleStorybook),
	Torch(&'static UnendingJungleTorch),
	RoryHead(&'static UnendingJungleRoryHead),
	WaialeaPalm(&'static UnendingJungleWaialeaPalm),
}

/// Authored geometry ranges for one mini Honu or Sope banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled descender probability band; lower values keep descenders sparse.
	pub descender_density: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one lower Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one small Jungle Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleJungleStorybook {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
	pub jungle_growth_density: UnitRange,
}

/// Authored geometry ranges for one Penmarch Torch accent (standard or red-stick palette).
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Rory's Head-trained accent.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleRoryHead {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Waialea Palm accent.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const SMALL_HONU_BANYAN: UnendingJungleBanyan = UnendingJungleBanyan {
	height: UnitRange::new(4.0, 6.0),
	stalk_radius: UnitRange::new(0.24, 0.36),
	canopy_spread: UnitRange::new(2.0, 4.5),
	descender_density: UnitRange::new(0.02, 0.04),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SMALL_SOPE_BANYAN: UnendingJungleBanyan = UnendingJungleBanyan {
	height: UnitRange::new(4.0, 6.0),
	stalk_radius: UnitRange::new(0.24, 0.36),
	canopy_spread: UnitRange::new(2.0, 4.5),
	descender_density: UnitRange::new(0.02, 0.04),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const LOWER_STORYBOOK: UnendingJungleStorybook = UnendingJungleStorybook {
	height: UnitRange::new(3.0, 5.0),
	stalk_radius: UnitRange::new(0.18, 0.28),
	canopy_spread: UnitRange::new(1.5, 3.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SMALL_JUNGLE_STORYBOOK: UnendingJungleJungleStorybook = UnendingJungleJungleStorybook {
	height: UnitRange::new(6.0, 8.0),
	canopy_density: DENSE_CANOPY_DENSITY,
	jungle_growth_density: MODERATE_CANOPY_DENSITY,
};

const PENUMARCH_ACCENT: UnendingJungleTorch = UnendingJungleTorch {
	height: UnitRange::new(3.0, 5.0),
	stalk_radius: UnitRange::new(0.12, 0.20),
	canopy_spread: UnitRange::new(1.2, 3.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const RED_JUNGLE_TORCH: UnendingJungleTorch = UnendingJungleTorch {
	height: UnitRange::new(3.0, 5.5),
	stalk_radius: UnitRange::new(0.12, 0.22),
	canopy_spread: UnitRange::new(1.2, 3.2),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const RORY_ACCENT: UnendingJungleRoryHead = UnendingJungleRoryHead {
	height: UnitRange::new(3.0, 7.0),
	stalk_radius: UnitRange::new(0.12, 0.28),
	canopy_spread: UnitRange::new(1.0, 2.8),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WAIALEA_PALM_ACCENT: UnendingJungleWaialeaPalm = UnendingJungleWaialeaPalm {
	height: UnitRange::new(6.0, 9.0),
	crown_density: MODERATE_CANOPY_DENSITY,
};

const HONU_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const HONU_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const SOPE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const SOPE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const JUNGLE_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_jungle_bark", "wet_brown"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const JUNGLE_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const PENUMARCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const PENUMARCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "lime_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const RED_JUNGLE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_jungle_bark", "copper_red"),
	PaletteSlot::new("wet_burgundy", "dark_bark"),
]);

const RED_JUNGLE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "lime_green"),
	PaletteSlot::new("blue_green", "fresh_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("vine_bark", "wet_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "deep_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

impl UnendingJungleCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `7.40` (RFC relative proportions); the `None` weight of `12.0` puts
	/// the placed share at `7.40 / 19.40 ≈ 0.38`, mid RFC `DENSITY_RANGE` (`0.24..0.52`).
	pub fn distribution() -> GroveDistribution<Self> {
		let honu = PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.42));
		let sope = PlacementConstraints::new(UnitRange::new(0.0, 0.52), UnitRange::new(0.0, 0.48));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.58), UnitRange::new(0.0, 0.64));
		let jungle_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.58));
		let penmarch =
			PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.64));
		let red_torch =
			PlacementConstraints::new(UnitRange::new(0.0, 0.48), UnitRange::new(0.0, 0.58));
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.76));
		let waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(8.0),
			GroveBucket::placed(2.0, honu, Self::SmallHonuBanyan),
			GroveBucket::placed(1.0, sope, Self::SmallSopeBanyan),
			GroveBucket::placed(2.0, storybook, Self::LowerStorybook),
			GroveBucket::placed(1.25, jungle_storybook, Self::SmallJungleStorybook),
			GroveBucket::placed(0.35, penmarch, Self::PenmarchAccent),
			GroveBucket::placed(0.20, red_torch, Self::RedJungleTorch),
			GroveBucket::placed(0.35, rory, Self::RoryAccent),
			GroveBucket::placed(0.55, waialea, Self::WaialeaPalmAccent),
		])
	}

	pub fn item(self) -> UnendingJungleItem {
		match self {
			Self::SmallHonuBanyan => UnendingJungleItem::Honu(&SMALL_HONU_BANYAN),
			Self::SmallSopeBanyan => UnendingJungleItem::Sope(&SMALL_SOPE_BANYAN),
			Self::LowerStorybook => UnendingJungleItem::Storybook(&LOWER_STORYBOOK),
			Self::SmallJungleStorybook => {
				UnendingJungleItem::JungleStorybook(&SMALL_JUNGLE_STORYBOOK)
			}
			Self::PenmarchAccent => UnendingJungleItem::Torch(&PENUMARCH_ACCENT),
			Self::RedJungleTorch => UnendingJungleItem::Torch(&RED_JUNGLE_TORCH),
			Self::RoryAccent => UnendingJungleItem::RoryHead(&RORY_ACCENT),
			Self::WaialeaPalmAccent => UnendingJungleItem::WaialeaPalm(&WAIALEA_PALM_ACCENT),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallHonuBanyan => HONU_STICK_MIX,
			Self::SmallSopeBanyan => SOPE_STICK_MIX,
			Self::LowerStorybook => STORYBOOK_STICK_MIX,
			Self::SmallJungleStorybook => JUNGLE_STORYBOOK_STICK_MIX,
			Self::PenmarchAccent => PENUMARCH_STICK_MIX,
			Self::RedJungleTorch => RED_JUNGLE_STICK_MIX,
			Self::RoryAccent => RORY_STICK_MIX,
			Self::WaialeaPalmAccent => WAIALEA_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallHonuBanyan => HONU_CANOPY_MIX,
			Self::SmallSopeBanyan => SOPE_CANOPY_MIX,
			Self::LowerStorybook => STORYBOOK_CANOPY_MIX,
			Self::SmallJungleStorybook => JUNGLE_STORYBOOK_CANOPY_MIX,
			Self::PenmarchAccent => PENUMARCH_CANOPY_MIX,
			Self::RedJungleTorch => RED_JUNGLE_CANOPY_MIX,
			Self::RoryAccent => RORY_CANOPY_MIX,
			Self::WaialeaPalmAccent => WAIALEA_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const UNENDING_JUNGLE_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const UNENDING_JUNGLE_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const UNENDING_JUNGLE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
	UNENDING_JUNGLE_STRUCTURAL_HIGH_FACTOR,
	UNENDING_JUNGLE_STRUCTURAL_MEDIUM_FACTOR,
	UNENDING_JUNGLE_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{UnendingJungle, UnendingJungleParams, UnendingJunglePlant};

#[cfg(test)]
mod tests;
