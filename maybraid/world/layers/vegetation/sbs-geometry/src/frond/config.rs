//! Per-frond geometry ([RFC-183 §3.1.2.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/07-fronds/README.md)).

/// Geometry for one arching frond strand (spine + leaflets).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrondConfig {
	/// Spine polyline sample count (reserved for segmented spine meshing).
	pub segments: u32,
	/// Arc length along the spine from anchor to tip.
	pub length: f32,
	/// Leaflet half-width at the anchor (tapers toward the tip).
	pub width: f32,
	/// Quadratic droop coefficient (−droop · t² on the spine at the tip).
	pub droop: f32,
	/// Mid-span lift (∝ t·(1−t)) before tip droop; arch-up-then-over when paired with [`Self::droop`].
	pub arch_lift: f32,
	/// Roll about the spine tangent at normalized height `t`.
	pub twist: f32,
	/// Leaflet count along the spine (≥ 2).
	pub leaflet_count: u32,
}

impl Default for FrondConfig {
	fn default() -> Self {
		Self {
			segments: 8,
			length: 1.4,
			width: 0.18,
			droop: 0.55,
			arch_lift: 0.0,
			twist: 0.35,
			leaflet_count: 12,
		}
	}
}
