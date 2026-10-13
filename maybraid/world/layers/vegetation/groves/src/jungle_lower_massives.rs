//! Jungle Lower Massives — massive lower-canopy grove beneath very tall upper canopy
//! ([RFC-183 §3.4.6.7], [#328](https://github.com/ramate-io/maybraid/issues/328)).
//!
//! Common 10–20 m jungle storybook and banyan forms with rare braid-oak accents.

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
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Jungle Lower Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`23` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JungleLowerMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(18.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-23.0, 23.0),
		),
		distribution: JungleLowerMassivesCell::distribution(),
	}
}

/// Ordered jungle lower-massive varietals ([RFC-183 §3.4.6.7]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JungleLowerMassivesCell {
	LowerMassiveJungleStorybook,
	LowerMassiveHonuBanyan,
	LowerMassiveSopesBanyan,
	LowerMassiveWaialeaPalm,
	RareLowerMassiveBraidOak,
}

/// Typed authored geometry for one jungle lower-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JungleLowerMassivesItem {
	JungleStorybook(&'static JungleLowerMassivesJungleStorybook),
	Honu(&'static JungleLowerMassivesBanyan),
	Sope(&'static JungleLowerMassivesBanyan),
	WaialeaPalm(&'static JungleLowerMassivesWaialeaPalm),
	BraidOak(&'static JungleLowerMassivesBraidOak),
}

/// Authored geometry ranges for one Honu or Sope banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Jungle Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesJungleStorybook {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
	pub jungle_growth_density: UnitRange,
}

/// Authored geometry ranges for one Waialea Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one rare Braid Oak accent.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const LOWER_MASSIVE_JUNGLE_STORYBOOK: JungleLowerMassivesJungleStorybook =
	JungleLowerMassivesJungleStorybook {
		height: UnitRange::new(10.0, 20.0),
		canopy_density: DENSE_CANOPY_DENSITY,
		jungle_growth_density: MODERATE_CANOPY_DENSITY,
	};

const LOWER_MASSIVE_HONU_BANYAN: JungleLowerMassivesBanyan = JungleLowerMassivesBanyan {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.48, 0.72),
	canopy_spread: UnitRange::new(4.0, 9.0),
	descender_density: UnitRange::new(0.01, 0.045),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_SOPE_BANYAN: JungleLowerMassivesBanyan = JungleLowerMassivesBanyan {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.48, 0.72),
	canopy_spread: UnitRange::new(4.0, 9.0),
	descender_density: UnitRange::new(0.01, 0.045),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_WAIALEA_PALM: JungleLowerMassivesWaialeaPalm = JungleLowerMassivesWaialeaPalm {
	height: UnitRange::new(10.0, 20.0),
	crown_density: DENSE_CANOPY_DENSITY,
};

const RARE_LOWER_MASSIVE_BRAID_OAK: JungleLowerMassivesBraidOak = JungleLowerMassivesBraidOak {
	height: UnitRange::new(10.0, 20.0),
	canopy_spread: UnitRange::new(3.0, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const JUNGLE_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_jungle_bark", "wet_brown"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const JUNGLE_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

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

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "green_brown"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "yellow_green"),
]);

impl JungleLowerMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `6.35` (RFC relative proportions); the `None` weight of `11.0` puts
	/// the placed share at `6.35 / 17.35 ≈ 0.37`, mid RFC `DENSITY_RANGE` (`0.20..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		let jungle_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.54), UnitRange::new(0.0, 0.54));
		let honu = PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.46));
		let sope = PlacementConstraints::new(UnitRange::new(0.0, 0.48), UnitRange::new(0.0, 0.50));
		let waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.44), UnitRange::new(0.0, 0.62));
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.02, 0.50), UnitRange::new(0.0, 0.52));
		GroveDistribution::new(vec![
			GroveBucket::none(8.0),
			GroveBucket::placed(2.0, jungle_storybook, Self::LowerMassiveJungleStorybook),
			GroveBucket::placed(2.0, honu, Self::LowerMassiveHonuBanyan),
			GroveBucket::placed(1.0, sope, Self::LowerMassiveSopesBanyan),
			GroveBucket::placed(1.0, waialea, Self::LowerMassiveWaialeaPalm),
			GroveBucket::placed(0.35, braid_oak, Self::RareLowerMassiveBraidOak),
		])
	}

	pub fn item(self) -> JungleLowerMassivesItem {
		match self {
			Self::LowerMassiveJungleStorybook => {
				JungleLowerMassivesItem::JungleStorybook(&LOWER_MASSIVE_JUNGLE_STORYBOOK)
			}
			Self::LowerMassiveHonuBanyan => {
				JungleLowerMassivesItem::Honu(&LOWER_MASSIVE_HONU_BANYAN)
			}
			Self::LowerMassiveSopesBanyan => {
				JungleLowerMassivesItem::Sope(&LOWER_MASSIVE_SOPE_BANYAN)
			}
			Self::LowerMassiveWaialeaPalm => {
				JungleLowerMassivesItem::WaialeaPalm(&LOWER_MASSIVE_WAIALEA_PALM)
			}
			Self::RareLowerMassiveBraidOak => {
				JungleLowerMassivesItem::BraidOak(&RARE_LOWER_MASSIVE_BRAID_OAK)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveJungleStorybook => JUNGLE_STORYBOOK_STICK_MIX,
			Self::LowerMassiveHonuBanyan => HONU_STICK_MIX,
			Self::LowerMassiveSopesBanyan => SOPE_STICK_MIX,
			Self::LowerMassiveWaialeaPalm => WAIALEA_STICK_MIX,
			Self::RareLowerMassiveBraidOak => BRAID_OAK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveJungleStorybook => JUNGLE_STORYBOOK_CANOPY_MIX,
			Self::LowerMassiveHonuBanyan => HONU_CANOPY_MIX,
			Self::LowerMassiveSopesBanyan => SOPE_CANOPY_MIX,
			Self::LowerMassiveWaialeaPalm => WAIALEA_CANOPY_MIX,
			Self::RareLowerMassiveBraidOak => BRAID_OAK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const JUNGLE_LOWER_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const JUNGLE_LOWER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const JUNGLE_LOWER_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	JUNGLE_LOWER_MASSIVES_STRUCTURAL_HIGH_FACTOR,
	JUNGLE_LOWER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
	JUNGLE_LOWER_MASSIVES_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{JungleLowerMassives, JungleLowerMassivesParams, JungleLowerMassivesPlant};

#[cfg(test)]
mod tests;
