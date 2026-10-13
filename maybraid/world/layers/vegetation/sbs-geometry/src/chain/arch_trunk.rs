//! Smooth arched trunk hysteresis for palms ([#255](https://github.com/ramate-io/maybraid/issues/255)).
//!
//! Walks a predetermined lateral lean curve with one child per step—tighter and smoother than
//! reusing [`super::BranchOut`] bias for persistent arch.

use std::ops::Range;

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::length_range;
use super::Hysteresis;

/// Horizontal unit direction for trunk lean from yaw about world +Y (degrees). `0` → +X.
pub fn arch_horizontal_direction_from_yaw_degrees(yaw_degrees: f32) -> Vec3 {
	let rad = yaw_degrees.to_radians();
	normalize_arch_horizontal_direction(Vec3::new(rad.cos(), 0.0, rad.sin()))
}

/// Flattens to the XZ plane and normalizes; falls back to +X when degenerate.
pub fn normalize_arch_horizontal_direction(dir: Vec3) -> Vec3 {
	let flat = Vec3::new(dir.x, 0.0, dir.z);
	if flat.length_squared() < 1e-8 {
		Vec3::X
	} else {
		flat.normalize()
	}
}

/// Layout inputs shared by [`arch_point`] and [`ArchTrunk`].
#[derive(Clone, Debug, PartialEq)]
pub struct ArchTrunkParams {
	pub base: Vec3,
	pub trunk_height: f32,
	/// Tip lateral offset as a fraction of [`Self::trunk_height`].
	pub arch_lateral_fraction: f32,
	/// Horizontal unit vector in the XZ plane (lean axis).
	pub arch_direction: Vec3,
	pub radius: f32,
	pub stalk_height: f32,
	pub segment_length_fraction: (f32, f32),
	pub total_steps: usize,
}

impl ArchTrunkParams {
	pub fn arch_direction_from_yaw_degrees(yaw_degrees: f32) -> Vec3 {
		arch_horizontal_direction_from_yaw_degrees(yaw_degrees)
	}
}

/// World position along the trunk arch at normalized height `t` in `[0, 1]`.
///
/// Lateral offset is zero at the base and reaches `arch_lateral_fraction * trunk_height` at the tip
/// along `arch_direction`.
pub fn arch_point(
	base: Vec3,
	trunk_height: f32,
	arch_lateral_fraction: f32,
	arch_direction: Vec3,
	t: f32,
) -> Vec3 {
	let t = t.clamp(0.0, 1.0);
	let dir = normalize_arch_horizontal_direction(arch_direction);
	let lean = arch_lateral_fraction * trunk_height * (2.0 * t - t * t);
	base + dir * lean + Vec3::Y * (t * trunk_height)
}

pub fn arch_point_from_params(params: &ArchTrunkParams, t: f32) -> Vec3 {
	arch_point(
		params.base,
		params.trunk_height,
		params.arch_lateral_fraction,
		params.arch_direction,
		t,
	)
}

/// Single-child hysteresis that samples the next point on an arched trunk polyline.
#[derive(Clone)]
pub struct ArchTrunk {
	pub node: BallStickNode,
	pub params: ArchTrunkParams,
	pub noise: NoiseConfig,
	pub step: usize,
}

impl ArchTrunk {
	pub fn from_params(params: ArchTrunkParams, noise: NoiseConfig) -> Self {
		let node = BallStickNode::new(params.base, params.radius);
		Self { node, params, noise, step: 0 }
	}

	pub fn new(
		base: Vec3,
		trunk_height: f32,
		arch_lateral_fraction: f32,
		arch_direction: Vec3,
		radius: f32,
		noise: NoiseConfig,
		stalk_height: f32,
		segment_length_fraction: (f32, f32),
		total_steps: usize,
	) -> Self {
		Self::from_params(
			ArchTrunkParams {
				base,
				trunk_height,
				arch_lateral_fraction,
				arch_direction: normalize_arch_horizontal_direction(arch_direction),
				radius,
				stalk_height,
				segment_length_fraction,
				total_steps: total_steps.max(1),
			},
			noise,
		)
	}

	fn length_range(&self) -> Range<f32> {
		let h = self.params.stalk_height.max(1e-6);
		let (lo, hi) = self.params.segment_length_fraction;
		h * lo..h * hi
	}
}

impl Hysteresis for ArchTrunk {
	fn ball_stick_node(&self) -> BallStickNode {
		self.node
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		if self.step >= self.params.total_steps {
			return Vec::new();
		}
		let next_step = self.step + 1;
		let t = next_step as f32 / self.params.total_steps as f32;
		let pos = arch_point_from_params(&self.params, t);
		let _len =
			length_range::sample_f32(&self.noise, self.length_range(), &self.node, self.step, 0);
		vec![Self {
			node: BallStickNode::new(pos, self.params.radius),
			step: next_step,
			..self.clone()
		}]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::NoiseParams;

	#[test]
	fn arch_point_reaches_lateral_tip_along_direction() -> Result<()> {
		let base = Vec3::ZERO;
		let h = 10.0;
		let dir = Vec3::X;
		let tip = arch_point(base, h, 0.12, dir, 1.0);
		assert!((tip.y - h).abs() < 1e-4);
		assert!((tip.x - 0.12 * h).abs() < 1e-3);
		assert!(tip.z.abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn arch_yaw_rotates_tip_into_z() -> Result<()> {
		let h = 10.0;
		let tip =
			arch_point(Vec3::ZERO, h, 0.12, arch_horizontal_direction_from_yaw_degrees(90.0), 1.0);
		assert!((tip.z - 0.12 * h).abs() < 1e-3);
		assert!(tip.x.abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn mid_chain_lean_along_arch_direction() -> Result<()> {
		let h = 10.0;
		let dir = arch_horizontal_direction_from_yaw_degrees(45.0);
		let mid = arch_point(Vec3::ZERO, h, 0.12, dir, 0.5);
		let lean = mid - Vec3::Y * (0.5 * h);
		let along = lean.dot(dir);
		assert!(along > 0.0, "expected lean along arch direction, got {lean:?}");
		assert!(lean.length() > 0.1);
		Ok(())
	}

	#[test]
	fn build_chain_leans_horizontally() -> Result<()> {
		let h = 12.0;
		let trunk_h = 0.85 * h;
		let seed = ArchTrunk::new(
			Vec3::ZERO,
			trunk_h,
			0.12,
			Vec3::X,
			0.025 * h,
			NoiseConfig::new(NoiseParams::default()),
			h,
			(0.05, 0.08),
			12,
		);
		let chain = crate::BallStickChain::build(vec![seed]);
		let tip = chain
			.nodes
			.iter()
			.max_by(|a, b| a.position.y.partial_cmp(&b.position.y).unwrap())
			.map(|n| n.position)
			.unwrap_or(Vec3::ZERO);
		assert!(tip.x > 0.5, "expected lateral lean at tip, got {tip:?}");
		assert!((tip.y - trunk_h).abs() < trunk_h * 0.1);
		Ok(())
	}

	#[test]
	fn normalize_arch_direction_rejects_vertical() -> Result<()> {
		let d = normalize_arch_horizontal_direction(Vec3::Y);
		assert!((d - Vec3::X).length() < 1e-5);
		let d2 = normalize_arch_horizontal_direction(Vec3::new(3.0, 1.0, 4.0));
		assert!((d2 - Vec3::new(3.0, 0.0, 4.0).normalize()).length() < 1e-5);
		Ok(())
	}

	#[test]
	fn default_yaw_is_positive_x() -> Result<()> {
		let d = arch_horizontal_direction_from_yaw_degrees(0.0);
		assert!((d.x - 1.0).abs() < 1e-5);
		assert!(d.z.abs() < 1e-5);
		let d90 = arch_horizontal_direction_from_yaw_degrees(90.0);
		assert!(d90.x.abs() < 1e-5);
		assert!((d90.z - 1.0).abs() < 1e-5);
		Ok(())
	}
}
