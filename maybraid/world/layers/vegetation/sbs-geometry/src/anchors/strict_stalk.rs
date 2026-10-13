//! **Strict stalk** segments in the **shared** [`crate::BallStickChain`]: hysteresis that keeps growth collinear (no lateral branch wander) so the trunk reads as a straight run from ground toward the anchor band.
//!
//! Used when [`super::Anchors`] emits the vertical chain of seed nodes along the stalk **radial centroid**; canopy ring seeds on that same axis then branch under the canopy [`crate::Hysteresis`] recipe.

use crate::anchors::Anchors;
use crate::chain::point_to_point::PointToPoint;
use crate::BallStickNode;
use crate::Hysteresis;
use bevy_math::Vec3;

/// Linear radius taper from base toward the crown on a multi-hop stalk.
const SEGMENTED_STALK_TAPER_RATE: f32 = 0.38;
/// Floor on crown radius as a fraction of [`StrictStalk::stalk_base_radius`].
const SEGMENTED_STALK_MIN_RADIUS_FRACTION: f32 = 0.58;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct StrictStalk {
	/// Height of the stalk.
	///
	/// NOTE: we prefix this with stalk for flattening.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 10.0))]
	pub stalk_height: f32,

	/// Base radius of the stalk.
	///
	/// NOTE: we prefix this with stalk for flattening.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.75))]
	pub stalk_base_radius: f32,
}

impl StrictStalk {
	/// Stalk radial centroid at height `t * height` above the tree-local origin, with `t` in `[0, 1]`.
	///
	/// Chains are generated in **tree-local space** (base at `Vec3::ZERO`); the spawned root
	/// entity owns world placement, and per-instance variation comes from caller-supplied seeds.
	pub fn centroid_at_height_fraction(&self, t: f32) -> Vec3 {
		let t = t.clamp(0.0, 1.0);
		Vec3::Y * (t * self.stalk_height)
	}

	pub fn point_to_point_anchors(&self) -> Vec<PointToPoint> {
		vec![PointToPoint::new_from_vec3(
			self.centroid_at_height_fraction(0.0),
			self.centroid_at_height_fraction(1.0),
			self.stalk_base_radius,
		)]
	}

	/// Multi-hop stalk along the centroid with tapering node radii.
	pub fn segmented_point_to_point(&self, section_count: u32) -> PointToPoint {
		let n = section_count.max(2) as usize;
		let nodes: Vec<BallStickNode> = (0..=n)
			.map(|i| {
				let t = i as f32 / n as f32;
				let r = self.stalk_base_radius
					* (1.0 - SEGMENTED_STALK_TAPER_RATE * t)
						.max(SEGMENTED_STALK_MIN_RADIUS_FRACTION);
				BallStickNode::new(self.centroid_at_height_fraction(t), r)
			})
			.collect();
		let start = nodes[0];
		let end = nodes.get(1).copied();
		let tail: Vec<_> = nodes.into_iter().skip(2).collect();
		PointToPoint { start, end, radius: start.radius, tail }
	}

	pub fn segmented_point_to_point_anchors(&self, section_count: u32) -> Vec<PointToPoint> {
		vec![self.segmented_point_to_point(section_count)]
	}
}

impl<T> Anchors<T> for StrictStalk
where
	T: Hysteresis + From<PointToPoint>,
{
	fn anchors(&self) -> Vec<T> {
		self.point_to_point_anchors().into_iter().map(|p| p.into()).collect()
	}
}
