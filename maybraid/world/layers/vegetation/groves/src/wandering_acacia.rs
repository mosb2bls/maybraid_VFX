//! Wandering Acacia — very-low-density dry open upper-canopy grove
//! ([RFC-183 §3.4.7.16], [#338](https://github.com/ramate-io/maybraid/issues/338)).
//!
//! Sparse acacia-like High Bush, dry Sope's Banyan, and rare vase and torch accents across open
//! country.

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

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.02, 0.65);
/// Sparse sampled descender-density band ([`0.02`, `0.04`]).
const SPARSE_DESCENDER_DENSITY: UnitRange = UnitRange::new(0.01, 0.04);
/// Flat sparse crown projection for acacia-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Wandering Acacia grove definition.
///
/// Cell footprint sits at the RFC midpoint (`37.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<WanderingAcaciaCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(37.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-37.0, 37.0),
		),
		distribution: WanderingAcaciaCell::distribution(),
	}
}

/// Ordered wandering-acacia varietals ([RFC-183 §3.4.7.16]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanderingAcaciaCell {
	WanderingHighBush,
	DryWanderingSopesBanyan,
	WanderingVaseTree,
	WanderingPenmarchTorch,
	WanderingKamakuraTorch,
}

/// Typed authored geometry for one wandering-acacia varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WanderingAcaciaItem {
	HighBush(&'static WanderingAcaciaHighBush),
	Sope(&'static WanderingAcaciaBanyan),
	VaseTree(&'static WanderingAcaciaVaseTree),
	PenmarchTorch(&'static WanderingAcaciaTorch),
	KamakuraTorch(&'static WanderingAcaciaTorch),
}

/// Authored geometry ranges for one acacia-impression Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one dry Sope's Banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Vase Tree accent.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one wandering torch form.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const WANDERING_HIGH_BUSH: WanderingAcaciaHighBush = WanderingAcaciaHighBush {
	height: UnitRange::new(5.0, 15.0),
	shoot_count: 5..=16,
	branch_depth: 2..=4,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.45, 0.72),
};

const DRY_WANDERING_SOPE: WanderingAcaciaBanyan = WanderingAcaciaBanyan {
	height: UnitRange::new(5.0, 20.0),
	stalk_radius: UnitRange::new(0.14, 0.38),
	canopy_spread: UnitRange::new(2.5, 7.0),
	descender_density: SPARSE_DESCENDER_DENSITY,
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WANDERING_VASE_TREE: WanderingAcaciaVaseTree = WanderingAcaciaVaseTree {
	height: UnitRange::new(4.0, 8.0),
	stalk_radius: UnitRange::new(0.22, 0.48),
	canopy_spread: UnitRange::new(0.5, 1.4),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WANDERING_PENMARCH_TORCH: WanderingAcaciaTorch = WanderingAcaciaTorch {
	height: UnitRange::new(5.0, 8.0),
	stalk_radius: UnitRange::new(0.14, 0.34),
	canopy_spread: UnitRange::new(0.5, 1.4),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WANDERING_KAMAKURA_TORCH: WanderingAcaciaTorch = WanderingAcaciaTorch {
	height: UnitRange::new(5.0, 8.0),
	stalk_radius: UnitRange::new(0.12, 0.30),
	canopy_spread: UnitRange::new(0.5, 1.4),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WANDERING_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const WANDERING_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "olive_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const DRY_SOPE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_banyan_bark", "tan_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const DRY_SOPE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dusty_green"),
	PaletteSlot::new("deep_green", "dry_green"),
]);

const WANDERING_VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sun_baked_bark", "acacia_bark"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const WANDERING_VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dusty_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const WANDERING_PENMARCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("dry_bark", "dark_bark"),
]);

const WANDERING_PENMARCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("flower_yellow", "yellow_green"),
	PaletteSlot::new("olive_green", "dry_green"),
]);

const WANDERING_KAMAKURA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "orange_bark"),
	PaletteSlot::new("acacia_bark", "dark_bark"),
]);

const WANDERING_KAMAKURA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("silver_green", "olive_green"),
	PaletteSlot::new("pale_green", "dry_green"),
]);

impl WanderingAcaciaCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.55`; the `None` weight of `37.0` puts the placed share at
	/// `3.55 / 40.55 ≈ 0.088`, mid RFC `DENSITY_RANGE` (`0.03..0.12`).
	pub fn distribution() -> GroveDistribution<Self> {
		let wandering_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.66));
		let dry_sope =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let wandering_vase =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.62));
		let wandering_penmarch =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.60));
		let wandering_kamakura =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		GroveDistribution::new(vec![
			GroveBucket::none(37.0),
			GroveBucket::placed(5.0, wandering_bush, Self::WanderingHighBush),
			GroveBucket::placed(1.0, dry_sope, Self::DryWanderingSopesBanyan),
			GroveBucket::placed(0.25, wandering_vase, Self::WanderingVaseTree),
			GroveBucket::placed(0.18, wandering_penmarch, Self::WanderingPenmarchTorch),
			GroveBucket::placed(0.12, wandering_kamakura, Self::WanderingKamakuraTorch),
		])
	}

	pub fn item(self) -> WanderingAcaciaItem {
		match self {
			Self::WanderingHighBush => WanderingAcaciaItem::HighBush(&WANDERING_HIGH_BUSH),
			Self::DryWanderingSopesBanyan => WanderingAcaciaItem::Sope(&DRY_WANDERING_SOPE),
			Self::WanderingVaseTree => WanderingAcaciaItem::VaseTree(&WANDERING_VASE_TREE),
			Self::WanderingPenmarchTorch => {
				WanderingAcaciaItem::PenmarchTorch(&WANDERING_PENMARCH_TORCH)
			}
			Self::WanderingKamakuraTorch => {
				WanderingAcaciaItem::KamakuraTorch(&WANDERING_KAMAKURA_TORCH)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::WanderingHighBush => WANDERING_BUSH_STICK_MIX,
			Self::DryWanderingSopesBanyan => DRY_SOPE_STICK_MIX,
			Self::WanderingVaseTree => WANDERING_VASE_STICK_MIX,
			Self::WanderingPenmarchTorch => WANDERING_PENMARCH_STICK_MIX,
			Self::WanderingKamakuraTorch => WANDERING_KAMAKURA_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::WanderingHighBush => WANDERING_BUSH_CANOPY_MIX,
			Self::DryWanderingSopesBanyan => DRY_SOPE_CANOPY_MIX,
			Self::WanderingVaseTree => WANDERING_VASE_CANOPY_MIX,
			Self::WanderingPenmarchTorch => WANDERING_PENMARCH_CANOPY_MIX,
			Self::WanderingKamakuraTorch => WANDERING_KAMAKURA_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const WANDERING_ACACIA_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const WANDERING_ACACIA_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const WANDERING_ACACIA_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	WANDERING_ACACIA_STRUCTURAL_HIGH_FACTOR,
	WANDERING_ACACIA_STRUCTURAL_MEDIUM_FACTOR,
	WANDERING_ACACIA_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{WanderingAcacia, WanderingAcaciaParams, WanderingAcaciaPlant};

#[cfg(test)]
mod tests;
