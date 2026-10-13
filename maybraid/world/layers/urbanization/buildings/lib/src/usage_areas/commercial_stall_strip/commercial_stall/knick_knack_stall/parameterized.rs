//! Parameterized knobs + fit for [`super::KnickKnackStall`].

use building_components::LabelStyle;
use procedural_common::{aabb3_to_plan, NoiseConfig, NoiseParams, OptionalFaceBand, PlanAxes};

use crate::fit::{Confines, FitError};
use crate::usage_areas::clearance::{PassageClearance, PlanHost};

use super::super::stall_layout::knick_knack::{
	KnickKnackDisplaySpec, KnickKnackPacked, KnickKnackRegions, KNICK_KNACK_DISPLAY_DEPTH_MAX,
	KNICK_KNACK_DISPLAY_DEPTH_MIN, KNICK_KNACK_DISPLAY_PLACE_RATE,
};

/// Noise / style knobs for [`super::KnickKnackStall`].
#[derive(Debug, Clone, PartialEq)]
pub struct KnickKnackStallParameterized {
	pub style: LabelStyle,
	pub displays: Vec<KnickKnackDisplaySpec>,
}

impl KnickKnackStallParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		if PassageClearance::collect_faces(confines, host).is_empty() {
			return Err(FitError::TooSmall { reason: "knick knack passage" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 80.0));

		let displays = PlanHost::faces(host)
			.into_iter()
			.enumerate()
			.map(|(i, face)| {
				let salt = 81.0 + i as f32 * 11.0;
				let place_u = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, salt);
				let along_max = face.along_len().max(0.5);
				let along = cfg.sample_range_f32_4d(
					(along_max * 0.35).max(0.5),
					along_max,
					c.x,
					c.y,
					c.z,
					salt + 1.0,
				);
				let depth = cfg.sample_range_f32_4d(
					KNICK_KNACK_DISPLAY_DEPTH_MIN,
					KNICK_KNACK_DISPLAY_DEPTH_MAX,
					c.x,
					c.y,
					c.z,
					salt + 2.0,
				);
				let along_t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, salt + 3.0);
				KnickKnackDisplaySpec {
					face,
					display: OptionalFaceBand {
						place: place_u < KNICK_KNACK_DISPLAY_PLACE_RATE,
						along,
						depth,
						along_t,
					},
				}
			})
			.collect();

		Ok(Self { style, displays })
	}

	fn regions(&self) -> KnickKnackRegions {
		KnickKnackRegions { displays: self.displays.clone() }
	}
}

/// Geometry resolved from [`KnickKnackStallParameterized`].
#[derive(Debug, Clone, PartialEq)]
pub struct KnickKnackStallPlan {
	pub parameterized: KnickKnackStallParameterized,
	pub packed: KnickKnackPacked,
}

impl KnickKnackStallPlan {
	pub fn from_parameterized(
		params: KnickKnackStallParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = params.regions().pack(confines)?;
		Ok(Self { parameterized: params, packed })
	}
}
