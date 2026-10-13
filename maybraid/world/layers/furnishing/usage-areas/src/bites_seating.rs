//! Sit-down pocket → communal tables with wide spacing.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use building_components::furniture::abutment::local_scale_for_yaw;
use building_components::{FurnitureNode, FurnitureUsageNode, Placement};

use crate::region::{along_is_x, along_span, clamp_grid, floor_height_aabb, stamp_make};

const COMMUNAL: Vec3 = Vec3::new(2.20, 1.00, 1.40);
const CAFE: Vec3 = Vec3::new(1.25, 1.00, 0.90);
const CHAIR: Vec3 = Vec3::new(0.52, 1.10, 0.52);
const CHAIR_GAP: f32 = 0.20;
const CELL: f32 = 3.8;
const MAX_TABLES: usize = 8;

/// Expand a [`FurnitureUsage::BitesSeating`](building_components::FurnitureUsage::BitesSeating) pocket.
pub struct BitesSeatingUsage;

impl BitesSeatingUsage {
	/// Expand a sit-down pocket carved from another usage leftover.
	pub fn expand_aabb(region: &Aabb3d, host: &Aabb3d) -> Vec<FurnitureNode> {
		let mut node = FurnitureUsageNode::bites_seating(Placement::IDENTITY);
		node.stamp_region(region, host);
		Self::expand(&node)
	}

	pub fn expand(node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
		let region = node.region_aabb();
		let sx = region.max.x - region.min.x;
		let sz = region.max.z - region.min.z;
		if sx < 1.5 || sz < 1.5 {
			return spare_chairs(&region, node);
		}

		let n_x = count_cells(sx, CELL, 4).max(1);
		let n_z = count_cells(sz, CELL, 4).max(1);
		let (n_x, n_z) = clamp_grid(n_x, n_z, MAX_TABLES);
		let cell_x = sx / n_x as f32;
		let cell_z = sz / n_z as f32;

		let mut out = Vec::new();
		for jz in 0..n_z {
			for ix in 0..n_x {
				let cell = Aabb3d::from_min_max(
					Vec3::new(
						region.min.x + ix as f32 * cell_x,
						region.min.y,
						region.min.z + jz as f32 * cell_z,
					),
					Vec3::new(
						region.min.x + (ix as f32 + 1.0) * cell_x,
						region.max.y,
						region.min.z + (jz as f32 + 1.0) * cell_z,
					),
				);
				out.extend(table_set(&cell, &region));
			}
		}
		if out.is_empty() {
			return spare_chairs(&region, node);
		}
		out
	}
}

fn count_cells(span: f32, target: f32, max: usize) -> usize {
	((span / target).floor() as usize).clamp(0, max)
}

fn table_set(cell: &Aabb3d, host: &Aabb3d) -> Vec<FurnitureNode> {
	let communal = try_table(cell, host, COMMUNAL, true);
	if !communal.is_empty() {
		return communal;
	}
	try_table(cell, host, CAFE, false)
}

fn try_table(cell: &Aabb3d, host: &Aabb3d, size: Vec3, six_chairs: bool) -> Vec<FurnitureNode> {
	let c = (cell.min + cell.max) * 0.5;
	let table = clamp_xz(
		Aabb3d::from_min_max(
			Vec3::new(c.x - size.x * 0.5, cell.min.y, c.z - size.z * 0.5),
			Vec3::new(c.x + size.x * 0.5, cell.min.y + size.y, c.z + size.z * 0.5),
		),
		cell,
	);
	if (table.max.x - table.min.x) < size.x * 0.72 || (table.max.z - table.min.z) < size.z * 0.72 {
		return Vec::new();
	}
	let mut out = vec![stamp_make(FurnitureNode::table, &table, host, None)];
	let tc = (table.min + table.max) * 0.5;
	let ox = (table.max.x - table.min.x) * 0.5 + CHAIR.x * 0.5 + CHAIR_GAP;
	let oz = (table.max.z - table.min.z) * 0.5 + CHAIR.z * 0.5 + CHAIR_GAP;
	let mut seats = vec![
		(0.0, -oz, 0.0_f32),
		(0.0, oz, std::f32::consts::PI),
		(-ox, 0.0, std::f32::consts::FRAC_PI_2),
		(ox, 0.0, -std::f32::consts::FRAC_PI_2),
	];
	if six_chairs {
		let hx = (table.max.x - table.min.x) * 0.28;
		seats = vec![
			(-hx, -oz, 0.0),
			(hx, -oz, 0.0),
			(-hx, oz, std::f32::consts::PI),
			(hx, oz, std::f32::consts::PI),
			(-ox, 0.0, std::f32::consts::FRAC_PI_2),
			(ox, 0.0, -std::f32::consts::FRAC_PI_2),
		];
	}
	for (dx, dz, yaw) in seats {
		let chair = Aabb3d::from_min_max(
			Vec3::new(tc.x + dx - CHAIR.x * 0.5, cell.min.y, tc.z + dz - CHAIR.z * 0.5),
			Vec3::new(tc.x + dx + CHAIR.x * 0.5, cell.min.y + CHAIR.y, tc.z + dz + CHAIR.z * 0.5),
		);
		if !contains_xz(cell, &chair) {
			continue;
		}
		out.push(stamp_facing(FurnitureNode::chair, &chair, host, yaw));
	}
	out
}

fn clamp_xz(slot: Aabb3d, host: &Aabb3d) -> Aabb3d {
	let pad = 0.10;
	Aabb3d::from_min_max(
		Vec3::new(slot.min.x.max(host.min.x + pad), slot.min.y, slot.min.z.max(host.min.z + pad)),
		Vec3::new(slot.max.x.min(host.max.x - pad), slot.max.y, slot.max.z.min(host.max.z - pad)),
	)
}

fn contains_xz(host: &Aabb3d, slot: &Aabb3d) -> bool {
	slot.min.x >= host.min.x - 1e-3
		&& slot.max.x <= host.max.x + 1e-3
		&& slot.min.z >= host.min.z - 1e-3
		&& slot.max.z <= host.max.z + 1e-3
}

fn stamp_facing(
	make: fn(Placement) -> FurnitureNode,
	slot: &Aabb3d,
	host: &Aabb3d,
	yaw: f32,
) -> FurnitureNode {
	let mut node = stamp_make(make, slot, host, None);
	node.abutment = None;
	node.placement.yaw = yaw;
	node.placement.scale = local_scale_for_yaw(slot, yaw);
	node
}

fn spare_chairs(region: &Aabb3d, node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
	let along_x = along_is_x(region, node.abutment);
	let along = along_span(region, along_x);
	let n = ((along / 1.4).floor() as usize).clamp(1, 3);
	let mut out = Vec::new();
	for i in 0..n {
		let t = (i as f32 + 0.5) / n as f32;
		let c = if along_x {
			Vec3::new(region.min.x + along * t, region.min.y, (region.min.z + region.max.z) * 0.5)
		} else {
			Vec3::new((region.min.x + region.max.x) * 0.5, region.min.y, region.min.z + along * t)
		};
		let slot = Aabb3d::from_min_max(
			Vec3::new(c.x - CHAIR.x * 0.5, c.y, c.z - CHAIR.z * 0.5),
			Vec3::new(c.x + CHAIR.x * 0.5, c.y + CHAIR.y, c.z + CHAIR.z * 0.5),
		);
		let slot = clamp_xz(slot, region);
		if (slot.max.x - slot.min.x) < 0.3 || (slot.max.z - slot.min.z) < 0.3 {
			continue;
		}
		out.push(stamp_make(FurnitureNode::chair, &slot, region, node.abutment));
	}
	if out.is_empty() {
		let slab = floor_height_aabb(region, CHAIR.y);
		let c = (slab.min + slab.max) * 0.5;
		let slot = Aabb3d::from_min_max(
			Vec3::new(c.x - CHAIR.x * 0.5, slab.min.y, c.z - CHAIR.z * 0.5),
			Vec3::new(c.x + CHAIR.x * 0.5, slab.min.y + CHAIR.y, c.z + CHAIR.z * 0.5),
		);
		out.push(stamp_make(FurnitureNode::chair, &clamp_xz(slot, region), region, node.abutment));
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::{FurnitureGeometry, Placement};

	fn seating_node(min: Vec3, max: Vec3) -> FurnitureUsageNode {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(16.0, 3.5, 12.0));
		let box_ = Aabb3d::from_min_max(min, max);
		let mut node = FurnitureUsageNode::bites_seating(Placement::IDENTITY);
		node.stamp_region(&box_, &host);
		node
	}

	#[test]
	fn roomy_pocket_gets_communal_tables() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(8.0, 3.2, 7.0));
		let pieces = BitesSeatingUsage::expand(&node);
		let tables = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Table).count();
		let chairs = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Chair).count();
		assert!(tables >= 2, "expected a couple communal tables, got {tables}");
		assert!(tables <= 4, "spacing should stay open, got {tables}");
		assert!(chairs >= 8, "communal tables should seat several, got {chairs}");
	}

	#[test]
	fn large_pocket_stays_sparse() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(12.0, 3.2, 8.0));
		let pieces = BitesSeatingUsage::expand(&node);
		let tables = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Table).count();
		assert!(
			(2..=MAX_TABLES).contains(&tables),
			"large seating should stay sparse, got {tables}"
		);
		let chairs = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Chair).count();
		assert!(
			chairs as f32 / tables as f32 >= 3.0,
			"more chairs per table, chairs {chairs} tables {tables}"
		);
	}

	#[test]
	fn shallow_door_face_pocket_still_gets_a_table() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 3.2, 1.7));
		let pieces = BitesSeatingUsage::expand(&node);
		assert!(
			pieces.iter().any(|n| n.geometry == FurnitureGeometry::Table)
				|| pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Chair).count() >= 2,
			"shallow seating should still allocate, got {:?}",
			pieces.iter().map(|n| n.geometry).collect::<Vec<_>>()
		);
	}

	#[test]
	fn tight_pocket_still_gets_a_chair() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.3, 3.0, 1.3));
		let pieces = BitesSeatingUsage::expand(&node);
		assert!(pieces.iter().any(|n| n.geometry == FurnitureGeometry::Chair));
	}
}
