//! One-pass shared-edge discovery via canonical undirected keys.

use std::collections::HashMap;

use super::types::{PanelPointId, PanelTriangle, SharedEdge};

/// Canonical undirected edge key, or [`None`] when `u == v`.
pub(super) fn canonical_edge(
	u: PanelPointId,
	v: PanelPointId,
) -> Option<(PanelPointId, PanelPointId)> {
	if u == v {
		None
	} else if u < v {
		Some((u, v))
	} else {
		Some((v, u))
	}
}

/// One-pass shared-edge discovery: canonical `(min,max)` keys, \(O(T)\) time.
///
/// Edges with one incidence are boundary (omitted). Edges with two incidences
/// become [`SharedEdge`]. Edges with three or more are recorded in
/// `non_manifold` and omitted from the shared list.
pub fn shared_edges(
	triangles: &[PanelTriangle],
) -> (Vec<SharedEdge>, Vec<(PanelPointId, PanelPointId)>) {
	let mut incidences: HashMap<(PanelPointId, PanelPointId), Vec<usize>> = HashMap::new();
	for (tri_idx, tri) in triangles.iter().enumerate() {
		let Some(edges) = tri.undirected_edges() else {
			continue;
		};
		for key in edges {
			incidences.entry(key).or_default().push(tri_idx);
		}
	}

	let mut shared = Vec::new();
	let mut non_manifold = Vec::new();
	for ((a, b), tris) in incidences {
		match tris.as_slice() {
			[] | [_] => {}
			[tri0, tri1] => shared.push(SharedEdge { a, b, tri0: *tri0, tri1: *tri1 }),
			_ => non_manifold.push((a, b)),
		}
	}
	(shared, non_manifold)
}
