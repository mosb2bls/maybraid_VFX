//! Parameterized (noise-sampled) knobs + deterministic fit for [`super::BitesStall`].

use bevy_math::bounding::Aabb3d;
use building_components::LabelStyle;
use procedural_common::{NoiseConfig, NoiseParams, OptionalFaceBand};

use crate::fit::{Confines, FitError};

use super::super::stall_layout::bites::{
	BITES_COUNTER_ALONG_MIN, BITES_COUNTER_PLACE_RATE, BITES_PASSAGE_REMAIN_MIN,
	BITES_REGION_MIN_PLAN,
};
use super::super::stall_layout::{
	BitesCounterChoice, BitesKitchen, BitesPassageSpec, EligibleBitesPassage, PackedBitesCounters,
};

/// Noise / style knobs for [`super::BitesStall`] (sampled above; fit below).
#[derive(Debug, Clone, PartialEq)]
pub struct BitesStallParameterized {
	pub style: LabelStyle,
	/// Passage snapshot with counter choices (no parallel-array re-query at pack).
	pub passages: Vec<BitesPassageSpec>,
}

impl BitesStallParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let eligible = EligibleBitesPassage::collect(confines);
		if eligible.is_empty() {
			return Err(FitError::TooSmall { reason: "bites counter passage" });
		}
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let mut passages: Vec<BitesPassageSpec> = eligible
			.into_iter()
			.enumerate()
			.map(|(i, passage)| {
				let counter = Self::sample_counter(&passage, &cfg, c, 32.0 + i as f32 * 17.0);
				BitesPassageSpec { passage, counter }
			})
			.collect();
		if !passages.iter().any(|p| p.counter.place) {
			let best = passages
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| {
					a.passage
						.along_len
						.partial_cmp(&b.passage.along_len)
						.unwrap_or(std::cmp::Ordering::Equal)
				})
				.map(|(i, _)| i)
				.unwrap_or(0);
			passages[best].counter.place = true;
		}
		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 33.0));
		Ok(Self { style, passages })
	}

	fn sample_counter(
		passage: &EligibleBitesPassage,
		cfg: &NoiseConfig,
		origin: bevy_math::Vec3,
		salt: f32,
	) -> BitesCounterChoice {
		let place_u = cfg.sample_range_f32_4d(0.0, 1.0, origin.x, origin.y, origin.z, salt);
		let place = place_u < BITES_COUNTER_PLACE_RATE;
		let max_along = (passage.along_len - BITES_PASSAGE_REMAIN_MIN).max(BITES_COUNTER_ALONG_MIN);
		let along = cfg.sample_range_f32_4d(
			BITES_COUNTER_ALONG_MIN,
			max_along,
			origin.x,
			origin.y,
			origin.z,
			salt + 1.0,
		);
		let depth = cfg.sample_range_f32_4d(0.65, 1.0, origin.x, origin.y, origin.z, salt + 2.0);
		let remain = (passage.along_len - along).max(0.0);
		let along_t = if remain + 1e-3 >= 2.0 * BITES_PASSAGE_REMAIN_MIN {
			cfg.sample_range_f32_4d(0.05, 0.95, origin.x, origin.y, origin.z, salt + 3.0)
		} else if cfg.sample_range_f32_4d(0.0, 1.0, origin.x, origin.y, origin.z, salt + 3.0) < 0.5
		{
			0.0
		} else {
			1.0
		};
		OptionalFaceBand { place, along, depth, along_t }
	}

	pub fn pack_counters(&self, confines: &Confines) -> Result<PackedBitesCounters, FitError> {
		PackedBitesCounters::from_specs(&confines.bounds, &self.passages)
	}
}

/// Geometry resolved from [`BitesStallParameterized`].
#[derive(Debug, Clone, PartialEq)]
pub struct BitesStallPlan {
	pub parameterized: BitesStallParameterized,
	pub counter_aabbs: Vec<Aabb3d>,
	pub kitchen_aabb: Aabb3d,
}

impl BitesStallPlan {
	pub fn from_parameterized(
		params: BitesStallParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = params.pack_counters(confines)?;
		let kitchen_aabb =
			BitesKitchen::pack(&confines.bounds, &packed.counters, &[], BITES_REGION_MIN_PLAN)
				.ok_or(FitError::TooSmall { reason: "bites kitchen" })?;
		Ok(Self { parameterized: params, counter_aabbs: packed.counters, kitchen_aabb })
	}
}
