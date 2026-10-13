//! Perturb non-stalk anchors around a shared [`StrictStalk`].
//!
//! Anchors are generated in **tree-local space** (stalk base at `Vec3::ZERO`), so noise here is
//! sampled over local node positions: two identical trees produce identical geometry regardless
//! of where their roots are placed. Per-instance variation comes from the caller supplying a
//! different [`NoiseParams`] seed (e.g. groves mix a placement seed per instance), not from
//! spatial offsets.

use std::ops::Range;

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::Anchors;
use crate::chain::point_to_point::PointToPoint;
use crate::chain::BranchOut;
use crate::{BallStickNode, Hysteresis};

/// Provides the stalk reference used to identify the unperturbed base anchor.
pub trait HasStrictStalk {
	fn strict_stalk(&self) -> &StrictStalk;
}

/// A sampled perturbation for one anchor seed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorPerturbation {
	pub vertical_offset: f32,
	pub angular_scale: f32,
	pub angular_u: f32,
	pub angular_v: f32,
	pub radius_offset: f32,
}

/// Hysteresis seeds opt into anchor perturbation without changing [`Hysteresis`].
pub trait PerturbAnchor: Hysteresis {
	fn perturb_anchor(self, perturbation: AnchorPerturbation) -> Self;
}

/// Wraps a strict-stalk-backed anchor generator and perturbs every non-base anchor.
#[derive(Clone, Debug, PartialEq)]
pub struct StalkPerturbation<A> {
	pub inner: A,
	pub noise: NoiseParams,
	pub vertical_offset: Range<f32>,
	pub angular_scale: Range<f32>,
	pub radius_offset: Range<f32>,
}

impl<A> StalkPerturbation<A> {
	pub fn new(inner: A) -> Self {
		Self {
			inner,
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl<A: Default> Default for StalkPerturbation<A> {
	fn default() -> Self {
		Self {
			inner: A::default(),
			noise: NoiseParams::default(),
			vertical_offset: 0.0..0.0,
			angular_scale: 0.0..0.0,
			radius_offset: 0.0..0.0,
		}
	}
}

impl HasStrictStalk for StrictStalk {
	fn strict_stalk(&self) -> &StrictStalk {
		self
	}
}

impl<A, T> Anchors<T> for StalkPerturbation<A>
where
	A: Anchors<T> + HasStrictStalk,
	T: PerturbAnchor,
{
	fn anchors(&self) -> Vec<T> {
		self.perturb_anchors(self.inner.anchors())
	}
}

impl<A> StalkPerturbation<A>
where
	A: HasStrictStalk,
{
	pub fn perturb_anchors<T>(&self, anchors: Vec<T>) -> Vec<T>
	where
		T: PerturbAnchor,
	{
		let noise = NoiseConfig::new(self.noise);

		anchors
			.into_iter()
			.enumerate()
			.map(|(i, anchor)| {
				let node = anchor.ball_stick_node();
				if is_stalk_base(node.position) {
					return anchor;
				}

				let perturbation = AnchorPerturbation {
					vertical_offset: sample_range(
						&noise,
						self.vertical_offset.clone(),
						&node,
						i,
						0.0,
					),
					angular_scale: sample_range(&noise, self.angular_scale.clone(), &node, i, 17.0),
					angular_u: sample_signed(&noise, &node, i, 23.0),
					angular_v: sample_signed(&noise, &node, i, 29.0),
					radius_offset: sample_range(&noise, self.radius_offset.clone(), &node, i, 31.0),
				};
				anchor.perturb_anchor(perturbation)
			})
			.collect()
	}
}

impl PerturbAnchor for PointToPoint {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.start = perturb_node(self.start, perturbation);
		self.radius = self.start.radius;
		self
	}
}

pub fn perturb_node(mut node: BallStickNode, perturbation: AnchorPerturbation) -> BallStickNode {
	node.position += Vec3::Y * perturbation.vertical_offset;
	node.radius = (node.radius + perturbation.radius_offset).max(1e-4);
	node
}

/// Apply vertical, angular, and radius perturbation to a [`BranchOut`] limb profile.
pub fn perturb_branch_out(mut branch: BranchOut, perturbation: AnchorPerturbation) -> BranchOut {
	branch.node = perturb_node(branch.node, perturbation);
	branch.incoming_ray = crate::chain::degree_range::perturb_direction(
		branch.incoming_ray,
		perturbation.angular_scale,
		perturbation.angular_u,
		perturbation.angular_v,
	);
	branch.bias_ray = crate::chain::degree_range::perturb_direction(
		branch.bias_ray,
		perturbation.angular_scale,
		perturbation.angular_u,
		perturbation.angular_v,
	);
	branch.radius_range = (branch.radius_range.start + perturbation.radius_offset).max(1e-4)
		..(branch.radius_range.end + perturbation.radius_offset).max(1e-4);
	if let Some(bounds) = &branch.radius_range_child_bounds {
		let lo = bounds.start.min(bounds.end);
		let hi = bounds.start.max(bounds.end);
		let radius = branch.node.radius.clamp(lo, hi);
		branch.node.radius = radius;
		branch.radius_range = radius..radius;
	}
	branch
}

fn sample_range(
	noise: &NoiseConfig,
	range: Range<f32>,
	node: &BallStickNode,
	anchor_index: usize,
	lane: f32,
) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	let t = noise.sample_unit_4d(
		node.position.x + lane,
		node.position.y + anchor_index as f32,
		node.position.z,
		lane,
	);
	lo + t * (hi - lo)
}

fn sample_signed(noise: &NoiseConfig, node: &BallStickNode, anchor_index: usize, lane: f32) -> f32 {
	noise.sample_4d(
		node.position.x + lane,
		node.position.y + anchor_index as f32,
		node.position.z,
		lane,
	)
}

/// Chains are tree-local, so the unperturbed stalk base is always the origin.
fn is_stalk_base(position: Vec3) -> bool {
	position.length_squared() <= 1e-8
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::sopes_banyan::SopesBanyanProtoAnchors;

	#[test]
	fn perturbs_ring_anchors_but_not_stalk_base() -> anyhow::Result<()> {
		let anchors =
			SopesBanyanProtoAnchors { ring_count: 1, anchors_per_ring: 1, ..Default::default() };
		let perturbation = StalkPerturbation {
			inner: anchors,
			noise: NoiseParams::default(),
			vertical_offset: 1.0..1.0,
			angular_scale: 0.0..0.0,
			radius_offset: 0.2..0.2,
		};

		let seeds = perturbation.anchors();
		assert_eq!(seeds.len(), 2);

		let branch_node = seeds[0].ball_stick_node();
		assert!((branch_node.position.y - 9.0).abs() < 1e-4);
		assert!((branch_node.radius - 0.45).abs() < 1e-4);

		let stalk_node = seeds[1].ball_stick_node();
		assert_eq!(stalk_node.position, Vec3::ZERO);
		assert_eq!(stalk_node.radius, 0.75);
		Ok(())
	}

	#[test]
	fn angular_perturbation_changes_non_stalk_anchor_direction() -> anyhow::Result<()> {
		let anchors =
			SopesBanyanProtoAnchors { ring_count: 1, anchors_per_ring: 1, ..Default::default() };
		let original_seed = anchors.anchors()[0].clone();
		let original_node = original_seed.ball_stick_node();
		let Some(original_profile) = original_seed.active_branch_profile() else {
			return Err(anyhow::anyhow!("expected branch seed profile"));
		};
		let original_incoming_ray = original_profile.incoming_ray;
		let original_bias_ray = original_profile.bias_ray;
		let perturbation = StalkPerturbation {
			inner: anchors,
			noise: NoiseParams::default(),
			vertical_offset: 0.0..0.0,
			angular_scale: 0.5..0.5,
			radius_offset: 0.0..0.0,
		};

		let perturbed_seed = perturbation.anchors()[0].clone();
		let perturbed_node = perturbed_seed.ball_stick_node();
		let Some(perturbed_profile) = perturbed_seed.active_branch_profile() else {
			return Err(anyhow::anyhow!("expected perturbed branch seed profile"));
		};

		assert_eq!(perturbed_node.position, original_node.position);
		assert!(
			perturbed_profile.incoming_ray.distance_squared(original_incoming_ray) > 1e-8
				|| perturbed_profile.bias_ray.distance_squared(original_bias_ray) > 1e-8
		);
		Ok(())
	}
}
