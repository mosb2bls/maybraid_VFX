//! Tuft placement helpers for ball-stick graphs.

use crate::render::mix_seed::node_mix_seed;
use crate::{BallStickChain, BallStickNode, Hysteresis};
use bevy::prelude::*;
use render_item::{CascadeChunk, RenderItem};
use std::marker::PhantomData;

/// Stable mixing key from graph index and node position.
pub fn tuft_mix_seed(node_idx: usize, position: Vec3) -> u32 {
	node_mix_seed(node_idx, position)
}

/// One tuft cluster rooted at the joint, growing in world +Y (scale sets world size).
pub fn tuft_transform_at_joint(node: &BallStickNode, world_scale: f32) -> Transform {
	Transform {
		translation: node.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(world_scale),
	}
}

pub trait TuftRenderRule<R: RenderItem, H: Hysteresis>: Clone {
	fn tuft_placements_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		_hysteresis: &H,
		_chain: &BallStickChain<H>,
	) -> Vec<(R, Transform)>;
}

#[derive(Clone)]
pub struct TuftRenderHelper<Item: RenderItem, Rule: TuftRenderRule<Item, H>, H: Hysteresis> {
	rule: Rule,
	chain: BallStickChain<H>,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, Rule: TuftRenderRule<Item, H>, H: Hysteresis>
	TuftRenderHelper<Item, Rule, H>
{
	pub fn new(chain: BallStickChain<H>, rule: Rule) -> Self {
		Self { chain, rule, __marker: PhantomData }
	}

	pub fn render_tufts(&self) -> Vec<(Item, Transform)> {
		self.chain
			.nodes_with_hysteresis_enumerated()
			.flat_map(|(node_idx, node, h)| {
				self.rule.tuft_placements_for(node_idx, node, h, &self.chain)
			})
			.collect()
	}
}

impl<Item: RenderItem, Rule: TuftRenderRule<Item, H>, H: Hysteresis> RenderItem
	for TuftRenderHelper<Item, Rule, H>
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.render_tufts()
			.into_iter()
			.flat_map(|(item, inner)| {
				item.spawn_render_items(commands, cascade_chunk, transform.mul_transform(inner))
			})
			.collect()
	}
}
