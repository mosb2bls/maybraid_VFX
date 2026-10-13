//! Last-tread contact: walkable leading, never the kit bleed.

use bevy_math::Vec2;
use building_components::stairs::{Stair, StairNode};

/// Leading edge and travel of the last tread — enough to reason about the exit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreadEnd {
	pub leading_outer: Vec2,
	pub leading_inner: Vec2,
	pub travel: Vec2,
	pub going: f32,
	pub width: f32,
}

impl TreadEnd {
	/// Last tread of a linear [`Stair::Straight`] node.
	///
	/// Leading is the **walkable** kit \(X = +1\) (placement center \(+\) half
	/// going), not the rearward \(X = -2\) bleed.
	pub fn from_straight(node: &StairNode) -> Self {
		let (width, going, n) = match &node.geometry {
			Stair::Straight(g) => (g.width, g.going_per_tread(), g.tread_count().max(1)),
			Stair::Spiral(_) => (0.5, 0.35, 1),
		};
		let travel = travel_xz(node.placement.yaw);
		let radial = Vec2::new(-travel.y, travel.x);
		let origin = Vec2::new(node.placement.translation.x, node.placement.translation.z);
		let last = origin + travel * ((n - 1) as f32 * going);
		let lead = last + travel * (0.5 * going);
		let half_w = 0.5 * width;
		Self {
			leading_outer: lead + radial * half_w,
			leading_inner: lead - radial * half_w,
			travel,
			going,
			width,
		}
	}

	pub fn from_last_straight(nodes: &[StairNode]) -> Option<Self> {
		nodes.last().map(Self::from_straight)
	}

	pub fn leading_mid(self) -> Vec2 {
		(self.leading_outer + self.leading_inner) * 0.5
	}

	/// Walkable plan quad: leading then trailing (outer / inner).
	pub fn plan_quad(self) -> [Vec2; 4] {
		let back = travel_xz_dir(self.travel) * self.going.max(1e-4);
		[
			self.leading_outer,
			self.leading_inner,
			self.leading_inner - back,
			self.leading_outer - back,
		]
	}
}

fn travel_xz_dir(travel: Vec2) -> Vec2 {
	let n = travel.length();
	if n < 1e-5 {
		Vec2::X
	} else {
		travel / n
	}
}

fn travel_xz(yaw: f32) -> Vec2 {
	Vec2::new(yaw.cos(), -yaw.sin())
}
