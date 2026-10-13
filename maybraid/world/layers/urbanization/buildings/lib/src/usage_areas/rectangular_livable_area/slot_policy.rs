//! Host-relative carve policy for open/closed quarters inside one RLA.

use super::RectQuarterKind;

/// How aggressively a kind claims a free host rect.
#[derive(Debug, Clone, Copy)]
pub struct SlotPolicy {
	/// Aspirational footprint (m²), clamped to host share.
	pub target_frac: f32,
	pub target_min: f32,
	pub target_max: f32,
	/// Absolute minimum area (m²) before attempting a fit.
	pub min_area: f32,
	/// Minimum edge (m).
	pub min_dim: f32,
	/// Take the whole host when `host_a < target * carve_threshold`.
	pub carve_threshold: f32,
	/// Fraction of host taken when carving (clamped).
	pub carve_frac_lo: f32,
	pub carve_frac_hi: f32,
}

impl SlotPolicy {
	pub const fn for_kind(kind: RectQuarterKind) -> Self {
		match kind {
			RectQuarterKind::Bedroom => Self {
				target_frac: 0.0,
				target_min: 12.0,
				target_max: 12.0,
				min_area: 12.0,
				min_dim: 3.2,
				carve_threshold: 1.7,
				carve_frac_lo: 0.28,
				carve_frac_hi: 0.65,
			},
			RectQuarterKind::Living => Self {
				target_frac: 0.42,
				target_min: 12.0,
				target_max: 42.0,
				min_area: 9.0,
				min_dim: 2.2,
				carve_threshold: 2.2,
				carve_frac_lo: 0.38,
				carve_frac_hi: 0.75,
			},
			// Compact kitchen pocket; leave remainder for living/sitting.
			RectQuarterKind::Eating => Self {
				target_frac: 0.24,
				target_min: 6.0,
				target_max: 22.0,
				min_area: 5.0,
				min_dim: 2.2,
				carve_threshold: 2.6,
				carve_frac_lo: 0.22,
				carve_frac_hi: 0.48,
			},
			RectQuarterKind::Kitchen => Self {
				target_frac: 0.2,
				target_min: 5.0,
				target_max: 16.0,
				min_area: 4.8,
				min_dim: 2.2,
				carve_threshold: 2.6,
				carve_frac_lo: 0.22,
				carve_frac_hi: 0.45,
			},
			RectQuarterKind::Dining => Self {
				target_frac: 0.18,
				target_min: 5.0,
				target_max: 14.0,
				min_area: 4.4,
				min_dim: 2.2,
				carve_threshold: 2.2,
				carve_frac_lo: 0.38,
				carve_frac_hi: 0.75,
			},
			RectQuarterKind::Bathroom => Self {
				target_frac: 0.0,
				target_min: 6.5,
				target_max: 6.5,
				min_area: 4.5,
				min_dim: 1.6,
				carve_threshold: 1.7,
				carve_frac_lo: 0.28,
				carve_frac_hi: 0.65,
			},
			RectQuarterKind::HalfBath => Self {
				target_frac: 0.0,
				target_min: 3.5,
				target_max: 3.5,
				min_area: 2.0,
				min_dim: 1.6,
				carve_threshold: 1.7,
				carve_frac_lo: 0.28,
				carve_frac_hi: 0.65,
			},
			RectQuarterKind::Sitting => Self {
				target_frac: 0.28,
				target_min: 6.0,
				target_max: 24.0,
				min_area: 5.0,
				min_dim: 2.2,
				carve_threshold: 2.2,
				carve_frac_lo: 0.38,
				carve_frac_hi: 0.75,
			},
			RectQuarterKind::Study => Self {
				target_frac: 0.0,
				target_min: 9.0,
				target_max: 9.0,
				min_area: 5.0,
				min_dim: 2.2,
				carve_threshold: 1.7,
				carve_frac_lo: 0.28,
				carve_frac_hi: 0.65,
			},
		}
	}

	pub fn target_area(self, host_a: f32) -> f32 {
		if self.target_frac <= 0.0 {
			self.target_min
		} else {
			(host_a * self.target_frac).clamp(self.target_min, self.target_max)
		}
	}

	pub fn carve_frac(self, host_a: f32) -> f32 {
		let want = self.target_area(host_a);
		(want / host_a.max(1e-3)).clamp(self.carve_frac_lo, self.carve_frac_hi)
	}
}

pub fn min_area_for(kind: RectQuarterKind) -> f32 {
	SlotPolicy::for_kind(kind).min_area
}
