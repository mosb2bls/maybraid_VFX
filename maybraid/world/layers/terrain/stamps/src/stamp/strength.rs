//! Per-leaf stamp amplitude via a unitless strength factor.
//!
//! Horizontal footprint stays leaf/cell-driven (`*_frac` × short edge).
//! Vertical knobs (`lift`, `depth`, `amplitude`, …) scale with strength.
//! Near-1.0 scales (`crest_scale`, …) lerp toward identity at strength 0.

/// Scale an additive relief knob (`lift`, `depth`, `amplitude`, `tilt`, …).
#[inline]
pub fn scale_additive(value: f32, strength: f32) -> f32 {
	value * strength.max(0.0)
}

/// Scale a multiplier near 1.0 so strength 0 → identity, strength 1 → `value`.
#[inline]
pub fn scale_near_one(value: f32, strength: f32) -> f32 {
	1.0 + (value - 1.0) * strength.max(0.0)
}

/// Apply a relative amplitude to stamp authoring params (`1.0` ≈ defaults).
pub trait StampStrength {
	fn with_strength(self, strength: f32) -> Self;
}
