//! Bush Scrub — well-known sparse tuft-and-bush grove
//! ([RFC-183 §3.4.4.3](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/03-bush-scrub/README.md),
//! [#303](https://github.com/ramate-io/maybraid/issues/303)).
//!
//! Low irregular scrub mixing 25–50 cm tufts with scaled-down Common High Bush forms. Patch
//! varietals scatter each tuft's blades as loose mounds and carry most of the tuft weight; small
//! bushes stay single-anchor.

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

/// RFC `projection_count: Low` — upright rounded low shrubs.
const LOW_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.20, 0.38);
const LOW_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.68, 0.88);

/// RFC `projection_count: VeryLow` — sapling-like upright growth.
const VERY_LOW_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.10, 0.22);
const VERY_LOW_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.78, 0.92);

/// Authored Bush Scrub grove definition.
///
/// Cell footprint sits in the lower third of the RFC's `CELL_SIZE_RANGE` (`2.0..5.0`) so preview
/// groves read denser than the nominal midpoint grid. The offset range is signed and ± one cell
/// so placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<BushScrubCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-2.5, 2.5)),
		distribution: BushScrubCell::distribution(),
	}
}

/// Ordered bush-scrub varietals ([RFC-183 §3.4.4.3]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BushScrubCell {
	DryTuft,
	GreenTuft,
	SmallBush,
	SaplingBush,
	DryTuftPatch,
	GreenTuftPatch,
}

/// Typed authored geometry for one bush-scrub varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BushScrubItem {
	Tuft(&'static BushScrubTuft),
	Patch(&'static GroveTuftPatch<BushScrubTuft>),
	Bush(&'static BushScrubBush),
}

/// Authored geometry ranges for one bush-scrub tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct BushScrubTuft {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Authored geometry ranges for one scaled-down Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct BushScrubBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	/// RFC `projection_count` — horizontal splay in shoot direction mix.
	pub radial_strength: UnitRange,
	/// RFC `projection_count` — upward bias in shoot direction mix.
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

const BLADE_COUNT: RangeInclusive<u32> = 6..=10;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=5;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.30);

const DRY_TUFT: BushScrubTuft = BushScrubTuft {
	height: UnitRange::new(0.25, 0.45),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const GREEN_TUFT: BushScrubTuft = BushScrubTuft {
	height: UnitRange::new(0.25, 0.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const SMALL_BUSH: BushScrubBush = BushScrubBush {
	height: UnitRange::new(0.35, 0.80),
	shoot_count: 4..=7,
	branch_depth: 1..=2,
	radial_strength: LOW_PROJECTION_RADIAL,
	vertical_bias: LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.04, 0.08),
};

const SAPLING_BUSH: BushScrubBush = BushScrubBush {
	height: UnitRange::new(0.50, 1.20),
	shoot_count: 3..=5,
	branch_depth: 1..=1,
	radial_strength: VERY_LOW_PROJECTION_RADIAL,
	vertical_bias: VERY_LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.03, 0.06),
};

// Patch varietals scatter each tuft's blades as loose mounds; they carry most of the tuft
// weight, so the single-anchor "cone" clump reads as the rarer silhouette.

const DRY_TUFT_PATCH: GroveTuftPatch<BushScrubTuft> = GroveTuftPatch {
	clump: DRY_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.9, 2.0),
	base_spread: UnitRange::new(0.10, 0.25),
};

const GREEN_TUFT_PATCH: GroveTuftPatch<BushScrubTuft> = GroveTuftPatch {
	clump: GREEN_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.9, 2.0),
	base_spread: UnitRange::new(0.12, 0.28),
};

const DRY_TUFT_MIX: PaletteMix = PaletteMix::new(&[PaletteSlot::new("dry_green", "straw_brown")]);
const GREEN_TUFT_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("dark_green", "light_green")]);

const SMALL_BUSH_STICK_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("dry_bark", "gray_brown")]);
const SMALL_BUSH_CANOPY_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("scrub_green", "dry_green")]);
const SAPLING_BUSH_STICK_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("young_bark", "green_brown")]);
const SAPLING_BUSH_CANOPY_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("young_green", "light_green")]);

impl BushScrubCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.0` (RFC relative proportions); the `None` weight of `12.0` puts
	/// the placed share at `5.0 / 17.0 ≈ 0.29`, toward the upper end of the RFC's
	/// `DENSITY_RANGE` (`0.10..0.30`) while keeping scrub sparse. Tuft weight (`3.5` total)
	/// leans on patch varietals (`2.8`); single-anchor tufts share the remaining `0.7`. Bush
	/// companions keep their original weights (`1.5`).
	pub fn distribution() -> GroveDistribution<Self> {
		let dry_tuft =
			PlacementConstraints::new(UnitRange::new(0.0, 0.85), UnitRange::new(0.0, 0.75));
		let green_tuft =
			PlacementConstraints::new(UnitRange::new(0.0, 0.85), UnitRange::new(0.0, 0.45));
		let small_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 0.85), UnitRange::new(0.0, 0.65));
		let sapling_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.45));
		GroveDistribution::new(vec![
			GroveBucket::none(12.0),
			GroveBucket::placed(0.4, dry_tuft, Self::DryTuft),
			GroveBucket::placed(0.3, green_tuft, Self::GreenTuft),
			GroveBucket::placed(1.0, small_bush, Self::SmallBush),
			GroveBucket::placed(0.5, sapling_bush, Self::SaplingBush),
			GroveBucket::placed(1.6, dry_tuft, Self::DryTuftPatch),
			GroveBucket::placed(1.2, green_tuft, Self::GreenTuftPatch),
		])
	}

	pub fn item(self) -> BushScrubItem {
		match self {
			Self::DryTuft => BushScrubItem::Tuft(&DRY_TUFT),
			Self::GreenTuft => BushScrubItem::Tuft(&GREEN_TUFT),
			Self::SmallBush => BushScrubItem::Bush(&SMALL_BUSH),
			Self::SaplingBush => BushScrubItem::Bush(&SAPLING_BUSH),
			Self::DryTuftPatch => BushScrubItem::Patch(&DRY_TUFT_PATCH),
			Self::GreenTuftPatch => BushScrubItem::Patch(&GREEN_TUFT_PATCH),
		}
	}

	pub fn palette_mix(self) -> PaletteMix {
		match self {
			Self::DryTuft | Self::DryTuftPatch => DRY_TUFT_MIX,
			Self::GreenTuft | Self::GreenTuftPatch => GREEN_TUFT_MIX,
			Self::SmallBush => SMALL_BUSH_CANOPY_MIX,
			Self::SaplingBush => SAPLING_BUSH_CANOPY_MIX,
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallBush => SMALL_BUSH_STICK_MIX,
			Self::SaplingBush => SAPLING_BUSH_STICK_MIX,
			_ => SMALL_BUSH_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallBush => SMALL_BUSH_CANOPY_MIX,
			Self::SaplingBush => SAPLING_BUSH_CANOPY_MIX,
			_ => GREEN_TUFT_MIX,
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
pub const BUSH_SCRUB_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
#[cfg(feature = "render")]
pub const BUSH_SCRUB_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
#[cfg(feature = "render")]
pub const BUSH_SCRUB_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
	BUSH_SCRUB_STRUCTURAL_HIGH_FACTOR,
	BUSH_SCRUB_STRUCTURAL_MEDIUM_FACTOR,
	BUSH_SCRUB_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{BushScrub, BushScrubParams};

#[cfg(test)]
mod tests;
