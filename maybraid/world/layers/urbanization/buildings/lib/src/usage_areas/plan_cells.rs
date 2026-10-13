//! Plan-space cell split, adjacency, and hallway-frontage grouping.
//!
//! Shared by storey typologies that pack multi-cell apartments (or similar
//! program groups) from residual rectangles after halls/shafts are carved.

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec2;
use procedural_common::{aabb2_area, Aabb2dPack};

use crate::usage_areas::plan_access::{GroupFootprint, PlanAccessParams};

const EPS: f32 = 1e-3;

/// Re-export residential suite-join default (canonical: [`PlanAccessParams`]).
pub use crate::usage_areas::plan_access::MIN_GROUP_CONNECTIVITY;

/// One axis-aligned plan cell (`Aabb2d` uses \(x → X\), \(y → Z\)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanCell {
	pub id: u32,
	pub bounds: Aabb2d,
}

impl PlanCell {
	pub fn new(id: u32, bounds: Aabb2d) -> Self {
		Self { id, bounds }
	}

	pub fn area(self) -> f32 {
		aabb2_area(self.bounds)
	}

	pub fn size(self) -> Vec2 {
		self.bounds.max - self.bounds.min
	}

	pub fn center(self) -> Vec2 {
		0.5 * (self.bounds.min + self.bounds.max)
	}
}

/// True when two cells share an edge (closed contact with positive overlap length).
pub fn cells_edge_adjacent(a: &PlanCell, b: &PlanCell, eps: f32) -> bool {
	shared_edge_length(a.bounds, b.bounds, eps).is_some_and(|len| len > eps)
}

/// True when two cells share an edge of at least `min_length` (well-connected).
pub fn cells_well_connected(a: &PlanCell, b: &PlanCell, min_length: f32, eps: f32) -> bool {
	let min_length = min_length.max(eps);
	shared_edge_length(a.bounds, b.bounds, eps).is_some_and(|len| len + eps >= min_length)
}

/// Length of the shared edge between two AABBs, if they touch with positive overlap.
pub fn shared_edge_length(a: Aabb2d, b: Aabb2d, eps: f32) -> Option<f32> {
	let touch_x = (a.max.x - b.min.x).abs() <= eps || (b.max.x - a.min.x).abs() <= eps;
	if touch_x {
		let lo = a.min.y.max(b.min.y);
		let hi = a.max.y.min(b.max.y);
		let len = hi - lo;
		if len > eps {
			return Some(len);
		}
	}
	let touch_y = (a.max.y - b.min.y).abs() <= eps || (b.max.y - a.min.y).abs() <= eps;
	if touch_y {
		let lo = a.min.x.max(b.min.x);
		let hi = a.max.x.min(b.max.x);
		let len = hi - lo;
		if len > eps {
			return Some(len);
		}
	}
	None
}

/// Longest shared-edge length between `cell` and any hallway band.
pub fn hall_frontage_length(cell: &PlanCell, halls: &[Aabb2d], eps: f32) -> f32 {
	halls
		.iter()
		.filter_map(|h| shared_edge_length(cell.bounds, *h, eps))
		.fold(0.0_f32, f32::max)
}

/// True when `cell` shares an edge of at least `min_length` with a hallway band.
///
/// Same pinch rule as inter-cell [`cells_well_connected`]: a point/kiss contact
/// is not real frontage for seeding or doors.
pub fn cell_has_hall_frontage(
	cell: &PlanCell,
	halls: &[Aabb2d],
	min_length: f32,
	eps: f32,
) -> bool {
	hall_frontage_length(cell, halls, eps) + eps >= min_length.max(eps)
}

/// Guillotine-split oversized cells toward `min_room` extents.
///
/// Prefer cutting the longer axis near half when both halves stay ≥ `min_room`.
/// Stops when a cell cannot be split without violating mins.
pub fn split_toward_min_room(
	cells: &[PlanCell],
	min_room: Vec2,
	next_id: &mut u32,
) -> Vec<PlanCell> {
	let min_room = Vec2::new(min_room.x.max(EPS), min_room.y.max(EPS));
	let mut queue: Vec<PlanCell> = cells.to_vec();
	let mut out = Vec::new();
	while let Some(cell) = queue.pop() {
		let size = cell.size();
		let can_cut_x = size.x >= 2.0 * min_room.x - EPS;
		let can_cut_z = size.y >= 2.0 * min_room.y - EPS;
		if !can_cut_x && !can_cut_z {
			out.push(cell);
			continue;
		}
		let cut_x = if can_cut_x && can_cut_z { size.x >= size.y } else { can_cut_x };
		let (a, b) = cell.bounds.bipartition_by_area(cut_x, true, 0.5);
		let sa = a.max - a.min;
		let sb = b.max - b.min;
		if sa.x + EPS < min_room.x
			|| sa.y + EPS < min_room.y
			|| sb.x + EPS < min_room.x
			|| sb.y + EPS < min_room.y
		{
			out.push(cell);
			continue;
		}
		let id_a = cell.id;
		let id_b = *next_id;
		*next_id = next_id.saturating_add(1);
		queue.push(PlanCell::new(id_a, a));
		queue.push(PlanCell::new(id_b, b));
	}
	out
}

/// Group room cells into edge-connected apartments that touch a hallway.
///
/// Convenience wrapper around [`pack_apartments_to_targets`] with one target.
pub fn group_cells_to_apartments(
	cells: &[PlanCell],
	halls: &[Aabb2d],
	min_room_size: Vec2,
	target_apartment_area: f32,
) -> Vec<Vec<u32>> {
	let access =
		PlanAccessParams::residential().with_room_min(min_room_size.x.min(min_room_size.y));
	pack_apartments_to_targets(cells, halls, &[target_apartment_area.max(EPS)], access)
}

/// Pack hall-connected apartments against a target-area catalog.
///
/// Stages: seed/grow (landlocked only, pinch-safe) → absorb orphans → force-one
/// frontage leftovers → post-pass merge of degenerate remnants. See
/// [`PlanAccessParams`] for join / aspect / room thresholds.
pub fn pack_apartments_to_targets(
	cells: &[PlanCell],
	halls: &[Aabb2d],
	targets: &[f32],
	access: PlanAccessParams,
) -> Vec<Vec<u32>> {
	let min_conn = access.group_connect.max(EPS);
	let eligible: Vec<PlanCell> = cells.to_vec();
	if eligible.is_empty() {
		return Vec::new();
	}

	let frontage: Vec<bool> = eligible
		.iter()
		.map(|c| cell_has_hall_frontage(c, halls, min_conn, EPS))
		.collect();

	let mut assigned = vec![false; eligible.len()];
	let mut groups: Vec<Vec<usize>> = Vec::new();

	for &raw_target in targets {
		let target = raw_target.max(EPS);
		let Some(seed) = best_seed(&eligible, &assigned, &frontage, target) else {
			continue;
		};
		let mut group = vec![seed];
		assigned[seed] = true;
		let mut area = eligible[seed].area();
		let accept = target * 0.85;
		while area + EPS < target {
			let Some(next) = best_grow_candidate(&eligible, &assigned, &group, &frontage, min_conn)
			else {
				break;
			};
			let next_area = eligible[next].area();
			if area + EPS >= accept && area + next_area > target * 1.35 {
				break;
			}
			if area + EPS >= accept {
				let mut probe = group.clone();
				probe.push(next);
				if cell_group_footprint(&eligible, &probe).aspect > access.soft_aspect {
					break;
				}
			}
			assigned[next] = true;
			group.push(next);
			area += next_area;
		}
		groups.push(group);
	}

	for i in 0..eligible.len() {
		if assigned[i] || frontage[i] {
			continue;
		}
		if let Some(gi) = find_absorb_group(&eligible, &groups, i, access) {
			groups[gi].push(i);
			assigned[i] = true;
		}
	}

	for i in 0..eligible.len() {
		if assigned[i] {
			continue;
		}
		if frontage[i] {
			groups.push(vec![i]);
			assigned[i] = true;
		}
	}

	merge_degenerate_frontage_groups(&eligible, &mut groups, access);

	groups
		.into_iter()
		.filter(|g| g.iter().any(|&i| frontage[i]))
		.map(|g| g.into_iter().map(|i| eligible[i].id).collect())
		.collect()
}

/// Guillotine-split cells whose area exceeds `max_area`, stopping at `min_room`.
pub fn split_oversized_cells(
	cells: &[PlanCell],
	max_area: f32,
	min_room: Vec2,
	next_id: &mut u32,
) -> Vec<PlanCell> {
	let max_area = max_area.max(EPS);
	let min_room = Vec2::new(min_room.x.max(EPS), min_room.y.max(EPS));
	let mut queue: Vec<PlanCell> = cells.to_vec();
	let mut out = Vec::new();
	while let Some(cell) = queue.pop() {
		if cell.area() <= max_area + EPS {
			out.push(cell);
			continue;
		}
		let size = cell.size();
		let can_cut_x = size.x >= 2.0 * min_room.x - EPS;
		let can_cut_z = size.y >= 2.0 * min_room.y - EPS;
		if !can_cut_x && !can_cut_z {
			out.push(cell);
			continue;
		}
		let cut_x = if can_cut_x && can_cut_z { size.x >= size.y } else { can_cut_x };
		let (a, b) = cell.bounds.bipartition_by_area(cut_x, true, 0.5);
		let sa = a.max - a.min;
		let sb = b.max - b.min;
		if sa.x + EPS < min_room.x
			|| sa.y + EPS < min_room.y
			|| sb.x + EPS < min_room.x
			|| sb.y + EPS < min_room.y
		{
			out.push(cell);
			continue;
		}
		let id_a = cell.id;
		let id_b = *next_id;
		*next_id = next_id.saturating_add(1);
		queue.push(PlanCell::new(id_a, a));
		queue.push(PlanCell::new(id_b, b));
	}
	out
}

fn best_seed(
	cells: &[PlanCell],
	assigned: &[bool],
	frontage: &[bool],
	target: f32,
) -> Option<usize> {
	// Prefer unassigned hall-frontage seeds. Large catalog entries first → start
	// from smaller seeds that still have room to grow toward the target.
	let mut best: Option<(usize, f32)> = None;
	for (i, cell) in cells.iter().enumerate() {
		if assigned[i] || !frontage[i] {
			continue;
		}
		let area = cell.area();
		// Score: prefer seeds below the target (room to grow), then larger seeds.
		let score = if area + EPS < target {
			area + target // prefer larger under-target seeds
		} else {
			// Already oversized for this slot — only use if nothing smaller remains.
			-area
		};
		match best {
			None => best = Some((i, score)),
			Some((_, bs)) if score > bs => best = Some((i, score)),
			_ => {}
		}
	}
	best.map(|(i, _)| i)
}

fn cell_group_footprint(cells: &[PlanCell], group: &[usize]) -> GroupFootprint {
	let mut b = cells[group[0]].bounds;
	let mut area = 0.0_f32;
	for &gi in group {
		b.min = b.min.min(cells[gi].bounds.min);
		b.max = b.max.max(cells[gi].bounds.max);
		area += cells[gi].area();
	}
	GroupFootprint::from_bounds(b, area)
}

/// Max well-connected shared length between `cell` and `group`, or `None` if any pinch.
fn max_pinch_safe_shared(
	cells: &[PlanCell],
	group: &[usize],
	cell: usize,
	min_conn: f32,
) -> Option<f32> {
	let bounds = cells[cell].bounds;
	let mut shared = 0.0_f32;
	let mut well = false;
	for &gi in group {
		let Some(len) = shared_edge_length(cells[gi].bounds, bounds, EPS) else {
			continue;
		};
		if len + EPS >= min_conn {
			well = true;
			shared = shared.max(len);
		} else if len > EPS {
			return None;
		}
	}
	well.then_some(shared)
}

fn groups_can_merge(cells: &[PlanCell], a: &[usize], b: &[usize], min_conn: f32) -> Option<f32> {
	let mut shared = 0.0_f32;
	let mut well = false;
	for &ai in a {
		for &bi in b {
			let Some(len) = shared_edge_length(cells[ai].bounds, cells[bi].bounds, EPS) else {
				continue;
			};
			if len + EPS >= min_conn {
				well = true;
				shared = shared.max(len);
			} else if len > EPS {
				return None;
			}
		}
	}
	well.then_some(shared)
}

fn best_grow_candidate(
	cells: &[PlanCell],
	assigned: &[bool],
	group: &[usize],
	frontage: &[bool],
	min_connectivity: f32,
) -> Option<usize> {
	let fp = cell_group_footprint(cells, group);
	let mut best: Option<(usize, f32)> = None;
	for (i, cell) in cells.iter().enumerate() {
		if assigned[i] || frontage[i] {
			continue;
		}
		let Some(max_shared) = max_pinch_safe_shared(cells, group, i, min_connectivity) else {
			continue;
		};
		let mut nb = fp.bounds;
		nb.min = nb.min.min(cell.bounds.min);
		nb.max = nb.max.max(cell.bounds.max);
		let probe = GroupFootprint::from_bounds(nb, fp.area + cell.area());
		let score = max_shared * (0.35 + probe.compact) * cell.area().sqrt();
		match best {
			None => best = Some((i, score)),
			Some((_, bs)) if score > bs => best = Some((i, score)),
			_ => {}
		}
	}
	best.map(|(i, _)| i)
}

fn merge_degenerate_frontage_groups(
	cells: &[PlanCell],
	groups: &mut Vec<Vec<usize>>,
	access: PlanAccessParams,
) {
	let min_conn = access.group_connect;
	let mut guard = 0;
	while guard < groups.len().saturating_mul(2).max(4) {
		guard += 1;
		let mut order: Vec<usize> = (0..groups.len()).collect();
		order.sort_by(|&a, &b| {
			let aa = cell_group_footprint(cells, &groups[a]).area;
			let ab = cell_group_footprint(cells, &groups[b]).area;
			aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal)
		});
		let mut did = false;
		for gi in order {
			if gi >= groups.len() || !cell_group_footprint(cells, &groups[gi]).is_degenerate(access)
			{
				continue;
			}
			let mut best: Option<(usize, f32, f32)> = None;
			for tj in 0..groups.len() {
				if tj == gi {
					continue;
				}
				let Some(shared) = groups_can_merge(cells, &groups[gi], &groups[tj], min_conn)
				else {
					continue;
				};
				let mut probe = groups[tj].clone();
				probe.extend_from_slice(&groups[gi]);
				let aspect = cell_group_footprint(cells, &probe).aspect;
				let better = match best {
					None => true,
					Some((_, bs, ba)) => {
						shared > bs + EPS || ((shared - bs).abs() <= EPS && aspect < ba - 1e-4)
					}
				};
				if better {
					best = Some((tj, shared, aspect));
				}
			}
			let Some((tj, _, _)) = best else {
				continue;
			};
			let mut absorbed = groups.remove(gi);
			let tj = if tj > gi { tj - 1 } else { tj };
			groups[tj].append(&mut absorbed);
			did = true;
			break;
		}
		if !did {
			break;
		}
	}
}

fn find_absorb_group(
	cells: &[PlanCell],
	groups: &[Vec<usize>],
	orphan: usize,
	access: PlanAccessParams,
) -> Option<usize> {
	let min_conn = access.group_connect;
	let mut best: Option<(usize, f32, bool, f32, f32)> = None;
	for (gi, group) in groups.iter().enumerate() {
		let Some(shared) = max_pinch_safe_shared(cells, group, orphan, min_conn) else {
			continue;
		};
		let mut probe = group.clone();
		probe.push(orphan);
		let fp = cell_group_footprint(cells, &probe);
		let soft_ok = fp.aspect <= access.soft_aspect;
		let better = match best {
			None => true,
			Some((_, bs, bsoft, bc, ba)) => {
				shared > bs + EPS
					|| ((shared - bs).abs() <= EPS && soft_ok && !bsoft)
					|| ((shared - bs).abs() <= EPS && soft_ok == bsoft && fp.compact > bc + 1e-4)
					|| ((shared - bs).abs() <= EPS
						&& soft_ok == bsoft
						&& (fp.compact - bc).abs() <= 1e-4
						&& fp.area < ba)
			}
		};
		if better {
			best = Some((gi, shared, soft_ok, fp.compact, fp.area));
		}
	}
	best.map(|(gi, _, _, _, _)| gi)
}

/// Decompose a rectilinear union of axis-aligned parts into large covering rectangles.
///
/// Repeatedly grows each remaining seed to a maximal rectangle still inside the
/// union, takes the largest, and subtracts it until scraps are gone. Prefer fewer
/// larger rects (L / T footprints stay multi-rect).
pub fn decompose_max_rects(parts: &[Aabb2d]) -> Vec<Aabb2d> {
	let mut remaining: Vec<Aabb2d> =
		parts.iter().copied().filter(|r| aabb2_area(*r) > EPS * EPS).collect();
	if remaining.is_empty() {
		return Vec::new();
	}
	let mut out = Vec::new();
	const MIN_SCRAP: f32 = 0.05;
	while !remaining.is_empty() {
		// Grow inside the *remaining* union so extracted rects do not overlap.
		let Some(best) = remaining
			.iter()
			.map(|seed| grow_maximal_in_union(*seed, &remaining))
			.max_by(|a, b| {
				aabb2_area(*a).partial_cmp(&aabb2_area(*b)).unwrap_or(std::cmp::Ordering::Equal)
			})
		else {
			break;
		};
		if aabb2_area(best) <= MIN_SCRAP {
			break;
		}
		out.push(best);
		let mut next = Vec::new();
		for r in remaining {
			next.extend(subtract_aabb2(r, &[best]));
		}
		remaining = next.into_iter().filter(|r| aabb2_area(*r) > MIN_SCRAP).collect();
	}
	out.sort_by(|a, b| {
		aabb2_area(*b).partial_cmp(&aabb2_area(*a)).unwrap_or(std::cmp::Ordering::Equal)
	});
	out
}

/// True when `rect` lies entirely inside the union of `parts`.
pub fn rect_in_union(rect: Aabb2d, parts: &[Aabb2d]) -> bool {
	if aabb2_area(rect) <= EPS * EPS {
		return true;
	}
	subtract_aabb2(rect, parts).is_empty()
}

/// Expand `seed` in all four directions to a maximal AABB still ⊆ union(`parts`).
fn grow_maximal_in_union(seed: Aabb2d, parts: &[Aabb2d]) -> Aabb2d {
	let mut xs = Vec::new();
	let mut ys = Vec::new();
	for p in parts {
		xs.push(p.min.x);
		xs.push(p.max.x);
		ys.push(p.min.y);
		ys.push(p.max.y);
	}
	xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	xs.dedup_by(|a, b| (*a - *b).abs() < EPS);
	ys.dedup_by(|a, b| (*a - *b).abs() < EPS);

	let mut r = seed;
	// −X
	for &x in xs.iter().rev() {
		if x >= r.min.x - EPS {
			continue;
		}
		let trial = Aabb2d { min: Vec2::new(x, r.min.y), max: r.max };
		if rect_in_union(trial, parts) {
			r = trial;
		}
	}
	// +X
	for &x in &xs {
		if x <= r.max.x + EPS {
			continue;
		}
		let trial = Aabb2d { min: r.min, max: Vec2::new(x, r.max.y) };
		if rect_in_union(trial, parts) {
			r = trial;
		}
	}
	// −Y
	for &y in ys.iter().rev() {
		if y >= r.min.y - EPS {
			continue;
		}
		let trial = Aabb2d { min: Vec2::new(r.min.x, y), max: r.max };
		if rect_in_union(trial, parts) {
			r = trial;
		}
	}
	// +Y
	for &y in &ys {
		if y <= r.max.y + EPS {
			continue;
		}
		let trial = Aabb2d { min: r.min, max: Vec2::new(r.max.x, y) };
		if rect_in_union(trial, parts) {
			r = trial;
		}
	}
	r
}

/// Shared edge span between two AABBs: `(along_x, lo, hi, mid)`.
///
/// `along_x == true` ⇒ contact is horizontal (constant plan-Y / world Z = `mid`).
pub fn shared_edge_span(a: Aabb2d, b: Aabb2d) -> Option<(bool, f32, f32, f32)> {
	let touch_x = (a.max.x - b.min.x).abs() <= EPS || (b.max.x - a.min.x).abs() <= EPS;
	if touch_x {
		let mid = if (a.max.x - b.min.x).abs() <= EPS { a.max.x } else { b.max.x };
		let lo = a.min.y.max(b.min.y);
		let hi = a.max.y.min(b.max.y);
		if hi - lo > EPS {
			return Some((false, lo, hi, mid));
		}
	}
	let touch_y = (a.max.y - b.min.y).abs() <= EPS || (b.max.y - a.min.y).abs() <= EPS;
	if touch_y {
		let mid = if (a.max.y - b.min.y).abs() <= EPS { a.max.y } else { b.max.y };
		let lo = a.min.x.max(b.min.x);
		let hi = a.max.x.min(b.max.x);
		if hi - lo > EPS {
			return Some((true, lo, hi, mid));
		}
	}
	None
}

/// Subtract axis-aligned `cuts` from `host`, returning residual rectangles.
///
/// Uses a simple guillotine difference against each cut in order.
pub fn subtract_aabb2(host: Aabb2d, cuts: &[Aabb2d]) -> Vec<Aabb2d> {
	let mut regions = vec![host];
	for cut in cuts {
		let mut next = Vec::new();
		for r in regions {
			next.extend(subtract_one(r, *cut));
		}
		regions = next;
	}
	regions.into_iter().filter(|r| aabb2_area(*r) > EPS * EPS).collect()
}

fn subtract_one(host: Aabb2d, cut: Aabb2d) -> Vec<Aabb2d> {
	let x0 = host.min.x.max(cut.min.x);
	let x1 = host.max.x.min(cut.max.x);
	let y0 = host.min.y.max(cut.min.y);
	let y1 = host.max.y.min(cut.max.y);
	if x1 - x0 <= EPS || y1 - y0 <= EPS {
		return vec![host];
	}
	let mut out = Vec::new();
	// Left
	if x0 - host.min.x > EPS {
		out.push(Aabb2d { min: host.min, max: Vec2::new(x0, host.max.y) });
	}
	// Right
	if host.max.x - x1 > EPS {
		out.push(Aabb2d { min: Vec2::new(x1, host.min.y), max: host.max });
	}
	// Bottom (between left/right slabs)
	if y0 - host.min.y > EPS {
		out.push(Aabb2d { min: Vec2::new(x0, host.min.y), max: Vec2::new(x1, y0) });
	}
	// Top
	if host.max.y - y1 > EPS {
		out.push(Aabb2d { min: Vec2::new(x0, y1), max: Vec2::new(x1, host.max.y) });
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn cell(id: u32, min: Vec2, max: Vec2) -> PlanCell {
		PlanCell::new(id, Aabb2d { min, max })
	}

	#[test]
	fn edge_adjacent_detects_shared_side() {
		let a = cell(0, Vec2::new(0.0, 0.0), Vec2::new(4.0, 4.0));
		let b = cell(1, Vec2::new(4.0, 0.0), Vec2::new(8.0, 4.0));
		let c = cell(2, Vec2::new(5.0, 0.0), Vec2::new(9.0, 4.0));
		assert!(cells_edge_adjacent(&a, &b, EPS));
		assert!(!cells_edge_adjacent(&a, &c, EPS));
	}

	#[test]
	fn split_toward_min_room_halves_long_run() {
		let mut next = 1;
		let cells = vec![cell(0, Vec2::new(0.0, 0.0), Vec2::new(12.0, 4.0))];
		let out = split_toward_min_room(&cells, Vec2::new(3.0, 3.0), &mut next);
		assert!(out.len() >= 2);
		assert!(out.iter().all(|c| c.size().x + EPS >= 3.0 && c.size().y + EPS >= 3.0));
	}

	#[test]
	fn group_requires_hall_frontage() {
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(4.0, 4.0)),
			cell(1, Vec2::new(4.0, 0.0), Vec2::new(8.0, 4.0)),
			// Isolated: no hall edge, no adjacency to other rooms.
			cell(2, Vec2::new(20.0, 20.0), Vec2::new(24.0, 24.0)),
		];
		let hall = Aabb2d { min: Vec2::new(0.0, 4.0), max: Vec2::new(8.0, 6.0) };
		let groups = group_cells_to_apartments(&rooms, &[hall], Vec2::new(2.0, 2.0), 20.0);
		let flat: Vec<u32> = groups.iter().flatten().copied().collect();
		assert!(flat.contains(&0));
		assert!(flat.contains(&1));
		assert!(!flat.contains(&2));
		assert!(groups.iter().all(|g| !g.is_empty()));
		// Every emitted group still touches the hall (via at least one piece).
		for g in &groups {
			assert!(g.iter().any(|&id| {
				let c = rooms.iter().find(|c| c.id == id).unwrap();
				cell_has_hall_frontage(c, &[hall], MIN_GROUP_CONNECTIVITY, EPS)
			}));
		}
	}

	#[test]
	fn pack_catalog_builds_multi_cell_group() {
		// Frontage seed + landlocked stack behind it — grow claims hinterland.
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0)), // frontage
			cell(1, Vec2::new(0.0, 3.0), Vec2::new(3.0, 6.0)),
			cell(2, Vec2::new(0.0, 6.0), Vec2::new(3.0, 9.0)),
			cell(3, Vec2::new(0.0, 9.0), Vec2::new(3.0, 12.0)),
		];
		let hall = Aabb2d { min: Vec2::new(0.0, -2.0), max: Vec2::new(3.0, 0.0) };
		let groups = pack_apartments_to_targets(
			&rooms,
			&[hall],
			&[30.0, 20.0],
			PlanAccessParams::residential().with_room_min(2.0),
		);
		assert!(
			groups.iter().any(|g| g.len() >= 2),
			"expected a multi-cell apartment, got {groups:?}"
		);
	}

	#[test]
	fn pinch_contact_does_not_group() {
		// Full-side neighbor vs pinch (0.4 m shared) — only the wide contact groups.
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(4.0, 4.0)),
			cell(1, Vec2::new(4.0, 0.0), Vec2::new(8.0, 4.0)), // 4 m shared
			cell(2, Vec2::new(4.0, 3.6), Vec2::new(8.0, 7.6)), // 0.4 m pinch with cell 0
		];
		let hall = Aabb2d { min: Vec2::new(0.0, -2.0), max: Vec2::new(8.0, 0.0) };
		assert!(cells_edge_adjacent(&rooms[0], &rooms[2], EPS));
		assert!(!cells_well_connected(&rooms[0], &rooms[2], MIN_GROUP_CONNECTIVITY, EPS));
		assert!(cells_well_connected(&rooms[0], &rooms[1], MIN_GROUP_CONNECTIVITY, EPS));

		let groups = pack_apartments_to_targets(
			&rooms,
			&[hall],
			&[40.0],
			PlanAccessParams::residential().with_room_min(2.0),
		);
		// Cell 0+1 may group; cell 2 must not join via the pinch alone.
		for g in &groups {
			if g.contains(&2) {
				assert!(!g.contains(&0), "pinch must not join cell 2 with cell 0: {groups:?}");
			}
		}
	}

	#[test]
	fn post_pass_merges_degenerate_frontage_strip() {
		// Fat suite (0+1+2) next to a thin frontage strip (3) that shares a full
		// side — strip is force-one after grow, then post-pass should absorb it.
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(6.0, 3.0)), // frontage
			cell(1, Vec2::new(0.0, 3.0), Vec2::new(6.0, 6.0)),
			cell(2, Vec2::new(0.0, 6.0), Vec2::new(6.0, 9.0)),
			cell(3, Vec2::new(6.0, 0.0), Vec2::new(8.0, 9.0)), // thin frontage strip
		];
		let hall = Aabb2d { min: Vec2::new(0.0, -2.0), max: Vec2::new(8.0, 0.0) };
		let groups = pack_apartments_to_targets(
			&rooms,
			&[hall],
			&[45.0],
			PlanAccessParams::residential().with_room_min(2.5),
		);
		let g_fat = groups.iter().find(|g| g.contains(&0));
		assert!(
			g_fat.is_some_and(|g| g.contains(&3)),
			"degenerate frontage strip should merge into neighbor: {groups:?}"
		);
	}

	#[test]
	fn grow_prefers_compact_over_linear_snake() {
		// Frontage + north stack + eastern wing: compact fill should claim the
		// wing before extending the bowling alley.
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(4.0, 3.0)), // frontage
			cell(1, Vec2::new(0.0, 3.0), Vec2::new(4.0, 6.0)),
			cell(2, Vec2::new(4.0, 3.0), Vec2::new(8.0, 6.0)), // compact wing
			cell(3, Vec2::new(0.0, 6.0), Vec2::new(4.0, 9.0)), // snake tip
		];
		let hall = Aabb2d { min: Vec2::new(0.0, -2.0), max: Vec2::new(4.0, 0.0) };
		let groups = pack_apartments_to_targets(
			&rooms,
			&[hall],
			&[40.0],
			PlanAccessParams::residential().with_room_min(2.0),
		);
		let g0 = groups.iter().find(|g| g.contains(&0)).expect("seed group");
		assert!(g0.contains(&2), "compact wing should join before/over snake tip: {groups:?}");
	}

	#[test]
	fn grow_does_not_merge_side_by_side_frontage_into_spur() {
		// Two hall-frontage cells side-by-side; landlocked stack north of the east
		// cell. East should grow north; west stays its own door seed (no spur).
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0)), // west frontage
			cell(1, Vec2::new(3.0, 0.0), Vec2::new(6.0, 3.0)), // east frontage
			cell(2, Vec2::new(3.0, 3.0), Vec2::new(6.0, 6.0)),
			cell(3, Vec2::new(3.0, 6.0), Vec2::new(6.0, 9.0)),
		];
		let hall = Aabb2d { min: Vec2::new(0.0, -2.0), max: Vec2::new(6.0, 0.0) };
		let groups = pack_apartments_to_targets(
			&rooms,
			&[hall],
			&[30.0, 20.0],
			PlanAccessParams::residential().with_room_min(2.0),
		);
		let g0 = groups.iter().find(|g| g.contains(&0)).expect("cell 0 grouped");
		assert!(
			!g0.contains(&1),
			"side-by-side frontage must not share a suite (spur): {groups:?}"
		);
	}

	#[test]
	fn split_oversized_respects_max_area() {
		let mut next = 1;
		let cells = vec![cell(0, Vec2::new(0.0, 0.0), Vec2::new(10.0, 8.0))];
		let out = split_oversized_cells(&cells, 20.0, Vec2::new(2.5, 2.5), &mut next);
		assert!(out.len() >= 2);
		assert!(out.iter().all(|c| c.area() <= 20.0 + 1.0));
	}

	#[test]
	fn subtract_carves_corridor() {
		let host = Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(10.0, 10.0) };
		let hall = Aabb2d { min: Vec2::new(0.0, 4.0), max: Vec2::new(10.0, 6.0) };
		let rem = subtract_aabb2(host, &[hall]);
		assert!(rem.len() >= 2);
		assert!(rem.iter().all(|r| aabb2_area(*r) > 0.0));
	}

	fn rect(min: Vec2, max: Vec2) -> Aabb2d {
		Aabb2d { min, max }
	}

	#[test]
	fn decompose_single_rect_unchanged() {
		let parts = [rect(Vec2::ZERO, Vec2::new(8.0, 6.0))];
		let out = decompose_max_rects(&parts);
		assert_eq!(out.len(), 1);
		assert!((aabb2_area(out[0]) - 48.0).abs() < 0.1);
	}

	#[test]
	fn decompose_l_shape_two_rects() {
		let parts = [
			rect(Vec2::ZERO, Vec2::new(8.0, 4.0)),
			rect(Vec2::new(0.0, 4.0), Vec2::new(4.0, 10.0)),
		];
		let input_area: f32 = parts.iter().map(|r| aabb2_area(*r)).sum();
		let out = decompose_max_rects(&parts);
		assert!(out.len() >= 2, "L should stay multi-rect, got {out:?}");
		let out_area: f32 = out.iter().map(|r| aabb2_area(*r)).sum();
		assert!((out_area - input_area).abs() < 0.5, "cover area {out_area} vs input {input_area}");
	}

	#[test]
	fn decompose_t_shape_covers_area() {
		let parts = [
			rect(Vec2::new(3.0, 0.0), Vec2::new(7.0, 6.0)),
			rect(Vec2::new(0.0, 6.0), Vec2::new(10.0, 10.0)),
		];
		let input_area: f32 = parts.iter().map(|r| aabb2_area(*r)).sum();
		let out = decompose_max_rects(&parts);
		assert!(out.len() >= 2);
		let out_area: f32 = out.iter().map(|r| aabb2_area(*r)).sum();
		assert!((out_area - input_area).abs() < 0.5);
	}

	#[test]
	fn decompose_merges_coaxial_pair() {
		let parts =
			[rect(Vec2::ZERO, Vec2::new(4.0, 3.0)), rect(Vec2::new(4.0, 0.0), Vec2::new(8.0, 3.0))];
		let out = decompose_max_rects(&parts);
		assert_eq!(out.len(), 1, "coaxial pair should merge: {out:?}");
		assert!((aabb2_area(out[0]) - 24.0).abs() < 0.1);
	}
}
