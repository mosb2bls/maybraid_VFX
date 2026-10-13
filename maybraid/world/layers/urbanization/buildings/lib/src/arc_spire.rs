//! Parameterized circular stair spire fitted to \(Y\) bindings.
//!
//! Construction:
//! 1. Sort / dedupe target \(Y\) bindings (tread tops, world or local — interpreted
//!    relative to [`ArcSpireParams::center_xz`].y when building).
//! 2. Best-fit a tread sequence: consecutive gaps are scaled toward
//!    [`ArcSpireParams::target_tread_height`] within [`FitTolerance::scale`], or
//!    subdivided / skipped (missed) when outside that range.
//! 3. Tessellate a spiral of rough-stone treads around [`ArcSpireParams::radius`].

use bevy_math::Vec3;
use building_components::stairs::{Stair, StairNode};
use building_components::{BuildingComponents, Layers, Placement};
use lod::scene::LodSceneLevel;

/// Inclusive scale range vs the target tread height used when fitting \(Y\) gaps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitTolerance {
	/// `(min, max)` scale of actual rise / [`ArcSpireParams::target_tread_height`].
	///
	/// - Below `min`: miss the upper binding (skip it).
	/// - Inside range: accept a single tread for that gap (scaled).
	/// - Above `max`: try to subdivide into multiple treads still inside range;
	///   if that fails, miss the upper binding.
	pub scale: (f32, f32),
}

impl Default for FitTolerance {
	fn default() -> Self {
		Self { scale: (0.85, 1.15) }
	}
}

/// Parameters for [`ArcSpire::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct ArcSpireParams {
	pub center_xz: Vec3,
	/// Centerline radius of the tread run.
	pub radius: f32,
	/// Radial tread width (world meters).
	pub tread_width: f32,
	/// Tangential tread depth / run (world meters).
	pub tread_depth: f32,
	/// Nominal rise used when scoring / subdividing gaps (~0.18 m).
	pub target_tread_height: f32,
	/// Target tread-top \(Y\) bindings (world). Best-fit may scale gaps or miss some.
	pub y_bindings: Vec<f32>,
	/// Tolerance for scaling or missing bindings.
	pub fit_tolerance: FitTolerance,
	/// Full turns over the fitted run (1.0 = one revolution).
	pub turns: f32,
}

/// Circular stair spire with treads fitted to \(Y\) bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcSpire {
	pub center_xz: Vec3,
	pub radius: f32,
	pub tread_width: f32,
	pub tread_depth: f32,
	pub target_tread_height: f32,
	pub fit_tolerance: FitTolerance,
	pub turns: f32,
	/// Accepted tread-top \(Y\)s in world space after best-fit.
	pub fitted_tops: Vec<f32>,
	/// Missed (skipped) bindings from the input list.
	pub missed_bindings: Vec<f32>,
	/// Spiral stair node (local tops relative to `center_xz.y`).
	pub stairs: StairNode,
}

impl ArcSpire {
	/// Best-fit treads to \(Y\) bindings, then build the spiral run.
	pub fn new(params: ArcSpireParams) -> Self {
		let radius = params.radius.max(1e-4);
		let tread_width = params.tread_width.max(1e-4);
		let tread_depth = params.tread_depth.max(1e-4);
		let target_h = params.target_tread_height.max(1e-4);
		let turns = params.turns.max(1e-4);
		let base_y = params.center_xz.y;

		let (fitted_tops, missed_bindings) =
			best_fit_y_bindings(&params.y_bindings, target_h, params.fit_tolerance);

		let local_tops: Vec<f32> =
			fitted_tops.iter().map(|y| y - base_y).filter(|y| *y > 1e-5).collect();

		let stairs = StairNode::rough_stone(
			Stair::spiral_fitted(radius, tread_width, tread_depth, local_tops, turns),
			Placement::new(params.center_xz, 0.0),
		);

		Self {
			center_xz: params.center_xz,
			radius,
			tread_width,
			tread_depth,
			target_tread_height: target_h,
			fit_tolerance: params.fit_tolerance,
			turns,
			fitted_tops,
			missed_bindings,
			stairs,
		}
	}
}

impl BuildingComponents for ArcSpire {
	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::from_free(vec![self.stairs.clone()])
	}
}

/// Best-fit ascending tread tops from target bindings.
///
/// Returns `(fitted_tops, missed_bindings)`.
pub fn best_fit_y_bindings(
	bindings: &[f32],
	target_tread_height: f32,
	tolerance: FitTolerance,
) -> (Vec<f32>, Vec<f32>) {
	let target_h = target_tread_height.max(1e-4);
	let (min_s, max_s) = ordered_scale(tolerance.scale);

	let mut targets: Vec<f32> = bindings.iter().copied().filter(|y| y.is_finite()).collect();
	targets.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	targets.dedup_by(|a, b| (*a - *b).abs() < 1e-5);

	if targets.is_empty() {
		return (Vec::new(), Vec::new());
	}

	let mut fitted = Vec::new();
	let mut missed = Vec::new();
	// Anchor: first binding is always kept as the first tread top.
	fitted.push(targets[0]);

	for &y in &targets[1..] {
		let prev = *fitted.last().expect("fitted starts non-empty");
		let gap = y - prev;
		if gap <= 1e-5 {
			missed.push(y);
			continue;
		}

		let scale = gap / target_h;
		if scale >= min_s && scale <= max_s {
			fitted.push(y);
			continue;
		}

		if scale < min_s {
			// Too tight — miss this binding.
			missed.push(y);
			continue;
		}

		// Too tall — try to subdivide into n treads with rise in tolerance.
		if let Some(n) = subdivision_count(gap, target_h, min_s, max_s) {
			let rise = gap / n as f32;
			for k in 1..=n {
				fitted.push(prev + k as f32 * rise);
			}
			// Last push equals y (within float error).
			if let Some(last) = fitted.last_mut() {
				*last = y;
			}
		} else {
			missed.push(y);
		}
	}

	(fitted, missed)
}

fn ordered_scale((a, b): (f32, f32)) -> (f32, f32) {
	if a <= b {
		(a.max(1e-4), b.max(1e-4))
	} else {
		(b.max(1e-4), a.max(1e-4))
	}
}

fn subdivision_count(gap: f32, target_h: f32, min_s: f32, max_s: f32) -> Option<u32> {
	let ideal = (gap / target_h).round().max(1.0) as u32;
	let mut best: Option<(f32, u32)> = None;
	for n in ideal.saturating_sub(2)..=ideal.saturating_add(2).max(1) {
		let n = n.max(1);
		let rise = gap / n as f32;
		let s = rise / target_h;
		if s >= min_s && s <= max_s {
			let err = (s - 1.0).abs();
			if best.map(|(e, _)| err < e).unwrap_or(true) {
				best = Some((err, n));
			}
		}
	}
	best.map(|(_, n)| n)
}

/// Uniform storey bindings: tread tops from one rise above `base_y` up to `base_y + height`.
pub fn uniform_storey_bindings(base_y: f32, height: f32, target_tread_height: f32) -> Vec<f32> {
	let height = height.max(1e-4);
	let target_h = target_tread_height.max(1e-4);
	let n = (height / target_h).ceil().max(1.0) as u32;
	let rise = height / n as f32;
	(1..=n).map(|i| base_y + i as f32 * rise).collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::stairs::SpiralStair;

	#[test]
	fn uniform_bindings_fit_exactly() -> anyhow::Result<()> {
		let bindings = uniform_storey_bindings(0.0, 3.0, SpiralStair::DEFAULT_TREAD_HEIGHT);
		let (fitted, missed) = best_fit_y_bindings(
			&bindings,
			SpiralStair::DEFAULT_TREAD_HEIGHT,
			FitTolerance::default(),
		);
		assert!(missed.is_empty());
		assert_eq!(fitted.len(), bindings.len());
		assert!((fitted.last().copied().unwrap_or(0.0) - 3.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn tight_binding_is_missed() -> anyhow::Result<()> {
		let (fitted, missed) =
			best_fit_y_bindings(&[0.18, 0.20, 0.36], 0.18, FitTolerance { scale: (0.85, 1.15) });
		assert!(missed.iter().any(|y| (*y - 0.20).abs() < 1e-5));
		assert!(fitted.iter().any(|y| (*y - 0.36).abs() < 1e-5));
		Ok(())
	}

	#[test]
	fn tall_gap_subdivides() -> anyhow::Result<()> {
		let (fitted, missed) =
			best_fit_y_bindings(&[0.18, 0.72], 0.18, FitTolerance { scale: (0.85, 1.15) });
		assert!(missed.is_empty());
		// 0.72 - 0.18 = 0.54 → three rises of 0.18
		assert_eq!(fitted.len(), 4);
		assert!((fitted[0] - 0.18).abs() < 1e-4);
		assert!((fitted.last().copied().unwrap_or(0.0) - 0.72).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn arc_spire_builds_stairs() -> anyhow::Result<()> {
		let spire = ArcSpire::new(ArcSpireParams {
			center_xz: Vec3::new(1.0, 2.0, 3.0),
			radius: 1.2,
			tread_width: 0.5,
			tread_depth: 0.3,
			target_tread_height: SpiralStair::DEFAULT_TREAD_HEIGHT,
			y_bindings: uniform_storey_bindings(2.0, 3.0, SpiralStair::DEFAULT_TREAD_HEIGHT),
			fit_tolerance: FitTolerance::default(),
			turns: 1.0,
		});
		assert!(!spire.fitted_tops.is_empty());
		assert!(matches!(spire.stairs.geometry, Stair::Spiral(_)));
		Ok(())
	}
}
