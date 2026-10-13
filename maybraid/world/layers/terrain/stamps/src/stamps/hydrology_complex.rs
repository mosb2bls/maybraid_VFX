//! Stamp Hydrology Complexes (multi-part landforms) — [RFC-105 §3.8.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#388-jersey-hydrology-complexes-multi-part-landforms).
//!
//! Packages interacting height stamps under one complex ID. Water marking/rendering deferred.

use crate::stamp::{StampSemantics, StampSet};
use crate::stamps::canyon::{Canyon, CanyonParams, CanyonVariant};
use crate::stamps::pocket_water::{PocketTermination, PocketWater, PocketWaterParams};
use crate::stamps::rolling_ground::{RollingGround, RollingGroundParams};
use crate::stamps::valley_basin::{
	ValleyBasin, ValleyBasinParams, ValleyCrossSection, ValleyFloorKind,
};
use procedural_common::{Bounds2, SeededHash};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HydrologyComplexKind {
	/// Fan head + distributaries + toe (approximated with valley + pocket + rolling).
	AlluvialFan,
	/// Stepped flats / sink–polje style (pocket + rolling + shallow valley).
	SteppedFlats,
	/// Plunge–pool ladder along a canyon reach.
	RapidLadder,
}

#[derive(Debug, Clone, Copy)]
pub struct HydrologyComplexParams {
	pub kind: HydrologyComplexKind,
}

impl Default for HydrologyComplexParams {
	fn default() -> Self {
		Self { kind: HydrologyComplexKind::AlluvialFan }
	}
}

#[derive(Debug, Clone)]
pub struct HydrologyComplex {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: HydrologyComplexParams,
	pub complex_id: u32,
	pub stamp: StampSet,
}

impl HydrologyComplex {
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: HydrologyComplexParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let hash = SeededHash::new(seed);
		let complex_id = seed.wrapping_mul(0x1656_67B1);
		let mut stamp = StampSet::empty();
		stamp.semantics = StampSemantics::default()
			.with_complex_id(complex_id)
			.with_tag("hydrology_complex");

		match params.kind {
			HydrologyComplexKind::AlluvialFan => {
				stamp.semantics = stamp
					.semantics
					.with_tag("alluvial_fan")
					.with_tag("fan_apex")
					.with_tag("main_stem")
					.with_tag("distributary");
				let apex = ValleyBasin::from_bounds(
					bounds,
					seed.wrapping_add(1),
					ValleyBasinParams {
						cross_section: ValleyCrossSection::V,
						floor: ValleyFloorKind::SpillwayReady,
						width_frac: 0.14,
						depth: 10.0 + 4.0 * hash.unit(1),
						floor_scale: 0.6,
					},
					height_at,
				);
				stamp.extend_with(apex.stamp);
				let toe = PocketWater::from_bounds(
					bounds,
					seed.wrapping_add(2),
					PocketWaterParams {
						termination: PocketTermination::MarshHint,
						pond_frac: 0.12,
						pond_depth: 7.0,
						run_width_frac: 0.06,
						run_depth: 4.0,
					},
					height_at,
				);
				stamp.extend_with(toe.stamp);
				let swell = RollingGround::from_bounds(
					bounds,
					seed.wrapping_add(3),
					RollingGroundParams { count: 3, size_frac: 0.1, amplitude: 2.5 },
				);
				stamp.extend_with(swell.stamp);
			}
			HydrologyComplexKind::SteppedFlats => {
				stamp.semantics =
					stamp.semantics.with_tag("stepped_flats").with_tag("overflow_sill");
				let basin = ValleyBasin::from_bounds(
					bounds,
					seed.wrapping_add(4),
					ValleyBasinParams {
						cross_section: ValleyCrossSection::U,
						floor: ValleyFloorKind::SpillwayReady,
						width_frac: 0.2,
						depth: 8.0,
						floor_scale: 0.7,
					},
					height_at,
				);
				stamp.extend_with(basin.stamp);
				let pocket = PocketWater::from_bounds(
					bounds,
					seed.wrapping_add(5),
					PocketWaterParams::default(),
					None,
				);
				stamp.extend_with(pocket.stamp);
				let flats = RollingGround::from_bounds(
					bounds,
					seed.wrapping_add(6),
					RollingGroundParams { count: 5, size_frac: 0.14, amplitude: 2.0 },
				);
				stamp.extend_with(flats.stamp);
			}
			HydrologyComplexKind::RapidLadder => {
				stamp.semantics = stamp
					.semantics
					.with_tag("rapid_ladder")
					.with_tag("plunge_pool")
					.with_tag("glide_pool");
				let canyon = Canyon::from_bounds(
					bounds,
					seed.wrapping_add(7),
					CanyonParams {
						variant: CanyonVariant::Chained,
						width_frac: 0.09,
						depth: 22.0,
						confinement: 0.9,
					},
					height_at,
				);
				stamp.extend_with(canyon.stamp);
				let pools = PocketWater::from_bounds(
					bounds,
					seed.wrapping_add(8),
					PocketWaterParams {
						termination: PocketTermination::HandOff,
						pond_frac: 0.1,
						pond_depth: 9.0,
						run_width_frac: 0.05,
						run_depth: 5.0,
					},
					height_at,
				);
				stamp.extend_with(pools.stamp);
			}
		}

		Self { bounds, seed, params, complex_id, stamp }
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, HydrologyComplexParams::default(), None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn complex_merges_children() -> anyhow::Result<()> {
		let c = HydrologyComplex::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 480.0, 480.0), 12);
		assert!(c.stamp.modulations.len() >= 3);
		assert_eq!(c.stamp.semantics.complex_id, Some(c.complex_id));
		assert!(c.stamp.semantics.tags.contains(&"hydrology_complex"));
		Ok(())
	}
}
