//! Shamanhome — well-known moderate sacred lower-canopy grove
//! ([RFC-183 §3.4.6.3], [#324](https://github.com/ramate-io/maybraid/issues/324)).
//!
//! Braid Oak dominates with uncommon ritual Date Palm and Sope Banyan accents.
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

/// Sparse sampled descender-density band ([`0.02`, `0.04`]).
const SPARSE_DESCENDER_DENSITY: UnitRange = UnitRange::new(0.02, 0.04);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse..moderate sampled canopy-density band.
const SPARSE_TO_MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.65);

/// Authored Shamanhome grove definition.
///
/// Cell footprint sits at the RFC midpoint (`10.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ShamanhomeCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(8.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-10.5, 10.5),
		),
		distribution: ShamanhomeCell::distribution(),
	}
}

/// Ordered shamanhome varietals ([RFC-183 §3.4.6.3]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShamanhomeCell {
	ShamanBraidOak,
	RedRitualBraidOak,
	GnarledElderBraidOak,
	SilverShrineBraidOak,
	CopperBranchBraidOak,
	RitualDatePalm,
	SmallSopeBanyan,
}

/// Typed authored geometry for one shamanhome varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShamanhomeItem {
	BraidOak(&'static ShamanhomeBraidOak),
	DatePalm(&'static ShamanhomeDatePalm),
	SopeBanyan(&'static ShamanhomeBanyan),
}

/// Authored geometry ranges for one Braid Oak form (shared geometry; palette differs per cell).
#[derive(Debug, Clone, PartialEq)]
pub struct ShamanhomeBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one ritual Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct ShamanhomeDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one small Sope Banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct ShamanhomeBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled descender probability band; lower values keep descenders sparse.
	pub descender_density: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const SHAMAN_BRAID_OAK: ShamanhomeBraidOak = ShamanhomeBraidOak {
	height: UnitRange::new(4.0, 7.0),
	canopy_spread: UnitRange::new(1.6, 3.8),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const ELDER_BRAID_OAK: ShamanhomeBraidOak = ShamanhomeBraidOak {
	height: UnitRange::new(5.0, 7.0),
	canopy_spread: UnitRange::new(2.0, 4.2),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SHRINE_BRAID_OAK: ShamanhomeBraidOak = ShamanhomeBraidOak {
	height: UnitRange::new(4.0, 6.0),
	canopy_spread: UnitRange::new(1.4, 3.2),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const RITUAL_DATE_PALM: ShamanhomeDatePalm =
	ShamanhomeDatePalm { height: UnitRange::new(4.0, 6.0), crown_density: MODERATE_CANOPY_DENSITY };

const SMALL_SOPE_BANYAN: ShamanhomeBanyan = ShamanhomeBanyan {
	height: UnitRange::new(5.0, 7.0),
	stalk_radius: UnitRange::new(0.26, 0.38),
	canopy_spread: UnitRange::new(2.2, 4.8),
	descender_density: SPARSE_DESCENDER_DENSITY,
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SHAMAN_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "moss_bark"),
	PaletteSlot::new("gnarled_brown", "gray_brown"),
]);

const SHAMAN_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("moss_green", "light_green"),
]);

const RED_RITUAL_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ritual_red_bark", "copper_red"),
	PaletteSlot::new("dark_bark", "moss_bark"),
]);

const RED_RITUAL_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("flower_red", "moss_green"),
]);

const GNARLED_ELDER_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gnarled_brown", "dark_bark"),
	PaletteSlot::new("moss_bark", "wet_bark"),
]);

const GNARLED_ELDER_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_green", "deep_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);

const SILVER_SHRINE_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ornamental_bark", "gray_brown"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const SILVER_SHRINE_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("silver_green", "pale_green"),
	PaletteSlot::new("olive_green", "moss_green"),
]);

const COPPER_BRANCH_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "ritual_red_bark"),
	PaletteSlot::new("gnarled_brown", "dark_bark"),
]);

const COPPER_BRANCH_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("yellow_green", "moss_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const RITUAL_DATE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("dry_brown", "gray_brown"),
]);

const RITUAL_DATE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "date_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

const SOPE_BANYAN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const SOPE_BANYAN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "wet_green"),
	PaletteSlot::new("blue_green", "deep_green"),
]);

impl ShamanhomeCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.10` (RFC braid-oak, date-palm, and banyan proportions plus three
	/// authored braid-oak accents); the `None` weight of `8.0` puts the placed share at
	/// `5.10 / 13.10 ≈ 0.39`, mid RFC `DENSITY_RANGE` (`0.22..0.48`).
	pub fn distribution() -> GroveDistribution<Self> {
		let shaman_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.62), UnitRange::new(0.0, 0.40));
		let red_ritual_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.58), UnitRange::new(0.0, 0.45));
		let gnarled_elder_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.65), UnitRange::new(0.0, 0.42));
		let silver_shrine_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.55), UnitRange::new(0.0, 0.38));
		let copper_branch_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.60), UnitRange::new(0.0, 0.44));
		let ritual_date_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.30));
		let small_sope_banyan =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.36));
		GroveDistribution::new(vec![
			GroveBucket::none(6.0),
			GroveBucket::placed(2.0, shaman_braid_oak, Self::ShamanBraidOak),
			GroveBucket::placed(0.45, red_ritual_braid_oak, Self::RedRitualBraidOak),
			GroveBucket::placed(0.55, gnarled_elder_braid_oak, Self::GnarledElderBraidOak),
			GroveBucket::placed(0.30, silver_shrine_braid_oak, Self::SilverShrineBraidOak),
			GroveBucket::placed(0.25, copper_branch_braid_oak, Self::CopperBranchBraidOak),
			GroveBucket::placed(0.75, ritual_date_palm, Self::RitualDatePalm),
			GroveBucket::placed(0.80, small_sope_banyan, Self::SmallSopeBanyan),
		])
	}

	pub fn item(self) -> ShamanhomeItem {
		match self {
			Self::ShamanBraidOak | Self::RedRitualBraidOak | Self::CopperBranchBraidOak => {
				ShamanhomeItem::BraidOak(&SHAMAN_BRAID_OAK)
			}
			Self::GnarledElderBraidOak => ShamanhomeItem::BraidOak(&ELDER_BRAID_OAK),
			Self::SilverShrineBraidOak => ShamanhomeItem::BraidOak(&SHRINE_BRAID_OAK),
			Self::RitualDatePalm => ShamanhomeItem::DatePalm(&RITUAL_DATE_PALM),
			Self::SmallSopeBanyan => ShamanhomeItem::SopeBanyan(&SMALL_SOPE_BANYAN),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShamanBraidOak => SHAMAN_BRAID_OAK_STICK_MIX,
			Self::RedRitualBraidOak => RED_RITUAL_BRAID_OAK_STICK_MIX,
			Self::GnarledElderBraidOak => GNARLED_ELDER_BRAID_OAK_STICK_MIX,
			Self::SilverShrineBraidOak => SILVER_SHRINE_BRAID_OAK_STICK_MIX,
			Self::CopperBranchBraidOak => COPPER_BRANCH_BRAID_OAK_STICK_MIX,
			Self::RitualDatePalm => RITUAL_DATE_PALM_STICK_MIX,
			Self::SmallSopeBanyan => SOPE_BANYAN_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShamanBraidOak => SHAMAN_BRAID_OAK_CANOPY_MIX,
			Self::RedRitualBraidOak => RED_RITUAL_BRAID_OAK_CANOPY_MIX,
			Self::GnarledElderBraidOak => GNARLED_ELDER_BRAID_OAK_CANOPY_MIX,
			Self::SilverShrineBraidOak => SILVER_SHRINE_BRAID_OAK_CANOPY_MIX,
			Self::CopperBranchBraidOak => COPPER_BRANCH_BRAID_OAK_CANOPY_MIX,
			Self::RitualDatePalm => RITUAL_DATE_PALM_CANOPY_MIX,
			Self::SmallSopeBanyan => SOPE_BANYAN_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const SHAMANHOME_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const SHAMANHOME_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const SHAMANHOME_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	SHAMANHOME_STRUCTURAL_HIGH_FACTOR,
	SHAMANHOME_STRUCTURAL_MEDIUM_FACTOR,
	SHAMANHOME_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{Shamanhome, ShamanhomeParams, ShamanhomePlant};

#[cfg(test)]
mod tests;
