//! Shared ~2 m square shelf towers, floor-to-ceiling partitions, leftover fill.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use building_components::{FurnitureAbutment, FurnitureNode};

use crate::bites_seating::BitesSeatingUsage;
use crate::region::{clamp_grid, floor_height_aabb, stamp_make};

pub const SHELF_PLAN: f32 = 2.0;
pub const SHELF_DECK: f32 = 0.75;
pub const SHELF_DECKS: usize = 5;
pub const SHELF_GAP: f32 = 1.15;

const PARTITION_LEN: f32 = 2.6;
const PARTITION_THICK: f32 = 0.22;
const HALL_PITCH: f32 = 5.0;
const KITCHEN_PITCH: f32 = 5.8;
const MAX_HALL_AXIS: usize = 8;
const MAX_KITCHEN_AXIS: usize = 4;
const MAX_HALL_CELLS: usize = 24;
const MAX_KITCHEN_CELLS: usize = 9;

/// Kitchen leftovers stay work-like; hall leftovers mix screens, stock, and lounge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftoverMood {
	Kitchen,
	Hall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CellKind {
	Partition,
	Shelf,
	Lounge,
}

/// Tile floor-standing shelf stacks in `floor`, skipping `keepouts` on XZ.
pub fn shelf_towers(
	floor: &Aabb3d,
	host: &Aabb3d,
	max_x: usize,
	max_z: usize,
	keepouts: &[Aabb3d],
) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	if sx < SHELF_PLAN + 0.3 || sz < SHELF_PLAN + 0.3 {
		return Vec::new();
	}
	let n_x = count_units(sx, max_x);
	let n_z = count_units(sz, max_z);
	if n_x == 0 || n_z == 0 {
		return Vec::new();
	}
	let pitch = SHELF_PLAN + SHELF_GAP;
	let used_x = n_x as f32 * SHELF_PLAN + n_x.saturating_sub(1) as f32 * SHELF_GAP;
	let used_z = n_z as f32 * SHELF_PLAN + n_z.saturating_sub(1) as f32 * SHELF_GAP;
	let x0 = floor.min.x + ((sx - used_x) * 0.5).max(0.12);
	let z0 = floor.min.z + ((sz - used_z) * 0.5).max(0.12);
	let decks = ((floor.max.y - floor.min.y - 0.15) / SHELF_DECK).floor() as usize;
	let decks = decks.clamp(2, SHELF_DECKS);
	let mut out = Vec::new();
	for iz in 0..n_z {
		for ix in 0..n_x {
			let x = x0 + ix as f32 * pitch;
			let z = z0 + iz as f32 * pitch;
			let plan = Aabb3d::from_min_max(
				Vec3::new(x, floor.min.y, z),
				Vec3::new(x + SHELF_PLAN, floor.max.y, z + SHELF_PLAN),
			);
			if keepouts.iter().any(|k| overlaps_xz(&plan, k)) {
				continue;
			}
			for deck in 0..decks {
				let y0 = floor.min.y + deck as f32 * SHELF_DECK;
				let slot = Aabb3d::from_min_max(
					Vec3::new(x, y0, z),
					Vec3::new(x + SHELF_PLAN, y0 + SHELF_DECK, z + SHELF_PLAN),
				);
				out.push(stamp_make(FurnitureNode::shelf, &slot, host, None));
			}
		}
	}
	out
}

/// Fill a leftover by tiling aisle cells in both plan axes.
pub fn leftover_fill(
	floor: &Aabb3d,
	host: &Aabb3d,
	keepouts: &[Aabb3d],
	mood: LeftoverMood,
) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	if sx < 1.6 || sz < 1.6 {
		return Vec::new();
	}
	let pitch = match mood {
		LeftoverMood::Kitchen => KITCHEN_PITCH,
		LeftoverMood::Hall => HALL_PITCH,
	};
	let (max_axis, max_cells) = match mood {
		LeftoverMood::Kitchen => (MAX_KITCHEN_AXIS, MAX_KITCHEN_CELLS),
		LeftoverMood::Hall => (MAX_HALL_AXIS, MAX_HALL_CELLS),
	};
	let n_x = count_pitch(sx, pitch, max_axis);
	let n_z = count_pitch(sz, pitch, max_axis);
	if n_x == 0 || n_z == 0 {
		return shelf_towers(floor, host, 2, 2, keepouts);
	}
	let (n_x, n_z) = clamp_grid(n_x, n_z, max_cells);
	let cell_x = sx / n_x as f32;
	let cell_z = sz / n_z as f32;
	let mut out = Vec::new();
	for iz in 0..n_z {
		for ix in 0..n_x {
			let cell = Aabb3d::from_min_max(
				Vec3::new(
					floor.min.x + ix as f32 * cell_x,
					floor.min.y,
					floor.min.z + iz as f32 * cell_z,
				),
				Vec3::new(
					floor.min.x + (ix as f32 + 1.0) * cell_x,
					floor.max.y,
					floor.min.z + (iz as f32 + 1.0) * cell_z,
				),
			);
			if keepouts.iter().any(|k| overlaps_xz(&cell, k)) {
				continue;
			}
			match cell_kind(ix, iz, n_x, n_z, mood) {
				CellKind::Partition => out.extend(partition_in_cell(&cell, host, ix, iz)),
				CellKind::Shelf => out.extend(shelf_towers(&cell, host, 1, 1, keepouts)),
				CellKind::Lounge => out.extend(BitesSeatingUsage::expand_aabb(&cell, host)),
			}
		}
	}
	if out.is_empty() {
		out.extend(shelf_towers(floor, host, 2, 2, keepouts));
	}
	out
}

fn cell_kind(ix: usize, iz: usize, n_x: usize, n_z: usize, mood: LeftoverMood) -> CellKind {
	if n_x * n_z == 1 {
		return match mood {
			LeftoverMood::Kitchen => CellKind::Shelf,
			LeftoverMood::Hall => CellKind::Partition,
		};
	}
	match mood {
		LeftoverMood::Kitchen => {
			if (ix + iz) % 2 == 0 {
				CellKind::Partition
			} else {
				CellKind::Shelf
			}
		}
		LeftoverMood::Hall => match (ix + iz) % 3 {
			0 => CellKind::Partition,
			1 => CellKind::Shelf,
			_ => CellKind::Lounge,
		},
	}
}

fn partition_in_cell(cell: &Aabb3d, host: &Aabb3d, ix: usize, iz: usize) -> Vec<FurnitureNode> {
	let sx = cell.max.x - cell.min.x;
	let sz = cell.max.z - cell.min.z;
	if sx.min(sz) < 1.1 {
		return Vec::new();
	}
	let along_x = if (ix + iz) % 2 == 0 { sx >= sz } else { sx > sz + 0.4 };
	let height = (cell.max.y - cell.min.y - 0.04).max(2.4);
	let len = PARTITION_LEN.min(if along_x { sx } else { sz } - 0.35).max(1.4);
	let (slot, side) = if along_x {
		let mid = (cell.min.z + cell.max.z) * 0.5;
		let x0 = (cell.min.x + cell.max.x) * 0.5 - len * 0.5;
		(
			Aabb3d::from_min_max(
				Vec3::new(x0, cell.min.y, mid - PARTITION_THICK * 0.5),
				Vec3::new(x0 + len, cell.min.y + height, mid + PARTITION_THICK * 0.5),
			),
			FurnitureAbutment::NegZ,
		)
	} else {
		let mid = (cell.min.x + cell.max.x) * 0.5;
		let z0 = (cell.min.z + cell.max.z) * 0.5 - len * 0.5;
		(
			Aabb3d::from_min_max(
				Vec3::new(mid - PARTITION_THICK * 0.5, cell.min.y, z0),
				Vec3::new(mid + PARTITION_THICK * 0.5, cell.min.y + height, z0 + len),
			),
			FurnitureAbutment::NegX,
		)
	};
	vec![stamp_make(FurnitureNode::partition, &floor_height_aabb(&slot, height), host, Some(side))]
}

pub fn overlaps_xz(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z
}

fn count_units(span: f32, max: usize) -> usize {
	if span < SHELF_PLAN + 0.3 {
		return 0;
	}
	let n = ((span + SHELF_GAP) / (SHELF_PLAN + SHELF_GAP)).floor() as usize;
	n.clamp(1, max)
}

fn count_pitch(span: f32, pitch: f32, max: usize) -> usize {
	if span < 2.4 {
		return 0;
	}
	((span / pitch).floor() as usize).clamp(1, max)
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::FurnitureGeometry;

	#[test]
	fn huge_hall_tiles_aisles_not_one_row() {
		let floor = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(20.0, 3.5, 20.0));
		let pieces = leftover_fill(&floor, &floor, &[], LeftoverMood::Hall);
		let partitions =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Partition).count();
		let tables = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Table).count();
		let shelves = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Shelf).count();
		assert!(partitions >= 4, "expected several screens, got {partitions}");
		assert!(tables >= 2, "expected more than one lounge cluster, got {tables}");
		assert!(shelves >= 2, "expected stock aisles, got {shelves}");
		let zs: Vec<_> = pieces
			.iter()
			.filter(|n| n.geometry == FurnitureGeometry::Partition)
			.map(|n| n.placement.translation.z)
			.collect();
		let spread = zs.iter().cloned().fold(f32::MAX, f32::min)
			- zs.iter().cloned().fold(f32::MIN, f32::max);
		assert!(
			spread.abs() > 3.0,
			"partitions should occupy more than one aisle row, z spread {spread}"
		);
	}
}
