//! Ordered weighted grove buckets and per-cell variant selection ([RFC-183 3.4.2]).
//!
//! Selection happens in two phases: [`GroveDistribution::prepare`] perturbs bucket weights once
//! per grove instance ([RFC-183 3.4.2.1.2]), then [`PreparedGroveDistribution::select_at`] runs a
//! bucket throw plus ordered first-fit walk for each cell ([RFC-183 3.4.2.5]).

use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::{
	perturb_weights, BucketThrow, FirstFitIndices, NoiseConfig, NoiseParams, MIN_BUCKET_WEIGHT,
};

use crate::grove::terrain::{GroveWorldSample, PlacementConstraints};
use crate::grove::GroveCellOutcome;

/// Noise lane for the per-cell bucket throw (offset from the placement position).
const SELECTION_LANE: Vec3 = Vec3::new(30.0, 0.0, 0.0);
/// Noise lane base for per-bucket weight perturbation (offset from the perturbation origin).
const PERTURBATION_LANE_BASE: f32 = 20.0;

/// One ordered bucket: weight, constraints, and an optional variant (`None` = empty cell).
#[derive(Debug, Clone, PartialEq)]
pub struct GroveBucket<V> {
	pub weight: f32,
	pub constraints: PlacementConstraints,
	pub item: Option<V>,
}

impl<V> GroveBucket<V> {
	/// Explicit `None` bucket: wins first-fit anywhere and leaves the cell empty.
	pub const fn none(weight: f32) -> Self {
		Self { weight, constraints: PlacementConstraints::UNCONSTRAINED, item: None }
	}

	pub const fn placed(weight: f32, constraints: PlacementConstraints, variant: V) -> Self {
		Self { weight, constraints, item: Some(variant) }
	}

	/// Whether this bucket may occupy `position` on `terrain`. `None` buckets always pass.
	///
	/// Authored [`PlacementConstraints`] are kept on the bucket but not applied — elevation
	/// / steepness bands need a forest-layer normalization domain.
	pub fn valid_at(&self, position: Vec3, world: &impl GroveWorldSample) -> bool {
		self.item.is_none() || world.allows_placement_at(position)
	}
}

/// Authored ordered weighted variants for one grove.
#[derive(Debug, Clone, PartialEq)]
pub struct GroveDistribution<V> {
	pub buckets: Vec<GroveBucket<V>>,
	/// Base perturbation strength before forest bias scaling ([RFC-183 3.4.2.1.2]).
	pub base_bucket_perturbation_strength: f32,
}

impl<V> GroveDistribution<V> {
	pub fn new(buckets: Vec<GroveBucket<V>>) -> Self {
		Self { buckets, base_bucket_perturbation_strength: 0.0 }
	}

	pub fn with_perturbation_strength(mut self, strength: f32) -> Self {
		self.base_bucket_perturbation_strength = strength;
		self
	}

	pub fn len(&self) -> usize {
		self.buckets.len()
	}

	pub fn is_empty(&self) -> bool {
		self.buckets.is_empty()
	}

	/// Perturb bucket weights once at `perturbation_origin` and freeze the throw for many cell
	/// draws. `bucket_mean_shift` and `bucket_perturbation_bias` come from the parent forest.
	pub fn prepare(
		self,
		bucket_mean_shift: f32,
		bucket_perturbation_bias: f32,
		noise: NoiseParams,
		perturbation_origin: Vec3,
	) -> PreparedGroveDistribution<V> {
		// Zero-weight buckets are excluded from the throw but stay in the ordered first-fit
		// walk, so a zeroed `None` bucket cannot shadow its placed neighbors.
		let mut base = BucketThrow::new();
		let mut throw_bucket_indices = Vec::new();
		for (index, bucket) in self.buckets.iter().enumerate() {
			if base.add(bucket.weight) {
				throw_bucket_indices.push(index);
			}
		}

		let strength = self.base_bucket_perturbation_strength * (1.0 + bucket_perturbation_bias);
		let throw = if strength.abs() <= f32::EPSILON || base.is_empty() {
			base
		} else {
			let n = NoiseConfig::new(noise);
			let bucket_noises: Vec<f32> = throw_bucket_indices
				.iter()
				.map(|&index| {
					n.sample_3d(
						perturbation_origin
							+ Vec3::new(PERTURBATION_LANE_BASE + index as f32, 0.0, 0.0),
					)
				})
				.collect();
			perturb_weights(&base, strength, &bucket_noises, MIN_BUCKET_WEIGHT)
		};
		let mean_anchor = bucket_mean_shift * throw.total_weight();
		PreparedGroveDistribution {
			buckets: self.buckets,
			bucket_throw: throw.with_mean_anchor(mean_anchor),
			throw_bucket_indices,
		}
	}
}

/// Grove distribution with bucket weights perturbed once per grove instance.
#[derive(Debug, Clone)]
pub struct PreparedGroveDistribution<V> {
	buckets: Vec<GroveBucket<V>>,
	bucket_throw: BucketThrow,
	/// Maps compressed [`BucketThrow`] indices back to [`Self::buckets`] indices.
	throw_bucket_indices: Vec<usize>,
}

impl<V: Clone> PreparedGroveDistribution<V> {
	pub fn is_empty(&self) -> bool {
		self.buckets.is_empty()
	}

	/// Throw into the weighted buckets at `position`, then first-fit walk to a valid variant.
	pub fn select_at(
		&self,
		position: Vec3,
		scale: f32,
		cell: Cell,
		noise: NoiseParams,
		world: &impl GroveWorldSample,
	) -> GroveCellOutcome<V> {
		if self.is_empty() {
			return GroveCellOutcome::Rejected { position };
		}
		let selection_noise = NoiseConfig::new(noise).sample_3d(position + SELECTION_LANE);
		let throw = selection_noise * self.bucket_throw.total_weight() * 0.5;
		let throw_index = self.bucket_throw.select(throw).unwrap_or(0);
		let start = self.throw_bucket_indices.get(throw_index).copied().unwrap_or(0);
		self.select_from(start, position, scale, cell, world)
	}

	/// First-fit walk from a known starting bucket index (also used by tests and debugging).
	pub fn select_from(
		&self,
		start: usize,
		position: Vec3,
		scale: f32,
		_cell: Cell,
		world: &impl GroveWorldSample,
	) -> GroveCellOutcome<V> {
		for index in FirstFitIndices::new(self.buckets.len(), start) {
			let bucket = &self.buckets[index];
			if !bucket.valid_at(position, world) {
				continue;
			}
			return match &bucket.item {
				Some(variant) => {
					GroveCellOutcome::Placed { variant: variant.clone(), position, scale }
				}
				None => GroveCellOutcome::Empty { position },
			};
		}
		GroveCellOutcome::Rejected { position }
	}
}

/// Per-bucket weight overrides aligned with [`GroveDistribution::buckets`] order.
///
/// `None` at index *i* keeps the authored weight for bucket *i*; `Some(w)` replaces it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VariantWeightOverrides {
	pub slots: Vec<Option<f32>>,
}

impl VariantWeightOverrides {
	pub fn apply_to<V>(&self, distribution: &mut GroveDistribution<V>) -> Result<(), String> {
		if self.slots.len() != distribution.buckets.len() {
			return Err(format!(
				"variant weight count {} does not match bucket count {}",
				self.slots.len(),
				distribution.buckets.len()
			));
		}
		for (slot, bucket) in self.slots.iter().zip(&mut distribution.buckets) {
			if let Some(weight) = slot {
				bucket.weight = *weight;
			}
		}
		Ok(())
	}
}

/// Parse `--variant-weights 1.0,x,2.5,3.0,x` (one slot per bucket; `x` keeps the default).
pub fn parse_variant_weights(s: &str) -> Result<VariantWeightOverrides, String> {
	let slots = s
		.split(',')
		.map(str::trim)
		.filter(|part| !part.is_empty())
		.map(|part| {
			if part.eq_ignore_ascii_case("x") {
				return Ok(None);
			}
			part.parse::<f32>()
				.map(Some)
				.map_err(|e| format!("invalid variant weight {part:?}: {e}"))
		})
		.collect::<Result<Vec<_>, String>>()?;
	if slots.is_empty() {
		return Err("expected at least one variant weight slot".into());
	}
	Ok(VariantWeightOverrides { slots })
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::terrain::FlatTerrainSample;
	use anyhow::Result;
	use procedural_common::UnitRange;

	fn flat(elevation: f32, steepness: f32) -> FlatTerrainSample {
		FlatTerrainSample { elevation, steepness }
	}

	fn prepared<V: Clone>(dist: GroveDistribution<V>) -> PreparedGroveDistribution<V> {
		dist.prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO)
	}

	#[test]
	fn none_bucket_yields_empty() -> Result<()> {
		let dist: GroveDistribution<()> = GroveDistribution::new(vec![GroveBucket::none(1.0)]);
		let outcome = prepared(dist).select_at(
			Vec3::ZERO,
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			NoiseParams::default(),
			&flat(0.5, 0.1),
		);
		assert!(matches!(outcome, GroveCellOutcome::Empty { .. }));
		Ok(())
	}

	#[test]
	fn constraints_are_not_evaluated() -> Result<()> {
		let dist = GroveDistribution::new(vec![GroveBucket::placed(
			1.0,
			PlacementConstraints::new(UnitRange::new(0.8, 1.0), UnitRange::new(0.0, 0.1)),
			"high_only",
		)]);
		let outcome = prepared(dist).select_from(
			0,
			Vec3::new(5.0, 0.0, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&flat(0.3, 0.2),
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => assert_eq!(variant, "high_only"),
			other => anyhow::bail!("expected Placed high_only, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn none_bucket_ignores_constraints() -> Result<()> {
		let dist: GroveDistribution<()> = GroveDistribution::new(vec![GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::new(
				UnitRange::new(0.9, 1.0),
				UnitRange::new(0.0, 0.1),
			),
			item: None,
		}]);
		let outcome = prepared(dist).select_at(
			Vec3::ZERO,
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			NoiseParams::default(),
			&flat(0.0, 0.99),
		);
		assert!(matches!(outcome, GroveCellOutcome::Empty { .. }));
		Ok(())
	}

	#[test]
	fn exclusion_zone_rejects_placed_buckets() -> Result<()> {
		use bevy_math::bounding::Aabb3d;

		struct Blocked {
			zones: Vec<Aabb3d>,
		}
		impl GroveWorldSample for Blocked {
			fn height_at(&self, _position: Vec3) -> f32 {
				0.0
			}
			fn steepness_at(&self, _position: Vec3) -> f32 {
				0.0
			}
			fn exclusion_zones(&self) -> &[Aabb3d] {
				&self.zones
			}
		}

		let dist = GroveDistribution::new(vec![GroveBucket::placed(
			1.0,
			PlacementConstraints::UNCONSTRAINED,
			"tree",
		)]);
		let outcome = prepared(dist).select_at(
			Vec3::new(0.5, 0.0, 0.5),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			NoiseParams::default(),
			&Blocked { zones: vec![Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)] },
		);
		assert!(matches!(outcome, GroveCellOutcome::Rejected { .. }));
		Ok(())
	}

	#[test]
	fn zero_weight_none_bucket_does_not_shadow_placed_variants() -> Result<()> {
		let mut dist = crate::braid_grass::BraidGrassCell::distribution();
		dist.buckets[0].weight = 0.0;
		dist.buckets[1].weight = 9.0;
		let noise = NoiseParams::from_scalar(0.0, 1.0, 0.0, 1);
		let prepared = dist.prepare(0.0, 0.0, noise, Vec3::ZERO);
		let terrain = flat(0.4, 0.1);
		let mut placed = 0usize;
		for x in 0..3 {
			for z in 0..3 {
				let position = Vec3::new(x as f32 * 4.25, 0.0, z as f32 * 4.25);
				if matches!(
					prepared.select_at(
						position,
						1.0,
						Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
						noise,
						&terrain
					),
					GroveCellOutcome::Placed { .. }
				) {
					placed += 1;
				}
			}
		}
		assert!(placed > 0, "expected placements when None weight is zero, got {placed}");
		Ok(())
	}

	#[test]
	fn selection_is_deterministic() -> Result<()> {
		let dist = GroveDistribution::new(vec![
			GroveBucket::none(1.0),
			GroveBucket::placed(2.0, PlacementConstraints::UNCONSTRAINED, "tree"),
		]);
		let prepared = prepared(dist);
		let position = Vec3::new(100.0, 0.0, 50.0);
		let a = prepared.select_at(
			position,
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			NoiseParams::default(),
			&flat(0.4, 0.1),
		);
		let b = prepared.select_at(
			position,
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			NoiseParams::default(),
			&flat(0.4, 0.1),
		);
		assert_eq!(a, b);
		Ok(())
	}

	#[test]
	fn parse_variant_weights_accepts_defaults_and_overrides() -> Result<()> {
		let overrides =
			parse_variant_weights("1.0,x,2.5,3.0,x").map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(overrides.slots, vec![Some(1.0), None, Some(2.5), Some(3.0), None]);
		Ok(())
	}

	#[test]
	fn apply_variant_weights_updates_only_set_slots_and_rejects_mismatch() -> Result<()> {
		let mut dist = GroveDistribution::new(vec![
			GroveBucket::none(2.5),
			GroveBucket::placed(2.0, PlacementConstraints::UNCONSTRAINED, "a"),
			GroveBucket::placed(1.0, PlacementConstraints::UNCONSTRAINED, "b"),
		]);
		let overrides = parse_variant_weights("0.1,x,4.0").map_err(|e| anyhow::anyhow!("{e}"))?;
		overrides.apply_to(&mut dist).map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(dist.buckets[0].weight, 0.1);
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].weight, 4.0);

		let mismatched = VariantWeightOverrides { slots: vec![Some(1.0)] };
		assert!(mismatched.apply_to(&mut dist).is_err());
		Ok(())
	}
}
