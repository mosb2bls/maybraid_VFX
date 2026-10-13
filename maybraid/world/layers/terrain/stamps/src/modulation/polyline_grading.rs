//! Softmask grading along a piecewise polyline grade.

use crate::region::{grade_along_polyline, Region2D, RegionNoise};
use bevy_math::Vec2;

/// How graded elevation blends with the incoming heightfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolylineGradeMode {
	/// Softmask lerp toward the grade.
	Blend,
	/// Never raise above the incoming elevation (channel floor).
	DepressionOnly,
	/// Never lower below the incoming elevation (bank / skirt lift).
	RaiseOnly,
}

/// Blend toward a surface graded along polyline node elevations.
///
/// Between nodes, elevation lerps along the owning segment. Near vertices,
/// inbound/outbound pitches are blended ([`grade_along_polyline`]).
#[derive(Debug, Clone)]
pub struct RegionPolylineGradingModulation {
	pub region: Region2D,
	pub path: Vec<Vec2>,
	/// Per-vertex elevations (`len` should match [`Self::path`]).
	pub levels: Vec<f32>,
	/// Path-distance blend radius for inbound/outbound pitch mixing at nodes.
	pub node_blend: f32,
	pub noise: Option<RegionNoise>,
	pub inner_radius: f32,
	pub outer_radius: f32,
	pub mode: PolylineGradeMode,
	/// Optional additive height noise on the graded target.
	pub height_noise: Option<RegionNoise>,
	/// When true, height noise only raises above the grade (`|sample|`).
	pub height_noise_add_only: bool,
}

impl RegionPolylineGradingModulation {
	pub fn new(
		region: Region2D,
		path: Vec<Vec2>,
		levels: Vec<f32>,
		inner_radius: f32,
		outer_radius: f32,
	) -> Self {
		Self {
			region,
			path,
			levels,
			node_blend: 0.0,
			noise: None,
			inner_radius,
			outer_radius: outer_radius.max(inner_radius + 0.001),
			mode: PolylineGradeMode::Blend,
			height_noise: None,
			height_noise_add_only: false,
		}
	}

	/// Convenience: two-endpoint grade sampled along the whole path length.
	pub fn from_head_toe(
		region: Region2D,
		path: Vec<Vec2>,
		head_elevation: f32,
		toe_elevation: f32,
		inner_radius: f32,
		outer_radius: f32,
	) -> Self {
		let n = path.len().max(1);
		let mut levels = Vec::with_capacity(n);
		if n == 1 {
			levels.push(head_elevation);
		} else {
			for i in 0..n {
				let t = i as f32 / (n - 1) as f32;
				levels.push(head_elevation + (toe_elevation - head_elevation) * t);
			}
		}
		Self::new(region, path, levels, inner_radius, outer_radius)
	}

	pub fn with_node_blend(mut self, node_blend: f32) -> Self {
		self.node_blend = node_blend.max(0.0);
		self
	}

	pub fn with_noise(mut self, noise: RegionNoise) -> Self {
		self.noise = Some(noise);
		self
	}

	pub fn with_height_noise(mut self, noise: RegionNoise) -> Self {
		self.height_noise = Some(noise);
		self.height_noise_add_only = false;
		self
	}

	/// Height noise that only raises above the grade (never lowers).
	pub fn with_height_noise_add_only(mut self, noise: RegionNoise) -> Self {
		self.height_noise = Some(noise);
		self.height_noise_add_only = true;
		self
	}

	pub fn depression_only(mut self) -> Self {
		self.mode = PolylineGradeMode::DepressionOnly;
		self
	}

	pub fn raise_only(mut self) -> Self {
		self.mode = PolylineGradeMode::RaiseOnly;
		self
	}

	/// Graded surface elevation at `(x, z)`.
	pub fn grade_at(&self, x: f32, z: f32) -> f32 {
		let mut graded =
			grade_along_polyline(&self.path, &self.levels, Vec2::new(x, z), self.node_blend);
		if let Some(hn) = &self.height_noise {
			let s = hn.sample_height(Vec2::new(x, z));
			graded += if self.height_noise_add_only { s.abs() } else { s };
		}
		graded
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		let weight = self.region.softmask_weight(
			p,
			self.inner_radius,
			self.outer_radius,
			self.noise.as_ref(),
		);
		let graded = self.grade_at(x, z);
		let toward = weight * elevation + (1.0 - weight) * graded;
		match self.mode {
			PolylineGradeMode::Blend => toward,
			PolylineGradeMode::DepressionOnly => toward.min(elevation),
			PolylineGradeMode::RaiseOnly => toward.max(elevation),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::region::PolylineRegion;

	#[test]
	fn grade_follows_piecewise_nodes() -> anyhow::Result<()> {
		let path = vec![Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0), Vec2::new(100.0, 0.0)];
		let levels = vec![50.0, 40.0, 40.0];
		let region = Region2D::Polyline(PolylineRegion::new(path.clone(), 8.0));
		let g = RegionPolylineGradingModulation::new(region, path, levels, 0.0, 2.0);
		assert!((g.grade_at(25.0, 0.0) - 45.0).abs() < 1e-3);
		assert!((g.grade_at(75.0, 0.0) - 40.0).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn depression_only_never_raises() -> anyhow::Result<()> {
		let path = vec![Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0)];
		let levels = vec![50.0, 40.0];
		let region = Region2D::Polyline(PolylineRegion::new(path.clone(), 6.0));
		let g =
			RegionPolylineGradingModulation::new(region, path, levels, 0.0, 2.0).depression_only();
		assert_eq!(g.modify_elevation(30.0, 10.0, 0.0), 30.0);
		assert!(g.modify_elevation(80.0, 10.0, 0.0) < 80.0);
		Ok(())
	}

	#[test]
	fn add_only_height_noise_never_lowers_grade() -> anyhow::Result<()> {
		let path = vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)];
		let levels = vec![20.0, 20.0];
		let region = Region2D::Polyline(PolylineRegion::new(path.clone(), 8.0));
		let noise = RegionNoise::from_seed(1, 0.05, 4.0);
		let bipolar = RegionPolylineGradingModulation::new(
			region.clone(),
			path.clone(),
			levels.clone(),
			0.0,
			2.0,
		)
		.with_height_noise(noise.clone());
		let add_only = RegionPolylineGradingModulation::new(region, path, levels, 0.0, 2.0)
			.with_height_noise_add_only(noise);
		let x = 20.0;
		let z = 0.0;
		let base = 20.0;
		assert!(add_only.grade_at(x, z) >= base - 1.0e-4);
		assert!(
			(add_only.grade_at(x, z) - base - (bipolar.grade_at(x, z) - base).abs()).abs() < 1.0e-4
		);
		Ok(())
	}
}
