//! Hysteresis connectivity graph collapsed to keypoints and corridors.
//!
//! Same keypoint / chain collapse as Watershed streams-graph thalwegs, reused so
//! development pads can be populated along a connecting grade.

use std::collections::VecDeque;

use bevy_math::Vec2;
use procedural_common::HysteresisGraph;

const DEGENERATE_VERTEX_EPS: f32 = 0.35;

/// Collapsed corridor between two keypoints.
#[derive(Debug, Clone)]
pub struct ConnectivityCorridor {
	pub from_key: usize,
	pub to_key: usize,
	pub path: Vec<Vec2>,
}

/// Keypoints plus directed corridors of a hysteresis graph.
#[derive(Debug, Clone)]
pub struct ConnectivityGraph {
	pub keypoints: Vec<Vec2>,
	pub corridors: Vec<ConnectivityCorridor>,
}

impl ConnectivityGraph {
	/// Collapse degree-1 chains, keeping junctions, the root, and tips.
	pub fn from_hysteresis(graph: &HysteresisGraph) -> Option<Self> {
		let n = graph.nodes.len();
		if n < 2 {
			return None;
		}
		let mut is_key = vec![false; n];
		for (i, key) in is_key.iter_mut().enumerate() {
			let out = graph.children.get(i).map(|c| c.len()).unwrap_or(0);
			*key = i == 0 || out != 1;
		}
		let mut key_graph_idx = Vec::new();
		for (i, key) in is_key.iter().enumerate() {
			if *key {
				key_graph_idx.push(i);
			}
		}
		if key_graph_idx.len() < 2 {
			return None;
		}
		let mut key_for_graph_node = vec![usize::MAX; n];
		for (key, &graph_node) in key_graph_idx.iter().enumerate() {
			key_for_graph_node[graph_node] = key;
		}

		let mut corridors = Vec::new();
		for &from in &key_graph_idx {
			for &child in graph.children.get(from).into_iter().flatten() {
				let mut path_idx = vec![from, child];
				let mut cur = child;
				let mut ok = true;
				while !is_key[cur] {
					let Some(&next) = graph.children.get(cur).and_then(|c| c.first()) else {
						ok = false;
						break;
					};
					path_idx.push(next);
					cur = next;
				}
				if !ok || path_idx.len() < 2 {
					continue;
				}
				let mut path: Vec<Vec2> = path_idx.iter().map(|&k| graph.nodes[k]).collect();
				let from_key = key_for_graph_node[from];
				if from_key == usize::MAX {
					continue;
				}
				let to_key = key_for_graph_node[cur];
				if to_key == usize::MAX {
					continue;
				}
				collapse_degenerate_vertices(&mut path, DEGENERATE_VERTEX_EPS);
				if path.len() < 2 {
					continue;
				}
				corridors.push(ConnectivityCorridor { from_key, to_key, path });
			}
		}
		if corridors.is_empty() {
			return None;
		}
		let keypoints = key_graph_idx.iter().map(|&i| graph.nodes[i]).collect();
		Some(Self { keypoints, corridors })
	}

	/// First incident corridor heading at `key`, used to yaw a building along the path.
	pub fn yaw_at(&self, key: usize) -> f32 {
		let Some(corridor) = self.corridors.iter().find(|c| c.from_key == key || c.to_key == key)
		else {
			return 0.0;
		};
		let (a, b) = if corridor.from_key == key {
			(corridor.path[0], *corridor.path.last().unwrap_or(&corridor.path[0]))
		} else {
			(*corridor.path.last().unwrap_or(&corridor.path[0]), corridor.path[0])
		};
		let dir = b - a;
		(-dir.y).atan2(dir.x)
	}

	/// Undirected adjacency: neighbor key and corridor arclength.
	pub fn undirected_adjacency(&self) -> Vec<Vec<(usize, f32)>> {
		let mut adj = vec![Vec::new(); self.keypoints.len()];
		for corridor in &self.corridors {
			let len = corridor.arclength();
			adj[corridor.from_key].push((corridor.to_key, len));
			adj[corridor.to_key].push((corridor.from_key, len));
		}
		adj
	}

	/// Assign pad heights from the highest sampled node, BFS along the graph.
	///
	/// The peak keeps its natural elevation. Each newly visited neighbor is
	/// clamped so the tree-edge slope stays within `max_grade`. Nodes that
	/// already have a height keep it; remaining edges just use both ends.
	/// Disconnected components restart from their own remaining peak.
	pub fn assign_graded_heights(
		&self,
		natural: &[Option<f32>],
		max_grade: f32,
	) -> Vec<Option<f32>> {
		let n = self.keypoints.len();
		let mut assigned = vec![None; n];
		if natural.len() != n {
			return assigned;
		}
		let adj = self.undirected_adjacency();
		while let Some(start) = highest_unassigned(natural, &assigned) {
			assigned[start] = natural[start];
			let mut queue = VecDeque::new();
			queue.push_back(start);
			while let Some(u) = queue.pop_front() {
				let Some(parent_h) = assigned[u] else {
					continue;
				};
				for &(v, dist) in &adj[u] {
					if assigned[v].is_some() {
						continue;
					}
					let Some(nat) = natural[v] else {
						continue;
					};
					let max_delta = max_grade.max(0.0) * dist.max(1e-3);
					assigned[v] = Some(nat.clamp(parent_h - max_delta, parent_h + max_delta));
					queue.push_back(v);
				}
			}
		}
		assigned
	}
}

impl ConnectivityCorridor {
	/// Sum of segment lengths along the collapsed polyline.
	pub fn arclength(&self) -> f32 {
		self.path.windows(2).map(|w| w[0].distance(w[1])).sum()
	}
}

fn highest_unassigned(natural: &[Option<f32>], assigned: &[Option<f32>]) -> Option<usize> {
	(0..natural.len())
		.filter(|&i| natural[i].is_some() && assigned[i].is_none())
		.max_by(|a, b| match natural[*a].partial_cmp(&natural[*b]) {
			Some(ord) => ord.then(a.cmp(b)),
			None => a.cmp(b),
		})
}

/// Linear grade along a corridor between two keypoint terrace heights.
pub fn corridor_levels(path: &[Vec2], height_a: f32, height_b: f32) -> Vec<f32> {
	let n = path.len();
	if n == 0 {
		return Vec::new();
	}
	if n == 1 {
		return vec![height_a];
	}
	let mut dist = vec![0.0; n];
	for i in 1..n {
		dist[i] = dist[i - 1] + path[i - 1].distance(path[i]);
	}
	let total = dist[n - 1].max(1e-3);
	dist.into_iter()
		.map(|d| height_a + (height_b - height_a) * (d / total))
		.collect()
}

fn collapse_degenerate_vertices(path: &mut Vec<Vec2>, eps: f32) {
	if path.len() < 2 {
		return;
	}
	let mut kept = vec![path[0]];
	for &p in path.iter().skip(1) {
		if kept.last().is_none_or(|q| q.distance(p) > eps) {
			kept.push(p);
		}
	}
	if kept.last() != path.last() {
		if let Some(last) = path.last().copied() {
			kept.push(last);
		}
	}
	*path = kept;
}

#[cfg(test)]
mod tests {
	use super::*;
	use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph};

	#[test]
	fn hysteresis_collapse_keeps_at_least_one_corridor() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 200.0, 200.0);
		let graph = HysteresisGraph::with_degree(
			2,
			bounds,
			11,
			Vec2::new(24.0, 24.0),
			Vec2::new(176.0, 176.0),
			&HysteresisConfig::default(),
		);
		let conn = ConnectivityGraph::from_hysteresis(&graph)
			.ok_or_else(|| anyhow::anyhow!("expected a connectivity graph"))?;
		anyhow::ensure!(conn.keypoints.len() >= 2);
		anyhow::ensure!(!conn.corridors.is_empty());
		Ok(())
	}

	#[test]
	fn corridor_levels_lerp_by_arclength() -> anyhow::Result<()> {
		let path = vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(20.0, 0.0)];
		let levels = corridor_levels(&path, 4.0, 8.0);
		anyhow::ensure!(levels.len() == 3);
		anyhow::ensure!((levels[0] - 4.0).abs() < 1e-4);
		anyhow::ensure!((levels[1] - 6.0).abs() < 1e-4);
		anyhow::ensure!((levels[2] - 8.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn bfs_from_peak_clamps_tree_edges_and_keeps_fixed_nodes() -> anyhow::Result<()> {
		let conn = ConnectivityGraph {
			keypoints: vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0), Vec2::new(200.0, 0.0)],
			corridors: vec![
				ConnectivityCorridor {
					from_key: 0,
					to_key: 1,
					path: vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)],
				},
				ConnectivityCorridor {
					from_key: 1,
					to_key: 2,
					path: vec![Vec2::new(100.0, 0.0), Vec2::new(200.0, 0.0)],
				},
			],
		};
		let natural = vec![Some(100.0), Some(0.0), Some(0.0)];
		let assigned = conn.assign_graded_heights(&natural, 0.15);
		let h0 = assigned[0].ok_or_else(|| anyhow::anyhow!("peak missing"))?;
		let h1 = assigned[1].ok_or_else(|| anyhow::anyhow!("mid missing"))?;
		let h2 = assigned[2].ok_or_else(|| anyhow::anyhow!("tip missing"))?;
		anyhow::ensure!((h0 - 100.0).abs() < 1e-4, "peak should keep natural height: {h0}");
		anyhow::ensure!((h1 - 85.0).abs() < 1e-3, "first hop should raise to max grade: {h1}");
		anyhow::ensure!((h2 - 70.0).abs() < 1e-3, "second hop should continue the grade: {h2}");
		Ok(())
	}

	#[test]
	fn back_edges_keep_both_assigned_heights() -> anyhow::Result<()> {
		let conn = ConnectivityGraph {
			keypoints: vec![Vec2::ZERO, Vec2::new(100.0, 0.0), Vec2::new(0.0, 50.0)],
			corridors: vec![
				ConnectivityCorridor {
					from_key: 0,
					to_key: 1,
					path: vec![Vec2::ZERO, Vec2::new(100.0, 0.0)],
				},
				ConnectivityCorridor {
					from_key: 1,
					to_key: 2,
					path: vec![Vec2::new(100.0, 0.0), Vec2::new(0.0, 50.0)],
				},
				ConnectivityCorridor {
					from_key: 0,
					to_key: 2,
					path: vec![Vec2::ZERO, Vec2::new(0.0, 50.0)],
				},
			],
		};
		let natural = vec![Some(100.0), Some(0.0), Some(0.0)];
		let assigned = conn.assign_graded_heights(&natural, 0.15);
		let h0 = assigned[0].ok_or_else(|| anyhow::anyhow!("peak missing"))?;
		let h1 = assigned[1].ok_or_else(|| anyhow::anyhow!("far missing"))?;
		let h2 = assigned[2].ok_or_else(|| anyhow::anyhow!("near missing"))?;
		anyhow::ensure!((h0 - 100.0).abs() < 1e-4);
		anyhow::ensure!((h1 - 85.0).abs() < 1e-3, "tree hop from peak should clamp: {h1}");
		anyhow::ensure!(
			(h2 - 92.5).abs() < 1e-3,
			"short hop from peak should clamp independently: {h2}"
		);
		let closing = (h1 - h2).abs() / conn.corridors[1].arclength();
		anyhow::ensure!(closing > 0.05, "closing edge should keep the two fixed heights");
		Ok(())
	}
}
