//! Curved frond spine sampling.

use bevy_math::Vec3;

use super::config::FrondConfig;

/// Spine position in frond-local space (+X along length, optional mid arch, −Y droop at tip).
pub fn spine_at(config: &FrondConfig, t: f32) -> Vec3 {
	let t = t.clamp(0.0, 1.0);
	let x = t * config.length;
	let y = config.arch_lift * 4.0 * t * (1.0 - t) - config.droop * t * t;
	Vec3::new(x, y, 0.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn spine_starts_at_origin_and_drops() -> Result<()> {
		let config = FrondConfig::default();
		assert!(spine_at(&config, 0.0).length() < 1e-5);
		assert!(spine_at(&config, 1.0).y < spine_at(&config, 0.5).y);
		Ok(())
	}

	#[test]
	fn spine_arch_lift_peaks_before_tip_droop() -> Result<()> {
		let config = FrondConfig { arch_lift: 0.4, droop: 0.5, ..FrondConfig::default() };
		assert!(spine_at(&config, 0.5).y > spine_at(&config, 0.0).y);
		assert!(spine_at(&config, 1.0).y < spine_at(&config, 0.5).y);
		Ok(())
	}
}
