//! Kitchen remainder → wall counters, peninsula, sit-on cooktop, fridge, leftover fill.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use building_components::{FurnitureAbutment, FurnitureNode, FurnitureUsageNode};

use crate::region::{
	along_is_x, along_span, cut_from_wall, depth_span, floor_height_aabb, longest_wall,
	sit_on_aabb, slice_along, stamp_make, stamp_make_yaw, unit, wall_strip, yaw_turns,
	COUNTER_SLOT_HEIGHT,
};
use crate::shelves::{leftover_fill, LeftoverMood};

const RUN_DEPTH: f32 = 0.75;
const RUN_PAD: f32 = 0.08;
const STATION: f32 = 1.45;
const PEN_LEN: f32 = 1.35;
const PEN_W: f32 = 0.80;
const COOKTOP: Vec3 = Vec3::new(0.98, 0.16, 0.64);
const FRIDGE: Vec3 = Vec3::new(1.90, 2.50, 1.10);
const BASIN: Vec3 = Vec3::new(0.82, 0.32, 0.64);
const FAUCET: Vec3 = Vec3::new(0.24, 0.44, 0.22);
const COOKWARE: Vec3 = Vec3::new(0.42, 0.26, 0.42);

/// Expand a [`FurnitureUsage::BitesKitchen`](building_components::FurnitureUsage::BitesKitchen) remainder.
pub struct BitesKitchenUsage;

impl BitesKitchenUsage {
	pub fn expand(node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
		let region = node.region_aabb();
		let wall = node.abutment.unwrap_or_else(|| longest_wall(&region));
		let run = wall_strip(&region, wall, RUN_DEPTH, RUN_PAD);
		let along_x = along_is_x(&run, Some(wall));
		let along = along_span(&run, along_x);
		if along < 0.6 || depth_span(&run, along_x) < 0.35 {
			return leftover_chest(&region, node);
		}

		let mut out = Vec::new();
		let n = ((along / STATION).floor() as usize).max(1);
		let station = along / n as f32;
		let range_i = ((unit(node.finish_seed, 5) * n as f32) as usize).min(n - 1);
		let basin_i = {
			let mut i = ((unit(node.finish_seed, 7) * n as f32) as usize).min(n - 1);
			if i == range_i && n > 1 {
				i = (i + 1) % n;
			}
			i
		};

		for i in 0..n {
			let slice = slice_along(&run, along_x, i as f32 * station, (i as f32 + 1.0) * station);
			let slab = floor_height_aabb(&slice, COUNTER_SLOT_HEIGHT);
			out.push(stamp_make(FurnitureNode::counter, &slab, &region, Some(wall)));
			if i == range_i {
				let cooktop = sit_on_aabb(&slab, COOKTOP);
				out.push(stamp_make(FurnitureNode::range, &cooktop, &region, Some(wall)));
				out.push(stamp_make_yaw(
					FurnitureNode::cookware,
					&sit_on_aabb(&cooktop, COOKWARE),
					&region,
					Some(wall),
					yaw_turns(node.finish_seed, 21 + i as u64),
				));
				if station > 1.35 {
					let mut extra = sit_on_aabb(&cooktop, COOKWARE);
					extra.min.x += 0.22;
					extra.max.x += 0.22;
					out.push(stamp_make_yaw(
						FurnitureNode::cookware,
						&extra,
						&region,
						Some(wall),
						yaw_turns(node.finish_seed, 21 + i as u64) + std::f32::consts::FRAC_PI_2,
					));
				}
			} else if i == basin_i {
				out.push(stamp_make(
					FurnitureNode::basin,
					&sit_on_aabb(&slab, BASIN),
					&region,
					Some(wall),
				));
				out.push(stamp_make(
					FurnitureNode::faucet,
					&sit_on_aabb(&slab, FAUCET),
					&region,
					Some(wall),
				));
			}
		}

		let mut keepouts = Vec::new();
		if let Some(pen) = peninsula_slab(&region, wall, along, range_i, station) {
			out.push(stamp_make(FurnitureNode::counter, &pen, &region, Some(wall)));
			keepouts.push(pad_xz(&pen, 0.35));
		}

		let mut interior = cut_from_wall(&region, wall, RUN_DEPTH + 0.22);
		if let Some((slot, side)) = fridge_on_wall(&region, wall) {
			out.push(stamp_make(FurnitureNode::fridge, &slot, &region, Some(side)));
			interior = cut_from_wall(&interior, side, FRIDGE.z + 0.12);
			keepouts.push(pad_xz(&slot, 0.15));
		} else if along >= FRIDGE.x + STATION {
			let width = FRIDGE.x;
			let t0 = if range_i + 1 == n { 0.0 } else { along - width };
			let slice = slice_along(&run, along_x, t0, t0 + width);
			let slot = floor_height_aabb(&slice, fridge_height(&region));
			out.push(stamp_make(FurnitureNode::fridge, &slot, &region, Some(wall)));
			keepouts.push(pad_xz(&slot, 0.15));
		}

		out.extend(leftover_fill(&interior, &region, &keepouts, LeftoverMood::Kitchen));
		out
	}
}

fn fridge_height(region: &Aabb3d) -> f32 {
	(region.max.y - region.min.y - 0.10).min(FRIDGE.y).max(2.15)
}

fn peninsula_slab(
	region: &Aabb3d,
	wall: FurnitureAbutment,
	along: f32,
	range_i: usize,
	station: f32,
) -> Option<Aabb3d> {
	let along_x = along_is_x(region, Some(wall));
	if along < 3.2 || depth_span(region, along_x) < RUN_DEPTH + PEN_LEN + 1.15 {
		return None;
	}
	let mut t0 = (along * 0.38).clamp(0.35, along - PEN_W - 0.35);
	let range_t0 = range_i as f32 * station;
	let range_t1 = (range_i as f32 + 1.0) * station;
	if t0 < range_t1 && t0 + PEN_W > range_t0 {
		t0 = if range_t0 > along * 0.5 { 0.35 } else { (range_t1 + 0.12).min(along - PEN_W - 0.2) };
	}
	let fat = wall_strip(region, wall, RUN_DEPTH + PEN_LEN, 0.0);
	let slice = slice_along(&fat, along_is_x(&fat, Some(wall)), t0, t0 + PEN_W);
	let inner = cut_from_wall(&slice, wall, RUN_DEPTH);
	if depth_span(&inner, along_is_x(&inner, Some(wall))) < 0.9 {
		return None;
	}
	Some(floor_height_aabb(&inner, COUNTER_SLOT_HEIGHT))
}

fn pad_xz(slot: &Aabb3d, pad: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(slot.min.x - pad, slot.min.y, slot.min.z - pad),
		Vec3::new(slot.max.x + pad, slot.max.y, slot.max.z + pad),
	)
}

fn fridge_on_wall(
	region: &Aabb3d,
	run_wall: FurnitureAbutment,
) -> Option<(Aabb3d, FurnitureAbutment)> {
	for side in fridge_walls(run_wall) {
		if let Some(slot) = fridge_against(region, run_wall, side) {
			return Some((slot, side));
		}
	}
	None
}

fn fridge_walls(run: FurnitureAbutment) -> [FurnitureAbutment; 3] {
	match run {
		FurnitureAbutment::NegZ => {
			[FurnitureAbutment::NegX, FurnitureAbutment::PosX, FurnitureAbutment::PosZ]
		}
		FurnitureAbutment::PosZ => {
			[FurnitureAbutment::PosX, FurnitureAbutment::NegX, FurnitureAbutment::NegZ]
		}
		FurnitureAbutment::NegX => {
			[FurnitureAbutment::NegZ, FurnitureAbutment::PosZ, FurnitureAbutment::PosX]
		}
		FurnitureAbutment::PosX => {
			[FurnitureAbutment::PosZ, FurnitureAbutment::NegZ, FurnitureAbutment::NegX]
		}
	}
}

fn fridge_against(
	region: &Aabb3d,
	run_wall: FurnitureAbutment,
	side: FurnitureAbutment,
) -> Option<Aabb3d> {
	let along_x = along_is_x(region, Some(side));
	let inset = if perpendicular(run_wall, side) { RUN_DEPTH + 0.12 } else { RUN_PAD };
	if along_span(region, along_x) < inset + FRIDGE.x + 0.08 {
		return None;
	}
	let depth = depth_span(region, along_x);
	if depth < FRIDGE.z + 0.22 {
		return None;
	}
	if !perpendicular(run_wall, side) && depth - RUN_DEPTH < FRIDGE.z + 0.08 {
		return None;
	}
	let strip = wall_strip(region, side, FRIDGE.z, RUN_PAD);
	let slot = slice_along(&strip, along_x, inset, inset + FRIDGE.x);
	Some(floor_height_aabb(&slot, fridge_height(region)))
}

fn perpendicular(a: FurnitureAbutment, b: FurnitureAbutment) -> bool {
	matches!(a, FurnitureAbutment::NegZ | FurnitureAbutment::PosZ)
		!= matches!(b, FurnitureAbutment::NegZ | FurnitureAbutment::PosZ)
}

fn leftover_chest(region: &Aabb3d, node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
	let fp = Vec3::new(region.max.x - region.min.x, 0.0, region.max.z - region.min.z);
	if fp.x < 1.1 || fp.z < 1.1 || unit(node.finish_seed, 17) > 0.65 {
		return Vec::new();
	}
	let slot = Aabb3d::from_min_max(
		Vec3::new(region.min.x + 0.2, region.min.y, region.min.z + 0.2),
		Vec3::new(region.min.x + 1.1, region.min.y + 0.75, region.min.z + 0.65),
	);
	vec![stamp_make(FurnitureNode::chest, &slot, region, node.abutment)]
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::{FurnitureGeometry, Placement};

	fn kitchen_node(min: Vec3, max: Vec3) -> FurnitureUsageNode {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.5, 8.0));
		let box_ = Aabb3d::from_min_max(min, max);
		let mut node = FurnitureUsageNode::bites_kitchen(Placement::IDENTITY);
		node.stamp_region(&box_, &host);
		node
	}

	fn kinds(pieces: &[FurnitureNode]) -> Vec<FurnitureGeometry> {
		pieces.iter().map(|n| n.geometry).collect()
	}

	#[test]
	fn roomy_kitchen_gets_cooktop_aisles_fridge_and_basin() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 2.0), Vec3::new(8.0, 3.5, 6.0));
		let pieces = BitesKitchenUsage::expand(&node);
		let counters: Vec<_> =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Counter).collect();
		assert!(!counters.is_empty());
		for c in &counters {
			let along = c.placement.scale.x.max(c.placement.scale.z);
			assert!(along < 6.5, "kitchen counter should be a station, got {along}");
		}
		assert!(
			counters.len() >= 2,
			"wall run plus a peninsula should add another counter, got {}",
			counters.len()
		);
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Shelf), "shelf {got:?}");
		let shelves: Vec<_> =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Shelf).collect();
		assert!(shelves.len() >= 2, "stacked shelf rows, got {}", shelves.len());
		assert!(shelves.len() <= 75, "a few 2 m towers, got {}", shelves.len());
		for s in &shelves {
			let px = s.placement.scale.x;
			let pz = s.placement.scale.z;
			assert!((px - pz).abs() < 0.15, "shelf XZ should stay square, got {px} x {pz}");
			assert!((s.placement.scale.y - crate::shelves::SHELF_DECK).abs() < 0.05);
		}
		assert!(got.contains(&FurnitureGeometry::Fridge), "fridge {got:?}");
		assert!(got.contains(&FurnitureGeometry::Basin), "basin {got:?}");
		assert!(got.contains(&FurnitureGeometry::Cookware), "cookware {got:?}");
		let tables = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Table).count();
		assert!(tables <= 1, "kitchen leftover should not become a dense cafe, got {tables}");
		let fridge = pieces.iter().find(|n| n.geometry == FurnitureGeometry::Fridge).unwrap();
		assert!(
			fridge.placement.scale.y >= 2.2,
			"fridge should read as a full-height cabinet, got {}",
			fridge.placement.scale.y
		);
		let plan = fridge.placement.scale.x * fridge.placement.scale.z;
		assert!(plan >= 1.7, "fridge plan should be commercial, got {plan}");
		let range = pieces.iter().find(|n| n.geometry == FurnitureGeometry::Range).unwrap();
		let counter_top = counters[0].placement.translation.y + counters[0].placement.scale.y * 0.5;
		assert!(
			range.placement.translation.y > counter_top - 0.05,
			"cooktop should sit on the counter, range y {} vs top {counter_top}",
			range.placement.translation.y
		);
		let yaws: Vec<_> = pieces
			.iter()
			.filter(|n| n.geometry == FurnitureGeometry::Cookware)
			.map(|n| n.placement.yaw)
			.collect();
		if yaws.len() > 1 {
			assert!(
				yaws.iter().any(|y| (y - yaws[0]).abs() > 0.2),
				"cookware should not share one yaw, got {yaws:?}"
			);
		}
	}

	#[test]
	fn short_run_still_gets_a_cooktop() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.2, 3.5, 2.4));
		let pieces = BitesKitchenUsage::expand(&node);
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "short run range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Counter), "short run counter {got:?}");
	}

	#[test]
	fn shallow_run_still_gets_a_cooktop() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(7.0, 3.5, 1.35));
		let pieces = BitesKitchenUsage::expand(&node);
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "shallow range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Fridge), "shallow fridge {got:?}");
	}

	#[test]
	fn tight_kitchen_does_not_panic() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.2, 3.0, 1.2));
		let _ = BitesKitchenUsage::expand(&node);
	}

	#[test]
	fn deep_kitchen_gets_a_peninsula() {
		let node = kitchen_node(Vec3::ZERO, Vec3::new(8.0, 3.5, 6.0));
		let pieces = BitesKitchenUsage::expand(&node);
		let wall = node.abutment.unwrap_or_else(|| longest_wall(&node.region_aabb()));
		let region = node.region_aabb();
		let inset = counters_inset_from_wall(&pieces, wall, &region);
		assert!(inset >= 1, "expected a peninsula standing off the wall, inset {inset}");
	}

	fn counters_inset_from_wall(
		pieces: &[FurnitureNode],
		wall: FurnitureAbutment,
		region: &Aabb3d,
	) -> usize {
		pieces
			.iter()
			.filter(|n| n.geometry == FurnitureGeometry::Counter)
			.filter(|c| {
				let t = c.placement.translation;
				match wall {
					FurnitureAbutment::NegZ => t.z > region.min.z + RUN_DEPTH + 0.35,
					FurnitureAbutment::PosZ => t.z < region.max.z - RUN_DEPTH - 0.35,
					FurnitureAbutment::NegX => t.x > region.min.x + RUN_DEPTH + 0.35,
					FurnitureAbutment::PosX => t.x < region.max.x - RUN_DEPTH - 0.35,
				}
			})
			.count()
	}
}
