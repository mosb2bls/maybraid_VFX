/// Blend two animations by weight: 0 = `from`, 1 = `to`.
#[derive(Debug, Clone)]
pub struct Mix<A, B> {
	pub from: A,
	pub to: B,
	pub weight: f32,
}

impl<A, B> Mix<A, B> {
	pub fn new(from: A, to: B, weight: f32) -> Self {
		Self { from, to, weight: weight.clamp(0.0, 1.0) }
	}
}

/// Like [`Mix`], but eases the blend weight with smoothstep.
#[derive(Debug, Clone)]
pub struct Smooth<A, B> {
	pub from: A,
	pub to: B,
	pub weight: f32,
}

impl<A, B> Smooth<A, B> {
	pub fn new(from: A, to: B, weight: f32) -> Self {
		Self { from, to, weight: weight.clamp(0.0, 1.0) }
	}

	pub fn into_mix(self) -> Mix<A, B> {
		let t = smoothstep(self.weight);
		Mix::new(self.from, self.to, t)
	}
}

pub fn smoothstep(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

/// Linear blend between two scalars (`t` is not clamped).
pub struct LinearBlend;

impl LinearBlend {
	#[inline]
	pub fn eval(a: f32, b: f32, t: f32) -> f32 {
		a + (b - a) * t
	}
}

/// Default fade width for [`burst_envelope`] (fraction of the unit interval at each end).
pub const DEFAULT_BURST_EDGE: f32 = 0.22;

/// Smoothstep ramps at both ends of `[0, 1]` with a flat plateau in the middle.
///
/// `edge` is each fade's width as a fraction of the interval (e.g. `0.22` ≈ 22%).
/// Callers pass `u` already normalized to the burst; values outside `[0, 1]` are
/// not clamped here so period wraps stay quiet when the burst window excludes them.
pub fn burst_envelope(u: f32, edge: f32) -> f32 {
	if u < edge {
		smoothstep(u / edge)
	} else if u > 1.0 - edge {
		smoothstep((1.0 - u) / edge)
	} else {
		1.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn smoothstep_endpoints() -> anyhow::Result<()> {
		assert!(smoothstep(0.0).abs() < 1e-5);
		assert!((smoothstep(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn burst_envelope_peaks_in_middle() -> anyhow::Result<()> {
		let edge = DEFAULT_BURST_EDGE;
		assert!(burst_envelope(0.0, edge) < 0.05);
		assert!((burst_envelope(0.5, edge) - 1.0).abs() < 1e-5);
		assert!(burst_envelope(1.0, edge) < 0.05);
		Ok(())
	}

	#[test]
	fn mix_clamps_weight() {
		let mix = Mix::new((), (), 1.5);
		assert_eq!(mix.weight, 1.0);
	}
}
