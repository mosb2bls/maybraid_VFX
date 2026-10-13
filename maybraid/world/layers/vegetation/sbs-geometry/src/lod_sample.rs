//! Silhouette-preserving LOD sampling over azimuth × height bands.
//!
//! Picks the outermost sample (max horizontal radius from the Y axis) in each
//! \((\text{azimuth}, \text{height})\) cell so vase / pinch profiles survive
//! thinning — unlike a global outer-radius shell.

use bevy_math::Vec3;

use crate::chain::{horizontal_radius_from_y_axis, BallStickChain, BallStickNode, Hysteresis};

/// Grid resolution for [`sample_max_horizontal_radius_by_azimuth_height`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AzimuthHeightBands {
	pub azimuth_bins: usize,
	pub height_bins: usize,
}

impl AzimuthHeightBands {
	pub const fn new(azimuth_bins: usize, height_bins: usize) -> Self {
		Self { azimuth_bins, height_bins }
	}

	pub fn cell_count(self) -> usize {
		self.azimuth_bins.saturating_mul(self.height_bins)
	}
}

/// One winning sample from an azimuth × height cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AzimuthHeightSample<T> {
	pub item: T,
	pub position: Vec3,
	pub horizontal_radius: f32,
	pub azimuth_bin: usize,
	pub height_bin: usize,
}

fn azimuth_bin(position: Vec3, azimuth_bins: usize) -> Option<usize> {
	if azimuth_bins == 0 {
		return None;
	}
	let r = horizontal_radius_from_y_axis(position);
	if r < 1e-4 {
		return None;
	}
	let u = (position.z.atan2(position.x) + std::f32::consts::PI) / std::f32::consts::TAU;
	let bin = (u * azimuth_bins as f32).floor() as usize;
	Some(bin.min(azimuth_bins - 1))
}

fn height_bin(y: f32, y_min: f32, y_span: f32, height_bins: usize) -> usize {
	if height_bins <= 1 {
		return 0;
	}
	let t = ((y - y_min) / y_span).clamp(0.0, 1.0 - 1e-6);
	((t * height_bins as f32).floor() as usize).min(height_bins - 1)
}

/// For each azimuth × height cell, keep the item with the largest horizontal radius.
///
/// Height bins span the observed \(Y\) range of input positions (inclusive). Samples
/// on / near the \(Y\) axis are skipped. Empty cells are omitted from the result.
///
/// Cost: one linear pass to measure \(Y\) range, one to fill the grid — \(O(n + AH)\).
pub fn sample_max_horizontal_radius_by_azimuth_height<'a, T>(
	items: impl IntoIterator<Item = &'a T>,
	position_of: impl Fn(&'a T) -> Vec3,
	bands: AzimuthHeightBands,
) -> Vec<AzimuthHeightSample<&'a T>> {
	let azimuth_bins = bands.azimuth_bins;
	let height_bins = bands.height_bins;
	if azimuth_bins == 0 || height_bins == 0 {
		return Vec::new();
	}

	let items: Vec<&'a T> = items.into_iter().collect();
	if items.is_empty() {
		return Vec::new();
	}

	let mut y_min = f32::INFINITY;
	let mut y_max = f32::NEG_INFINITY;
	let mut any = false;
	for item in &items {
		let p = position_of(item);
		if azimuth_bin(p, azimuth_bins).is_none() {
			continue;
		}
		y_min = y_min.min(p.y);
		y_max = y_max.max(p.y);
		any = true;
	}
	if !any {
		return Vec::new();
	}
	let y_span = (y_max - y_min).max(1e-4);

	let cell_count = azimuth_bins * height_bins;
	let mut best: Vec<Option<(f32, &'a T, Vec3, usize, usize)>> = vec![None; cell_count];

	for item in items {
		let p = position_of(item);
		let Some(a) = azimuth_bin(p, azimuth_bins) else {
			continue;
		};
		let h = height_bin(p.y, y_min, y_span, height_bins);
		let r = horizontal_radius_from_y_axis(p);
		let idx = h * azimuth_bins + a;
		let replace = match best[idx] {
			None => true,
			Some((prev_r, _, _, _, _)) => r > prev_r,
		};
		if replace {
			best[idx] = Some((r, item, p, a, h));
		}
	}

	best.into_iter()
		.flatten()
		.map(|(horizontal_radius, item, position, azimuth_bin, height_bin)| AzimuthHeightSample {
			item,
			position,
			horizontal_radius,
			azimuth_bin,
			height_bin,
		})
		.collect()
}

impl<H: Hysteresis> BallStickChain<H> {
	/// Outermost graph nodes per azimuth × height cell ([`sample_max_horizontal_radius_by_azimuth_height`]).
	pub fn sample_radius_azimuth(
		&self,
		bands: AzimuthHeightBands,
	) -> Vec<AzimuthHeightSample<&BallStickNode>> {
		sample_max_horizontal_radius_by_azimuth_height(
			self.nodes.iter(),
			|node| node.position,
			bands,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::chain::Hysteresis;

	#[derive(Debug, Clone, Copy, PartialEq)]
	struct Pt(Vec3);

	#[test]
	fn picks_outermost_per_azimuth_height_cell() -> anyhow::Result<()> {
		// Lower band: narrow waist at r=1; upper band: wide rim at r=3.
		let pts = [
			Pt(Vec3::new(1.0, 0.0, 0.0)),
			Pt(Vec3::new(0.5, 0.0, 0.0)), // inner, same cell — lose
			Pt(Vec3::new(3.0, 10.0, 0.0)),
			Pt(Vec3::new(2.0, 10.0, 0.0)), // inner upper — lose
			Pt(Vec3::new(0.0, 0.0, 1.0)),  // lower +Z
			Pt(Vec3::new(0.0, 10.0, 3.0)), // upper +Z
		];
		let samples = sample_max_horizontal_radius_by_azimuth_height(
			&pts,
			|p| p.0,
			AzimuthHeightBands::new(4, 2),
		);
		assert!(samples.len() >= 4);
		// Waist sample must survive (narrowing), not only the wide rim.
		assert!(samples.iter().any(|s| (s.horizontal_radius - 1.0).abs() < 1e-4));
		assert!(samples.iter().any(|s| (s.horizontal_radius - 3.0).abs() < 1e-4));
		Ok(())
	}

	#[test]
	fn empty_and_axis_samples_yield_empty() -> anyhow::Result<()> {
		let empty: [Pt; 0] = [];
		assert!(sample_max_horizontal_radius_by_azimuth_height(
			&empty,
			|p| p.0,
			AzimuthHeightBands::new(4, 2),
		)
		.is_empty());
		let axis = [Pt(Vec3::new(0.0, 1.0, 0.0))];
		assert!(sample_max_horizontal_radius_by_azimuth_height(
			&axis,
			|p| p.0,
			AzimuthHeightBands::new(4, 2),
		)
		.is_empty());
		Ok(())
	}

	#[derive(Clone)]
	struct LeafH(BallStickNode);

	impl Hysteresis for LeafH {
		fn ball_stick_node(&self) -> BallStickNode {
			self.0
		}

		fn next_hysteresis(&self) -> Vec<Self> {
			Vec::new()
		}
	}

	#[test]
	fn chain_sample_radius_azimuth_matches_free_fn() -> anyhow::Result<()> {
		let chain = BallStickChain::build(vec![
			LeafH(BallStickNode::new(Vec3::new(1.0, 0.0, 0.0), 0.1)),
			LeafH(BallStickNode::new(Vec3::new(0.5, 0.0, 0.0), 0.1)),
			LeafH(BallStickNode::new(Vec3::new(3.0, 10.0, 0.0), 0.1)),
			LeafH(BallStickNode::new(Vec3::new(2.0, 10.0, 0.0), 0.1)),
		]);
		let bands = AzimuthHeightBands::new(4, 2);
		let from_chain = chain.sample_radius_azimuth(bands);
		let from_free = sample_max_horizontal_radius_by_azimuth_height(
			chain.nodes.iter(),
			|n| n.position,
			bands,
		);
		assert_eq!(from_chain.len(), from_free.len());
		assert!(from_chain.iter().any(|s| (s.horizontal_radius - 1.0).abs() < 1e-4));
		assert!(from_chain.iter().any(|s| (s.horizontal_radius - 3.0).abs() < 1e-4));
		Ok(())
	}
}
