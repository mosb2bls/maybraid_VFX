//! **Branch-out** state: joint [`BallStickNode`] plus noisy fan-out, child radius, and stick projection (bias, DOF, length).
//!
//! Sampling uses the small helpers in [`super::child_count`], [`super::radius_range`], [`super::length_range`], and [`super::degree_range`].
//!
//! For [`super::Hysteresis`], this type carries its own [`NoiseConfig`] and segment context so [`Self::next_hysteresis`] can sample children without external rule types.

use std::ops::Range;

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::BallStickNode;

use super::child_count;
use super::degree_range;
use super::length_range;
use super::radius_range;
use super::Hysteresis;

/// Picks number of children, lengths, directions, of branches
#[derive(Clone)]
pub struct BranchOut {
	pub node: BallStickNode,
	pub noise: NoiseConfig,
	pub segment_index: usize,
	pub incoming_ray: Vec3,
	/// Half-open range for noisy child count.
	pub child_count: Range<usize>,
	/// Half-open range for noisy radius at each child joint.
	pub radius_range: Range<f32>,
	/// Multipliers for the next hysteresis step: child [`Self::radius_range`] is
	/// `(parent.start * .0)..(parent.end * .1)`.
	pub radius_range_child_scale: (f32, f32),
	/// Optional absolute clamp for sampled and propagated child radii.
	pub radius_range_child_bounds: Option<Range<f32>>,
	/// Half-open range for noisy segment length toward each child.
	pub length: Range<f32>,
	/// Degrees of freedom for noisy ray direction.
	pub ray_degrees_of_freedom: f32,
	/// Bias ray direction.
	pub bias_ray: Vec3,
	/// Bias ray blend weight.
	/// Blend weight into [`Self::bias_ray`] (`1` = bias only).
	pub bias_blend: f32,
}

impl Default for BranchOut {
	fn default() -> Self {
		Self {
			node: BallStickNode::new(Vec3::ZERO, 0.05),
			noise: NoiseConfig::new(NoiseParams::default()),
			segment_index: 0,
			incoming_ray: Vec3::Y,
			child_count: 1..3,
			radius_range: 0.05..0.2,
			radius_range_child_scale: (1.0, 1.0),
			radius_range_child_bounds: None,
			length: 0.2..0.5,
			ray_degrees_of_freedom: 0.14,
			bias_ray: Vec3::Y,
			bias_blend: 0.5,
		}
	}
}

impl BranchOut {
	pub fn up(node: BallStickNode) -> Self {
		Self {
			node,
			noise: NoiseConfig::new(NoiseParams::default()),
			segment_index: 0,
			incoming_ray: Vec3::Y,
			child_count: 1..3,
			radius_range: 0.05..0.2,
			radius_range_child_scale: (1.0, 1.0),
			radius_range_child_bounds: None,
			length: 0.2..0.5,
			ray_degrees_of_freedom: 0.14,
			bias_ray: Vec3::Y,
			bias_blend: 1.0,
		}
	}

	pub fn with_ball_radius(mut self, radius: f32) -> Self {
		self.node.radius = radius;
		self
	}

	pub fn with_radius_range(mut self, radius_range: Range<f32>) -> Self {
		self.radius_range = radius_range;
		self
	}

	pub fn with_radius_range_child_scale(mut self, radius_range_child_scale: (f32, f32)) -> Self {
		self.radius_range_child_scale = radius_range_child_scale;
		self
	}

	pub fn with_radius_range_child_bounds(mut self, radius_range_child_bounds: Range<f32>) -> Self {
		self.radius_range_child_bounds = Some(radius_range_child_bounds);
		self
	}

	pub fn down(node: BallStickNode) -> Self {
		Self {
			node,
			noise: NoiseConfig::new(NoiseParams::default()),
			segment_index: 0,
			incoming_ray: -Vec3::Y,
			child_count: 1..3,
			radius_range: 0.05..0.2,
			radius_range_child_scale: (1.0, 1.0),
			radius_range_child_bounds: None,
			length: 0.2..0.5,
			ray_degrees_of_freedom: 0.14,
			bias_ray: -Vec3::Y,
			bias_blend: 1.0,
		}
	}

	/// Outward in the horizontal plane (XZ), for ring-canopy seeds: bias and incoming ray align with `direction_xz`.
	pub fn radial_out_horizontal(node: BallStickNode, mut direction_xz: Vec3) -> Self {
		direction_xz.y = 0.0;
		let radial = direction_xz.normalize_or_zero();
		let radial = if radial.length_squared() < 1e-12 { Vec3::X } else { radial };
		Self {
			node,
			noise: NoiseConfig::new(NoiseParams::default()),
			segment_index: 0,
			incoming_ray: radial,
			child_count: 1..4,
			radius_range: 0.05..0.2,
			radius_range_child_scale: (1.0, 1.0),
			radius_range_child_bounds: None,
			length: 0.2..0.5,
			ray_degrees_of_freedom: 0.08,
			bias_ray: radial,
			bias_blend: 0.88,
		}
	}

	pub fn with_noise(mut self, noise: NoiseConfig) -> Self {
		self.noise = noise;
		self
	}

	pub fn with_hysteresis_context(
		mut self,
		noise: NoiseConfig,
		segment_index: usize,
		incoming_ray: Vec3,
	) -> Self {
		self.noise = noise;
		self.segment_index = segment_index;
		self.incoming_ray = incoming_ray;
		self
	}

	pub fn with_child_count(mut self, child_count: Range<usize>) -> Self {
		self.child_count = child_count;
		self
	}

	pub fn with_ray_degrees_of_freedom(mut self, ray_degrees_of_freedom: f32) -> Self {
		self.ray_degrees_of_freedom = ray_degrees_of_freedom;
		self
	}

	pub fn with_length(mut self, length: Range<f32>) -> Self {
		self.length = length;
		self
	}

	pub fn with_bias_blend(mut self, bias_blend: f32) -> Self {
		self.bias_blend = bias_blend;
		self
	}

	pub fn with_bias_ray(mut self, bias_ray: Vec3) -> Self {
		self.bias_ray = bias_ray;
		self
	}

	pub fn single_child(mut self) -> Self {
		self.child_count = 1..2;
		self
	}

	/// One noisy stick step from this joint (child index `0`), using this profile's segment context.
	pub fn project_tip(&self) -> BallStickNode {
		let parent = self.node;
		let ray = self.sample_ray(&self.noise, &parent, self.segment_index, 0, self.incoming_ray);
		let r = self.sample_child_radius(&self.noise, &parent, self.segment_index, 0);
		BallStickNode::new(parent.position + ray, r)
	}

	fn expand_children(&self) -> Vec<BranchOut> {
		let parent = self.node;
		let n = self.sample_child_count(&self.noise, &parent, self.segment_index);
		(0..n)
			.map(|ci| {
				let ray = self.sample_ray(
					&self.noise,
					&parent,
					self.segment_index,
					ci as u32,
					self.incoming_ray,
				);
				let rad =
					self.sample_child_radius(&self.noise, &parent, self.segment_index, ci as u32);
				let child_node = BallStickNode::new(parent.position + ray, rad);
				let inc = child_node.position - parent.position;
				let (s_lo, s_hi) = self.radius_range_child_scale;
				let rr = &self.radius_range;
				let child_radius_range = clamp_radius_range(
					(rr.start * s_lo)..(rr.end * s_hi),
					&self.radius_range_child_bounds,
				);
				Self {
					node: child_node,
					noise: self.noise.clone(),
					segment_index: self.segment_index + 1,
					incoming_ray: inc,
					child_count: self.child_count.clone(),
					radius_range: child_radius_range,
					radius_range_child_scale: self.radius_range_child_scale,
					radius_range_child_bounds: self.radius_range_child_bounds.clone(),
					length: self.length.clone(),
					ray_degrees_of_freedom: self.ray_degrees_of_freedom,
					bias_ray: self.bias_ray,
					bias_blend: self.bias_blend,
				}
			})
			.collect()
	}

	pub fn sample_child_count(
		&self,
		noise: &NoiseConfig,
		parent: &BallStickNode,
		segment_index: usize,
	) -> usize {
		child_count::sample_usize(noise, self.child_count.clone(), parent, segment_index)
	}

	pub fn sample_child_radius(
		&self,
		noise: &NoiseConfig,
		parent: &BallStickNode,
		segment_index: usize,
		child_index: u32,
	) -> f32 {
		let radius = radius_range::sample_f32(
			noise,
			self.radius_range.clone(),
			parent,
			segment_index,
			child_index,
		);
		clamp_radius(radius, &self.radius_range_child_bounds)
	}

	pub fn sample_ray(
		&self,
		noise: &NoiseConfig,
		parent: &BallStickNode,
		segment_index: usize,
		child_index: u32,
		incoming_ray: Vec3,
	) -> Vec3 {
		let mean = Self::blend_direction(incoming_ray, self.bias_ray, self.bias_blend);
		let u = noise.sample_signed_4d(
			parent.position.x + child_index as f32 * 0.37,
			parent.position.y,
			parent.position.z,
			segment_index as f32 + 11.0,
		);
		let v = noise.sample_signed_4d(
			parent.position.x,
			parent.position.y + child_index as f32 * 0.41,
			parent.position.z,
			segment_index as f32 + 13.0,
		);
		let dir = degree_range::perturb_direction(mean, self.ray_degrees_of_freedom, u, v);
		let len = length_range::sample_f32(
			noise,
			self.length.clone(),
			parent,
			segment_index,
			child_index,
		);
		dir * len
	}

	/// Blend incoming growth toward `bias_ray` with weight `t` in `[0, 1]`.
	pub fn blend_direction(incoming_ray: Vec3, bias_ray: Vec3, t: f32) -> Vec3 {
		degree_range::blend_direction(incoming_ray, bias_ray, t)
	}
}

fn clamp_radius(radius: f32, bounds: &Option<Range<f32>>) -> f32 {
	if let Some(bounds) = bounds {
		let lo = bounds.start.min(bounds.end);
		let hi = bounds.start.max(bounds.end);
		radius.clamp(lo, hi)
	} else {
		radius
	}
}

fn clamp_radius_range(range: Range<f32>, bounds: &Option<Range<f32>>) -> Range<f32> {
	if let Some(bounds) = bounds {
		let lo = bounds.start.min(bounds.end);
		let hi = bounds.start.max(bounds.end);
		range.start.clamp(lo, hi)..range.end.clamp(lo, hi)
	} else {
		range
	}
}

impl Hysteresis for BranchOut {
	fn ball_stick_node(&self) -> BallStickNode {
		self.node
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		self.expand_children()
	}
}
