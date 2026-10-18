//! Leaf / cell domain mask: hard-clip outside bounds + ease-out near the edge.
//!
//! Composes with construction softmasks as
//! \(w_{\mathrm{final}} = w_{\mathrm{construction}}\,w_{\mathrm{cell}}\) via
//! identity blending:
//!
//! \[
//! \widetilde M(x,z) = z + w_{\mathrm{cell}}(x)\bigl(M(x,z)-z\bigr).
//! \]
//!
//! Outside the cell, \(w_{\mathrm{cell}}=0\) exactly (identity), so neighboring
//! terrain chunks may omit the modulation.

use bevy_math::Vec2;
use procedural_common::{Bounds2, Smoothstep01};

/// Default ease band (world units) inside the cell edge.
pub const DEFAULT_CELL_DOMAIN_EASE: f32 = 20.0;

/// Max fraction of the short cell edge used for the ease band.
pub const CELL_DOMAIN_EASE_FRAC: f32 = 0.15;

/// Axis-aligned cell domain with hard exterior clip and interior edge ease.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellDomainMask {
	pub bounds: Bounds2,
	/// Interior distance over which weight rises from 0 (at the edge) to 1.
	pub ease_width: f32,
}

impl CellDomainMask {
	pub fn new(bounds: Bounds2, ease_width: f32) -> Self {
		Self { bounds, ease_width: ease_width.max(0.0) }
	}

	/// Ease width clamped to a fraction of the short cell edge.
	pub fn for_bounds(bounds: Bounds2) -> Self {
		let short = bounds.extent().min_element().max(1e-3);
		let ease = DEFAULT_CELL_DOMAIN_EASE.min(short * CELL_DOMAIN_EASE_FRAC);
		Self::new(bounds, ease)
	}

	/// Weight in `[0, 1]`: `0` outside / on the exterior, `1` deep inside.
	pub fn weight(&self, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return 0.0;
		}
		let dist = interior_edge_distance(self.bounds, p);
		let ease = self.ease_width;
		if ease <= 1e-6 {
			return 1.0;
		}
		if dist >= ease {
			1.0
		} else {
			Smoothstep01::eval(dist / ease)
		}
	}
}

/// Positive distance from `p` to the nearest edge while inside `bounds`.
fn interior_edge_distance(bounds: Bounds2, p: Vec2) -> f32 {
	let dx = (p.x - bounds.min.x).min(bounds.max.x - p.x);
	let dz = (p.y - bounds.min.y).min(bounds.max.y - p.y);
	dx.min(dz).max(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn outside_is_hard_zero() -> Result<()> {
		let domain = CellDomainMask::new(Bounds2::from_xz(0.0, 0.0, 100.0, 100.0), 10.0);
		assert_eq!(domain.weight(-1.0, 50.0), 0.0);
		assert_eq!(domain.weight(50.0, 101.0), 0.0);
		Ok(())
	}

	#[test]
	fn deep_interior_is_one() -> Result<()> {
		let domain = CellDomainMask::new(Bounds2::from_xz(0.0, 0.0, 100.0, 100.0), 10.0);
		assert!((domain.weight(50.0, 50.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn edge_band_eases() -> Result<()> {
		let domain = CellDomainMask::new(Bounds2::from_xz(0.0, 0.0, 100.0, 100.0), 10.0);
		let near = domain.weight(2.0, 50.0);
		let mid = domain.weight(5.0, 50.0);
		assert!(near > 0.0 && near < 1.0);
		assert!(mid > near);
		assert!(mid < 1.0);
		Ok(())
	}
}
