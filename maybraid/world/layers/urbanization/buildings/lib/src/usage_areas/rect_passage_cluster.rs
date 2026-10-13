//! Max-rect decomposition + spanning-tree passages for a union of plan AABBs.
//!
//! Geometric only: no room program. Callers (e.g. [`super::livable_apartment`])
//! assign kinds per rect after clustering.

use bevy_math::bounding::{Aabb2d, BoundingVolume};
use procedural_common::{aabb2_area, aabb3_to_plan, PlanAxes, PlanOpeningFace};

use crate::fit::Confines;
use crate::openings::{OpeningLabel, Openings};
use crate::usage_areas::plan_cells::{decompose_max_rects, shared_edge_span};
use crate::usage_areas::plan_geom::{
	aabb2_near_eq, confines_from_xz, connecting_passage, synthetic_edge_passage, DOOR_WIDTH,
	MIN_ROOM,
};

const EPS: f32 = 1e-3;

/// Knobs for clustering plan parts into passage-linked max-rects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectPassageClusterParams {
	/// Minimum edge length (m) retained after max-rect filter.
	pub min_room: f32,
	/// Minimum footprint area (m²) for a kept max-rect.
	pub min_rect_area: f32,
	/// Minimum shared-edge length (m) for a tree adjacency / passage.
	pub min_access: f32,
	/// Opening-id scope (caller module name).
	pub scope: &'static str,
}

impl Default for RectPassageClusterParams {
	fn default() -> Self {
		Self {
			min_room: MIN_ROOM,
			min_rect_area: 8.0,
			min_access: 1.0,
			scope: "rect_passage_cluster",
		}
	}
}

/// Passage-linked maximal rectangles covering a plan union.
#[derive(Debug, Clone, PartialEq)]
pub struct RectPassageCluster {
	pub rects: Vec<Aabb2d>,
	/// Openings authored on each rect (tree + optional root tip).
	pub openings: Vec<Openings>,
	pub root: usize,
	pub tree_edges: Vec<(usize, usize)>,
	pub params: RectPassageClusterParams,
	y0: f32,
	y1: f32,
	roll: f32,
	region_id: u32,
}

impl RectPassageCluster {
	/// Decompose `parts`, pick a root near `root_hint`, wire a Prim spanning tree
	/// of passages, and optionally tip the root toward `root_hint` when they abut.
	pub fn from_parts(
		parts: &[Aabb2d],
		root_hint: Option<Aabb2d>,
		y0: f32,
		y1: f32,
		roll: f32,
		region_id: u32,
		params: RectPassageClusterParams,
	) -> Option<Self> {
		let mut rects = decompose_max_rects(parts);
		rects.retain(|r| {
			let s = r.max - r.min;
			s.x + EPS >= params.min_room
				&& s.y + EPS >= params.min_room
				&& aabb2_area(*r) > params.min_rect_area
		});
		if rects.is_empty() {
			return None;
		}

		let root = pick_root_rect(&rects, root_hint);
		let tree_edges = spanning_tree_edges(&rects, root, params.min_access);
		let mut openings: Vec<Openings> = (0..rects.len()).map(|_| Openings::new()).collect();

		if let Some(hint) = root_hint {
			if let Some((along_x, lo, hi, mid)) = shared_edge_span(hint, rects[root]) {
				if hi - lo + EPS >= params.min_access {
					if let Some((id, opening)) = connecting_passage(
						params.scope,
						"connect",
						along_x,
						lo,
						hi,
						mid,
						y0,
						y1,
						format!("{region_id}_hint_{root}"),
					) {
						openings[root].insert(id, opening);
					}
				}
			}
		}

		for &(a, b) in &tree_edges {
			let Some((along_x, lo, hi, mid)) = shared_edge_span(rects[a], rects[b]) else {
				continue;
			};
			let Some((id, opening)) = connecting_passage(
				params.scope,
				"connect",
				along_x,
				lo,
				hi,
				mid,
				y0,
				y1,
				format!("{region_id}_{a}_{b}"),
			) else {
				continue;
			};
			openings[a].insert(id.clone(), opening.clone());
			openings[b].insert(id, opening);
		}

		Some(Self { rects, openings, root, tree_edges, params, y0, y1, roll, region_id })
	}

	/// Confines for rect `i`, ensuring at least one passage (synthetic if needed).
	pub fn confines_ensured(&self, i: usize, root_hint: Option<Aabb2d>) -> Confines {
		self.confines_ensured_onto(i, root_hint, &[])
	}

	/// Like [`Self::confines_ensured`], but also authors door-width passages onto
	/// every `circulation` band that shares a long edge with the rect.
	///
	/// Tree passages between max-rects alone are not enough: a kitchen that
	/// already opens to a sibling can still sit flush on an entry stem. We add
	/// missing entry/hall doors without replacing existing openings.
	pub fn confines_ensured_onto(
		&self,
		i: usize,
		root_hint: Option<Aabb2d>,
		circulation: &[Aabb2d],
	) -> Confines {
		let mut confines =
			confines_from_xz(self.rects[i], self.y0, self.y1, self.roll, &self.openings[i]);
		let rect = self.rects[i];
		let mut tip = 0u32;
		for band in circulation {
			tip += ensure_door_onto(
				&mut confines,
				rect,
				*band,
				self.params.scope,
				"entry",
				self.y0,
				self.y1,
				format!("{}_{i}_e{tip}", self.region_id),
			) as u32;
		}
		if let Some(hint) = root_hint {
			ensure_door_onto(
				&mut confines,
				rect,
				hint,
				self.params.scope,
				"connect",
				self.y0,
				self.y1,
				format!("{}_{i}_tip", self.region_id),
			);
		}
		let has = confines.openings.iter().any(|(_, o)| matches!(o.label, OpeningLabel::Passage));
		if has {
			return confines;
		}
		let target = root_hint.or_else(|| self.rects.get(self.root).copied());
		if let Some(t) = target {
			if !aabb2_near_eq(rect, t) {
				if let Some((along_x, lo, hi, mid)) = shared_edge_span(rect, t) {
					if let Some((id, opening)) = connecting_passage(
						self.params.scope,
						"connect",
						along_x,
						lo,
						hi,
						mid,
						self.y0,
						self.y1,
						format!("{}_{i}_tip", self.region_id),
					) {
						confines.openings.insert(id, opening);
						return confines;
					}
				}
			}
		}
		let (id, opening) = synthetic_edge_passage(
			self.params.scope,
			"synthetic",
			rect,
			self.y0,
			self.y1,
			format!("{}_{i}", self.region_id),
		);
		confines.openings.insert(id, opening);
		confines
	}
}

/// Author a door-width passage on the shared edge if that wall has none yet.
fn ensure_door_onto(
	confines: &mut Confines,
	rect: Aabb2d,
	target: Aabb2d,
	scope: &'static str,
	kind: &str,
	y0: f32,
	y1: f32,
	id_tag: impl AsRef<str>,
) -> bool {
	if aabb2_near_eq(rect, target) {
		return false;
	}
	let Some((along_x, lo, hi, mid)) = shared_edge_span(rect, target) else {
		return false;
	};
	if hi - lo + EPS < DOOR_WIDTH * 0.7 {
		return false;
	}
	if wall_has_passage(confines, rect, along_x, mid) {
		return false;
	}
	let Some((id, opening)) = connecting_passage(scope, kind, along_x, lo, hi, mid, y0, y1, id_tag)
	else {
		return false;
	};
	confines.openings.insert(id, opening);
	true
}

fn wall_has_passage(confines: &Confines, rect: Aabb2d, along_x: bool, mid: f32) -> bool {
	for (_, o) in confines.openings.iter() {
		if !matches!(o.label, OpeningLabel::Passage) {
			continue;
		}
		let plan = aabb3_to_plan(&o.bounds, PlanAxes::XZ);
		let Some(face) = PlanOpeningFace::from_passage(rect, plan) else {
			continue;
		};
		let on_wall = if along_x {
			!face.thru_is_x && (face.thru - mid).abs() < 0.25
		} else {
			face.thru_is_x && (face.thru - mid).abs() < 0.25
		};
		if on_wall {
			return true;
		}
	}
	false
}

fn pick_root_rect(rects: &[Aabb2d], hint: Option<Aabb2d>) -> usize {
	let Some(hint) = hint else {
		return 0;
	};
	let mut best = 0usize;
	let mut best_score = f32::NEG_INFINITY;
	for (i, r) in rects.iter().enumerate() {
		let score = shared_edge_span(hint, *r).map(|(_, lo, hi, _)| hi - lo).unwrap_or(0.0)
			+ 1.0 / (1.0 + (hint.center() - r.center()).length());
		if score > best_score {
			best_score = score;
			best = i;
		}
	}
	best
}

fn spanning_tree_edges(rects: &[Aabb2d], root: usize, min_access: f32) -> Vec<(usize, usize)> {
	let n = rects.len();
	if n <= 1 {
		return Vec::new();
	}
	let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
	for i in 0..n {
		for j in (i + 1)..n {
			if let Some((_, lo, hi, _)) = shared_edge_span(rects[i], rects[j]) {
				let len = hi - lo;
				if len + EPS >= min_access {
					adj[i].push((j, len));
					adj[j].push((i, len));
				}
			}
		}
	}
	let mut in_tree = vec![false; n];
	in_tree[root] = true;
	let mut edges = Vec::new();
	for _ in 1..n {
		let mut best: Option<(usize, usize, f32)> = None;
		for i in 0..n {
			if !in_tree[i] {
				continue;
			}
			for &(j, len) in &adj[i] {
				if in_tree[j] {
					continue;
				}
				if best.map(|(_, _, l)| len > l).unwrap_or(true) {
					best = Some((i, j, len));
				}
			}
		}
		let Some((a, b, _)) = best else {
			break;
		};
		in_tree[b] = true;
		edges.push((a, b));
	}
	edges
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec2;

	#[test]
	fn l_shape_yields_linked_rects() {
		let parts = [
			Aabb2d { min: Vec2::ZERO, max: Vec2::new(8.0, 4.0) },
			Aabb2d { min: Vec2::new(0.0, 4.0), max: Vec2::new(4.0, 10.0) },
		];
		let cluster = RectPassageCluster::from_parts(
			&parts,
			None,
			0.0,
			3.0,
			0.0,
			1,
			RectPassageClusterParams { scope: "test", ..Default::default() },
		)
		.expect("cluster");
		assert!(cluster.rects.len() >= 2);
		assert_eq!(cluster.openings.len(), cluster.rects.len());
		assert!(!cluster.tree_edges.is_empty());
	}
}
