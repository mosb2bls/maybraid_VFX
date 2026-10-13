//! Layer 2: floor / ceiling slabs cut by slab-relevant openings.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::floors::{Floor, FloorNode};
use building_components::Placement;

use crate::openings::Openings;

use super::ring::{aabb3d_intersects, EPS};
use super::{ArcFloorParams, ArcFloorSlab};

/// Inscribed-square half-extent as a fraction of outer radius → full side = 1.4·R.
const INSCRIBED_HALF_FRAC: f32 = 0.7;
/// Floor / ceiling slab Y scale.
const FLOOR_SLAB_Y_SCALE: f32 = 0.2;

impl ArcFloorParams {
	/// Resolve floor slab nodes for the authored [`Self::floor`] presentation.
	pub(super) fn resolve_floor_nodes(&self) -> Vec<FloorNode> {
		self.resolve_slab(self.floor, self.center_xz, self.center_xz.y)
	}

	/// Resolve ceiling slab nodes for the authored [`Self::ceiling`] presentation.
	pub(super) fn resolve_ceiling_nodes(&self) -> Vec<FloorNode> {
		let center = self.center_xz + Vec3::Y * self.storey_height;
		self.resolve_slab(self.ceiling, center, center.y)
	}

	fn resolve_slab(&self, base: ArcFloorSlab, center: Vec3, slab_y: f32) -> Vec<FloorNode> {
		match base {
			ArcFloorSlab::None => Vec::new(),
			ArcFloorSlab::Solid => resolve_solid_slab(center, self.radius, slab_y, &self.openings),
		}
	}
}

fn resolve_solid_slab(
	center: Vec3,
	radius: f32,
	slab_y: f32,
	openings: &Openings,
) -> Vec<FloorNode> {
	let max_side = 2.0 * INSCRIBED_HALF_FRAC * radius; // 1.4·R
	let slab_aabb = slab_volume_aabb(center, radius, slab_y);
	let mut hole_side: Option<f32> = None;
	let mut remove_all = false;
	for (_id, opening) in openings.iter() {
		if !opening.label.cuts_slab() {
			continue;
		}
		if !aabb3d_intersects(&opening.bounds, &slab_aabb) {
			continue;
		}
		let Some(inter) = aabb_intersection(&opening.bounds, &slab_aabb) else {
			continue;
		};
		let extent = Vec3::from(inter.max - inter.min);
		// Characteristic horizontal scale of the intersection.
		let scale = extent.x.max(extent.z);
		if scale + EPS >= max_side {
			remove_all = true;
			break;
		}
		hole_side = Some(hole_side.map_or(scale, |s| s.max(scale)));
	}
	if remove_all {
		Vec::new()
	} else if let Some(side) = hole_side {
		let inscribed_half = INSCRIBED_HALF_FRAC * radius;
		let hole_half = (side * 0.5).clamp(1e-4, inscribed_half * 0.95);
		let (caps, rects) = squared_floor_with_hole(center, radius, hole_half);
		let mut nodes = caps.to_vec();
		nodes.extend(rects);
		nodes
	} else {
		let mut nodes = inscribed_caps(center, radius);
		nodes.push(rect_slab(center, max_side, max_side));
		nodes
	}
}

fn slab_volume_aabb(center: Vec3, radius: f32, slab_y: f32) -> Aabb3d {
	let half = radius.max(1e-4);
	let y_half = FLOOR_SLAB_Y_SCALE;
	Aabb3d::from_min_max(
		Vec3::new(center.x - half, slab_y - y_half, center.z - half),
		Vec3::new(center.x + half, slab_y + y_half, center.z + half),
	)
}

fn aabb_intersection(a: &Aabb3d, b: &Aabb3d) -> Option<Aabb3d> {
	if !aabb3d_intersects(a, b) {
		return None;
	}
	let min = Vec3::from(a.min).max(Vec3::from(b.min));
	let max = Vec3::from(a.max).min(Vec3::from(b.max));
	Some(Aabb3d::from_min_max(min, max))
}

fn inscribed_caps(center_xz: Vec3, radius: f32) -> Vec<FloorNode> {
	let radius = radius.max(1e-4);
	let ring_scale = Vec3::new(radius, FLOOR_SLAB_Y_SCALE, radius);
	vec![
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, 0.0).with_scale(ring_scale),
		),
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, std::f32::consts::FRAC_PI_2).with_scale(ring_scale),
		),
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, std::f32::consts::PI).with_scale(ring_scale),
		),
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, std::f32::consts::PI + std::f32::consts::FRAC_PI_2)
				.with_scale(ring_scale),
		),
	]
}

fn squared_floor_with_hole(
	center_xz: Vec3,
	radius: f32,
	hole_half: f32,
) -> ([FloorNode; 4], [FloorNode; 4]) {
	let radius = radius.max(1e-4);
	let inscribed_half = INSCRIBED_HALF_FRAC * radius;
	let hole_half = hole_half.clamp(1e-4, inscribed_half * 0.95);
	let caps = inscribed_caps(center_xz, radius);
	let caps = [caps[0].clone(), caps[1].clone(), caps[2].clone(), caps[3].clone()];
	let inscribed_side = 2.0 * inscribed_half;

	let cx = center_xz.x;
	let cz = center_xz.z;
	let y = center_xz.y;
	let inscribed_min_z = cz - inscribed_half;
	let inscribed_max_z = cz + inscribed_half;
	let inscribed_min_x = cx - inscribed_half;
	let inscribed_max_x = cx + inscribed_half;
	let hole_min_z = cz - hole_half;
	let hole_max_z = cz + hole_half;
	let hole_min_x = cx - hole_half;
	let hole_max_x = cx + hole_half;

	let gap_s = (hole_min_z - inscribed_min_z).max(0.0);
	let gap_n = (inscribed_max_z - hole_max_z).max(0.0);
	let gap_w = (hole_min_x - inscribed_min_x).max(0.0);
	let gap_e = (inscribed_max_x - hole_max_x).max(0.0);

	let south =
		rect_slab(Vec3::new(cx, y, 0.5 * (inscribed_min_z + hole_min_z)), inscribed_side, gap_s);
	let north =
		rect_slab(Vec3::new(cx, y, 0.5 * (hole_max_z + inscribed_max_z)), inscribed_side, gap_n);
	let west =
		rect_slab(Vec3::new(0.5 * (inscribed_min_x + hole_min_x), y, cz), gap_w, inscribed_side);
	let east =
		rect_slab(Vec3::new(0.5 * (hole_max_x + inscribed_max_x), y, cz), gap_e, inscribed_side);

	(caps, [south, north, west, east])
}

fn rect_slab(center: Vec3, width_x: f32, depth_z: f32) -> FloorNode {
	let width_x = width_x.max(1e-4);
	let depth_z = depth_z.max(1e-4);
	let origin = Vec3::new(center.x - 0.5 * width_x, center.y, center.z - 0.5 * depth_z);
	FloorNode::rough_stone(
		Floor::rectangle(),
		Placement::new(origin, 0.0).with_scale(Vec3::new(width_x, FLOOR_SLAB_Y_SCALE, depth_z)),
	)
}
