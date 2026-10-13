//! Jungle Massives — giant upper-canopy grove above jungle lower massives
//! ([RFC-183 §3.4.7.1], [#331](https://github.com/ramate-io/maybraid/issues/331)).
//!
//! Common 70–220 m jungle storybook and banyan skyline forms.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Dense sampled canopy-density band ([`0.20`, `0.60`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.2, 0.6);
/// Dense sampled jungle-growth band ([`0.20`, `0.60`]).
const DENSE_JUNGLE_GROWTH_DENSITY: UnitRange = UnitRange::new(0.2, 0.6);
/// Dense sampled descender-density band ([`0.01`, `0.03`]).
const DENSE_DESCENDER_DENSITY: UnitRange = UnitRange::new(0.01, 0.03);

/// Authored Jungle Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`44` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JungleMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(44.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-44.0, 44.0),
		),
		distribution: JungleMassivesCell::distribution(),
	}
}

/// Ordered jungle-massive varietals ([RFC-183 §3.4.7.1]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JungleMassivesCell {
	MassiveJungleStorybook,
	MassiveHonuBanyan,
	MassiveSopesBanyan,
}

/// Typed authored geometry for one jungle-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JungleMassivesItem {
	JungleStorybook(&'static JungleMassivesJungleStorybook),
	Honu(&'static JungleMassivesBanyan),
	Sope(&'static JungleMassivesBanyan),
}

/// Authored geometry ranges for one Honu or Sope banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleMassivesBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Jungle Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleMassivesJungleStorybook {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
	pub jungle_growth_density: UnitRange,
}

const MASSIVE_JUNGLE_STORYBOOK: JungleMassivesJungleStorybook = JungleMassivesJungleStorybook {
	height: UnitRange::new(70.0, 160.0),
	canopy_density: DENSE_CANOPY_DENSITY,
	jungle_growth_density: DENSE_JUNGLE_GROWTH_DENSITY,
};

const MASSIVE_HONU_BANYAN: JungleMassivesBanyan = JungleMassivesBanyan {
	height: UnitRange::new(70.0, 200.0),
	stalk_radius: UnitRange::new(3.0, 8.0),
	canopy_spread: UnitRange::new(25.0, 75.0),
	descender_density: DENSE_DESCENDER_DENSITY,
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_SOPE_BANYAN: JungleMassivesBanyan = JungleMassivesBanyan {
	height: UnitRange::new(60.0, 220.0),
	stalk_radius: UnitRange::new(3.0, 9.0),
	canopy_spread: UnitRange::new(28.0, 85.0),
	descender_density: DENSE_DESCENDER_DENSITY,
	canopy_density: DENSE_CANOPY_DENSITY,
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

impl JungleMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.0` (RFC relative proportions); the `None` weight of `24.0` puts
	/// the placed share at `5.0 / 29.0 ≈ 0.17`, lower RFC `DENSITY_RANGE` (`0.16..0.34`).
	pub fn distribution() -> GroveDistribution<Self> {
		let jungle_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.44));
		let honu = PlacementConstraints::new(UnitRange::new(0.0, 0.46), UnitRange::new(0.0, 0.38));
		let sope = PlacementConstraints::new(UnitRange::new(0.0, 0.44), UnitRange::new(0.0, 0.42));
		GroveDistribution::new(vec![
			GroveBucket::none(24.0),
			GroveBucket::placed(2.0, jungle_storybook, Self::MassiveJungleStorybook),
			GroveBucket::placed(2.0, honu, Self::MassiveHonuBanyan),
			GroveBucket::placed(1.0, sope, Self::MassiveSopesBanyan),
		])
	}

	pub fn item(self) -> JungleMassivesItem {
		match self {
			Self::MassiveJungleStorybook => {
				JungleMassivesItem::JungleStorybook(&MASSIVE_JUNGLE_STORYBOOK)
			}
			Self::MassiveHonuBanyan => JungleMassivesItem::Honu(&MASSIVE_HONU_BANYAN),
			Self::MassiveSopesBanyan => JungleMassivesItem::Sope(&MASSIVE_SOPE_BANYAN),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveJungleStorybook => JUNGLE_STORYBOOK_STICK_MIX,
			Self::MassiveHonuBanyan => HONU_STICK_MIX,
			Self::MassiveSopesBanyan => SOPE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveJungleStorybook => JUNGLE_STORYBOOK_CANOPY_MIX,
			Self::MassiveHonuBanyan => HONU_CANOPY_MIX,
			Self::MassiveSopesBanyan => SOPE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Typical large types ~180 m (jungle storybook / honu). `grove_bands_for_typical_height(180)`.
pub const JUNGLE_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const JUNGLE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 12.0;
#[cfg(feature = "render")]
pub const JUNGLE_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 24.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	JUNGLE_MASSIVES_STRUCTURAL_HIGH_FACTOR,
	JUNGLE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
	JUNGLE_MASSIVES_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{JungleMassives, JungleMassivesParams, JungleMassivesPlant};

#[cfg(test)]
mod tests;
