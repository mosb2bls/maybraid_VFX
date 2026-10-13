//! Outer / inner wall sides and frame slab bands for [`RectRingFloor`].

use bevy_math::{Vec2, Vec3};

use crate::shells::ortho::{PlanRect, WallEdge, EPS};

use super::RectRingFloorParams;

/// Axis-aligned plan rectangle (full extents) for frame slab bands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlanAabb {
	pub min_x: f32,
	pub max_x: f32,
	pub min_z: f32,
	pub max_z: f32,
}

impl PlanAabb {
	pub fn new(min_x: f32, max_x: f32, min_z: f32, max_z: f32) -> Self {
		Self {
			min_x: min_x.min(max_x),
			max_x: min_x.max(max_x),
			min_z: min_z.min(max_z),
			max_z: min_z.max(max_z),
		}
	}

	pub fn to_plan_rect(self, y: f32) -> PlanRect {
		PlanRect::new(
			Vec3::new(0.5 * (self.min_x + self.max_x), y, 0.5 * (self.min_z + self.max_z)),
			(self.max_x - self.min_x).max(EPS),
			(self.max_z - self.min_z).max(EPS),
		)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RectRingGeom {
	pub y0: f32,
	pub height: f32,
	pub slab_rects: Vec<PlanAabb>,
	pub edges: Vec<WallEdge>,
}

impl RectRingFloorParams {
	pub(super) fn resolve_geometry(&self) -> RectRingGeom {
		let cx = self.center_xz.x;
		let cz = self.center_xz.z;
		let y0 = self.center_xz.y;
		let height = self.storey_height.max(EPS);

		let ox0 = cx - self.outer.x * 0.5;
		let ox1 = cx + self.outer.x * 0.5;
		let oz0 = cz - self.outer.y * 0.5;
		let oz1 = cz + self.outer.y * 0.5;
		let ix0 = cx - self.inner.x * 0.5;
		let ix1 = cx + self.inner.x * 0.5;
		let iz0 = cz - self.inner.y * 0.5;
		let iz1 = cz + self.inner.y * 0.5;

		let mut edges = Vec::new();
		// Outer loop: CCW S→E→N→W. Gallery-facing normals point into the ring.
		push_cardinal_sides(
			&mut edges,
			height,
			[
				// South: SW→SE, gallery +Z
				(Vec3::new(ox0, y0, oz0), Vec3::new(ox1, y0, oz0), Vec2::new(0.0, 1.0)),
				// East: SE→NE, gallery −X
				(Vec3::new(ox1, y0, oz0), Vec3::new(ox1, y0, oz1), Vec2::new(-1.0, 0.0)),
				// North: NE→NW, gallery −Z
				(Vec3::new(ox1, y0, oz1), Vec3::new(ox0, y0, oz1), Vec2::new(0.0, -1.0)),
				// West: NW→SW, gallery +X
				(Vec3::new(ox0, y0, oz1), Vec3::new(ox0, y0, oz0), Vec2::new(1.0, 0.0)),
			],
		);
		// Inner loop: CW so gallery-facing normals point into the ring corridor.
		if self.inner_walls {
			push_cardinal_sides(
				&mut edges,
				height,
				[
					// South: SE→SW (CW), gallery −Z
					(Vec3::new(ix1, y0, iz0), Vec3::new(ix0, y0, iz0), Vec2::new(0.0, -1.0)),
					// East: NE→SE (CW), gallery +X
					(Vec3::new(ix1, y0, iz1), Vec3::new(ix1, y0, iz0), Vec2::new(1.0, 0.0)),
					// North: NW→NE (CW), gallery +Z
					(Vec3::new(ix0, y0, iz1), Vec3::new(ix1, y0, iz1), Vec2::new(0.0, 1.0)),
					// West: SW→NW (CW), gallery −X
					(Vec3::new(ix0, y0, iz0), Vec3::new(ix0, y0, iz1), Vec2::new(-1.0, 0.0)),
				],
			);
		}

		// Frame bands: N/S take full outer width; E/W take the inner depth only.
		let mut slab_rects = Vec::new();
		if oz1 - iz1 > EPS {
			slab_rects.push(PlanAabb::new(ox0, ox1, iz1, oz1)); // North
		}
		if ix0 - ox0 > EPS {
			slab_rects.push(PlanAabb::new(ox0, ix0, iz0, iz1)); // West
		}
		if iz0 - oz0 > EPS {
			slab_rects.push(PlanAabb::new(ox0, ox1, oz0, iz0)); // South
		}
		if ox1 - ix1 > EPS {
			slab_rects.push(PlanAabb::new(ix1, ox1, iz0, iz1)); // East
		}

		RectRingGeom { y0, height, slab_rects, edges }
	}
}

fn push_cardinal_sides(edges: &mut Vec<WallEdge>, height: f32, sides: [(Vec3, Vec3, Vec2); 4]) {
	for (start, end, outward) in sides {
		if start.distance(end) < EPS {
			continue;
		}
		edges.push(WallEdge::new(start, end, height, outward));
	}
}
