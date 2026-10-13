//! Portal vocabulary and assignment along a unit path \(t \in [0, 1)\).
//!
//! Used by [`crate::arcs`] ring walls and [`crate::wall_demo`] noisy strip demos.

use building_components::partitions::SLICE_KIT_HEIGHT;
use procedural_common::NoiseConfig;

/// Lintel / top-slice baseline as a fraction of storey height.
///
/// Slice kits span [`SLICE_KIT_HEIGHT`] in \(Y\); with wall \(Y\)-scale \(H\) they
/// occupy \(0.2\,H\), so the lintel sits at \(0.8\,H\) to meet the storey top.
pub const SLICE_Y_FRAC: f32 = 1.0 - SLICE_KIT_HEIGHT;

/// Opening cut into a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Portal {
	Door,
	Window,
}

/// Inclusive–exclusive interval on the unit path \(t \in [0, 1)\).
///
/// When `start == end` the region is a **point** locus (preferred / forbidden \(t\)).
/// When `start > end` the interval wraps across \(0\) (closed paths only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallRegion {
	pub start: f32,
	pub end: f32,
}

/// Alias kept for arc-wall call sites; prefer [`WallRegion`].
pub type ArcRegion = WallRegion;

impl WallRegion {
	/// Point region at normalized \(t\).
	pub fn point(t: f32) -> Self {
		let t = norm_t(t);
		Self { start: t, end: t }
	}

	/// Half-open span `[start, end)` on the unit path (wraps if `start > end`).
	pub fn span(start: f32, end: f32) -> Self {
		Self { start: norm_t(start), end: norm_t(end) }
	}

	pub fn is_point(self) -> bool {
		(self.start - self.end).abs() < 1e-6
	}

	/// Path length in \(t\)-units (points have length 0).
	pub fn length(self) -> f32 {
		if self.is_point() {
			return 0.0;
		}
		if self.end >= self.start {
			self.end - self.start
		} else {
			self.end + 1.0 - self.start
		}
	}

	pub fn midpoint(self) -> f32 {
		if self.is_point() {
			return self.start;
		}
		norm_t(self.start + 0.5 * self.length())
	}

	/// Whether normalized \(t\) lies in this region (points match within epsilon).
	pub fn contains_t(self, t: f32) -> bool {
		let t = norm_t(t);
		if self.is_point() {
			return circular_dist(t, self.start) < 1e-5;
		}
		if self.end >= self.start {
			t >= self.start && t < self.end
		} else {
			t >= self.start || t < self.end
		}
	}
}

/// Required portal: best-fit into `region`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MustAssignPortal {
	pub region: WallRegion,
	pub portal: Portal,
}

impl MustAssignPortal {
	pub fn at(t: f32, portal: Portal) -> Self {
		Self { region: WallRegion::point(t), portal }
	}
}

/// Portal assigned on the path (center \(t\), kind).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssignedPortal {
	pub t: f32,
	pub portal: Portal,
}

/// Path-length portal footprint helpers (open or closed).
#[derive(Debug, Clone, Copy)]
pub struct PortalFootprint {
	/// Half-width of each portal in \(t\)-units.
	pub half_t: f32,
	pub closed: bool,
}

impl PortalFootprint {
	pub fn width_t(self) -> f32 {
		self.half_t * 2.0
	}

	pub fn interval(self, center: f32) -> WallRegion {
		WallRegion::span(center - self.half_t, center + self.half_t)
	}

	pub fn snap_center(self, t: f32, slots: Option<u32>) -> f32 {
		let mut snapped = if let Some(n) = slots {
			let n = n.max(1) as f32;
			(norm_t(t) * n).round().rem_euclid(n) / n
		} else {
			norm_t(t)
		};
		if !self.closed {
			snapped = snapped.clamp(self.half_t, (1.0 - self.half_t).max(self.half_t));
		}
		snapped
	}
}

pub fn optional_count(noise: &NoiseConfig, (min, max): (u32, u32)) -> u32 {
	let min = min as usize;
	let max = max as usize;
	if max < min {
		return min as u32;
	}
	noise.sample_range_usize_4d(min, max + 1, 0.17, 0.0, 0.0, 1.0) as u32
}

/// Kit-aligned (or uniformly sampled) centers whose portal footprint lies in can-assign space.
pub fn can_assign_centers(
	foot: PortalFootprint,
	slots: u32,
	placed: &[AssignedPortal],
	must_assign: &[MustAssignPortal],
	must_not: &[WallRegion],
) -> Vec<f32> {
	let mut blocked: Vec<WallRegion> = must_assign.iter().map(|m| m.region).collect();
	blocked.extend(must_not.iter().copied());
	for p in placed {
		blocked.push(foot.interval(p.t));
	}

	let n = slots.max(1);
	let half = foot.half_t;
	(0..n)
		.map(|i| i as f32 / n as f32)
		.filter(|&c| {
			if !foot.closed && (c < half - 1e-5 || c > 1.0 - half + 1e-5) {
				return false;
			}
			if !foot.closed {
				let w = foot.width_t();
				if c - half < -1e-5 || c + half > 1.0 + 1e-5 || w > 1.0 + 1e-5 {
					return false;
				}
			}
			let interval = foot.interval(c);
			!blocked.iter().any(|b| regions_overlap(interval, *b, foot.closed))
		})
		.collect()
}

pub fn place_optional_portals(
	noise: &NoiseConfig,
	foot: PortalFootprint,
	portals: &mut Vec<AssignedPortal>,
	candidates: &[f32],
	count: u32,
) {
	if count == 0 || candidates.is_empty() {
		return;
	}

	let mut scored: Vec<(f32, f32)> = candidates
		.iter()
		.copied()
		.map(|t| {
			let (s, c) = (t * std::f32::consts::TAU).sin_cos();
			let score = noise.sample_unit_4d(c, 0.0, s, 2.0);
			(score, t)
		})
		.collect();
	scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

	let mut blocked: Vec<WallRegion> = portals.iter().map(|p| foot.interval(p.t)).collect();
	let mut placed = 0u32;
	for (_, t) in scored {
		if placed >= count {
			break;
		}
		let interval = foot.interval(t);
		if blocked.iter().any(|b| regions_overlap(interval, *b, true)) {
			continue;
		}
		portals.push(AssignedPortal { t, portal: Portal::Window });
		blocked.push(interval);
		placed += 1;
	}
}

/// Assign must portals, then optional windows in can-assign space.
pub fn assign_portals(
	noise: &NoiseConfig,
	must_assign: &[MustAssignPortal],
	must_not_assign: &[WallRegion],
	optional_portals: (u32, u32),
	foot: PortalFootprint,
	slots: u32,
) -> Vec<AssignedPortal> {
	let mut portals = Vec::new();
	for must in must_assign {
		let t = foot.snap_center(must.region.midpoint(), Some(slots));
		portals.push(AssignedPortal { t, portal: must.portal });
	}

	let optional_n = optional_count(noise, optional_portals);
	if optional_n > 0 {
		let candidates = can_assign_centers(foot, slots, &portals, must_assign, must_not_assign);
		place_optional_portals(noise, foot, &mut portals, &candidates, optional_n);
	}

	portals.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
	portals
}

pub fn norm_t(t: f32) -> f32 {
	let mut t = t % 1.0;
	if t < 0.0 {
		t += 1.0;
	}
	t
}

pub fn circular_dist(a: f32, b: f32) -> f32 {
	let d = (norm_t(a) - norm_t(b)).abs();
	d.min(1.0 - d)
}

pub fn regions_overlap(a: WallRegion, b: WallRegion, _allow_wrap: bool) -> bool {
	if a.is_point() && b.is_point() {
		return circular_dist(a.start, b.start) < 1e-5;
	}
	if a.is_point() {
		return b.contains_t(a.start);
	}
	if b.is_point() {
		return a.contains_t(b.start);
	}
	interval_overlap_unwrap(a, b)
}

fn interval_overlap_unwrap(a: WallRegion, b: WallRegion) -> bool {
	fn unwrap(r: WallRegion) -> Vec<(f32, f32)> {
		if r.is_point() {
			return vec![(r.start, r.start)];
		}
		if r.end >= r.start {
			vec![(r.start, r.end)]
		} else {
			vec![(r.start, 1.0), (0.0, r.end)]
		}
	}
	for (a0, a1) in unwrap(a) {
		for (b0, b1) in unwrap(b) {
			if a0 < b1 && b0 < a1 {
				return true;
			}
		}
	}
	false
}
