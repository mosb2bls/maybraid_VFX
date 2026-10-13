//! Shared authored tuft-patch wrapper for well-known groves.

use std::ops::RangeInclusive;

use procedural_common::UnitRange;

#[cfg(feature = "render")]
use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams};
#[cfg(feature = "render")]
use sbs_geometry::BladeTuftShape;
#[cfg(feature = "render")]
use sbs_trees::tuft_patch::TuftPatchParams;

/// Authored tuft-patch layout around a grove's blade clump geometry `C`: a few blade tufts
/// scattered over an XZ footprint instead of radiating from a single anchor
/// ([`sbs_trees::tuft_patch::TuftPatch`]).
#[derive(Debug, Clone, PartialEq)]
pub struct GroveTuftPatch<C> {
	pub clump: C,
	pub clump_count: RangeInclusive<u32>,
	/// Square patch footprint side length (m).
	pub patch_extent_xz: UnitRange,
	/// Per-clump blade base scatter radius (m, [`BladeTuftShape::base_spread`]); keeps
	/// individual clumps reading as loose mounds instead of radiating cones.
	pub base_spread: UnitRange,
}

#[cfg(feature = "render")]
impl<C> GroveTuftPatch<C>
where
	C: BuildWithNoise<BladeTuftShape>,
{
	/// Build [`TuftPatchParams`] for one placement: layout sampled from `noise`
	/// (salt lanes `6`–`8`, past the wrapped clump's geometry lanes), blade shape from the
	/// wrapped clump with the authored base spread applied.
	pub fn build_tuft_patch(&self, noise: NoiseParams) -> TuftPatchParams {
		let config = NoiseConfig::new(noise);
		let clump_count = {
			let lo = *self.clump_count.start() as usize;
			let hi = (*self.clump_count.end() as usize).saturating_add(1);
			config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, 6.0) as u32
		};
		let patch_extent_xz = config.sample_range_f32_4d(
			self.patch_extent_xz.start.min(self.patch_extent_xz.end),
			self.patch_extent_xz.start.max(self.patch_extent_xz.end),
			0.0,
			0.0,
			0.0,
			7.0,
		);
		let mut shape = self.clump.build_with_noise(noise);
		shape.base_spread = config.sample_range_f32_4d(
			self.base_spread.start.min(self.base_spread.end),
			self.base_spread.start.max(self.base_spread.end),
			0.0,
			0.0,
			0.0,
			8.0,
		);
		TuftPatchParams::new(clump_count, patch_extent_xz, shape)
	}
}
