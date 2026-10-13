//! Dryland — very-low-density arid upper-canopy grove with Liam's Conifer and Vase Tree
//! ([RFC-183 §3.4.7.13], [#335](https://github.com/ramate-io/maybraid/issues/335)).
//!
//! Sparse dry highland canopy with evenly common Liam's Conifer and Vase Tree forms.

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

/// Authored Dryland grove definition.
///
/// Cell footprint sits at the RFC midpoint (`35.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<DrylandCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(35.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-35.0, 35.0),
		),
		distribution: DrylandCell::distribution(),
	}
}

/// Ordered dryland varietals ([RFC-183 §3.4.7.13]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrylandCell {
	DrylandLiamsConifer,
	DrylandVaseTree,
}

/// Typed authored geometry for one dryland varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrylandItem {
	LiamsConifer(&'static DrylandLiamsConifer),
	VaseTree(&'static DrylandVaseTree),
}

/// Authored geometry ranges for one dry Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct DrylandLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct DrylandVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const DRYLAND_LIAMS: DrylandLiamsConifer = DrylandLiamsConifer {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.25, 0.50),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRYLAND_VASE: DrylandVaseTree = DrylandVaseTree {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.34, 0.68),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRYLAND_LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_conifer_bark", "tan_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const DRYLAND_LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sage_green", "dusty_green"),
	PaletteSlot::new("deep_green", "olive_green"),
]);

const DRYLAND_VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sun_baked_bark", "tan_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const DRYLAND_VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dusty_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

impl DrylandCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.0`; the `None` weight of `24.7` puts the placed share at
	/// `2.0 / 26.7 ≈ 0.075`, mid RFC `DENSITY_RANGE` (`0.03..0.12`).
	pub fn distribution() -> GroveDistribution<Self> {
		let liams = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.82));
		let vase = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(24.7),
			GroveBucket::placed(1.0, liams, Self::DrylandLiamsConifer),
			GroveBucket::placed(1.0, vase, Self::DrylandVaseTree),
		])
	}

	pub fn item(self) -> DrylandItem {
		match self {
			Self::DrylandLiamsConifer => DrylandItem::LiamsConifer(&DRYLAND_LIAMS),
			Self::DrylandVaseTree => DrylandItem::VaseTree(&DRYLAND_VASE),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DrylandLiamsConifer => DRYLAND_LIAMS_STICK_MIX,
			Self::DrylandVaseTree => DRYLAND_VASE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DrylandLiamsConifer => DRYLAND_LIAMS_CANOPY_MIX,
			Self::DrylandVaseTree => DRYLAND_VASE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const DRYLAND_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const DRYLAND_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const DRYLAND_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	DRYLAND_STRUCTURAL_HIGH_FACTOR,
	DRYLAND_STRUCTURAL_MEDIUM_FACTOR,
	DRYLAND_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{Dryland, DrylandParams, DrylandPlant};

#[cfg(test)]
mod tests;
