//! Portal assignment → [`ClippedArcSweep`] for circular storey rings.

use bevy_math::Vec3;
use building_components::partitions::PartitionStyle;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::arcs::clipped_sweep::ClippedArcSweep;
use crate::portals::{
	assign_portals, AssignedPortal, MustAssignPortal, PortalFootprint, WallRegion,
};

/// Kit segment size (degrees); portal width is two segments → 30°.
const SEG_DEG: f32 = 15.0;
const PORTAL_SEGS: u32 = 2;
const OPEN_HALF_DEG: f32 = SEG_DEG * (PORTAL_SEGS as f32) * 0.5;

/// Parameters for [`portal_ring_wall`].
#[derive(Debug, Clone)]
pub struct PortalRingParams {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	/// Degrees of arc (\((0, 360]\); \(360\) is a closed ring).
	pub arc_degrees: f32,
	/// World yaw (radians) of the sweep start (\(t = 0\)).
	pub start_yaw: f32,
	pub must_assign: Vec<MustAssignPortal>,
	pub must_not_assign: Vec<WallRegion>,
	pub portal_noise: NoiseParams,
	pub optional_portals: (u32, u32),
	pub style: PartitionStyle,
}

/// Assigned portals plus the fitted clipped circular sweep.
#[derive(Debug, Clone, PartialEq)]
pub struct PortalRingWall {
	pub portals: Vec<AssignedPortal>,
	pub sweep: ClippedArcSweep,
}

/// Assign portals, convert footprints to clip intervals, build [`ClippedArcSweep`].
pub fn portal_ring_wall(params: PortalRingParams) -> PortalRingWall {
	let radius = params.radius.max(1e-4);
	let storey_height = params.storey_height.max(1e-4);
	let arc_degrees = params.arc_degrees.clamp(SEG_DEG, 360.0);
	let closed = (arc_degrees - 360.0).abs() < 0.5;
	let slots = ((arc_degrees / SEG_DEG).round() as u32).max(1);
	let half_t = OPEN_HALF_DEG / arc_degrees.max(SEG_DEG);
	let foot = PortalFootprint { half_t, closed };
	let noise = NoiseConfig::new(params.portal_noise);
	let portals = assign_portals(
		&noise,
		&params.must_assign,
		&params.must_not_assign,
		params.optional_portals,
		foot,
		slots,
	);
	let clips = portals
		.iter()
		.flat_map(|p| portal_clip_intervals(p.t, half_t))
		.collect::<Vec<_>>();
	let sweep = ClippedArcSweep::new(
		params.center_xz,
		radius,
		storey_height,
		arc_degrees,
		params.start_yaw,
		params.style,
		clips,
	);
	PortalRingWall { portals, sweep }
}

/// Non-wrapping clip pieces for a portal centered at \(t\) with half-width `half_t`.
fn portal_clip_intervals(t: f32, half_t: f32) -> Vec<(f32, f32)> {
	let t0 = t - half_t;
	let t1 = t + half_t;
	if t0 >= 0.0 && t1 <= 1.0 {
		vec![(t0, t1)]
	} else if t0 < 0.0 {
		let mut out = Vec::new();
		if t1 > 1e-4 {
			out.push((0.0, t1.min(1.0)));
		}
		let wrap0 = (t0 + 1.0).clamp(0.0, 1.0);
		if 1.0 - wrap0 > 1e-4 {
			out.push((wrap0, 1.0));
		}
		out
	} else {
		let mut out = Vec::new();
		if 1.0 - t0 > 1e-4 {
			out.push((t0.min(1.0), 1.0));
		}
		let wrap1 = (t1 - 1.0).clamp(0.0, 1.0);
		if wrap1 > 1e-4 {
			out.push((0.0, wrap1));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::portals::Portal;
	use building_components::partitions::Partition;

	fn cardinal() -> Vec<MustAssignPortal> {
		vec![
			MustAssignPortal::at(0.0, Portal::Door),
			MustAssignPortal::at(0.25, Portal::Window),
			MustAssignPortal::at(0.5, Portal::Window),
			MustAssignPortal::at(0.75, Portal::Window),
		]
	}

	#[test]
	fn closed_ring_assigns_cardinals() {
		let wall = portal_ring_wall(PortalRingParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			arc_degrees: 360.0,
			start_yaw: 0.0,
			must_assign: cardinal(),
			must_not_assign: vec![],
			portal_noise: NoiseParams { seed: 1, ..NoiseParams::default() },
			optional_portals: (0, 0),
			style: PartitionStyle::RoughStonework,
		});
		assert_eq!(wall.portals.len(), 4);
		assert!(wall
			.sweep
			.partitions
			.iter()
			.any(|p| matches!(p.geometry, Partition::SliceArc(_))));
		assert!(wall.sweep.partitions.iter().any(|p| matches!(p.geometry, Partition::Arc(_))));
	}

	#[test]
	fn portal_at_zero_splits_wrap_clips() {
		let clips = portal_clip_intervals(0.0, 0.05);
		assert_eq!(clips.len(), 2);
	}
}
