//! [`BuildWithNoise`] for [`CommonTuftClump`].
use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::BladeTuftShape;

use crate::common_tufts::CommonTuftClump;

/// Sample a clump's authored geometry ranges into a blade tuft shape.
///
/// Blade width is **length-proportional** (`length * width_factor`), so short and tall
/// varietals stay equally grass-thin.
impl BuildWithNoise<BladeTuftShape> for CommonTuftClump {
	fn build_with_noise(&self, noise: NoiseParams) -> BladeTuftShape {
		let config = NoiseConfig::new(noise);
		let sample_f32 = |range: UnitRange, salt| {
			let lo = range.start.min(range.end);
			let hi = range.start.max(range.end);
			config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
		};

		let sample_u32 = |range: &std::ops::RangeInclusive<u32>, salt| {
			let lo = *range.start() as usize;
			let hi = (*range.end() as usize).saturating_add(1);
			config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
		};

		let blade_length = sample_f32(self.height, 1.0).max(0.05);
		let blade_width = blade_length * sample_f32(self.width_factor, 2.0);

		BladeTuftShape {
			blade_count: sample_u32(&self.blade_count, 3.0),
			blade_length,
			blade_width,
			max_tilt_radians: sample_f32(self.max_tilt_radians, 4.0).max(0.01),
			bend_segments: sample_u32(&self.bend_segments, 5.0).max(1),
			seed: noise.seed,
			..BladeTuftShape::default()
		}
	}
}

/// A shape for a common tuft clump.
///
/// We store the shape under a separate type for querability.
/// This plugs with the gimme-gen API: https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-142-gimme#34-hierarchical-generation
#[derive(Debug, Clone, PartialEq)]
pub struct CommonTuftClumpShape(pub BladeTuftShape);

impl CommonTuftClumpShape {
	/// Build a shape from noise and a clump.
	///
	/// RFC-style: https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-142-gimme#34-hierarchical-generation
	pub fn from_noise_and_clump(noise: NoiseParams, clump: &CommonTuftClump) -> Self {
		Self(clump.build_with_noise(noise))
	}
}
