//! Jerry's Chaparral — well-known moderately dense dry scrub understory grove
//! ([RFC-183 §3.4.5.7], [#318](https://github.com/ramate-io/maybraid/issues/318)).
//!
//! Mixes Rory's Head-trained forms, Common High Bush chaparral mass, and rare small Friend's
//! Conifer accents.

use std::ops::RangeInclusive;

use bevy_math::{Vec2, Vec3};
use procedural_common::UnitRange;

pub mod variants;

#[cfg(feature = "render")]
mod vc;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveWorldSample,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Uniform terrain tuned for chaparral placement constraints (RFC min elevation > 0).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Terrain"))]
pub struct ChaparralFlatTerrain {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.35))]
	pub elevation: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.15))]
	pub steepness: f32,
}

impl Default for ChaparralFlatTerrain {
	fn default() -> Self {
		Self { elevation: 0.35, steepness: 0.15 }
	}
}

impl GroveWorldSample for ChaparralFlatTerrain {
	fn height_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// RFC `projection_count: Moderate` — chaparral high-bush varietal.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// Authored Jerry's Chaparral grove definition.
///
/// Cell footprint sits at the RFC midpoint (`6.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JerrysChaparralCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(6.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-6.5, 6.5)),
		distribution: JerrysChaparralCell::distribution(),
	}
}

/// Ordered chaparral varietals ([RFC-183 §3.4.5.7]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JerrysChaparralCell {
	DryRoryHeadTrained,
	ChaparralHighBush,
	SmallFriendsConifer,
	ManzanitaRory,
}

/// Typed authored geometry for one chaparral varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JerrysChaparralItem {
	RoryHead(&'static JerrysChaparralRoryHead),
	Bush(&'static JerrysChaparralBush),
	FriendsConifer(&'static JerrysChaparralFriendsConifer),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralRoryHead {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.030 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one small Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const DRY_RORY_HEAD: JerrysChaparralRoryHead = JerrysChaparralRoryHead {
	height: UnitRange::new(1.20, 3.20),
	stalk_radius: UnitRange::new(0.036, 0.096),
	canopy_spread: UnitRange::new(0.80, 2.00),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const MANZANITA_RORY: JerrysChaparralRoryHead = JerrysChaparralRoryHead {
	height: UnitRange::new(1.40, 3.00),
	stalk_radius: UnitRange::new(0.042, 0.090),
	canopy_spread: UnitRange::new(0.90, 2.10),
	canopy_density: UnitRange::new(0.0, 0.35),
};

const CHAPARRAL_HIGH_BUSH: JerrysChaparralBush = JerrysChaparralBush {
	height: UnitRange::new(1.00, 2.40),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const SMALL_FRIENDS_CONIFER: JerrysChaparralFriendsConifer = JerrysChaparralFriendsConifer {
	height: UnitRange::new(2.00, 6.00),
	stalk_radius: UnitRange::new(0.05, 0.15),
	canopy_spread: UnitRange::new(0.50, 1.40),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const DRY_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "gray_brown"),
	PaletteSlot::new("vine_bark", "tan_brown"),
]);

const DRY_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("scrub_green", "pale_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

const CHAPARRAL_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);

const CHAPARRAL_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_green", "olive_green"),
	PaletteSlot::new("scrub_green", "tan_green"),
	PaletteSlot::new("dark_green", "pale_green"),
]);

const FRIENDS_CONIFER_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "dry_bark"),
]);

const FRIENDS_CONIFER_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "blue_green"),
	PaletteSlot::new("dry_green", "deep_green"),
	PaletteSlot::new("olive_green", "needle_green"),
]);

const MANZANITA_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("manzanita_red", "copper_red"),
	PaletteSlot::new("smooth_burgundy", "orange_bark"),
]);

const MANZANITA_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "pale_green"),
	PaletteSlot::new("flower_white", "dry_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

impl JerrysChaparralCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.3` (RFC relative proportions); the `None` weight of `7.0` puts
	/// the placed share at `4.3 / 11.3 ≈ 0.38`, mid RFC `DENSITY_RANGE` (`0.24..0.52`).
	pub fn distribution() -> GroveDistribution<Self> {
		let dry_rory =
			PlacementConstraints::new(UnitRange::new(0.10, 0.65), UnitRange::new(0.0, 0.78));
		let bush = PlacementConstraints::new(UnitRange::new(0.05, 0.70), UnitRange::new(0.0, 0.55));
		let conifer =
			PlacementConstraints::new(UnitRange::new(0.15, 0.75), UnitRange::new(0.0, 0.65));
		let manzanita =
			PlacementConstraints::new(UnitRange::new(0.15, 0.70), UnitRange::new(0.0, 0.72));
		GroveDistribution::new(vec![
			GroveBucket::none(7.0),
			GroveBucket::placed(1.5, dry_rory, Self::DryRoryHeadTrained),
			GroveBucket::placed(2.0, bush, Self::ChaparralHighBush),
			GroveBucket::placed(0.45, conifer, Self::SmallFriendsConifer),
			GroveBucket::placed(0.35, manzanita, Self::ManzanitaRory),
		])
	}

	pub fn item(self) -> JerrysChaparralItem {
		match self {
			Self::DryRoryHeadTrained => JerrysChaparralItem::RoryHead(&DRY_RORY_HEAD),
			Self::ChaparralHighBush => JerrysChaparralItem::Bush(&CHAPARRAL_HIGH_BUSH),
			Self::SmallFriendsConifer => {
				JerrysChaparralItem::FriendsConifer(&SMALL_FRIENDS_CONIFER)
			}
			Self::ManzanitaRory => JerrysChaparralItem::RoryHead(&MANZANITA_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryRoryHeadTrained => DRY_RORY_STICK_MIX,
			Self::ChaparralHighBush => CHAPARRAL_BUSH_STICK_MIX,
			Self::SmallFriendsConifer => FRIENDS_CONIFER_STICK_MIX,
			Self::ManzanitaRory => MANZANITA_RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryRoryHeadTrained => DRY_RORY_CANOPY_MIX,
			Self::ChaparralHighBush => CHAPARRAL_BUSH_CANOPY_MIX,
			Self::SmallFriendsConifer => FRIENDS_CONIFER_CANOPY_MIX,
			Self::ManzanitaRory => MANZANITA_RORY_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
use crate::grove::WoodyGroveLod;

#[cfg(feature = "render")]
pub const JERRYS_CHAPARRAL_STRUCTURAL_HIGH_FACTOR: f32 = 6.0;
#[cfg(feature = "render")]
pub const JERRYS_CHAPARRAL_STRUCTURAL_MEDIUM_FACTOR: f32 = 8.0;
#[cfg(feature = "render")]
pub const JERRYS_CHAPARRAL_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

#[cfg(feature = "render")]
const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
	JERRYS_CHAPARRAL_STRUCTURAL_HIGH_FACTOR,
	JERRYS_CHAPARRAL_STRUCTURAL_MEDIUM_FACTOR,
	JERRYS_CHAPARRAL_STRUCTURAL_LOW_FACTOR,
);

#[cfg(feature = "render")]
pub use vc::{JerrysChaparral, JerrysChaparralParams, JerrysChaparralPlant};

#[cfg(test)]
mod tests;
