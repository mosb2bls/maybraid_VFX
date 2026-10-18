//! Noisy 3D path → [`ClippedRectangularStrip`] of oriented rectangle kits.
//!
//! Strip nodes follow the sampled path with authored height and roll `0`
//! (top toward `+Y`). Optional portal at \(t\) becomes a [`RectInset`] on the
//! bay that contains that arc-length fraction.

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::{AllowedAngles, NoiseParams, NoisyPathParams, StepLenRange};

use crate::paneling::{
	ClippedRectangularStrip, PanelComplexJointPolicy, RectInset, RectangularStripNode,
	DEFAULT_PANEL_THICKNESS,
};
use crate::portals::{
	assign_portals, AssignedPortal, MustAssignPortal, PortalFootprint, WallRegion,
};

/// Parameters for [`NoisyRectangularWall::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyRectangularWallParams {
	pub start: Vec3,
	pub initial_dir: Vec3,
	pub distance: f32,
	pub step_len: StepLenRange,
	pub allowed_angles: AllowedAngles,
	pub path_noise: NoiseParams,
	pub height: f32,
	pub thickness: f32,
	pub style: PanelStyle,
	pub joint_policy: PanelComplexJointPolicy,
	/// Uniform inset applied on portal bays (panel units).
	pub portal_inset: f32,
	pub must_assign: Vec<MustAssignPortal>,
	pub must_not_assign: Vec<WallRegion>,
	pub portal_noise: NoiseParams,
	pub optional_portals: (u32, u32),
}

impl Default for NoisyRectangularWallParams {
	fn default() -> Self {
		Self {
			start: Vec3::ZERO,
			initial_dir: Vec3::Z,
			distance: 12.0,
			step_len: StepLenRange::new(0.75, 1.25),
			allowed_angles: AllowedAngles::yaw_pitch(
				std::f32::consts::FRAC_PI_6,
				std::f32::consts::FRAC_PI_8,
			),
			path_noise: NoiseParams { seed: 1337, frequency: 0.35, ..NoiseParams::default() },
			height: 3.0,
			thickness: DEFAULT_PANEL_THICKNESS,
			style: PanelStyle::RoughStonework,
			joint_policy: PanelComplexJointPolicy::default(),
			portal_inset: 0.35,
			must_assign: vec![],
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
		}
	}
}

/// Noisy path wall demo → rectangle strip (+ optional inset openings).
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyRectangularWall {
	pub path_noise: NoiseParams,
	pub allowed_angles: AllowedAngles,
	pub distance: f32,
	pub step_len: StepLenRange,
	pub points: Vec<Vec3>,
	pub portals: Vec<AssignedPortal>,
	pub strip: ClippedRectangularStrip,
}

impl NoisyRectangularWall {
	pub fn new(params: NoisyRectangularWallParams) -> Self {
		let step_len = StepLenRange::new(params.step_len.min, params.step_len.max);
		let points = NoisyPathParams {
			start: params.start,
			initial_dir: params.initial_dir,
			distance: params.distance,
			step_len,
			allowed_angles: params.allowed_angles,
			noise: params.path_noise,
		}
		.generate();

		let bay_count = points.len().saturating_sub(1);
		let closed = false;
		let half_t = 0.5 / bay_count.max(1) as f32;
		let foot = PortalFootprint { half_t, closed };
		let portals = assign_portals(
			&procedural_common::NoiseConfig::new(params.portal_noise),
			&params.must_assign,
			&params.must_not_assign,
			params.optional_portals,
			foot,
			bay_count.max(1) as u32,
		);

		let mut insets: Vec<Option<RectInset>> = vec![None; bay_count];
		let cum = cumulative_lengths(&points);
		let total = *cum.last().unwrap_or(&1.0);
		for portal in &portals {
			if let Some(bay) = bay_for_t(&cum, total, portal.t) {
				if bay < insets.len() {
					insets[bay] = Some(RectInset::uniform(params.portal_inset.max(0.05)));
				}
			}
		}

		let height = params.height.max(1e-4);
		let thick = params.thickness.max(1e-4);
		let nodes: Vec<RectangularStripNode> = points
			.iter()
			.map(|p| RectangularStripNode::new(*p, height, thick, 0.0))
			.collect();

		let strip = ClippedRectangularStrip::from_nodes(params.style, nodes, insets)
			.with_joint_policy(params.joint_policy);

		Self {
			path_noise: params.path_noise,
			allowed_angles: params.allowed_angles,
			distance: params.distance.max(0.0),
			step_len,
			points,
			portals,
			strip,
		}
	}
}

impl BuildingComponents for NoisyRectangularWall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.strip.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.strip.joint_nodes_for_level(level)
	}
}

fn cumulative_lengths(points: &[Vec3]) -> Vec<f32> {
	let mut out = vec![0.0];
	for w in points.windows(2) {
		out.push(out.last().copied().unwrap_or(0.0) + (w[1] - w[0]).length());
	}
	out
}

fn bay_for_t(cum: &[f32], total: f32, t: f32) -> Option<usize> {
	if cum.len() < 2 {
		return None;
	}
	let s = t.clamp(0.0, 1.0) * total.max(1e-4);
	for i in 0..cum.len() - 1 {
		if s <= cum[i + 1] + 1e-5 {
			return Some(i);
		}
	}
	Some(cum.len() - 2)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::portals::Portal;

	#[test]
	fn builds_strip_from_noisy_path() {
		let wall = NoisyRectangularWall::new(NoisyRectangularWallParams {
			distance: 10.0,
			step_len: StepLenRange::new(0.75, 1.25),
			allowed_angles: AllowedAngles::yaw_pitch(0.4, 0.2),
			path_noise: NoiseParams { seed: 9, ..NoiseParams::default() },
			must_assign: vec![MustAssignPortal::at(0.5, Portal::Window)],
			optional_portals: (0, 0),
			..NoisyRectangularWallParams::default()
		});
		assert!(wall.points.len() >= 2);
		assert!(!wall.strip.pieces().is_empty());
		assert_eq!(wall.portals.len(), 1);
		assert!(wall
			.strip
			.pieces()
			.iter()
			.any(|p| matches!(p, crate::paneling::ClippedRectangularStripPiece::Clipped(_))));
	}
}
