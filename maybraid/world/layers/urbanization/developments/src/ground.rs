//! [`SiteGround`]: the ground under a development site, as its layout probes it.

use bevy_math::Vec2;

use crate::PadPlan;

const SITE_SAMPLE_SIDE: usize = 9;
const SITE_HEIGHT_QUANTILE: f32 = 0.95;

/// The ground under one development site, as its layout algorithms read it.
pub trait SiteGround {
	/// Ground height at `(x, z)`, or `None` where the site has no ground.
	fn height_at(&mut self, x: f32, z: f32) -> Option<f32>;

	/// Whether water meets the ground `pad` would reshape.
	fn hydro_overlaps(&mut self, pad: &PadPlan) -> bool;

	/// Robust high elevation over a yawed rectangular support.
	///
	/// A dense grid over the complete pad influence catches uphill terrain outside
	/// the flatten core. The 95th percentile sits close to that local high without
	/// letting one narrow terrain spike lift the entire terrace.
	fn height_upper_on_rect(&mut self, center: Vec2, half: Vec2, yaw: f32) -> Option<f32> {
		let (sin, cos) = yaw.sin_cos();
		let mut heights = [0.0; SITE_SAMPLE_SIDE * SITE_SAMPLE_SIDE];
		let mut count = 0;
		for iz in 0..SITE_SAMPLE_SIDE {
			for ix in 0..SITE_SAMPLE_SIDE {
				let u = ix as f32 / (SITE_SAMPLE_SIDE - 1) as f32;
				let v = iz as f32 / (SITE_SAMPLE_SIDE - 1) as f32;
				let local = Vec2::new(-half.x + 2.0 * half.x * u, -half.y + 2.0 * half.y * v);
				let p = center
					+ Vec2::new(cos * local.x + sin * local.y, -sin * local.x + cos * local.y);
				if let Some(h) = self.height_at(p.x, p.y) {
					heights[count] = h;
					count += 1;
				}
			}
		}
		upper_quantile(&mut heights[..count], SITE_HEIGHT_QUANTILE)
	}
}

fn upper_quantile(values: &mut [f32], quantile: f32) -> Option<f32> {
	if values.is_empty() {
		return None;
	}
	let index = ((values.len() - 1) as f32 * quantile.clamp(0.0, 1.0)).ceil() as usize;
	let (_, value, _) = values.select_nth_unstable_by(index, f32::total_cmp);
	Some(*value)
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;

	/// Level ground at `height`, wet everywhere or nowhere.
	pub(crate) struct FlatGround {
		pub height: f32,
		pub wet: bool,
	}

	impl SiteGround for FlatGround {
		fn height_at(&mut self, _x: f32, _z: f32) -> Option<f32> {
			Some(self.height)
		}

		fn hydro_overlaps(&mut self, _pad: &PadPlan) -> bool {
			self.wet
		}
	}

	#[test]
	fn upper_site_height_ignores_one_narrow_spike() -> anyhow::Result<()> {
		let mut heights: Vec<f32> = (0..80).map(|i| i as f32).collect();
		heights.push(1000.0);
		let high = upper_quantile(&mut heights, SITE_HEIGHT_QUANTILE)
			.ok_or_else(|| anyhow::anyhow!("expected a quantile"))?;
		anyhow::ensure!(high >= 70.0, "site height should remain near the local high: {high}");
		anyhow::ensure!(high < 1000.0, "one spike should not lift the terrace: {high}");
		Ok(())
	}
}
