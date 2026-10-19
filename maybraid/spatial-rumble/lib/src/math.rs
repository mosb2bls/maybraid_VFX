//! Envelope, distance falloff, and hashed noise. Methods live on the types.

/// Attack / sustain / release over a source lifetime, in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RumbleEnvelope {
	pub attack: f32,
	pub release: f32,
}

impl Default for RumbleEnvelope {
	fn default() -> Self {
		Self { attack: 0.04, release: 0.2 }
	}
}

impl RumbleEnvelope {
	pub fn new(attack: f32, release: f32) -> Self {
		Self { attack: attack.max(0.0), release: release.max(0.0) }
	}

	/// `1` through the sustain, `0` before start and after `duration`.
	pub fn sample(self, age: f32, duration: f32) -> f32 {
		if !duration.is_finite() || duration <= 0.0 || age < 0.0 || age >= duration {
			return 0.0;
		}
		let attack = self.attack.min(duration);
		let release = self.release.min((duration - attack).max(0.0));
		if age < attack {
			return if attack > 0.0 { (age / attack).clamp(0.0, 1.0) } else { 1.0 };
		}
		let release_start = duration - release;
		if age <= release_start {
			return 1.0;
		}
		if release <= 0.0 {
			return 0.0;
		}
		(1.0 - (age - release_start) / release).clamp(0.0, 1.0)
	}
}

/// How [`RumbleFalloff::sample`] interpolates the inner→outer band.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RumbleFalloffCurve {
	/// `(1 - t)²` across the band. Bray / landing presets use this.
	#[default]
	Squared,
	/// Linear `(1 - t)` across the band.
	Linear,
}

/// Full intensity inside `inner_radius`, zero at and beyond `outer_radius`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RumbleFalloff {
	pub inner_radius: f32,
	pub outer_radius: f32,
	pub curve: RumbleFalloffCurve,
}

impl Default for RumbleFalloff {
	fn default() -> Self {
		Self { inner_radius: 2.0, outer_radius: 24.0, curve: RumbleFalloffCurve::Squared }
	}
}

impl RumbleFalloff {
	pub fn new(inner_radius: f32, outer_radius: f32) -> Self {
		let inner = inner_radius.max(0.0);
		Self {
			inner_radius: inner,
			outer_radius: outer_radius.max(inner),
			curve: RumbleFalloffCurve::Squared,
		}
	}

	/// Linear `(1 - t)` band. Inner / outer still clamp like [`Self::new`].
	pub fn linear(inner_radius: f32, outer_radius: f32) -> Self {
		Self { curve: RumbleFalloffCurve::Linear, ..Self::new(inner_radius, outer_radius) }
	}

	/// Interpolate between the inner and outer radii.
	pub fn sample(self, distance: f32) -> f32 {
		if !(distance.is_finite()) || distance >= self.outer_radius {
			return 0.0;
		}
		if distance <= self.inner_radius {
			return 1.0;
		}
		let span = self.outer_radius - self.inner_radius;
		if span <= 0.0 {
			return 0.0;
		}
		let t = ((distance - self.inner_radius) / span).clamp(0.0, 1.0);
		let remain = (1.0 - t).clamp(0.0, 1.0);
		match self.curve {
			RumbleFalloffCurve::Squared => remain * remain,
			RumbleFalloffCurve::Linear => remain,
		}
	}
}

pub fn mix64(mut x: u64) -> u64 {
	x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
	x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
	x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
	x ^ (x >> 31)
}

pub fn unit(seed: u64) -> f32 {
	let bits = mix64(seed) >> 40;
	(bits as f32) / ((1u32 << 24) as f32)
}

pub fn signed(seed: u64) -> f32 {
	unit(seed) * 2.0 - 1.0
}

/// Deterministic sample in `[-1, 1]` for a source clock.
pub fn noise(seed: u64, age_ticks: u32, axis: u32) -> f32 {
	signed(mix64(seed ^ (u64::from(axis) << 32) ^ u64::from(age_ticks).wrapping_mul(0x45d9_f3bb)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn falloff_is_full_inside_and_zero_outside() {
		let falloff = RumbleFalloff::new(4.0, 12.0);
		assert!((falloff.sample(0.0) - 1.0).abs() < 1e-5);
		assert!((falloff.sample(4.0) - 1.0).abs() < 1e-5);
		assert!(falloff.sample(12.0).abs() < 1e-5);
		assert!(falloff.sample(20.0).abs() < 1e-5);
	}

	#[test]
	fn falloff_decreases_through_the_band() {
		let falloff = RumbleFalloff::new(0.0, 10.0);
		let near = falloff.sample(2.0);
		let mid = falloff.sample(5.0);
		let far = falloff.sample(8.0);
		assert!(near > mid && mid > far);
		assert!((mid - 0.25).abs() < 1e-5);
	}

	#[test]
	fn inverted_radii_collapse_to_a_valid_band() {
		let falloff = RumbleFalloff::new(10.0, 2.0);
		assert!((falloff.inner_radius - 10.0).abs() < 1e-5);
		assert!((falloff.outer_radius - 10.0).abs() < 1e-5);
		assert!((falloff.sample(1.0) - 1.0).abs() < 1e-5);
		assert!(falloff.sample(10.0).abs() < 1e-5);
	}

	#[test]
	fn linear_band_matches_one_minus_normalized_distance() {
		let falloff = RumbleFalloff::linear(0.0, 10.0);
		assert!((falloff.sample(0.0) - 1.0).abs() < 1e-5);
		assert!((falloff.sample(4.0) - 0.6).abs() < 1e-5);
		assert!(falloff.sample(10.0).abs() < 1e-5);
		assert!(falloff.sample(4.0) > RumbleFalloff::new(0.0, 10.0).sample(4.0));
	}

	#[test]
	fn envelope_attacks_holds_and_releases() {
		let envelope = RumbleEnvelope::new(0.2, 0.2);
		assert!(envelope.sample(-0.1, 1.0).abs() < 1e-5);
		assert!((envelope.sample(0.1, 1.0) - 0.5).abs() < 1e-5);
		assert!((envelope.sample(0.5, 1.0) - 1.0).abs() < 1e-5);
		assert!((envelope.sample(0.9, 1.0) - 0.5).abs() < 1e-5);
		assert!(envelope.sample(1.0, 1.0).abs() < 1e-5);
		assert!(envelope.sample(0.2, 0.0).abs() < 1e-5);
	}

	#[test]
	fn noise_is_stable_and_axis_distinct() {
		assert!((noise(7, 3, 1) - noise(7, 3, 1)).abs() < 1e-6);
		assert!((noise(7, 3, 1) - noise(7, 3, 2)).abs() > 1e-4);
		assert!((noise(1u64 << 40, 0, 0) - noise((1u64 << 40) + 1, 0, 0)).abs() > 1e-6);
	}
}
