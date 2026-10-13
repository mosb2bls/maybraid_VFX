//! Per-cell placement sampling ([RFC-183 3.4.1]).
//!
//! Each vegetation cell deterministically samples its instance scale and horizontal offset from
//! authored ranges, biased toward parent-forest means. The cell center is the noise position;
//! small fixed lane offsets keep the per-parameter streams independent.

use bevy_math::{Vec2, Vec3};
use gimme_gen::Cell;
use procedural_common::{NoiseConfig, NoiseParams, UnitRange};

/// Independent noise lanes for per-cell parameter sampling (offsets from the cell center).
const SCALE_LANE: Vec3 = Vec3::new(2.0, 0.0, 0.0);
const OFFSET_X_LANE: Vec3 = Vec3::new(6.0, 0.0, 0.0);
const OFFSET_Z_LANE: Vec3 = Vec3::new(0.0, 0.0, 7.0);

/// Unit-interval preferred means inside grove-authored placement ranges
/// ([RFC-183 3.5.1.1]).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Grove Biases"))]
pub struct ForestGroveBiases {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.5))]
	pub scale_mean: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.5))]
	pub offset_mean: f32,
	/// Shifts the bucket-throw anchor as a fraction of total weight.
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.0))]
	pub bucket_mean_shift: f32,
	/// Scales the authored bucket-weight perturbation strength.
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.0))]
	pub bucket_perturbation_bias: f32,
}

impl Default for ForestGroveBiases {
	fn default() -> Self {
		Self {
			scale_mean: 0.5,
			offset_mean: 0.5,
			bucket_mean_shift: 0.0,
			bucket_perturbation_bias: 0.0,
		}
	}
}

/// Authored ranges sampled independently for each vegetation cell.
///
/// Cell grid footprint and fill density (bucket weights, including `None`) are owned by the
/// grove definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrovePlacementRanges {
	/// Uniform instance scale.
	pub scale: UnitRange,
	/// Signed world-metre shift on each horizontal axis from the cell center.
	pub offset: UnitRange,
}

impl GrovePlacementRanges {
	pub const fn new(scale: UnitRange, offset: UnitRange) -> Self {
		Self { scale, offset }
	}

	/// Sample placement parameters using this cell's center as the deterministic noise position.
	pub fn sample_cell(
		&self,
		biases: &ForestGroveBiases,
		noise: NoiseParams,
		cell: &Cell,
	) -> PlacementSample {
		self.sample_at(biases, noise, cell_center(cell))
	}

	/// Sample placement parameters at an explicit cell center.
	pub fn sample_at(
		&self,
		biases: &ForestGroveBiases,
		noise: NoiseParams,
		cell_center: Vec3,
	) -> PlacementSample {
		let n = NoiseConfig::new(noise);
		PlacementSample {
			scale: biased_sample(
				self.scale,
				biases.scale_mean,
				n.sample_3d(cell_center + SCALE_LANE),
			),
			offset: Vec2::new(
				biased_sample(
					self.offset,
					biases.offset_mean,
					n.sample_3d(cell_center + OFFSET_X_LANE),
				),
				biased_sample(
					self.offset,
					biases.offset_mean,
					n.sample_3d(cell_center + OFFSET_Z_LANE),
				),
			),
		}
	}
}

/// One per-cell placement draw: uniform scale and signed XZ offset in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementSample {
	pub scale: f32,
	pub offset: Vec2,
}

impl PlacementSample {
	/// Candidate world point for this sample in `cell`, before grove-extent validation.
	pub fn position_in(&self, cell: &Cell) -> Vec3 {
		let center = cell_center(cell);
		Vec3::new(center.x + self.offset.x, center.y, center.z + self.offset.y)
	}
}

/// Parent cell center used for placement ownership.
pub fn cell_center(cell: &Cell) -> Vec3 {
	use bevy_math::bounding::BoundingVolume;
	cell.as_region().center().into()
}

/// Map noise in `[-1, 1]` into `range`, centered on `mean_unit` (clamped to the range).
fn biased_sample(range: UnitRange, mean_unit: f32, noise: f32) -> f32 {
	let mean_unit = mean_unit.clamp(0.0, 1.0);
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	let mean = lo + (hi - lo) * mean_unit;
	let radius = (mean - lo).max(hi - mean);
	(mean + noise * radius).clamp(lo, hi)
}

/// Mix world position into a foliage noise seed lane for one placed instance.
pub fn placement_noise(base: NoiseParams, position: Vec3) -> NoiseParams {
	NoiseParams {
		seed: base.seed
			^ position.x.to_bits() as i32
			^ position.z.to_bits() as i32
			^ position.y.to_bits() as i32,
		..base
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn test_ranges() -> GrovePlacementRanges {
		GrovePlacementRanges::new(UnitRange::new(0.8, 1.2), UnitRange::new(-0.2, 0.2))
	}

	#[test]
	fn biased_sample_stays_in_range() -> Result<()> {
		let range = UnitRange::new(0.2, 0.8);
		for noise in [-1.0_f32, 0.0, 1.0] {
			let v = biased_sample(range, 0.5, noise);
			assert!((0.2..=0.8).contains(&v));
		}
		Ok(())
	}

	#[test]
	fn sample_is_deterministic_for_cell_center() -> Result<()> {
		let pos = Vec3::new(10.0, 0.0, 20.0);
		let biases = ForestGroveBiases::default();
		let a = test_ranges().sample_at(&biases, NoiseParams::default(), pos);
		let b = test_ranges().sample_at(&biases, NoiseParams::default(), pos);
		assert_eq!(a, b);
		Ok(())
	}

	#[test]
	fn offset_varies_with_cell_center() -> Result<()> {
		let ranges = GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-1.0, 1.0));
		let biases = ForestGroveBiases::default();
		let a = ranges.sample_at(&biases, NoiseParams::default(), Vec3::new(2.0, 0.5, 4.0));
		let b = ranges.sample_at(&biases, NoiseParams::default(), Vec3::new(2.0, 0.5, 24.0));
		assert_ne!(a.offset, b.offset, "offset should vary when the cell center moves");
		Ok(())
	}

	#[test]
	fn position_in_shifts_from_cell_center() -> Result<()> {
		use bevy_math::bounding::Aabb3d;
		let cell = Cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0)));
		let sample = PlacementSample { scale: 1.0, offset: Vec2::new(1.0, -2.0) };
		let p = sample.position_in(&cell);
		assert!((p.x - 6.0).abs() < 1e-5);
		assert!((p.z - 3.0).abs() < 1e-5);
		Ok(())
	}
}
