//! Hopscotch forest-layering selection ([RFC-183 §3.5.2.2]).

use bevy_math::Vec3;
use procedural_common::{BucketThrow, NoiseConfig, NoiseParams, UnitRange};

/// Anchor throw lane (offset from the forest-cell center).
const ANCHOR_LANE: Vec3 = Vec3::new(11.0, 0.0, 0.0);
/// Hop-budget sample lane.
const BUDGET_LANE: Vec3 = Vec3::new(0.0, 0.0, 13.0);
/// Base lane for successive hop-edge throws (`+ step` on X).
const EDGE_LANE_X: f32 = 17.0;
/// Keep world origin off OpenSimplex's zero so the first bucket is not forced.
pub(crate) const SAMPLE_ORIGIN: Vec3 = Vec3::new(10_007.0, 0.0, 10_009.0);

/// One Hopscotch node: anchor weight, outgoing edges, and the layering it selects.
#[derive(Debug, Clone, PartialEq)]
pub struct HopscotchNode<T> {
	pub weight: f32,
	pub adjacencies: Vec<(T, f32)>,
	pub item: T,
}

impl<T> HopscotchNode<T> {
	pub fn new(weight: f32, item: T, adjacencies: Vec<(T, f32)>) -> Self {
		Self { weight, adjacencies, item }
	}
}

/// Select a node item by Hopscotch from `position` (forest-cell center).
pub fn select<T: Copy + PartialEq>(
	nodes: &[HopscotchNode<T>],
	hop_budget: UnitRange,
	noise: NoiseParams,
	position: Vec3,
) -> Option<T> {
	if nodes.is_empty() {
		return None;
	}
	let n = NoiseConfig::new(noise);
	let throw = BucketThrow::from_weights(nodes.iter().map(|node| node.weight), 0.0);
	let sample = n.sample_3d(position + ANCHOR_LANE + SAMPLE_ORIGIN) * throw.total_weight();
	let mut index = throw.select(sample)?;
	let mut current = &nodes[index];
	let budget_at = position + BUDGET_LANE + SAMPLE_ORIGIN;
	let t = n.sample_unit_3d(budget_at.x, budget_at.y, budget_at.z);
	let mut budget = hop_budget.start + (hop_budget.end - hop_budget.start) * t;
	let mut step = 0u32;
	while budget >= current.weight && !current.adjacencies.is_empty() {
		budget -= current.weight;
		let edges = BucketThrow::from_weights(current.adjacencies.iter().map(|(_, w)| *w), 0.0);
		let edge_sample = n
			.sample_3d(position + Vec3::new(EDGE_LANE_X + step as f32, 0.0, 0.0) + SAMPLE_ORIGIN)
			* edges.total_weight();
		let Some(edge_i) = edges.select(edge_sample) else {
			break;
		};
		let neighbor = current.adjacencies[edge_i].0;
		index = nodes.iter().position(|node| node.item == neighbor)?;
		current = &nodes[index];
		step = step.saturating_add(1);
	}
	Some(current.item)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	enum Kind {
		A,
		B,
	}

	fn graph() -> Vec<HopscotchNode<Kind>> {
		vec![
			HopscotchNode::new(4.0, Kind::A, vec![(Kind::A, 2.0), (Kind::B, 1.0)]),
			HopscotchNode::new(1.0, Kind::B, vec![(Kind::A, 1.0), (Kind::B, 1.0)]),
		]
	}

	#[test]
	fn selection_is_deterministic() -> Result<()> {
		let nodes = graph();
		let noise = NoiseParams::from_scalar(1.0, 0.02, 1.0, 1);
		let at = Vec3::new(800.0, 0.0, 800.0);
		let hop = UnitRange::new(0.0, 3.0);
		let a = select(&nodes, hop, noise, at);
		let b = select(&nodes, hop, noise, at);
		assert_eq!(a, b);
		assert!(a.is_some());
		Ok(())
	}

	#[test]
	fn empty_graph_selects_nothing() -> Result<()> {
		assert!(select::<Kind>(&[], UnitRange::new(0.0, 1.0), NoiseParams::default(), Vec3::ZERO)
			.is_none());
		Ok(())
	}
}
