//! Levantine Scrub — well-known dry Mediterranean scrub understory grove
//! ([RFC-183 §3.4.5.8], [#320](https://github.com/ramate-io/maybraid/issues/320)).
//!
//! Mixes Rory's Head-trained forms, small Vase Trees, Common High Bush scrub mass, Penmarch Torch
//! accents, occasional small Braid Oak forms, and Simpleman's Hedge bands.

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

/// RFC `projection_count: Moderate` — dry high-bush varietal.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// RFC `density: Moderate` for hedge bands.
const MODERATE_HEDGE_DENSITY: UnitRange = UnitRange::new(0.40, 0.60);

/// Authored Levantine Scrub grove definition.
///
/// Cell footprint sits at the RFC midpoint (`5.75` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<LevantineScrubCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(5.75),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-5.75, 5.75),
		),
		distribution: LevantineScrubCell::distribution(),
	}
}

/// Ordered scrub varietals ([RFC-183 §3.4.5.8]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevantineScrubCell {
	DryRoryHeadTrained,
	SmallVaseTree,
	DryHighBush,
	SmallPenmarchTorch,
	RedOliveTorch,
	SmallBraidOak,
	ScrubHedge,
}

/// Typed authored geometry for one scrub varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LevantineScrubItem {
	RoryHead(&'static LevantineScrubRoryHead),
	VaseTree(&'static LevantineScrubVaseTree),
	Bush(&'static LevantineScrubBush),
	PenmarchTorch(&'static LevantineScrubTorch),
	BraidOak(&'static LevantineScrubBraidOak),
	Hedge(&'static LevantineScrubHedge),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubRoryHead {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.030 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one small Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one Penmarch Torch form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one small Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Simpleman's Hedge band.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubHedge {
	pub height: UnitRange,
	pub width: UnitRange,
	pub density: UnitRange,
}

const DRY_RORY_HEAD: LevantineScrubRoryHead = LevantineScrubRoryHead {
	height: UnitRange::new(1.20, 3.00),
	stalk_radius: UnitRange::new(0.036, 0.090),
	canopy_spread: UnitRange::new(0.80, 2.20),
	canopy_density: UnitRange::new(0.0, 0.35),
};

const SMALL_VASE_TREE: LevantineScrubVaseTree = LevantineScrubVaseTree {
	height: UnitRange::new(1.20, 3.00),
	stalk_radius: UnitRange::new(0.036, 0.090),
	canopy_spread: UnitRange::new(0.70, 1.80),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const DRY_HIGH_BUSH: LevantineScrubBush = LevantineScrubBush {
	height: UnitRange::new(1.00, 2.50),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const SMALL_PENMARCH_TORCH: LevantineScrubTorch = LevantineScrubTorch {
	height: UnitRange::new(1.40, 3.20),
	stalk_radius: UnitRange::new(0.042, 0.096),
	canopy_spread: UnitRange::new(0.50, 1.30),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const RED_OLIVE_TORCH: LevantineScrubTorch = LevantineScrubTorch {
	height: UnitRange::new(1.60, 3.40),
	stalk_radius: UnitRange::new(0.048, 0.102),
	canopy_spread: UnitRange::new(0.55, 1.35),
	canopy_density: UnitRange::new(0.0, 0.35),
};

const SMALL_BRAID_OAK: LevantineScrubBraidOak = LevantineScrubBraidOak {
	height: UnitRange::new(2.00, 5.50),
	canopy_spread: UnitRange::new(1.20, 3.00),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const SCRUB_HEDGE: LevantineScrubHedge = LevantineScrubHedge {
	height: UnitRange::new(0.80, 1.60),
	width: UnitRange::new(0.70, 1.80),
	density: MODERATE_HEDGE_DENSITY,
};

const DRY_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "gray_brown"),
	PaletteSlot::new("vine_bark", "olive_brown"),
]);

const DRY_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("silver_green", "pale_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

const VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ornamental_bark", "gray_brown"),
	PaletteSlot::new("dry_bark", "tan_brown"),
]);

const VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "light_green"),
	PaletteSlot::new("dry_green", "flower_white"),
	PaletteSlot::new("dark_green", "silver_green"),
]);

const DRY_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);

const DRY_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("scrub_green", "tan_green"),
	PaletteSlot::new("pale_green", "yellow_green"),
]);

const PENMARCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "dark_bark"),
	PaletteSlot::new("ornamental_bark", "gray_brown"),
]);

const PENMARCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "olive_green"),
	PaletteSlot::new("dry_green", "light_green"),
	PaletteSlot::new("flower_yellow", "pale_green"),
]);

const RED_OLIVE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "orange_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const RED_OLIVE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "silver_green"),
	PaletteSlot::new("flower_yellow", "light_green"),
	PaletteSlot::new("dark_green", "pale_green"),
]);

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "gray_brown"),
	PaletteSlot::new("olive_brown", "tan_brown"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("silver_green", "pale_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

const SCRUB_HEDGE_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("hedge_green", "olive_green"),
	PaletteSlot::new("dry_green", "pale_green"),
	PaletteSlot::new("flower_white", "leaf_green"),
]);

impl LevantineScrubCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.45`; the `None` weight of `11.0` puts the placed share at
	/// `5.45 / 16.45 ≈ 0.33`, mid RFC `DENSITY_RANGE` (`0.18..0.48`).
	pub fn distribution() -> GroveDistribution<Self> {
		let dry_rory =
			PlacementConstraints::new(UnitRange::new(0.05, 0.70), UnitRange::new(0.0, 0.70));
		let vase = PlacementConstraints::new(UnitRange::new(0.05, 0.65), UnitRange::new(0.0, 0.52));
		let bush = PlacementConstraints::new(UnitRange::new(0.00, 0.72), UnitRange::new(0.0, 0.65));
		let penmarch =
			PlacementConstraints::new(UnitRange::new(0.10, 0.70), UnitRange::new(0.0, 0.64));
		let red_olive =
			PlacementConstraints::new(UnitRange::new(0.10, 0.68), UnitRange::new(0.0, 0.60));
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.08, 0.75), UnitRange::new(0.0, 0.68));
		let hedge =
			PlacementConstraints::new(UnitRange::new(0.00, 0.65), UnitRange::new(0.0, 0.35));
		GroveDistribution::new(vec![
			GroveBucket::none(11.0),
			GroveBucket::placed(1.2, dry_rory, Self::DryRoryHeadTrained),
			GroveBucket::placed(0.70, vase, Self::SmallVaseTree),
			GroveBucket::placed(2.0, bush, Self::DryHighBush),
			GroveBucket::placed(0.45, penmarch, Self::SmallPenmarchTorch),
			GroveBucket::placed(0.25, red_olive, Self::RedOliveTorch),
			GroveBucket::placed(0.35, braid_oak, Self::SmallBraidOak),
			GroveBucket::placed(0.50, hedge, Self::ScrubHedge),
		])
	}

	pub fn item(self) -> LevantineScrubItem {
		match self {
			Self::DryRoryHeadTrained => LevantineScrubItem::RoryHead(&DRY_RORY_HEAD),
			Self::SmallVaseTree => LevantineScrubItem::VaseTree(&SMALL_VASE_TREE),
			Self::DryHighBush => LevantineScrubItem::Bush(&DRY_HIGH_BUSH),
			Self::SmallPenmarchTorch => LevantineScrubItem::PenmarchTorch(&SMALL_PENMARCH_TORCH),
			Self::RedOliveTorch => LevantineScrubItem::PenmarchTorch(&RED_OLIVE_TORCH),
			Self::SmallBraidOak => LevantineScrubItem::BraidOak(&SMALL_BRAID_OAK),
			Self::ScrubHedge => LevantineScrubItem::Hedge(&SCRUB_HEDGE),
		}
	}

	pub fn stick_palette_mix(self) -> Option<PaletteMix> {
		match self {
			Self::DryRoryHeadTrained => Some(DRY_RORY_STICK_MIX),
			Self::SmallVaseTree => Some(VASE_STICK_MIX),
			Self::DryHighBush => Some(DRY_BUSH_STICK_MIX),
			Self::SmallPenmarchTorch => Some(PENMARCH_STICK_MIX),
			Self::RedOliveTorch => Some(RED_OLIVE_STICK_MIX),
			Self::SmallBraidOak => Some(BRAID_OAK_STICK_MIX),
			Self::ScrubHedge => None,
		}
	}

	pub fn canopy_palette_mix(self) -> Option<PaletteMix> {
		match self {
			Self::DryRoryHeadTrained => Some(DRY_RORY_CANOPY_MIX),
			Self::SmallVaseTree => Some(VASE_CANOPY_MIX),
			Self::DryHighBush => Some(DRY_BUSH_CANOPY_MIX),
			Self::SmallPenmarchTorch => Some(PENMARCH_CANOPY_MIX),
			Self::RedOliveTorch => Some(RED_OLIVE_CANOPY_MIX),
			Self::SmallBraidOak => Some(BRAID_OAK_CANOPY_MIX),
			Self::ScrubHedge => None,
		}
	}

	pub fn palette_mix(self) -> Option<PaletteMix> {
		match self {
			Self::ScrubHedge => Some(SCRUB_HEDGE_MIX),
			_ => None,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Structural High band (× footprint).
pub const LEVANTINE_SCRUB_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
/// Structural Medium band (× footprint).
pub const LEVANTINE_SCRUB_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
/// Structural Low band (× footprint).
pub const LEVANTINE_SCRUB_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
	LEVANTINE_SCRUB_STRUCTURAL_HIGH_FACTOR,
	LEVANTINE_SCRUB_STRUCTURAL_MEDIUM_FACTOR,
	LEVANTINE_SCRUB_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{LevantineScrub, LevantineScrubParams, LevantineScrubPlant};

#[cfg(test)]
mod tests;
