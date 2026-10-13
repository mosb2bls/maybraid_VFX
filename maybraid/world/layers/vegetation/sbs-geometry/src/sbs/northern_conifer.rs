//! Restricted **Northern Conifer** geometry for CLI and playgrounds ([#232](https://github.com/ramate-io/maybraid/issues/232)).
//!
//! Wraps [`LiamsConiferSbs`](super::liams_conifer::LiamsConiferSbs) with Northern ring, projection, and growth defaults
//! (RFC §3.1.7.11). Flattened clap exposes Liam field defaults; call [`NorthernConiferSbs::apply_northern_preset`]
//! after CLI parse (see [`sbs_trees::northern_conifer`](../../sbs-trees/src/northern_conifer.rs)).

#[cfg(feature = "clap")]
use clap::Args;
use procedural_common::{NoiseConfig, UnitRange};

use super::liams_conifer::{LiamsConiferSbs, RingAnchorParams};
use crate::anchors::liams_conifer::{LiamsConiferAnchors, LiamsConiferProtoAnchors};
use crate::anchors::{Anchors, AnchorsToChain};
use crate::sbs::scale::{stalk_radius_scaled_range, stalk_scaled_range};
use crate::sbs::storybook_tree::{apply_storybook_field_preset, apply_unit_range_preset};
use crate::{BallStickChain, LiamsConiferChain};

/// Full-size Liam's Conifer defaults used to scale mini-tree anchor perturbation.
const REFERENCE_STALK_HEIGHT: f32 = 30.0;
const REFERENCE_STALK_BASE_RADIUS: f32 = 0.025 * REFERENCE_STALK_HEIGHT;

/// First ring height as a fraction of stalk height (higher than Liam's `0.10`).
pub const NORTHERN_RING_HEIGHTS_START: f32 = 0.1;
/// Last ring through the crown.
pub const NORTHERN_RING_HEIGHTS_END: f32 = 1.0;
pub const NORTHERN_RING_SPACING: f32 = 0.035;

/// Stalk base radius as a fraction of `H` (Liam default `0.025`).
pub const NORTHERN_STALK_BASE_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.032;

/// Max projection as a fraction of `H` (Liam default `0.20`; Northern slightly longer).
pub const NORTHERN_MAX_PROJECTION_FRACTION_OF_HEIGHT: f32 = 0.24;

/// Joint radius at ring spokes as a fraction of stalk base radius (Liam `0.1`).
pub const NORTHERN_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.14;

/// Fixed limb hops per spoke ([`crate::chain::liams_conifer::SEGMENT_FRACS`]); taper is ring [`projection_length`] only.
pub const NORTHERN_BRANCH_DEPTH: usize = 3;

/// Downward radial bias in degrees (Liam default `2.0`).
pub const NORTHERN_DOWNWARD_BIAS_DEGREES: f32 = 25.0;

/// Art-directed front-end for Northern Conifer: Liam's SBS with Northern defaults.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct NorthernConiferSbs {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub liams: LiamsConiferSbs,
}

fn northern_liams_fields() -> LiamsConiferSbs {
	let mut liams = LiamsConiferSbs::default();
	NorthernConiferSbs::apply_northern_defaults(&mut liams);
	liams
}

impl NorthernConiferSbs {
	pub fn apply_northern_defaults(liams: &mut LiamsConiferSbs) {
		liams.rings.height_range =
			UnitRange::new(NORTHERN_RING_HEIGHTS_START, NORTHERN_RING_HEIGHTS_END);
		liams.rings.spacing = NORTHERN_RING_SPACING;
		liams.projection.length_fraction_of_height =
			UnitRange::new(NORTHERN_MAX_PROJECTION_FRACTION_OF_HEIGHT, 0.0);
		liams.growth.branch_depth = NORTHERN_BRANCH_DEPTH;
		liams.growth.downward_bias_degrees = NORTHERN_DOWNWARD_BIAS_DEGREES;
	}

	/// Reapply Northern preset after flattened clap parse.
	///
	/// Only fields that still equal [`LiamsConiferSbs::default`] are overwritten, so explicit CLI
	/// overrides (e.g. `--stalk-height`, `--ring-heights`) are preserved.
	pub fn apply_northern_preset(&mut self) {
		let northern = Self::default();
		let liams = LiamsConiferSbs::default();
		let l = &mut self.liams;
		let h = l.scale.stalk_height.max(1e-6);

		let liams_stalk = liams.scale.stalk_base_radius_or_default();
		let northern_stalk = h * NORTHERN_STALK_BASE_RADIUS_FRACTION_OF_HEIGHT;
		match l.scale.stalk_base_radius {
			None => l.scale.stalk_base_radius = Some(northern_stalk),
			Some(r) if (r - liams_stalk).abs() < 1e-4 => {
				l.scale.stalk_base_radius = Some(northern_stalk);
			}
			_ => {}
		}

		apply_northern_ring_preset(&mut l.rings, &liams.rings, &northern.liams.rings);
		apply_unit_range_preset(
			&mut l.projection.length_fraction_of_height,
			&liams.projection.length_fraction_of_height,
			&northern.liams.projection.length_fraction_of_height,
		);
		apply_storybook_field_preset(
			&mut l.growth.branch_depth,
			&liams.growth.branch_depth,
			&northern.liams.growth.branch_depth,
		);
		apply_storybook_field_preset(
			&mut l.growth.downward_bias_degrees,
			&liams.growth.downward_bias_degrees,
			&northern.liams.growth.downward_bias_degrees,
		);
	}

	pub fn height(&self) -> f32 {
		self.liams.scale.stalk_height
	}

	pub fn to_anchors(&self) -> LiamsConiferAnchors {
		let l = &self.liams;
		let h = l.scale.stalk_height.max(1e-6);
		let mut stalk = l.scale.to_stalk();
		let liams_stalk = 0.025 * h;
		if l.scale.stalk_base_radius.is_none()
			|| (stalk.stalk_base_radius - liams_stalk).abs() < 1e-4
		{
			stalk.stalk_base_radius = h * NORTHERN_STALK_BASE_RADIUS_FRACTION_OF_HEIGHT;
		}
		let mut perturbation = l.anchor_perturbation.to_perturbation();
		let vertical = stalk_scaled_range(
			l.anchor_perturbation.vertical_offset,
			l.scale.stalk_height,
			REFERENCE_STALK_HEIGHT,
		);
		perturbation.vertical_offset = vertical.start..vertical.end;
		let radius = stalk_radius_scaled_range(
			l.anchor_perturbation.radius_offset,
			stalk.stalk_base_radius,
			REFERENCE_STALK_BASE_RADIUS,
		);
		perturbation.radius_offset = radius.start..radius.end;
		LiamsConiferAnchors::new(LiamsConiferProtoAnchors {
			stalk,
			first_ring_unit_height: l.rings.height_range.start,
			last_ring_unit_height: l.rings.height_range.end,
			ring_spacing_unit_height: l.rings.spacing,
			anchors_per_ring: l.rings.anchors_per_ring,
			max_projection_fraction_of_height: l.projection.length_fraction_of_height.start,
			min_projection_fraction_of_max: 0.0,
			downward_bias_radians: l.growth.downward_bias_degrees.to_radians(),
			branch_angle_tolerance: l.growth.angle_tolerance_degrees.to_radians(),
			branch_depth: NORTHERN_BRANCH_DEPTH,
			branch_base_radius_fraction_of_stalk: NORTHERN_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale: (0.72, 0.80),
			linear_projection_taper: true,
		})
		.with_perturbation(perturbation)
	}

	pub fn hysteresis_seeds(&self) -> Vec<LiamsConiferChain> {
		let noise = NoiseConfig::new(self.liams.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<LiamsConiferChain> {
		AnchorsToChain::build_chain(self)
	}
}

fn apply_northern_ring_preset(
	current: &mut RingAnchorParams,
	liams: &RingAnchorParams,
	northern: &RingAnchorParams,
) {
	apply_unit_range_preset(&mut current.height_range, &liams.height_range, &northern.height_range);
	apply_storybook_field_preset(&mut current.spacing, &liams.spacing, &northern.spacing);
}

impl Default for NorthernConiferSbs {
	fn default() -> Self {
		Self { liams: northern_liams_fields() }
	}
}

impl std::ops::Deref for NorthernConiferSbs {
	type Target = LiamsConiferSbs;
	fn deref(&self) -> &Self::Target {
		&self.liams
	}
}

impl Anchors<LiamsConiferChain> for NorthernConiferSbs {
	fn anchors(&self) -> Vec<LiamsConiferChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use procedural_common::NoiseParams;

	#[test]
	fn default_projection_longer_than_liams() {
		let northern = NorthernConiferSbs::default();
		let liams = LiamsConiferSbs::default();
		assert!(
			northern.liams.projection.length_fraction_of_height.start
				> liams.projection.length_fraction_of_height.start
		);
	}

	#[test]
	fn default_frontend_converts_to_anchor_recipe() {
		let sbs = NorthernConiferSbs::default();
		let anchors = sbs.to_anchors();
		let proto = anchors.proto();

		assert_eq!(proto.first_ring_unit_height, NORTHERN_RING_HEIGHTS_START);
		assert_eq!(proto.last_ring_unit_height, NORTHERN_RING_HEIGHTS_END);
		assert_eq!(
			proto.max_projection_fraction_of_height,
			NORTHERN_MAX_PROJECTION_FRACTION_OF_HEIGHT
		);
		let h = sbs.liams.scale.stalk_height;
		assert!(
			(proto.stalk.stalk_base_radius - h * NORTHERN_STALK_BASE_RADIUS_FRACTION_OF_HEIGHT)
				.abs() < 1e-5
		);
		assert_eq!(
			proto.branch_base_radius_fraction_of_stalk,
			NORTHERN_BRANCH_BASE_RADIUS_FRACTION_OF_STALK
		);
		assert!(proto.linear_projection_taper);
		assert_eq!(proto.branch_depth, NORTHERN_BRANCH_DEPTH);
		assert!(
			(proto.downward_bias_radians - NORTHERN_DOWNWARD_BIAS_DEGREES.to_radians()).abs()
				< 1e-6
		);
	}

	#[test]
	fn linear_projection_tapers_to_zero_at_crown() {
		let sbs = NorthernConiferSbs::default();
		let anchors = sbs.to_anchors();
		let proto = anchors.proto();
		let lo = proto.projection_length(0.0);
		let hi = proto.projection_length(1.0);
		assert!(lo > hi);
		assert!(hi < 1e-3);
	}

	#[test]
	fn apply_northern_preset_after_liams_cli_defaults() -> anyhow::Result<()> {
		let mut geometry = NorthernConiferSbs { liams: LiamsConiferSbs::default() };
		geometry.apply_northern_preset();
		assert_eq!(geometry.liams.rings.height_range.start, NORTHERN_RING_HEIGHTS_START);
		assert!(
			(geometry.liams.projection.length_fraction_of_height.start
				- NORTHERN_MAX_PROJECTION_FRACTION_OF_HEIGHT)
				.abs() < 1e-6
		);
		let anchors = geometry.to_anchors();
		let proto = anchors.proto();
		assert!(
			(proto.max_projection_fraction_of_height - NORTHERN_MAX_PROJECTION_FRACTION_OF_HEIGHT)
				.abs() < 1e-6
		);
		Ok(())
	}

	#[test]
	fn build_chain_uses_northern_to_anchors_not_liams_frontend() -> anyhow::Result<()> {
		let mut geometry = NorthernConiferSbs { liams: LiamsConiferSbs::default() };
		let before = geometry.to_anchors().proto().max_projection_fraction_of_height;
		geometry.apply_northern_preset();
		let after = geometry.to_anchors().proto().max_projection_fraction_of_height;
		assert!((before - 0.20).abs() < 1e-6);
		assert_eq!(after, NORTHERN_MAX_PROJECTION_FRACTION_OF_HEIGHT);
		assert!(geometry.build_chain().nodes.len() > 10);
		Ok(())
	}

	#[test]
	fn build_chain_has_stalk_and_branches() -> anyhow::Result<()> {
		let chain = NorthernConiferSbs::default().build_chain();
		assert!(chain.nodes.len() > 10);
		Ok(())
	}

	#[test]
	fn mini_sapling_perturbation_scales_with_stalk() -> anyhow::Result<()> {
		let mut sbs = NorthernConiferSbs::default();
		sbs.liams.scale.stalk_height = 3.0;
		let anchors = sbs.to_anchors();
		let scale = sbs.liams.scale.stalk_height / REFERENCE_STALK_HEIGHT;
		assert!((anchors.perturbation.vertical_offset.start + scale).abs() < 1e-4);
		assert!((anchors.perturbation.vertical_offset.end - scale).abs() < 1e-4);
		assert!(
			anchors.perturbation.radius_offset.end
				<= anchors.proto().stalk.stalk_base_radius * 0.01
		);
		Ok(())
	}

	#[test]
	fn mini_sapling_branch_radii_stay_within_stalk_relative_bounds() -> anyhow::Result<()> {
		let mut sbs = NorthernConiferSbs::default();
		sbs.liams.scale.stalk_height = 3.0;
		sbs.liams.canopy_noise = NoiseParams::from_scalar(7.0, 1.0, 1.0, 1);
		let chain = sbs.build_chain();
		let stalk = sbs.liams.scale.stalk_base_radius_or_default();
		let max = chain
			.nodes_with_hysteresis()
			.filter_map(|(node, hysteresis)| {
				hysteresis.active_branch_profile().map(|_| node.radius)
			})
			.fold(0.0_f32, f32::max);
		assert!(max <= stalk * 0.36, "northern max branch radius {max} vs stalk {stalk}");
		Ok(())
	}
}
