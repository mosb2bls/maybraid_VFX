//! **High-bush** ground radial shoot anchors ([#225](https://github.com/ramate-io/maybraid/issues/225), RFC §3.1.6.3).

use std::f32::consts::TAU;

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use super::Anchors;
use crate::chain::high_bush::{
	high_bush_branch_depth, HighBushChain, HighBushPhase, ShootSeedSpec,
};
use crate::{AnchorsToChain, BallStickNode};

/// Default total height `H` for playground previews.
pub const DEFAULT_HEIGHT: f32 = 10.0;

/// Near-ground anchor lift as a fraction of `H` (Common High Bush `0.02`).
pub const DEFAULT_ANCHOR_LIFT_FRACTION: f32 = 0.02;

/// Default radial spokes (Common High Bush uses `7..=10`).
pub const DEFAULT_SHOOT_COUNT: u32 = 8;

/// RFC §3.1.7.12 direction mix defaults.
pub const DEFAULT_RADIAL_STRENGTH: f32 = 0.45;
pub const DEFAULT_VERTICAL_BIAS: f32 = 0.75;

/// [`BranchOut::ray_degrees_of_freedom`] at shoot seeds (torch trees use `24°`).
pub const DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES: f32 = 24.0;

/// Full weight on the upward [`ShootSeedSpec::bias_ray`] (see [`super::torch_tree::TORCH_BIAS_BLEND`]).
pub const DEFAULT_BIAS_BLEND: f32 = 1.0;

/// Per-segment length as fractions of `H` (RFC `0.08..0.16`).
pub const DEFAULT_SEGMENT_LENGTH_FRACTION_LO: f32 = 0.08;
pub const DEFAULT_SEGMENT_LENGTH_FRACTION_HI: f32 = 0.16;

/// Joint radius range as fractions of `H` (RFC `0.012..0.025`).
pub const DEFAULT_SEGMENT_RADIUS_FRACTION_LO: f32 = 0.012;
pub const DEFAULT_SEGMENT_RADIUS_FRACTION_HI: f32 = 0.025;

/// Ground joint radius as a fraction of `H`.
pub const DEFAULT_ROOT_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.018;

/// Upward-biased shoot direction from horizontal radial and mix weights.
pub fn high_bush_shoot_direction(
	radial_xz: Vec3,
	radial_strength: f32,
	vertical_bias: f32,
) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::Y;
	}
	(radial * radial_strength + Vec3::Y * vertical_bias).normalize_or_zero()
}

/// Trunkless radial shoot parameters before [`Self::hysteresis_seeds`].
#[derive(Clone, Debug, PartialEq)]
pub struct HighBushProtoAnchors {
	pub height: f32,
	pub anchor_lift_fraction: f32,
	pub shoot_count: u32,
	pub radial_strength: f32,
	pub vertical_bias: f32,
	pub branch_depth: usize,
	pub child_count_min: u32,
	pub child_count_max: u32,
	pub angle_tolerance_radians: f32,
	pub bias_blend: f32,
	pub segment_length_fraction_lo: f32,
	pub segment_length_fraction_hi: f32,
	pub segment_radius_fraction_lo: f32,
	pub segment_radius_fraction_hi: f32,
	pub root_radius_fraction_of_height: f32,
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for HighBushProtoAnchors {
	fn default() -> Self {
		Self {
			height: DEFAULT_HEIGHT,
			anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
			shoot_count: DEFAULT_SHOOT_COUNT,
			radial_strength: DEFAULT_RADIAL_STRENGTH,
			vertical_bias: DEFAULT_VERTICAL_BIAS,
			branch_depth: 4,
			child_count_min: 1,
			child_count_max: 2,
			angle_tolerance_radians: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			bias_blend: DEFAULT_BIAS_BLEND,
			segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
			segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
			segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
			segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
			root_radius_fraction_of_height: DEFAULT_ROOT_RADIUS_FRACTION_OF_HEIGHT,
			branch_radius_child_scale: (0.72, 0.80),
		}
	}
}

impl HighBushProtoAnchors {
	/// Tree-local shoot anchor (pure Y lift above the origin; the root entity owns world placement).
	pub fn anchor_position(&self) -> Vec3 {
		Vec3::Y * (self.height * self.anchor_lift_fraction)
	}

	pub fn root_radius(&self) -> f32 {
		(self.height * self.root_radius_fraction_of_height).max(1e-4)
	}

	pub fn shoot_limb_radius(&self) -> f32 {
		let h = self.height.max(1e-6);
		let mid = (self.segment_radius_fraction_lo + self.segment_radius_fraction_hi) * 0.5 * h;
		mid.max(1e-4)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<HighBushChain> {
		let h = self.height.max(1e-6);
		let anchor_y = self.anchor_position().y;
		let depth = high_bush_branch_depth(self.branch_depth);
		let root_node = BallStickNode::new(self.anchor_position(), self.root_radius());
		let k = self.shoot_count.max(1);

		let shoot_specs: Vec<ShootSeedSpec> = (0..k)
			.map(|i| {
				let theta = TAU * i as f32 / k as f32;
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let bias =
					high_bush_shoot_direction(radial, self.radial_strength, self.vertical_bias);
				ShootSeedSpec { radial_xz: radial, bias_ray: bias }
			})
			.collect();

		let child_count =
			self.child_count_min as usize..(self.child_count_max as usize).saturating_add(1);

		vec![HighBushChain::new(
			chain_noise,
			h,
			anchor_y,
			depth,
			None,
			self.angle_tolerance_radians,
			self.bias_blend,
			child_count,
			self.segment_length_fraction_lo,
			self.segment_length_fraction_hi,
			self.branch_radius_child_scale,
			HighBushPhase::Root { node: root_node, shoot_specs },
		)]
	}

	pub fn build_chain(&self) -> crate::BallStickChain<HighBushChain> {
		AnchorsToChain::build_chain(self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighBushAnchors {
	pub proto: HighBushProtoAnchors,
}

impl HighBushAnchors {
	pub fn new(proto: HighBushProtoAnchors) -> Self {
		Self { proto }
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<HighBushChain> {
		self.proto.hysteresis_seeds(chain_noise)
	}
}

impl Default for HighBushAnchors {
	fn default() -> Self {
		Self::new(HighBushProtoAnchors::default())
	}
}

impl Anchors<HighBushChain> for HighBushAnchors {
	fn anchors(&self) -> Vec<HighBushChain> {
		self.hysteresis_seeds(NoiseConfig::new(procedural_common::NoiseParams::default()))
	}
}

impl Anchors<HighBushChain> for HighBushProtoAnchors {
	fn anchors(&self) -> Vec<HighBushChain> {
		self.hysteresis_seeds(NoiseConfig::new(procedural_common::NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Hysteresis;

	#[test]
	fn shoot_direction_is_normalized_and_upward() -> anyhow::Result<()> {
		let dir = high_bush_shoot_direction(Vec3::X, 0.45, 0.75);
		assert!((dir.length() - 1.0).abs() < 1e-5);
		assert!(dir.y > 0.0);
		Ok(())
	}

	#[test]
	fn default_shoot_count_in_common_high_bush_band() {
		let proto = HighBushProtoAnchors::default();
		assert!((7..=10).contains(&proto.shoot_count));
	}

	#[test]
	fn build_chain_has_root_and_shoot_branches() -> anyhow::Result<()> {
		let chain = HighBushProtoAnchors::default().build_chain();
		assert!(chain.nodes.len() > 30, "nodes {}", chain.nodes.len());
		assert_eq!(chain.children[0].len(), DEFAULT_SHOOT_COUNT as usize);
		Ok(())
	}

	#[test]
	fn shoot_first_segment_grows_upward() -> anyhow::Result<()> {
		let proto = HighBushProtoAnchors::default();
		let seeds = proto.hysteresis_seeds(NoiseConfig::new(procedural_common::NoiseParams {
			seed: 7,
			..Default::default()
		}));
		let shoots = seeds[0].next_hysteresis();
		for shoot in &shoots {
			let branch = shoot.active_branch_profile().expect("shoot branch");
			let parent = branch.node.position;
			let tip = branch.project_tip();
			let delta = tip.position - parent;
			assert!(delta.y > 0.0, "shoot should grow upward, delta {delta:?}");
			let horiz = (delta.x * delta.x + delta.z * delta.z).sqrt();
			let elev_deg = (delta.y / horiz.max(1e-6)).atan().to_degrees();
			assert!(elev_deg > 35.0, "expected strong vertical bias, elevation {elev_deg}°");
		}
		Ok(())
	}

	#[test]
	fn default_angle_tolerance_matches_torch_trees() {
		let proto = HighBushProtoAnchors::default();
		assert!(
			(proto.angle_tolerance_radians.to_degrees() - DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES)
				.abs() < 1e-4
		);
		assert!((proto.bias_blend - DEFAULT_BIAS_BLEND).abs() < 1e-5);
	}

	#[test]
	fn shoot_indices_are_distinct() -> anyhow::Result<()> {
		let seeds = HighBushProtoAnchors::default()
			.hysteresis_seeds(NoiseConfig::new(procedural_common::NoiseParams::default()));
		assert_eq!(seeds.len(), 1);
		let root = &seeds[0];
		let shoots = root.next_hysteresis();
		let indices: Vec<_> = shoots.iter().filter_map(|s| s.shoot_index).collect();
		assert_eq!(indices.len(), DEFAULT_SHOOT_COUNT as usize);
		assert_eq!(indices.iter().collect::<std::collections::HashSet<_>>().len(), indices.len());
		Ok(())
	}
}
