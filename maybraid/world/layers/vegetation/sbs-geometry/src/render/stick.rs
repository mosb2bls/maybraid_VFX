use crate::{BallStickChain, BallStickSegment, Hysteresis};
use bevy::prelude::*;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};
use std::marker::PhantomData;

pub trait StickRenderRule<R: RenderItem, H: Hysteresis>: Clone {
	fn stick_render_item_for(
		&self,
		segment: &BallStickSegment<'_>,
		parent_hysteresis: &H,
		child_hysteresis: &H,
	) -> Option<R>;
}

#[derive(Clone)]
pub struct AlwaysStickRenderRule<Item> {
	item: Item,
}

impl<Item> AlwaysStickRenderRule<Item> {
	pub fn new(item: Item) -> Self {
		Self { item }
	}
}

impl<Item: FromScalarNoise> AlwaysStickRenderRule<Item> {
	pub fn from_noise_params(params: NoiseParams) -> Self {
		Self::new(params.build_scalar())
	}
}

impl<Item, H> StickRenderRule<Item, H> for AlwaysStickRenderRule<Item>
where
	Item: RenderItem + Clone,
	H: Hysteresis,
{
	fn stick_render_item_for(
		&self,
		_segment: &BallStickSegment<'_>,
		_parent_hysteresis: &H,
		_child_hysteresis: &H,
	) -> Option<Item> {
		Some(self.item.clone())
	}
}

/// Renders sticks (edges) of a [`BallStickChain`].
///
/// Assumes each [`RenderItem`] is authored as a **unit cylinder along +Y centered at the origin**:
///
/// - local `y = -0.5` to `y = 0.5`
/// - local center at `(0, 0, 0)`
///
/// The helper scales `Y` by segment length, rotates +Y onto the segment direction,
/// and places the transform at the segment midpoint.
///
/// Each stick gets a **chain-local** transform; the caller's `transform` composes on the left.
/// Tree assemblies spawning under a root entity pass `Transform::IDENTITY` and parent via
/// [`RenderItem::spawn_render_items_under`].
#[derive(Clone)]
pub struct StickRenderHelper<Item: RenderItem, Rule: StickRenderRule<Item, H>, H: Hysteresis> {
	rule: Rule,
	chain: BallStickChain<H>,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, Rule: StickRenderRule<Item, H>, H: Hysteresis>
	StickRenderHelper<Item, Rule, H>
{
	pub fn new(chain: BallStickChain<H>, rule: Rule) -> Self {
		Self { chain, rule, __marker: PhantomData }
	}

	pub fn render_sticks(&self) -> Vec<(Item, Transform)> {
		self.chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent_h, child_h)| {
				let inner = stick_transform(&segment)?;
				let item = self.rule.stick_render_item_for(&segment, parent_h, child_h)?;
				Some((item, inner))
			})
			.collect()
	}

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

impl<Item: RenderItem, Rule: StickRenderRule<Item, H>, H: Hysteresis> RenderItem
	for StickRenderHelper<Item, Rule, H>
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.render_sticks()
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

impl<Item: RenderItem, Rule: StickRenderRule<Item, H> + FromScalarNoise, H: Hysteresis>
	StickRenderHelper<Item, Rule, H>
{
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

/// Local transform: centered at segment midpoint, +Y along the segment, `scale.y` = length.
fn stick_transform(segment: &BallStickSegment<'_>) -> Option<Transform> {
	let ray = segment.ray();
	let len_sq = ray.length_squared();

	if len_sq < 1e-12 {
		return None;
	}

	let len = len_sq.sqrt();
	let dir = ray / len;
	let rotation = align_positive_y_to(dir);
	let radius = segment.start.radius;

	Some(Transform {
		translation: segment.start.position + ray * 0.5,
		rotation,
		scale: Vec3::new(radius, len, radius),
	})
}

fn align_positive_y_to(dir: Vec3) -> Quat {
	let y = Vec3::Y;
	let d = dir.normalize_or_zero();
	if d.length_squared() < 1e-12 {
		return Quat::IDENTITY;
	}
	let dot = y.dot(d);
	if dot > 1.0 - 1e-5 {
		return Quat::IDENTITY;
	}
	if dot < -1.0 + 1e-5 {
		return Quat::from_axis_angle(Vec3::X, std::f32::consts::PI);
	}
	Quat::from_rotation_arc(y, d)
}
