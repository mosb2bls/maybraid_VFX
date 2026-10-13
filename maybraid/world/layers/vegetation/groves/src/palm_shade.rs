//! Palm Shade — sparse upper-canopy grove with Waialea and Date Palm variants
//! ([RFC-183 §3.4.7.10], [#332](https://github.com/ramate-io/maybraid/issues/332)).
//!
//! Tower Waialea columns, dense lower Waialea crowns, and clustered Date Palms for oasis shade.
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

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Palm Shade grove definition.
///
/// Cell footprint sits at the RFC midpoint (`24.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<PalmShadeCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(24.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-24.0, 24.0),
		),
		distribution: PalmShadeCell::distribution(),
	}
}

/// Ordered palm-shade varietals ([RFC-183 §3.4.7.10]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PalmShadeCell {
	TowerWaialeaPalm,
	LowerWaialeaPalm,
	ShadeDatePalm,
	ClusterDatePalm,
}

/// Typed authored geometry for one palm-shade varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PalmShadeItem {
	WaialeaPalm(&'static PalmShadeWaialeaPalm),
	DatePalm(&'static PalmShadeDatePalm),
}

/// Authored geometry ranges for one Waialea Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct PalmShadeWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct PalmShadeDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const TOWER_WAIALEA_PALM: PalmShadeWaialeaPalm = PalmShadeWaialeaPalm {
	height: UnitRange::new(20.0, 40.0),
	crown_density: MODERATE_CANOPY_DENSITY,
};

const LOWER_WAIALEA_PALM: PalmShadeWaialeaPalm =
	PalmShadeWaialeaPalm { height: UnitRange::new(8.0, 20.0), crown_density: DENSE_CANOPY_DENSITY };

const SHADE_DATE_PALM: PalmShadeDatePalm =
	PalmShadeDatePalm { height: UnitRange::new(6.0, 20.0), crown_density: MODERATE_CANOPY_DENSITY };

const CLUSTER_DATE_PALM: PalmShadeDatePalm =
	PalmShadeDatePalm { height: UnitRange::new(6.0, 12.0), crown_density: DENSE_CANOPY_DENSITY };

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const SHADE_DATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "date_trunk"),
	PaletteSlot::new("tan_bark", "dry_brown"),
]);

const SHADE_DATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_green", "olive_green"),
	PaletteSlot::new("fresh_green", "yellow_green"),
]);

const CLUSTER_DATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("date_trunk", "dry_brown"),
	PaletteSlot::new("tan_bark", "palm_bark"),
]);

const CLUSTER_DATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_green", "olive_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

impl PalmShadeCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.2` (RFC relative proportions); the `None` weight of `10.7` puts
	/// the placed share at `3.2 / 14.0 ≈ 0.23`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let tower_waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.46), UnitRange::new(0.0, 0.56));
		let lower_waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.62));
		let shade_date =
			PlacementConstraints::new(UnitRange::new(0.0, 0.52), UnitRange::new(0.0, 0.42));
		let cluster_date =
			PlacementConstraints::new(UnitRange::new(0.0, 0.44), UnitRange::new(0.0, 0.36));
		GroveDistribution::new(vec![
			GroveBucket::none(10.7),
			GroveBucket::placed(0.8, tower_waialea, Self::TowerWaialeaPalm),
			GroveBucket::placed(0.8, lower_waialea, Self::LowerWaialeaPalm),
			GroveBucket::placed(1.0, shade_date, Self::ShadeDatePalm),
			GroveBucket::placed(0.6, cluster_date, Self::ClusterDatePalm),
		])
	}

	pub fn item(self) -> PalmShadeItem {
		match self {
			Self::TowerWaialeaPalm => PalmShadeItem::WaialeaPalm(&TOWER_WAIALEA_PALM),
			Self::LowerWaialeaPalm => PalmShadeItem::WaialeaPalm(&LOWER_WAIALEA_PALM),
			Self::ShadeDatePalm => PalmShadeItem::DatePalm(&SHADE_DATE_PALM),
			Self::ClusterDatePalm => PalmShadeItem::DatePalm(&CLUSTER_DATE_PALM),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::TowerWaialeaPalm | Self::LowerWaialeaPalm => WAIALEA_STICK_MIX,
			Self::ShadeDatePalm => SHADE_DATE_STICK_MIX,
			Self::ClusterDatePalm => CLUSTER_DATE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::TowerWaialeaPalm | Self::LowerWaialeaPalm => WAIALEA_CANOPY_MIX,
			Self::ShadeDatePalm => SHADE_DATE_CANOPY_MIX,
			Self::ClusterDatePalm => CLUSTER_DATE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
/// Typical Waialea ~32 m; plant Medium is 36. `grove_bands_for_typical_height_and_plant_medium(32, 36)`.
pub const PALM_SHADE_STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const PALM_SHADE_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
#[cfg(feature = "render")]
pub const PALM_SHADE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::keep_low_plants(
	PALM_SHADE_STRUCTURAL_HIGH_FACTOR,
	PALM_SHADE_STRUCTURAL_MEDIUM_FACTOR,
	PALM_SHADE_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{PalmShade, PalmShadeParams, PalmShadePlant};

#[cfg(test)]
mod tests;
