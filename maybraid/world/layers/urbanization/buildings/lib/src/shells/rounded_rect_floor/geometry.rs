//! Straight runs and quarter-arc corner loci for a rounded rectangle.

use bevy_math::{Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

use crate::shells::ortho::{OrthoSide, PlanRect, WallEdge};

use super::RoundedRectFloorParams;

/// Corner of a rounded rectangle (SE, NE, NW, SW in CCW order from south-east).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoundedRectCorner {
	/// +X, −Z
	SouthEast = 0,
	/// +X, +Z
	NorthEast = 1,
	/// −X, +Z
	NorthWest = 2,
	/// −X, −Z
	SouthWest = 3,
}

impl RoundedRectCorner {
	pub fn all() -> [Self; 4] {
		[Self::SouthEast, Self::NorthEast, Self::NorthWest, Self::SouthWest]
	}

	pub fn index(self) -> usize {
		self as usize
	}

	/// Arc center in plan.
	pub fn center(self, plan: PlanRect, radius: f32) -> Vec3 {
		let r = radius.max(0.0);
		match self {
			Self::SouthEast => {
				Vec3::new(plan.center.x + plan.half_x - r, plan.y, plan.center.z - plan.half_z + r)
			}
			Self::NorthEast => {
				Vec3::new(plan.center.x + plan.half_x - r, plan.y, plan.center.z + plan.half_z - r)
			}
			Self::NorthWest => {
				Vec3::new(plan.center.x - plan.half_x + r, plan.y, plan.center.z + plan.half_z - r)
			}
			Self::SouthWest => {
				Vec3::new(plan.center.x - plan.half_x + r, plan.y, plan.center.z - plan.half_z + r)
			}
		}
	}

	/// Plan angle (from +X toward +Z) of the exterior quarter start, walking CCW.
	pub fn start_angle(self) -> f32 {
		match self {
			Self::SouthEast => -FRAC_PI_2,
			Self::NorthEast => 0.0,
			Self::NorthWest => FRAC_PI_2,
			Self::SouthWest => std::f32::consts::PI,
		}
	}

	/// [`crate::arcs::ArcSweep`] / [`ClippedArcSweep`] placement yaw for this quarter.
	///
	/// Kit start is local +X; [`building_components::arc_ring_dir`] maps yaw
	/// \(\phi\) to \((\cos\phi,\,-\sin\phi)\). That equals our plan start when
	/// \(\phi = -\texttt{start_angle}\).
	pub fn start_yaw(self) -> f32 {
		-self.start_angle()
	}

	pub fn outward(self) -> Vec2 {
		match self {
			Self::SouthEast => Vec2::new(1.0, -1.0).normalize(),
			Self::NorthEast => Vec2::new(1.0, 1.0).normalize(),
			Self::NorthWest => Vec2::new(-1.0, 1.0).normalize(),
			Self::SouthWest => Vec2::new(-1.0, -1.0).normalize(),
		}
	}
}

/// Resolved wall geometry for one storey.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RoundedRectGeom {
	pub plan: PlanRect,
	pub radius: f32,
	pub height: f32,
	/// Straight edges in OrthoSide order (South, East, North, West).
	pub straights: [WallEdge; 4],
}

impl RoundedRectFloorParams {
	pub(super) fn resolve_geometry(&self, plan: PlanRect, radius: f32) -> RoundedRectGeom {
		let height = self.storey_height.max(1e-4);
		let r = radius.max(0.0);

		let straights = if r < 1e-4 {
			[
				WallEdge::new(plan.sw(), plan.se(), height, OrthoSide::South.orientation()),
				WallEdge::new(plan.se(), plan.ne(), height, OrthoSide::East.orientation()),
				WallEdge::new(plan.ne(), plan.nw(), height, OrthoSide::North.orientation()),
				WallEdge::new(plan.nw(), plan.sw(), height, OrthoSide::West.orientation()),
			]
		} else {
			[
				WallEdge::new(
					Vec3::new(plan.center.x - plan.half_x + r, plan.y, plan.center.z - plan.half_z),
					Vec3::new(plan.center.x + plan.half_x - r, plan.y, plan.center.z - plan.half_z),
					height,
					OrthoSide::South.orientation(),
				),
				WallEdge::new(
					Vec3::new(plan.center.x + plan.half_x, plan.y, plan.center.z - plan.half_z + r),
					Vec3::new(plan.center.x + plan.half_x, plan.y, plan.center.z + plan.half_z - r),
					height,
					OrthoSide::East.orientation(),
				),
				WallEdge::new(
					Vec3::new(plan.center.x + plan.half_x - r, plan.y, plan.center.z + plan.half_z),
					Vec3::new(plan.center.x - plan.half_x + r, plan.y, plan.center.z + plan.half_z),
					height,
					OrthoSide::North.orientation(),
				),
				WallEdge::new(
					Vec3::new(plan.center.x - plan.half_x, plan.y, plan.center.z + plan.half_z - r),
					Vec3::new(plan.center.x - plan.half_x, plan.y, plan.center.z - plan.half_z + r),
					height,
					OrthoSide::West.orientation(),
				),
			]
		};

		RoundedRectGeom { plan, radius: r, height, straights }
	}
}

/// Core plan rectangle inset by the corner radius (straight fill between corners).
pub(super) fn core_plan(plan: PlanRect, radius: f32) -> PlanRect {
	let r = radius.max(0.0);
	PlanRect::new(
		plan.center,
		(plan.full_x() - 2.0 * r).max(1e-3),
		(plan.full_z() - 2.0 * r).max(1e-3),
	)
}
