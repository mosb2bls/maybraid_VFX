use crate::{BallStickChain, BallStickNode, Hysteresis};
use bevy::prelude::*;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};
use std::marker::PhantomData;

pub trait BallRenderRule<R: RenderItem, H: Hysteresis>: Clone {
	/// Returns the render item for this graph vertex and a **radius scale** applied to [`BallStickNode::radius`]
	/// when building the spawn transform (typically `1.0`; canopy foliage may use `> 1.0`).
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &H,
		chain: &BallStickChain<H>,
	) -> Option<(R, f32)>;
}

#[derive(Clone)]
pub struct AlwaysBallRenderRule<Item> {
	item: Item,
}

impl<Item> AlwaysBallRenderRule<Item> {
	pub fn new(item: Item) -> Self {
		Self { item }
	}
}

impl<Item: FromScalarNoise> AlwaysBallRenderRule<Item> {
	pub fn from_noise_params(params: NoiseParams) -> Self {
		Self::new(params.build_scalar())
	}
}

impl<Item, H> BallRenderRule<Item, H> for AlwaysBallRenderRule<Item>
where
	Item: RenderItem + Clone,
	H: Hysteresis,
{
	fn ball_render_item_for(
		&self,
		_node_idx: usize,
		_node: &BallStickNode,
		_hysteresis: &H,
		_chain: &BallStickChain<H>,
	) -> Option<(Item, f32)> {
		Some((self.item.clone(), 1.0))
	}
}

/// A useful common helper for rendering balls.
///
/// Each ball gets a **chain-local** transform (node position + radius scale); the caller's
/// `transform` composes on the left. Tree assemblies spawning under a root entity pass
/// `Transform::IDENTITY` and parent via [`RenderItem::spawn_render_items_under`].
#[derive(Clone)]
pub struct BallRenderHelper<Item: RenderItem, Rule: BallRenderRule<Item, H>, H: Hysteresis> {
	rule: Rule,
	chain: BallStickChain<H>,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, Rule: BallRenderRule<Item, H>, H: Hysteresis>
	BallRenderHelper<Item, Rule, H>
{
	pub fn new(chain: BallStickChain<H>, rule: Rule) -> Self {
		Self { chain, rule, __marker: PhantomData }
	}

	pub fn render_balls(&self) -> Vec<(Item, Transform)> {
		self.chain
			.nodes_with_hysteresis_enumerated()
			.filter_map(|(node_idx, node, h)| {
				self.rule.ball_render_item_for(node_idx, node, h, &self.chain).map(
					|(item, radius_scale)| {
						(
							item,
							Transform::from_translation(node.position)
								.with_scale(Vec3::splat(node.radius * radius_scale)),
						)
					},
				)
			})
			.collect()
	}

	/// Often we'll want to construct this over a chain and then yield it back to the caller.
	pub fn spawn_render_items_and_yield(
		self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> BallStickChain<H> {
		self.spawn_render_items(commands, cascade_chunk, transform);
		self.chain
	}
}

impl<Item: RenderItem, Rule: BallRenderRule<Item, H>, H: Hysteresis> RenderItem
	for BallRenderHelper<Item, Rule, H>
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.render_balls()
			.into_iter()
			.flat_map(|(item, inner_transform)| {
				item.spawn_render_items(
					commands,
					cascade_chunk,
					transform.mul_transform(inner_transform),
				)
			})
			.collect()
	}
}

impl<Item: RenderItem, Rule: BallRenderRule<Item, H> + FromScalarNoise, H: Hysteresis>
	BallRenderHelper<Item, Rule, H>
{
	/// Construct a ball renderer from a scalar noise value.
	///
	/// NOTE: this isn't pulled into the scalar hierarchy directly
	/// because BallRenderHelper is a helper, it still needs a particular BallStickChain
	pub fn new_from_noise(
		chain: BallStickChain<H>,
		scalar: f32,
		amplitude: f32,
		frequency: f32,
		octaves: u32,
	) -> Self {
		let rule =
			Rule::from_scalar(NoiseParams::from_scalar(scalar, frequency, amplitude, octaves));
		Self::new(chain, rule)
	}
}
