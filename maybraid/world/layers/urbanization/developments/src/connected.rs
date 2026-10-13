//! Reusable node-edge wrapper for developments organized by a higher-order graph.

use bevy_math::bounding::Aabb3d;

/// One connection between two indices in [`ConnectedDevelopment::nodes`].
#[derive(Debug, Clone, PartialEq)]
pub struct DevelopmentEdge<E> {
	pub from: usize,
	pub to: usize,
	pub payload: E,
}

impl<E> DevelopmentEdge<E> {
	pub fn new(from: usize, to: usize, payload: E) -> Self {
		Self { from, to, payload }
	}

	pub fn connects(&self, node: usize) -> bool {
		self.from == node || self.to == node
	}

	pub fn other(&self, node: usize) -> Option<usize> {
		if self.from == node {
			Some(self.to)
		} else if self.to == node {
			Some(self.from)
		} else {
			None
		}
	}
}

/// A development whose sites and connecting structures form a graph.
///
/// Node and edge payloads remain development-specific. This wrapper owns only
/// topology, bounds, and common traversal helpers.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedDevelopment<N, E> {
	pub bounds: Aabb3d,
	pub nodes: Vec<N>,
	pub edges: Vec<DevelopmentEdge<E>>,
}

impl<N, E> ConnectedDevelopment<N, E> {
	pub fn new(bounds: Aabb3d, nodes: Vec<N>, edges: Vec<DevelopmentEdge<E>>) -> Self {
		debug_assert!(
			edges.iter().all(|edge| edge.from < nodes.len() && edge.to < nodes.len()),
			"connected development edge endpoint is out of bounds"
		);
		Self { bounds, nodes, edges }
	}

	pub fn incident_edges(&self, node: usize) -> impl Iterator<Item = &DevelopmentEdge<E>> {
		self.edges.iter().filter(move |edge| edge.connects(node))
	}

	pub fn edge_endpoints(&self, edge: &DevelopmentEdge<E>) -> Option<(&N, &N)> {
		Some((self.nodes.get(edge.from)?, self.nodes.get(edge.to)?))
	}

	pub fn topology_is_valid(&self) -> bool {
		self.edges
			.iter()
			.all(|edge| edge.from < self.nodes.len() && edge.to < self.nodes.len())
	}
}

#[cfg(test)]
mod tests {
	use bevy_math::{bounding::Aabb3d, Vec3};

	use super::*;

	#[test]
	fn connected_development_resolves_endpoints() -> anyhow::Result<()> {
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let connected = ConnectedDevelopment::new(
			bounds,
			vec!["peak", "hut"],
			vec![DevelopmentEdge::new(0, 1, "path")],
		);
		anyhow::ensure!(connected.topology_is_valid());
		let edge = connected
			.incident_edges(0)
			.next()
			.ok_or_else(|| anyhow::anyhow!("peak should have an incident path"))?;
		let (from, to) = connected
			.edge_endpoints(edge)
			.ok_or_else(|| anyhow::anyhow!("path endpoints should resolve"))?;
		anyhow::ensure!(*from == "peak" && *to == "hut");
		anyhow::ensure!(edge.other(0) == Some(1));
		Ok(())
	}
}
