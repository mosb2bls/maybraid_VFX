//! Orthogonal hall network connecting shafts and passages on a rectangular host.
//!
//! [`HallsToShafts`] does not author walls. It carves an interior-biased
//! rectilinear MST on the Hanan grid, rings each shaft with a hall-width
//! clearance band, then emits [`SpaceKind::Hallway`] bands and residual
//! [`SpaceKind::InternalSpace`] rectangles (so apartment doors never sit on
//! the shaft face).

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{aabb2_area, NoiseConfig, NoiseParams};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::fit::{aabb_xz_extent, Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::openings::{Opening, OpeningLabel, Openings};
use crate::usage_areas::plan_cells::subtract_aabb2;
use crate::usage_areas::plan_geom::host_xz;

const EPS: f32 = 1e-3;
/// Default noisy hall-width range (meters).
pub const MIN_HALL_WIDTH: f32 = 2.0;
/// Default noisy hall-width range (meters).
pub const MAX_HALL_WIDTH: f32 = 4.0;
const MIN_HOST: f32 = 2.0;

/// Optional knobs for [`HallsToShafts::from_confines_with`].
///
/// When [`Self::hall_width`] is `None`, width is sampled in
/// \[[`MIN_HALL_WIDTH`], [`MAX_HALL_WIDTH`]\]. Callers that need a typology-
/// fixed corridor (e.g. always 3 m) pass `Some(width)`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HallsToShaftsOptions {
	/// Explicit hall clear width in meters (`None` ⇒ sample from noise).
	pub hall_width: Option<f32>,
}

/// Fitted orthogonal halls connecting shafts/passages; no wall geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct HallsToShafts {
	pub confines: Confines,
	/// Merged plan-space hall bands (`Aabb2d` \(x → X\), \(y → Z\)).
	pub hall_bands: Vec<Aabb2d>,
	pub hall_width: f32,
}

impl HallsToShafts {
	pub fn from_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines_with(confines, noise, HallsToShaftsOptions::default())
	}

	/// Fit with optional fixed [`HallsToShaftsOptions::hall_width`].
	pub fn from_confines_with(
		confines: &Confines,
		noise: NoiseParams,
		options: HallsToShaftsOptions,
	) -> Result<(Self, FillableRegions), FitError> {
		let footprint = aabb_xz_extent(&confines.bounds);
		if footprint.x + EPS < MIN_HOST || footprint.y + EPS < MIN_HOST {
			return Err(FitError::TooSmall { reason: "halls_to_shafts_host" });
		}

		let host = host_xz(&confines.bounds);
		let y0 = Vec3::from(confines.bounds.min).y;
		let y1 = Vec3::from(confines.bounds.max).y;
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let hall_width = options
			.hall_width
			.unwrap_or_else(|| {
				cfg.sample_range_f32_4d(MIN_HALL_WIDTH, MAX_HALL_WIDTH, c.x, c.y, c.z, 110.0)
			})
			.clamp(MIN_HALL_WIDTH * 0.5, MAX_HALL_WIDTH * 1.5);
		let beta = cfg.sample_range_f32_4d(1.0, 3.0, c.x, c.y, c.z, 111.0);

		let terminals = collect_terminals(&confines.openings, host);
		// Hall-width ring around each shaft so residuals never abut the shaft face.
		let shaft_rings = shaft_clearance_bands(&confines.openings, host, hall_width);
		let (hall_bands, cuts) = if terminals.len() < 2 {
			let bands = merge_collinear_bands(shaft_rings);
			let mut cuts = bands.clone();
			cuts.extend(shaft_cuts(&confines.openings, host));
			(bands, cuts)
		} else {
			let paths = connect_terminals(&terminals, host, beta, &cfg, c);
			let mut bands = thicken_paths(&paths, hall_width, host);
			bands.extend(shaft_rings);
			bands = merge_collinear_bands(bands);
			let mut cuts = bands.clone();
			cuts.extend(shaft_cuts(&confines.openings, host));
			(bands, cuts)
		};

		let residuals = if cuts.is_empty() {
			vec![host]
		} else {
			merge_rect_residuals(subtract_aabb2(host, &cuts))
		};

		let mut within = Vec::new();
		for band in &hall_bands {
			let bounds = aabb2_to_aabb3(*band, y0, y1);
			let openings = openings_touching(&confines.openings, *band);
			within.push(FillRegion::new(
				SpaceKind::Hallway,
				Confines::new(bounds, confines.roll, openings),
			));
		}
		for rect in residuals {
			if aabb2_area(rect) <= EPS * EPS {
				continue;
			}
			let bounds = aabb2_to_aabb3(rect, y0, y1);
			let openings = openings_intersecting(&confines.openings, rect);
			within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(bounds, confines.roll, openings),
			));
		}

		Ok((
			Self { confines: confines.clone(), hall_bands, hall_width },
			FillableRegions { within, atop: Vec::new() },
		))
	}
}

impl Fit for HallsToShafts {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines(confines, noise)
	}
}

// --- plan helpers -----------------------------------------------------------

fn opening_xz(opening: &Opening) -> Aabb2d {
	let min = Vec3::from(opening.bounds.min);
	let max = Vec3::from(opening.bounds.max);
	Aabb2d { min: Vec2::new(min.x, min.z), max: Vec2::new(max.x, max.z) }
}

fn aabb2_to_aabb3(a: Aabb2d, y0: f32, y1: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(a.min.x, y0, a.min.y), Vec3::new(a.max.x, y1, a.max.y))
}

fn aabb2_intersects(a: Aabb2d, b: Aabb2d, eps: f32) -> bool {
	a.min.x < b.max.x - eps
		&& a.max.x > b.min.x + eps
		&& a.min.y < b.max.y - eps
		&& a.max.y > b.min.y + eps
}

fn aabb2_touches(a: Aabb2d, b: Aabb2d, eps: f32) -> bool {
	let x_overlap = a.min.x < b.max.x + eps && a.max.x > b.min.x - eps;
	let y_overlap = a.min.y < b.max.y + eps && a.max.y > b.min.y - eps;
	if !(x_overlap && y_overlap) {
		return false;
	}
	aabb2_intersects(a, b, -eps)
}

fn clamp_point(p: Vec2, host: Aabb2d) -> Vec2 {
	Vec2::new(p.x.clamp(host.min.x, host.max.x), p.y.clamp(host.min.y, host.max.y))
}

fn clamp_aabb2(a: Aabb2d, host: Aabb2d) -> Option<Aabb2d> {
	let out = Aabb2d {
		min: Vec2::new(a.min.x.max(host.min.x), a.min.y.max(host.min.y)),
		max: Vec2::new(a.max.x.min(host.max.x), a.max.y.min(host.max.y)),
	};
	if out.max.x - out.min.x <= EPS || out.max.y - out.min.y <= EPS {
		None
	} else {
		Some(out)
	}
}

#[derive(Debug, Clone)]
struct Terminal {
	anchor: Vec2,
}

fn collect_terminals(openings: &Openings, host: Aabb2d) -> Vec<Terminal> {
	let mut out = Vec::new();
	for (_, opening) in openings.iter() {
		if !matches!(opening.label, OpeningLabel::Shaft | OpeningLabel::Passage) {
			continue;
		}
		let xz = opening_xz(opening);
		if !aabb2_intersects(xz, host, -EPS) {
			continue;
		}
		let center = 0.5 * (xz.min + xz.max);
		out.push(Terminal { anchor: clamp_point(center, host) });
	}
	out
}

fn shaft_cuts(openings: &Openings, host: Aabb2d) -> Vec<Aabb2d> {
	let mut out = Vec::new();
	for (_, opening) in openings.iter() {
		if opening.label != OpeningLabel::Shaft {
			continue;
		}
		if let Some(c) = clamp_aabb2(opening_xz(opening), host) {
			out.push(c);
		}
	}
	out
}

/// Expand each shaft by `clearance` on all sides and clamp to the host.
///
/// These bands are authored as hallways so InternalSpace residuals keep a
/// corridor ring between living area and the shaft void.
fn shaft_clearance_bands(openings: &Openings, host: Aabb2d, clearance: f32) -> Vec<Aabb2d> {
	let clearance = clearance.max(0.0);
	if clearance <= EPS {
		return Vec::new();
	}
	let pad = Vec2::splat(clearance);
	let mut out = Vec::new();
	for (_, opening) in openings.iter() {
		if opening.label != OpeningLabel::Shaft {
			continue;
		}
		let xz = opening_xz(opening);
		if !aabb2_intersects(xz, host, -EPS) {
			continue;
		}
		let expanded = Aabb2d { min: xz.min - pad, max: xz.max + pad };
		if let Some(c) = clamp_aabb2(expanded, host) {
			// Degenerate if the shaft already fills the host.
			if aabb2_area(c) > EPS * EPS {
				out.push(c);
			}
		}
	}
	out
}

fn openings_intersecting(openings: &Openings, region: Aabb2d) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		if aabb2_intersects(opening_xz(opening), region, EPS) {
			out.insert(id.clone(), opening.clone());
		}
	}
	out
}

fn openings_touching(openings: &Openings, region: Aabb2d) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		if !matches!(opening.label, OpeningLabel::Shaft | OpeningLabel::Passage) {
			continue;
		}
		if aabb2_touches(opening_xz(opening), region, EPS) {
			out.insert(id.clone(), opening.clone());
		}
	}
	out
}

// --- Hanan MST --------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct Segment {
	a: Vec2,
	b: Vec2,
}

fn connect_terminals(
	terminals: &[Terminal],
	host: Aabb2d,
	beta: f32,
	cfg: &NoiseConfig,
	center: Vec3,
) -> Vec<Vec<Segment>> {
	let n = terminals.len();
	let mut xs: Vec<f32> = terminals.iter().map(|t| t.anchor.x).collect();
	let mut zs: Vec<f32> = terminals.iter().map(|t| t.anchor.y).collect();
	xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
	zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
	xs.dedup_by(|a, b| (*a - *b).abs() < EPS);
	zs.dedup_by(|a, b| (*a - *b).abs() < EPS);

	let graph = HananGraph::new(&xs, &zs, host, beta, cfg, center);

	// Pairwise shortest paths.
	let mut pair_cost = vec![vec![f32::INFINITY; n]; n];
	let mut pair_path: Vec<Vec<Option<Vec<Segment>>>> = vec![vec![None; n]; n];
	for i in 0..n {
		for j in (i + 1)..n {
			if let Some((cost, path)) =
				graph.shortest_path(terminals[i].anchor, terminals[j].anchor)
			{
				pair_cost[i][j] = cost;
				pair_cost[j][i] = cost;
				pair_path[i][j] = Some(path.clone());
				pair_path[j][i] = Some(path);
			}
		}
	}

	// Kruskal MST.
	let mut edges: Vec<(f32, usize, usize)> = Vec::new();
	for i in 0..n {
		for j in (i + 1)..n {
			if pair_cost[i][j].is_finite() {
				edges.push((pair_cost[i][j], i, j));
			}
		}
	}
	edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

	let mut uf = UnionFind::new(n);
	let mut paths = Vec::new();
	for (cost, i, j) in edges {
		let _ = cost;
		if uf.union(i, j) {
			if let Some(path) = pair_path[i][j].take() {
				paths.push(path);
			}
		}
	}
	paths
}

struct HananGraph {
	xs: Vec<f32>,
	zs: Vec<f32>,
	/// node_id = iz * nx + ix
	nx: usize,
	nz: usize,
	/// adjacency: node -> (neighbor, weight)
	adj: Vec<Vec<(usize, f32)>>,
}

impl HananGraph {
	fn new(
		xs: &[f32],
		zs: &[f32],
		host: Aabb2d,
		beta: f32,
		cfg: &NoiseConfig,
		center: Vec3,
	) -> Self {
		let nx = xs.len();
		let nz = zs.len();
		let n = nx * nz;
		let mut adj = vec![Vec::new(); n];
		let half_min = 0.5 * (host.max.x - host.min.x).min(host.max.y - host.min.y).max(EPS);

		let weight = |a: Vec2, b: Vec2| -> f32 {
			let mid = 0.5 * (a + b);
			let dist = dist_to_boundary(mid, host);
			let facadeness = (1.0 - (dist / half_min).clamp(0.0, 1.0)).clamp(0.0, 1.0);
			let len = (b - a).length();
			let noise = cfg.sample_range_f32_4d(
				0.0,
				0.02,
				center.x + mid.x * 0.01,
				center.y,
				center.z + mid.y * 0.01,
				112.0,
			);
			len * (1.0 + beta * facadeness) + noise
		};

		for iz in 0..nz {
			for ix in 0..nx {
				let id = iz * nx + ix;
				let p = Vec2::new(xs[ix], zs[iz]);
				if ix + 1 < nx {
					let q = Vec2::new(xs[ix + 1], zs[iz]);
					let w = weight(p, q);
					let nid = iz * nx + (ix + 1);
					adj[id].push((nid, w));
					adj[nid].push((id, w));
				}
				if iz + 1 < nz {
					let q = Vec2::new(xs[ix], zs[iz + 1]);
					let w = weight(p, q);
					let nid = (iz + 1) * nx + ix;
					adj[id].push((nid, w));
					adj[nid].push((id, w));
				}
			}
		}

		Self { xs: xs.to_vec(), zs: zs.to_vec(), nx, nz, adj }
	}

	fn nearest_node(&self, p: Vec2) -> usize {
		let mut best = 0;
		let mut best_d = f32::INFINITY;
		for iz in 0..self.nz {
			for ix in 0..self.nx {
				let q = Vec2::new(self.xs[ix], self.zs[iz]);
				let d = (q - p).length_squared();
				if d < best_d {
					best_d = d;
					best = iz * self.nx + ix;
				}
			}
		}
		best
	}

	fn point(&self, id: usize) -> Vec2 {
		let ix = id % self.nx;
		let iz = id / self.nx;
		Vec2::new(self.xs[ix], self.zs[iz])
	}

	fn shortest_path(&self, a: Vec2, b: Vec2) -> Option<(f32, Vec<Segment>)> {
		let start = self.nearest_node(a);
		let goal = self.nearest_node(b);
		if start == goal {
			return Some((0.0, Vec::new()));
		}

		#[derive(Copy, Clone, PartialEq)]
		struct State {
			cost: f32,
			node: usize,
		}
		impl Eq for State {}
		impl Ord for State {
			fn cmp(&self, other: &Self) -> Ordering {
				other
					.cost
					.partial_cmp(&self.cost)
					.unwrap_or(Ordering::Equal)
					.then_with(|| self.node.cmp(&other.node))
			}
		}
		impl PartialOrd for State {
			fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
				Some(self.cmp(other))
			}
		}

		let n = self.adj.len();
		let mut dist = vec![f32::INFINITY; n];
		let mut parent = vec![None; n];
		let mut heap = BinaryHeap::new();
		dist[start] = 0.0;
		heap.push(State { cost: 0.0, node: start });

		while let Some(State { cost, node }) = heap.pop() {
			if cost > dist[node] + 1e-6 {
				continue;
			}
			if node == goal {
				break;
			}
			for &(next, w) in &self.adj[node] {
				let next_cost = cost + w;
				if next_cost + 1e-6 < dist[next] {
					dist[next] = next_cost;
					parent[next] = Some(node);
					heap.push(State { cost: next_cost, node: next });
				}
			}
		}

		if !dist[goal].is_finite() {
			return None;
		}

		let mut nodes = Vec::new();
		let mut cur = goal;
		nodes.push(cur);
		while let Some(p) = parent[cur] {
			nodes.push(p);
			cur = p;
		}
		nodes.reverse();

		let mut segs = Vec::new();
		// Stub from true anchor to first grid node / last grid node.
		let first = self.point(nodes[0]);
		if (a - first).length() > EPS {
			segs.push(Segment { a, b: first });
		}
		for w in nodes.windows(2) {
			segs.push(Segment { a: self.point(w[0]), b: self.point(w[1]) });
		}
		let last = self.point(*nodes.last().unwrap());
		if (b - last).length() > EPS {
			segs.push(Segment { a: last, b });
		}
		Some((dist[goal], segs))
	}
}

fn dist_to_boundary(p: Vec2, host: Aabb2d) -> f32 {
	(p.x - host.min.x)
		.min(host.max.x - p.x)
		.min(p.y - host.min.y)
		.min(host.max.y - p.y)
		.max(0.0)
}

struct UnionFind {
	parent: Vec<usize>,
	rank: Vec<u8>,
}

impl UnionFind {
	fn new(n: usize) -> Self {
		Self { parent: (0..n).collect(), rank: vec![0; n] }
	}

	fn find(&mut self, mut x: usize) -> usize {
		while self.parent[x] != x {
			self.parent[x] = self.parent[self.parent[x]];
			x = self.parent[x];
		}
		x
	}

	fn union(&mut self, a: usize, b: usize) -> bool {
		let mut ra = self.find(a);
		let mut rb = self.find(b);
		if ra == rb {
			return false;
		}
		if self.rank[ra] < self.rank[rb] {
			std::mem::swap(&mut ra, &mut rb);
		}
		self.parent[rb] = ra;
		if self.rank[ra] == self.rank[rb] {
			self.rank[ra] = self.rank[ra].saturating_add(1);
		}
		true
	}
}

fn thicken_paths(paths: &[Vec<Segment>], hall_width: f32, host: Aabb2d) -> Vec<Aabb2d> {
	let half = hall_width * 0.5;
	let mut bands = Vec::new();
	for path in paths {
		for seg in path {
			let dx = (seg.b.x - seg.a.x).abs();
			let dz = (seg.b.y - seg.a.y).abs();
			if dx < EPS && dz < EPS {
				continue;
			}
			// Extend each end by `half` so an L/T joint fills the full
			// perpendicular corridor width (not just the centerline).
			let band = if dx >= dz {
				// Horizontal (along X).
				let x0 = seg.a.x.min(seg.b.x) - half;
				let x1 = seg.a.x.max(seg.b.x) + half;
				let z = 0.5 * (seg.a.y + seg.b.y);
				Aabb2d { min: Vec2::new(x0, z - half), max: Vec2::new(x1, z + half) }
			} else {
				// Vertical (along Z).
				let z0 = seg.a.y.min(seg.b.y) - half;
				let z1 = seg.a.y.max(seg.b.y) + half;
				let x = 0.5 * (seg.a.x + seg.b.x);
				Aabb2d { min: Vec2::new(x - half, z0), max: Vec2::new(x + half, z1) }
			};
			if let Some(c) = clamp_aabb2(band, host) {
				bands.push(c);
			}
		}
	}
	bands
}

/// Merge residual rectangles whose union is still a rectangle (shared full edge).
fn merge_rect_residuals(mut rects: Vec<Aabb2d>) -> Vec<Aabb2d> {
	let mut changed = true;
	while changed {
		changed = false;
		'outer: for i in 0..rects.len() {
			for j in (i + 1)..rects.len() {
				if let Some(m) = try_merge_rect(rects[i], rects[j]) {
					rects[i] = m;
					rects.swap_remove(j);
					changed = true;
					break 'outer;
				}
			}
		}
	}
	rects
}

fn try_merge_rect(a: Aabb2d, b: Aabb2d) -> Option<Aabb2d> {
	// Same Y extent, abutting/overlapping in X → horizontal merge.
	if (a.min.y - b.min.y).abs() <= EPS && (a.max.y - b.max.y).abs() <= EPS {
		if a.max.x + EPS >= b.min.x && b.max.x + EPS >= a.min.x {
			return Some(Aabb2d {
				min: Vec2::new(a.min.x.min(b.min.x), a.min.y),
				max: Vec2::new(a.max.x.max(b.max.x), a.max.y),
			});
		}
	}
	// Same X extent, abutting/overlapping in Y → vertical merge.
	if (a.min.x - b.min.x).abs() <= EPS && (a.max.x - b.max.x).abs() <= EPS {
		if a.max.y + EPS >= b.min.y && b.max.y + EPS >= a.min.y {
			return Some(Aabb2d {
				min: Vec2::new(a.min.x, a.min.y.min(b.min.y)),
				max: Vec2::new(a.max.x, a.max.y.max(b.max.y)),
			});
		}
	}
	None
}

fn merge_collinear_bands(mut bands: Vec<Aabb2d>) -> Vec<Aabb2d> {
	let mut changed = true;
	while changed {
		changed = false;
		'outer: for i in 0..bands.len() {
			for j in (i + 1)..bands.len() {
				if let Some(m) = try_merge_band(bands[i], bands[j]) {
					bands[i] = m;
					bands.swap_remove(j);
					changed = true;
					break 'outer;
				}
			}
		}
	}
	bands
}

fn try_merge_band(a: Aabb2d, b: Aabb2d) -> Option<Aabb2d> {
	let aw = a.max.x - a.min.x;
	let ah = a.max.y - a.min.y;
	let bw = b.max.x - b.min.x;
	let bh = b.max.y - b.min.y;
	let a_h = aw >= ah;
	let b_h = bw >= bh;
	if a_h != b_h {
		return None;
	}
	if a_h {
		// Same horizontal strip (matching Z range).
		if (a.min.y - b.min.y).abs() > EPS || (a.max.y - b.max.y).abs() > EPS {
			return None;
		}
		if a.max.x + EPS < b.min.x || b.max.x + EPS < a.min.x {
			return None;
		}
		Some(Aabb2d {
			min: Vec2::new(a.min.x.min(b.min.x), a.min.y),
			max: Vec2::new(a.max.x.max(b.max.x), a.max.y),
		})
	} else {
		if (a.min.x - b.min.x).abs() > EPS || (a.max.x - b.max.x).abs() > EPS {
			return None;
		}
		if a.max.y + EPS < b.min.y || b.max.y + EPS < a.min.y {
			return None;
		}
		Some(Aabb2d {
			min: Vec2::new(a.min.x, a.min.y.min(b.min.y)),
			max: Vec2::new(a.max.x, a.max.y.max(b.max.y)),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::OpeningId;

	fn rect_confines(min: Vec3, max: Vec3, openings: Openings) -> Confines {
		Confines::new(Aabb3d::from_min_max(min, max), 0.0, openings)
	}

	fn shaft(id: &str, cx: f32, cz: f32, side: f32) -> (OpeningId, Opening) {
		let h = side * 0.5;
		(
			OpeningId::new(id),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(cx - h, 0.0, cz - h),
					Vec3::new(cx + h, 3.0, cz + h),
				),
				OpeningLabel::Shaft,
			),
		)
	}

	fn passage(id: &str, cx: f32, cz: f32, w: f32, d: f32) -> (OpeningId, Opening) {
		(
			OpeningId::new(id),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(cx - w * 0.5, 0.0, cz - d * 0.5),
					Vec3::new(cx + w * 0.5, 3.0, cz + d * 0.5),
				),
				OpeningLabel::Passage,
			),
		)
	}

	#[test]
	fn single_shaft_gets_clearance_ring() {
		let mut openings = Openings::new();
		let (id, o) = shaft("s0", 0.0, 0.0, 2.0);
		openings.insert(id, o);
		let confines =
			rect_confines(Vec3::new(-10.0, 0.0, -10.0), Vec3::new(10.0, 3.0, 10.0), openings);
		let (hts, regions) = HallsToShafts::from_confines_with(
			&confines,
			NoiseParams::default(),
			HallsToShaftsOptions { hall_width: Some(2.5) },
		)
		.unwrap();
		assert!(!hts.hall_bands.is_empty(), "shaft clearance should be hallway");
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Hallway));
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::InternalSpace));
		assert_residuals_clear_of_shafts(&regions, &confines.openings, 2.5);
	}

	#[test]
	fn residuals_keep_clearance_from_shafts() {
		let mut openings = Openings::new();
		let (a, oa) = shaft("s0", -6.0, -6.0, 2.0);
		let (b, ob) = shaft("s1", 6.0, 6.0, 2.0);
		openings.insert(a, oa);
		openings.insert(b, ob);
		let confines =
			rect_confines(Vec3::new(-12.0, 0.0, -12.0), Vec3::new(12.0, 3.0, 12.0), openings);
		let hall_width = 2.5;
		let (_hts, regions) = HallsToShafts::from_confines_with(
			&confines,
			NoiseParams::default(),
			HallsToShaftsOptions { hall_width: Some(hall_width) },
		)
		.unwrap();
		assert_residuals_clear_of_shafts(&regions, &confines.openings, hall_width);
	}

	fn assert_residuals_clear_of_shafts(
		regions: &FillableRegions,
		openings: &Openings,
		clearance: f32,
	) {
		let pad = Vec2::splat(clearance);
		for region in &regions.within {
			if region.kind != SpaceKind::InternalSpace {
				continue;
			}
			let room = host_xz(&region.confines.bounds);
			for (_, opening) in openings.iter() {
				if opening.label != OpeningLabel::Shaft {
					continue;
				}
				let shaft = opening_xz(opening);
				let keepout = Aabb2d { min: shaft.min - pad, max: shaft.max + pad };
				// Boundary touch with the clearance ring is fine; forbid interior overlap.
				assert!(
					!aabb2_intersects(room, keepout, EPS),
					"InternalSpace {:?} intersects shaft keepout {:?}",
					room,
					keepout
				);
			}
		}
	}

	#[test]
	fn two_shafts_carve_hallway() {
		let mut openings = Openings::new();
		let (a, oa) = shaft("s0", -6.0, -6.0, 2.0);
		let (b, ob) = shaft("s1", 6.0, 6.0, 2.0);
		openings.insert(a, oa);
		openings.insert(b, ob);
		let confines =
			rect_confines(Vec3::new(-12.0, 0.0, -12.0), Vec3::new(12.0, 3.0, 12.0), openings);
		let (hts, regions) =
			HallsToShafts::from_confines(&confines, NoiseParams::default()).unwrap();
		assert!(!hts.hall_bands.is_empty());
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Hallway));
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::InternalSpace));
	}

	#[test]
	fn shaft_and_passage_connect() {
		let mut openings = Openings::new();
		let (a, oa) = shaft("s0", 0.0, 0.0, 2.0);
		let (b, ob) = passage("p0", 10.0, 0.0, 1.2, 0.4);
		openings.insert(a, oa);
		openings.insert(b, ob);
		let confines =
			rect_confines(Vec3::new(-12.0, 0.0, -8.0), Vec3::new(12.0, 3.0, 8.0), openings);
		let (hts, regions) =
			HallsToShafts::from_confines(&confines, NoiseParams::default()).unwrap();
		assert!(!hts.hall_bands.is_empty());
		let hall_area: f32 = hts.hall_bands.iter().map(|b| aabb2_area(*b)).sum();
		assert!(hall_area > 0.0);
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Hallway));
	}

	#[test]
	fn residuals_do_not_overlap_halls() {
		let mut openings = Openings::new();
		let (a, oa) = shaft("s0", -5.0, 0.0, 2.0);
		let (b, ob) = shaft("s1", 5.0, 0.0, 2.0);
		openings.insert(a, oa);
		openings.insert(b, ob);
		let confines =
			rect_confines(Vec3::new(-12.0, 0.0, -8.0), Vec3::new(12.0, 3.0, 8.0), openings);
		let (hts, regions) =
			HallsToShafts::from_confines(&confines, NoiseParams::default()).unwrap();
		for region in &regions.within {
			if region.kind != SpaceKind::InternalSpace {
				continue;
			}
			let rz = host_xz(&region.confines.bounds);
			for band in &hts.hall_bands {
				assert!(!aabb2_intersects(rz, *band, EPS), "room overlaps hall");
			}
		}
	}

	#[test]
	fn l_junction_fills_full_corridor_width() {
		// Two shafts on a clear L: horizontal then vertical through (0,0).
		let mut openings = Openings::new();
		let (a, oa) = shaft("s0", -6.0, 0.0, 2.0);
		let (b, ob) = shaft("s1", 0.0, 6.0, 2.0);
		openings.insert(a, oa);
		openings.insert(b, ob);
		let confines =
			rect_confines(Vec3::new(-12.0, 0.0, -12.0), Vec3::new(12.0, 3.0, 12.0), openings);
		let width = 3.0;
		let (hts, _) = HallsToShafts::from_confines_with(
			&confines,
			NoiseParams::default(),
			HallsToShaftsOptions { hall_width: Some(width) },
		)
		.unwrap();
		assert!((hts.hall_width - width).abs() < 1e-4);
		let half = width * 0.5;
		// Corner square of the L must be covered (not a half-width indent).
		let corner = Aabb2d { min: Vec2::new(-half, -half), max: Vec2::new(half, half) };
		let covered: f32 = hts
			.hall_bands
			.iter()
			.map(|b| {
				let x0 = b.min.x.max(corner.min.x);
				let x1 = b.max.x.min(corner.max.x);
				let y0 = b.min.y.max(corner.min.y);
				let y1 = b.max.y.min(corner.max.y);
				(x1 - x0).max(0.0) * (y1 - y0).max(0.0)
			})
			.sum();
		assert!(
			covered + 1e-2 >= aabb2_area(corner),
			"L junction missing corner fill: covered={covered}"
		);
	}

	#[test]
	fn fixed_hall_width_is_honored() {
		let mut openings = Openings::new();
		let (a, oa) = shaft("s0", -5.0, 0.0, 2.0);
		let (b, ob) = shaft("s1", 5.0, 0.0, 2.0);
		openings.insert(a, oa);
		openings.insert(b, ob);
		let confines =
			rect_confines(Vec3::new(-12.0, 0.0, -8.0), Vec3::new(12.0, 3.0, 8.0), openings);
		let (hts, _) = HallsToShafts::from_confines_with(
			&confines,
			NoiseParams::default(),
			HallsToShaftsOptions { hall_width: Some(3.5) },
		)
		.unwrap();
		assert!((hts.hall_width - 3.5).abs() < 1e-4);
		let mut corridor_checked = 0usize;
		for band in &hts.hall_bands {
			let sx = band.max.x - band.min.x;
			let sy = band.max.y - band.min.y;
			// Shaft clearance rings are fat on both axes; skip those.
			if sx > 3.5 + 1.0 && sy > 3.5 + 1.0 {
				continue;
			}
			let w = sx.min(sy);
			assert!((w - 3.5).abs() < 0.05, "expected ~3.5m clear width, got {w}");
			corridor_checked += 1;
		}
		assert!(corridor_checked > 0, "expected at least one corridor-width hall band");
	}

	#[test]
	fn interior_bias_prefers_core_channel() {
		// Corner terminals on a roomy host: halls should leave outer SE/NW pockets.
		let mut openings = Openings::new();
		let (a, oa) = shaft("s0", -10.0, -10.0, 2.0);
		let (b, ob) = shaft("s1", 10.0, 10.0, 2.0);
		openings.insert(a, oa);
		openings.insert(b, ob);
		let confines =
			rect_confines(Vec3::new(-16.0, 0.0, -16.0), Vec3::new(16.0, 3.0, 16.0), openings);
		let (hts, regions) = HallsToShafts::from_confines_with(
			&confines,
			NoiseParams::default(),
			HallsToShaftsOptions { hall_width: Some(2.5) },
		)
		.unwrap();
		assert!(!hts.hall_bands.is_empty());
		let mut area = 0.0;
		let mut moment = Vec2::ZERO;
		for b in &hts.hall_bands {
			let a = aabb2_area(*b);
			area += a;
			moment += a * (0.5 * (b.min + b.max));
		}
		let centroid = moment / area.max(EPS);
		assert!(centroid.length() < 10.0, "expected interior-biased halls, centroid={centroid:?}");
		// SE façade corner pocket should remain residual (not a perimeter L).
		let se_pocket = Aabb2d { min: Vec2::new(12.0, -16.0), max: Vec2::new(16.0, -12.0) };
		let residual_on_se: f32 = regions
			.within
			.iter()
			.filter(|r| r.kind == SpaceKind::InternalSpace)
			.map(|r| {
				let b = host_xz(&r.confines.bounds);
				let x0 = b.min.x.max(se_pocket.min.x);
				let x1 = b.max.x.min(se_pocket.max.x);
				let y0 = b.min.y.max(se_pocket.min.y);
				let y1 = b.max.y.min(se_pocket.max.y);
				(x1 - x0).max(0.0) * (y1 - y0).max(0.0)
			})
			.sum();
		assert!(
			residual_on_se > 8.0,
			"expected residual façade pocket, residual_on_se={residual_on_se}"
		);
	}
}
