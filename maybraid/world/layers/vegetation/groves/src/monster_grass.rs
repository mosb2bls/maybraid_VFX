//! Monster Grass — well-known oversized understory blade grove
//! ([RFC-183 §3.4.5.2](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/02-monster-grass/README.md),
//! [#308](https://github.com/ramate-io/maybraid/issues/308)).
//!
//! Dense 2–6 m understory blades for jungle, swamp, and elder-tree floors — structurally
//! Braid Grass at monster scale. Authored cells resolve to [`GroveTuftPatch`] (single-clump
//! cells use `clump_count = 1`). Under `render`, [`MonsterGrassParams::build`] grows
//! [`TuftPatch`](sbs_trees::TuftPatch) plants quantized through
//! [`TuftPatchParams::into_unit_from_num`](sbs_trees::TuftPatchParams::into_unit_from_num)
//! (`patch_variants`, default `100`). High/Medium collections merge to one
//! `MultiSceneMerge` per plant. The grove `LodScene` emits those as lazy flattened
//! kits (no nested [`FoliageNode`](vegetation_components::FoliageNode) hosts).
//! Optional square XZ fold via [`MonsterGrassParams::merge_collections`]
//! (default `0` = one collection per placement).
//!
//! Structural LOD (× grove footprint): High (full clumps); Medium = ~¼ of High tufts
//! (same geometry, thinned); Low ≈ one upright proxy per ~8 cells; UltraLow = 2×2 carpets.
//! Per-plant leaf color: [`MonsterGrassCell::palette_mix`] + [`PaletteMix::pick_color`] with a
//! placement seed → [`vegetation_components::frond_material_ref`].with_palette([color]).

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

/// Authored Monster Grass grove definition.
///
/// Cell footprint is denser than the RFC's nominal `4.0..9.0` grid (like Braid Grass) so preview
/// groves read as continuous tall understory rather than sparse screens. The offset range is
/// signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<MonsterGrassCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-2.5, 2.5)),
		distribution: MonsterGrassCell::distribution(),
	}
}

/// Ordered monster-grass varietals ([RFC-183 §3.4.5.2]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterGrassCell {
	GiantWetBlade,
	BroadJungleBlade,
	PaleGiantReed,
	RedRibbedBlade,
	GiantWetBladePatch,
	BroadJungleBladePatch,
	PaleGiantReedPatch,
	RedRibbedBladePatch,
}

/// Authored geometry ranges for one monster-grass blade clump.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	/// RFC `droop` — splay/sag departure from vertical on upward-biased blade tufts.
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2.5–4.5 % of blade length — broader than Braid Grass for the
/// heavy, wall-like read at 2–6 m.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.025, 0.045);
/// Match default [`sbs_trees::TuftPatch`] kink budget (1–3 segments, not a tall polyline).
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=3;
const SINGLE: RangeInclusive<u32> = 1..=1;
const NO_EXTENT: UnitRange = UnitRange::new(0.0, 0.0);
const NO_SPREAD: UnitRange = UnitRange::new(0.0, 0.0);

const GIANT_WET_BLADE_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.00, 6.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=28,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.25, 0.70),
};

const BROAD_JUNGLE_BLADE_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.50, 5.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.35, 0.85),
};

const PALE_GIANT_REED_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.00, 4.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=22,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.15, 0.50),
};

const RED_RIBBED_BLADE_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.20, 4.20),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.20, 0.65),
};

const GIANT_WET_BLADE: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: GIANT_WET_BLADE_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const BROAD_JUNGLE_BLADE: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: BROAD_JUNGLE_BLADE_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const PALE_GIANT_REED: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: PALE_GIANT_REED_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const RED_RIBBED_BLADE: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: RED_RIBBED_BLADE_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const GIANT_WET_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: GIANT_WET_BLADE_CLUMP,
	clump_count: 3..=5,
	patch_extent_xz: UnitRange::new(1.8, 4.4),
	base_spread: UnitRange::new(0.25, 0.50),
};

const BROAD_JUNGLE_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: BROAD_JUNGLE_BLADE_CLUMP,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.6, 4.8),
	base_spread: UnitRange::new(0.30, 0.55),
};

const PALE_GIANT_REED_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: PALE_GIANT_REED_CLUMP,
	clump_count: 3..=5,
	patch_extent_xz: UnitRange::new(2.0, 2.8),
	base_spread: UnitRange::new(0.20, 0.45),
};

const RED_RIBBED_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: RED_RIBBED_BLADE_CLUMP,
	clump_count: 2..=5,
	patch_extent_xz: UnitRange::new(1.8, 4.4),
	base_spread: UnitRange::new(0.25, 0.50),
};

impl MonsterGrassCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.6` (RFC relative proportions); the `None` weight of `1.5` puts
	/// the placed share at `4.6 / 6.1 ≈ 0.75`. Patches carry `3.68` of the placed weight;
	/// single-anchor clumps share the remaining `0.92`.
	pub fn distribution() -> GroveDistribution<Self> {
		let low_wet =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.50));
		let red_ribbed =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(1.5),
			GroveBucket::placed(0.40, low_wet, Self::GiantWetBlade),
			GroveBucket::placed(0.30, low_wet, Self::BroadJungleBlade),
			GroveBucket::placed(0.15, low_wet, Self::PaleGiantReed),
			GroveBucket::placed(0.07, red_ribbed, Self::RedRibbedBlade),
			GroveBucket::placed(1.60, low_wet, Self::GiantWetBladePatch),
			GroveBucket::placed(1.20, low_wet, Self::BroadJungleBladePatch),
			GroveBucket::placed(0.60, low_wet, Self::PaleGiantReedPatch),
			GroveBucket::placed(0.28, red_ribbed, Self::RedRibbedBladePatch),
		])
	}

	/// Authored tuft-patch layout for this varietal (single-clump cells use `clump_count = 1`).
	pub fn patch(self) -> &'static GroveTuftPatch<MonsterGrassClump> {
		match self {
			Self::GiantWetBlade => &GIANT_WET_BLADE,
			Self::BroadJungleBlade => &BROAD_JUNGLE_BLADE,
			Self::PaleGiantReed => &PALE_GIANT_REED,
			Self::RedRibbedBlade => &RED_RIBBED_BLADE,
			Self::GiantWetBladePatch => &GIANT_WET_BLADE_PATCH,
			Self::BroadJungleBladePatch => &BROAD_JUNGLE_BLADE_PATCH,
			Self::PaleGiantReedPatch => &PALE_GIANT_REED_PATCH,
			Self::RedRibbedBladePatch => &RED_RIBBED_BLADE_PATCH,
		}
	}

	/// Authored palette ranges for this varietal (placement seed picks one endpoint color).
	pub fn palette_mix(self) -> PaletteMix {
		const GIANT_WET_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "wet_green"),
			PaletteSlot::new("blue_green", "dark_green"),
			PaletteSlot::new("emerald_green", "fresh_green"),
		]);
		const BROAD_JUNGLE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("lush_green", "bright_green"),
			PaletteSlot::new("wet_green", "lime_green"),
			PaletteSlot::new("dark_green", "blue_green"),
		]);
		const PALE_GIANT_REED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "pale_straw"),
			PaletteSlot::new("dry_green", "tan_green"),
			PaletteSlot::new("light_green", "fresh_green"),
		]);
		const RED_RIBBED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("dark_red", "deep_green"),
			PaletteSlot::new("copper_red", "wet_green"),
			PaletteSlot::new("red_green", "blue_green"),
		]);
		match self {
			Self::GiantWetBlade | Self::GiantWetBladePatch => GIANT_WET_MIX,
			Self::BroadJungleBlade | Self::BroadJungleBladePatch => BROAD_JUNGLE_MIX,
			Self::PaleGiantReed | Self::PaleGiantReedPatch => PALE_GIANT_REED_MIX,
			Self::RedRibbedBlade | Self::RedRibbedBladePatch => RED_RIBBED_MIX,
		}
	}
}

#[cfg(feature = "render")]
/// Structural High band (× footprint): full authored clumps.
pub const MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
/// Structural Medium band (× footprint): ~¼ of High tufts (same blade geometry).
pub const MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
/// Structural Low band (× footprint): one upright proxy per ~8 placement cells; beyond → UltraLow.
pub const MONSTER_GRASS_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
pub use vc::{MonsterGrass, MonsterGrassParams, MonsterGrassPlant};

#[cfg(test)]
mod tests;
