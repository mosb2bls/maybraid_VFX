//! Knick-knack plan packer: passage keep-outs + wall display bands only.
//!
//! No interior rooms — just clearance bands and opportunistic `PlanOpeningFace`
//! bands along host walls (including free segments beside passages).

use bevy_math::bounding::Aabb3d;
use procedural_common::{
	aabb3_to_plan, intersects_aabb2, plan_to_aabb3, Aabb2dPack, OptionalFaceBand, PlanAxes,
	PlanOpeningFace,
};

use crate::fit::{Confines, FitError};
use crate::usage_areas::clearance::{PassageClearance, PlanHost};

/// Display depth sample / pack range.
pub const KNICK_KNACK_DISPLAY_DEPTH_MIN: f32 = 0.5;
pub const KNICK_KNACK_DISPLAY_DEPTH_MAX: f32 = 1.0;
/// Minimum along-length for a display segment.
pub const KNICK_KNACK_DISPLAY_ALONG_MIN: f32 = 0.75;
/// Default place-rate for optional wall displays.
pub const KNICK_KNACK_DISPLAY_PLACE_RATE: f32 = 0.7;

/// Per-wall display choice.
pub type KnickKnackDisplayChoice = OptionalFaceBand;

/// Host wall snapshotted with its display choice.
#[derive(Debug, Clone, PartialEq)]
pub struct KnickKnackDisplaySpec {
	pub face: PlanOpeningFace,
	pub display: KnickKnackDisplayChoice,
}

/// Noise knobs consumed by [`KnickKnackRegions::pack`].
#[derive(Debug, Clone, PartialEq)]
pub struct KnickKnackRegions {
	pub displays: Vec<KnickKnackDisplaySpec>,
}

/// Geometry produced by [`KnickKnackRegions::pack`].
#[derive(Debug, Clone, PartialEq)]
pub struct KnickKnackPacked {
	pub displays: Vec<Aabb3d>,
}

impl KnickKnackRegions {
	pub fn pack(&self, confines: &Confines) -> Result<KnickKnackPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall { reason: "knick knack passage" });
		}

		let mut hard = PassageClearance::bands_std(host, &passage_faces);
		let mut displays = Vec::new();

		// Sampled display choices first (one attempt per host wall).
		for spec in &self.displays {
			let depth = spec
				.display
				.depth
				.clamp(KNICK_KNACK_DISPLAY_DEPTH_MIN, KNICK_KNACK_DISPLAY_DEPTH_MAX);
			let choice = OptionalFaceBand {
				place: spec.display.place,
				along: spec.display.along,
				depth,
				along_t: spec.display.along_t,
			};
			let Some(band) = choice.resolve(host, spec.face) else {
				continue;
			};
			if !band.is_clear_of(&hard) {
				continue;
			}
			if displays.iter().any(|s| intersects_aabb2(band, *s)) {
				continue;
			}
			displays.push(band);
			hard.push(band);
		}

		// Opportunistic fill: more discontiguous bands on free wall segments.
		for face in PlanHost::faces(host) {
			let depth = self
				.displays
				.iter()
				.find(|s| PlanHost::same_wall(s.face, face))
				.map(|s| {
					s.display
						.depth
						.clamp(KNICK_KNACK_DISPLAY_DEPTH_MIN, KNICK_KNACK_DISPLAY_DEPTH_MAX)
				})
				.unwrap_or(0.75);
			for _ in 0..8 {
				let Some((seg0, seg1)) =
					face.longest_free_segment(&hard, KNICK_KNACK_DISPLAY_ALONG_MIN)
				else {
					break;
				};
				let avail = seg1 - seg0;
				let seg_face = PlanOpeningFace { along0: seg0, along1: seg1, ..face };
				let Some(band) = seg_face.band(host, avail, depth, 0.5) else {
					break;
				};
				if !band.is_clear_of(&hard) {
					break;
				}
				if displays.iter().any(|s| intersects_aabb2(band, *s)) {
					break;
				}
				displays.push(band);
				hard.push(band);
			}
		}

		if displays.is_empty() {
			return Err(FitError::TooSmall { reason: "knick knack display" });
		}

		Ok(KnickKnackPacked {
			displays: displays.into_iter().map(|d| plan_to_aabb3(host3, d, PlanAxes::XZ)).collect(),
		})
	}
}
