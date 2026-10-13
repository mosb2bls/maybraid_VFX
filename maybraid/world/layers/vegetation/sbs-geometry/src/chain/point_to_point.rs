//! Simply defines a hysteresis rule from one point to another.

use crate::BallStickNode;
use crate::Hysteresis;
use bevy_math::Vec3;

#[derive(Clone, Debug)]
pub struct PointToPoint {
	pub start: BallStickNode,
	pub end: Option<BallStickNode>,
	pub radius: f32,
	/// Further targets after [`Self::end`] (multi-section stalk).
	pub tail: Vec<BallStickNode>,
}

impl PointToPoint {
	pub fn new(start: BallStickNode, end: BallStickNode, radius: f32) -> Self {
		Self { start, end: Some(end), radius, tail: Vec::new() }
	}

	pub fn new_from_vec3(start: Vec3, end: Vec3, radius: f32) -> Self {
		Self::new(BallStickNode::new(start, radius), BallStickNode::new(end, radius), radius)
	}

	pub fn with_tail(mut self, tail: Vec<BallStickNode>) -> Self {
		self.tail = tail;
		self
	}
}

impl Hysteresis for PointToPoint {
	fn ball_stick_node(&self) -> BallStickNode {
		self.start
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		if let Some(end) = self.end {
			let mut tail = self.tail.clone();
			let next_end = tail.first().copied();
			if next_end.is_some() {
				tail.remove(0);
			}
			vec![Self { start: end, end: next_end, radius: end.radius, tail }]
		} else {
			vec![]
		}
	}
}
