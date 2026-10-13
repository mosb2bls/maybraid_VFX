//! Multi-polyline band composition: soft-voronoi ownership / blend in one pass.
//!
//! Used by Watershed stream graphs so overlapping corridors do not stack solo
//! apron / channel stamps. Soft-voronoi weights from path distance:
//! \(w_i \propto \exp(-\gamma d_i)\). High \(\gamma\) ≈ hard ownership; low
//! \(\gamma\) blends apron targets before a single raise-only apply.

use crate::modulation::polyline_grading::PolylineGradeMode;
use crate::region::{
	closest_on_polyline, grade_along_polyline, PolylineRegion, Region2D, RegionNoise,
};
use bevy_math::Vec2;

/// One corridor contributing to a multi-path band modulation.
#[derive(Debug, Clone)]
pub struct MultiPolylineBandPart {
	pub path: Vec<Vec2>,
	/// Per-vertex elevations (`len` should match [`Self::path`]).
	pub levels: Vec<f32>,
	pub half_width: f32,
	pub node_blend: f32,
	/// Softmask fade past the stadium edge (world units).
	pub fade: f32,
	pub boundary_noise: Option<RegionNoise>,
}

impl MultiPolylineBandPart {
	pub fn new(path: Vec<Vec2>, levels: Vec<f32>, half_width: f32, fade: f32) -> Self {
		Self {
			path,
			levels,
			half_width: half_width.max(1e-3),
			node_blend: 0.0,
			fade: fade.max(0.001),
			boundary_noise: None,
		}
	}

	pub fn with_node_blend(mut self, node_blend: f32) -> Self {
		self.node_blend = node_blend.max(0.0);
		self
	}

	pub fn with_boundary_noise(mut self, noise: RegionNoise) -> Self {
		self.boundary_noise = Some(noise);
		self
	}

	fn region(&self) -> Region2D {
		Region2D::Polyline(PolylineRegion::new(self.path.clone(), self.half_width))
	}

	fn softmask_at(&self, p: Vec2) -> f32 {
		self.region().softmask_weight(p, 0.0, self.fade, self.boundary_noise.as_ref())
	}

	fn grade_at(&self, p: Vec2) -> f32 {
		grade_along_polyline(&self.path, &self.levels, p, self.node_blend)
	}

	fn path_distance(&self, p: Vec2) -> f32 {
		closest_on_polyline(&self.path, p).distance
	}
}

/// Soft-voronoi multi-path grading with optional min-floor compose for carves.
#[derive(Debug, Clone)]
pub struct MultiPolylineBandModulation {
	pub parts: Vec<MultiPolylineBandPart>,
	pub mode: PolylineGradeMode,
	/// Soft-voronoi sharpness; high ≈ nearest-path ownership.
	pub ownership_gamma: f32,
	/// When true with [`PolylineGradeMode::DepressionOnly`], take the deepest
	/// carve among contributors (RFC min-floor safety at junctions).
	pub min_compose: bool,
	pub height_noise: Option<RegionNoise>,
	pub height_noise_add_only: bool,
	/// Hard cap on add-only height noise (rim uplift budget). `None` = uncapped.
	pub height_noise_cap: Option<f32>,
}

impl MultiPolylineBandModulation {
	pub fn new(parts: Vec<MultiPolylineBandPart>, ownership_gamma: f32) -> Self {
		Self {
			parts,
			mode: PolylineGradeMode::Blend,
			ownership_gamma: ownership_gamma.max(0.0),
			min_compose: false,
			height_noise: None,
			height_noise_add_only: false,
			height_noise_cap: None,
		}
	}

	pub fn depression_only(mut self) -> Self {
		self.mode = PolylineGradeMode::DepressionOnly;
		self
	}

	pub fn raise_only(mut self) -> Self {
		self.mode = PolylineGradeMode::RaiseOnly;
		self
	}

	pub fn with_min_compose(mut self, min_compose: bool) -> Self {
		self.min_compose = min_compose;
		self
	}

	pub fn with_height_noise_add_only(mut self, noise: RegionNoise) -> Self {
		self.height_noise = Some(noise);
		self.height_noise_add_only = true;
		self
	}

	pub fn with_height_noise_cap(mut self, cap: f32) -> Self {
		self.height_noise_cap = Some(cap.max(0.0));
		self
	}

	fn height_noise_at(&self, p: Vec2) -> f32 {
		let Some(hn) = &self.height_noise else {
			return 0.0;
		};
		let mut s = hn.sample_height(p);
		if self.height_noise_add_only {
			s = s.abs();
		}
		if let Some(cap) = self.height_noise_cap {
			s = s.min(cap);
		}
		s
	}

	/// Soft-voronoi weights from path distances (sum to 1 when any part is finite).
	pub fn voronoi_weights(&self, p: Vec2) -> Vec<f32> {
		soft_voronoi_weights(
			self.parts.iter().map(|part| part.path_distance(p)),
			self.ownership_gamma,
		)
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		if self.parts.is_empty() {
			return elevation;
		}
		let p = Vec2::new(x, z);
		let weights = self.voronoi_weights(p);
		let noise = self.height_noise_at(p);

		if self.min_compose && self.mode == PolylineGradeMode::DepressionOnly {
			let mut h = elevation;
			for (i, part) in self.parts.iter().enumerate() {
				let soft = part.softmask_at(p);
				if soft >= 1.0 - 1e-5 {
					continue;
				}
				// Keep a floor of ownership so a slightly farther deeper channel
				// can still win near a junction (weights gate weak contributors).
				if weights.get(i).copied().unwrap_or(0.0) < 1e-4 {
					continue;
				}
				let graded = part.grade_at(p) + noise;
				let toward = soft * elevation + (1.0 - soft) * graded;
				h = h.min(toward.min(elevation));
			}
			return h;
		}

		let mut num = 0.0;
		let mut den = 0.0;
		for (i, part) in self.parts.iter().enumerate() {
			let soft = part.softmask_at(p);
			let inside = 1.0 - soft;
			if inside <= 1e-5 {
				continue;
			}
			let w = weights.get(i).copied().unwrap_or(0.0);
			let inf = inside * w;
			if inf <= 1e-8 {
				continue;
			}
			num += inf * (part.grade_at(p) + noise);
			den += inf;
		}
		if den <= 1e-8 {
			return elevation;
		}
		let blended = num / den;
		let influence = den.clamp(0.0, 1.0);
		let toward = (1.0 - influence) * elevation + influence * blended;
		match self.mode {
			PolylineGradeMode::Blend => toward,
			PolylineGradeMode::DepressionOnly => toward.min(elevation),
			PolylineGradeMode::RaiseOnly => toward.max(elevation),
		}
	}
}

/// Relative offset cut/raise along multiple polylines with soft-voronoi ownership.
///
/// Used for thalweg affine cuts: `h' = h + influence * offset` (typically negative).
#[derive(Debug, Clone)]
pub struct MultiPolylineOffsetPart {
	pub path: Vec<Vec2>,
	pub half_width: f32,
	pub fade: f32,
	pub offset: f32,
	pub boundary_noise: Option<RegionNoise>,
}

impl MultiPolylineOffsetPart {
	pub fn new(path: Vec<Vec2>, half_width: f32, fade: f32, offset: f32) -> Self {
		Self {
			path,
			half_width: half_width.max(1e-3),
			fade: fade.max(0.001),
			offset,
			boundary_noise: None,
		}
	}

	pub fn with_boundary_noise(mut self, noise: RegionNoise) -> Self {
		self.boundary_noise = Some(noise);
		self
	}

	fn softmask_at(&self, p: Vec2) -> f32 {
		Region2D::Polyline(PolylineRegion::new(self.path.clone(), self.half_width)).softmask_weight(
			p,
			0.0,
			self.fade,
			self.boundary_noise.as_ref(),
		)
	}

	fn path_distance(&self, p: Vec2) -> f32 {
		closest_on_polyline(&self.path, p).distance
	}
}

#[derive(Debug, Clone)]
pub struct MultiPolylineOffsetModulation {
	pub parts: Vec<MultiPolylineOffsetPart>,
	pub ownership_gamma: f32,
	/// Optional bipolar / add-only height noise added to the offset.
	pub height_noise: Option<RegionNoise>,
	pub height_noise_add_only: bool,
	/// When true, only the nearest path (max voronoi weight) may cut.
	pub winner_take_all: bool,
}

impl MultiPolylineOffsetModulation {
	pub fn new(parts: Vec<MultiPolylineOffsetPart>, ownership_gamma: f32) -> Self {
		Self {
			parts,
			ownership_gamma: ownership_gamma.max(0.0),
			height_noise: None,
			height_noise_add_only: false,
			winner_take_all: false,
		}
	}

	pub fn with_height_noise(mut self, noise: RegionNoise) -> Self {
		self.height_noise = Some(noise);
		self.height_noise_add_only = false;
		self
	}

	pub fn winner_take_all(mut self) -> Self {
		self.winner_take_all = true;
		self
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		if self.parts.is_empty() {
			return elevation;
		}
		let p = Vec2::new(x, z);
		let weights = soft_voronoi_weights(
			self.parts.iter().map(|part| part.path_distance(p)),
			self.ownership_gamma,
		);
		let mut noise = 0.0;
		if let Some(hn) = &self.height_noise {
			let s = hn.sample_height(p);
			noise = if self.height_noise_add_only { s.abs() } else { s };
		}
		if self.winner_take_all {
			let Some((owner, _)) = weights
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
			else {
				return elevation;
			};
			let part = &self.parts[owner];
			let soft = part.softmask_at(p);
			let inside = 1.0 - soft;
			if inside <= 1e-5 {
				return elevation;
			}
			return elevation + inside * (part.offset + noise);
		}
		// Soft ownership: weight-blend offsets (not min — deepest-wins digs canyons).
		let mut delta = 0.0;
		let mut den = 0.0;
		for (i, part) in self.parts.iter().enumerate() {
			let soft = part.softmask_at(p);
			let inside = 1.0 - soft;
			if inside <= 1e-5 {
				continue;
			}
			let w = weights.get(i).copied().unwrap_or(0.0);
			if w < 1e-4 {
				continue;
			}
			let inf = inside * w;
			delta += inf * (part.offset + noise);
			den += inf;
		}
		if den <= 1e-8 {
			elevation
		} else {
			let influence = den.clamp(0.0, 1.0);
			elevation + influence * (delta / den)
		}
	}
}

/// Soft-voronoi weights \(w_i \propto \exp(-\gamma d_i)\), normalized.
pub fn soft_voronoi_weights(distances: impl IntoIterator<Item = f32>, gamma: f32) -> Vec<f32> {
	let dists: Vec<f32> = distances.into_iter().collect();
	let n = dists.len();
	if n == 0 {
		return Vec::new();
	}
	if n == 1 {
		return vec![1.0];
	}
	let gamma = gamma.max(0.0);
	let min_d = dists.iter().copied().fold(f32::INFINITY, f32::min);
	if !min_d.is_finite() {
		return vec![1.0 / n as f32; n];
	}
	// Subtract min for numeric stability; when gamma is huge this ≈ one-hot.
	let mut weights = Vec::with_capacity(n);
	let mut sum = 0.0;
	for &d in &dists {
		let w = (-gamma * (d - min_d)).exp();
		weights.push(w);
		sum += w;
	}
	if sum <= 1e-20 {
		return vec![1.0 / n as f32; n];
	}
	for w in &mut weights {
		*w /= sum;
	}
	weights
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hard_gamma_picks_nearest_path() -> anyhow::Result<()> {
		let a = MultiPolylineBandPart::new(
			vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)],
			vec![50.0, 50.0],
			6.0,
			2.0,
		);
		let b = MultiPolylineBandPart::new(
			vec![Vec2::new(0.0, 20.0), Vec2::new(40.0, 20.0)],
			vec![30.0, 30.0],
			6.0,
			2.0,
		);
		let m = MultiPolylineBandModulation::new(vec![a, b], 8.0).raise_only();
		// On path A.
		let h = m.modify_elevation(10.0, 20.0, 0.0);
		assert!((h - 50.0).abs() < 0.5, "nearest apron should own: {h}");
		// On path B.
		let h = m.modify_elevation(10.0, 20.0, 20.0);
		assert!((h - 30.0).abs() < 0.5, "nearest apron should own: {h}");
		Ok(())
	}

	#[test]
	fn soft_gamma_blends_between_parallel_aprons() -> anyhow::Result<()> {
		let a = MultiPolylineBandPart::new(
			vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)],
			vec![40.0, 40.0],
			12.0,
			4.0,
		);
		let b = MultiPolylineBandPart::new(
			vec![Vec2::new(0.0, 10.0), Vec2::new(40.0, 10.0)],
			vec![20.0, 20.0],
			12.0,
			4.0,
		);
		let m = MultiPolylineBandModulation::new(vec![a, b], 0.15).raise_only();
		let h = m.modify_elevation(0.0, 20.0, 5.0);
		assert!(h > 20.0 && h < 40.0, "midline between parallel aprons should blend: {h}");
		Ok(())
	}

	#[test]
	fn min_compose_takes_deeper_channel() -> anyhow::Result<()> {
		let shallow = MultiPolylineBandPart::new(
			vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)],
			vec![45.0, 45.0],
			8.0,
			2.0,
		);
		let deep = MultiPolylineBandPart::new(
			vec![Vec2::new(0.0, 2.0), Vec2::new(40.0, 2.0)],
			vec![30.0, 30.0],
			8.0,
			2.0,
		);
		let m = MultiPolylineBandModulation::new(vec![shallow, deep], 0.5)
			.depression_only()
			.with_min_compose(true);
		let h = m.modify_elevation(60.0, 20.0, 1.0);
		assert!(h <= 31.0, "min compose should prefer deeper bed: {h}");
		Ok(())
	}

	#[test]
	fn offset_thalweg_cuts_under_ownership() -> anyhow::Result<()> {
		let a = MultiPolylineOffsetPart::new(
			vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)],
			3.0,
			1.0,
			-8.0,
		);
		let b = MultiPolylineOffsetPart::new(
			vec![Vec2::new(0.0, 20.0), Vec2::new(40.0, 20.0)],
			3.0,
			1.0,
			-2.0,
		);
		let m = MultiPolylineOffsetModulation::new(vec![a, b], 10.0);
		let h = m.modify_elevation(100.0, 20.0, 0.0);
		assert!((h - 92.0).abs() < 0.5, "owned thalweg cut: {h}");
		Ok(())
	}

	#[test]
	fn voronoi_one_hot_at_high_gamma() -> anyhow::Result<()> {
		let w = soft_voronoi_weights([0.0, 5.0, 10.0], 20.0);
		assert!(w[0] > 0.99);
		assert!(w[1] < 0.01);
		Ok(())
	}
}
