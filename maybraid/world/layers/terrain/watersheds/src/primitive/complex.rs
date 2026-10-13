//! Indexed hydrology complex: member nodes + sample-time correction.
//!
//! Authored leaves emit [`crate::primitive::node::HydroNode`]s (optional
//! per-node [`crate::primitive::backfill::HydroBackfill`]).
//! [`HydroComplex`] is a bag of nodes to blend — no graph / connectivity.

use crate::primitive::fill::WaterFill;
use crate::primitive::hydro::{FootprintIndex, HydroPrimitive};
use crate::primitive::node::HydroNode;
use crate::primitive::parameters::ComplexParams;
use bevy_math::{Vec2, Vec3};
use procedural_common::Bounds2;
use std::sync::Arc;

/// Stamp products derived from a [`HydroComplex`] (fills).
#[derive(Debug, Clone)]
pub struct CompiledWatershed {
	pub bounds: Bounds2,
	pub seed: u32,
	/// Indexed complex used for elevation / fill sampling.
	pub complex: Arc<HydroComplex>,
	pub fills: Vec<WaterFill>,
}

impl CompiledWatershed {
	pub fn is_empty(&self) -> bool {
		self.complex.is_empty()
	}

	pub fn has_hydro(&self) -> bool {
		!self.complex.is_empty()
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		self.complex.modify_elevation(elevation, x, z)
	}
}

/// Captures and indexes hydrology nodes; owns carve → rim → apron modulation
/// (plus optional per-node backfill) and the pocket water SDF (carve × half-space below \(W\)).
#[derive(Debug, Clone)]
pub struct HydroComplex {
	pub bounds: Bounds2,
	pub seed: u32,
	/// Member hydrology nodes (source of truth for correction).
	pub hydrology: Vec<HydroNode>,
	/// Broadphase over [`Self::hydrology`] (rebuilt when nodes change).
	pub index: FootprintIndex,
}

impl HydroComplex {
	pub fn new(bounds: Bounds2, seed: u32) -> Self {
		let mut complex =
			Self { bounds, seed, hydrology: Vec::new(), index: FootprintIndex::empty() };
		complex.reindex();
		complex
	}

	/// Test helper: wrap bare primitives with shared apron parameters.
	pub fn from_primitives(
		bounds: Bounds2,
		seed: u32,
		primitives: Vec<HydroPrimitive>,
		apron: ComplexParams,
	) -> Self {
		let params = apron.into_params();
		let extent = params.correction_pad();
		let hydrology = primitives
			.into_iter()
			.map(|primitive| HydroNode::new(primitive, params.clone(), extent))
			.collect();
		Self::new(bounds, seed).with_hydro(hydrology)
	}

	pub fn with_hydro(mut self, nodes: Vec<HydroNode>) -> Self {
		self.hydrology = nodes;
		self.reindex();
		self
	}

	pub fn is_empty(&self) -> bool {
		self.hydrology.is_empty()
	}

	fn reindex(&mut self) {
		let short = self.bounds.extent().min_element().max(1.0);
		let cell = (short * 0.08).clamp(8.0, 64.0);
		self.index = FootprintIndex::build_nodes(self.bounds, &self.hydrology, cell);
	}

	pub fn candidate_ids(&self, p: Vec2) -> Vec<u16> {
		let raw = self.index.candidates(p);
		if raw.is_empty() {
			return Vec::new();
		}
		let mut ids = raw.to_vec();
		ids.sort_unstable();
		ids.dedup();
		ids
	}

	/// Nodes whose correction support contains `p` (index pad + SDF filter).
	pub fn nodes_intersecting(&self, p: Vec2) -> Vec<&HydroNode> {
		let mut out = Vec::new();
		for id in self.candidate_ids(p) {
			let Some(node) = self.hydrology.get(id as usize) else {
				continue;
			};
			if node.phi(p) > node.index_pad() {
				continue;
			}
			out.push(node);
		}
		out
	}

	/// Class-priority blend over intersecting nodes (carve → rim → apron + backfill).
	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return elevation;
		}
		let nodes = self.nodes_intersecting(p);
		HydroNode::elevation_blend(&nodes, elevation, p)
	}

	/// Soft-min free surface over carve-owned nodes at `(x, z)`.
	pub fn surface_at(&self, x: f32, z: f32) -> Option<f32> {
		let p = Vec2::new(x, z);
		let nodes = self.nodes_intersecting(p);
		HydroNode::blend_surface_elevation(&nodes, p)
	}

	/// Min \(\phi\) over intersecting nodes (`None` if none in index).
	pub fn min_phi_at(&self, x: f32, z: f32) -> Option<f32> {
		let p = Vec2::new(x, z);
		let mut phi = f32::INFINITY;
		for node in self.nodes_intersecting(p) {
			phi = phi.min(node.phi(p));
		}
		phi.is_finite().then_some(phi)
	}

	/// Hard carve test: any intersecting node classifies as Carve.
	pub fn inside_carve(&self, x: f32, z: f32) -> bool {
		self.surface_at(x, z).is_some()
	}

	/// Cheap exterior distance toward the wet carve (\(\phi = 0\)).
	///
	/// Prefer indexed \(\phi\); if the sample misses the broadphase, fall back to
	/// a full member scan so MC sees a continuous field instead of a huge sentinel.
	pub fn approximate_distance_to_carve(&self, x: f32, z: f32) -> f32 {
		if let Some(phi) = self.min_phi_at(x, z) {
			return phi;
		}
		let p = Vec2::new(x, z);
		let mut phi = f32::INFINITY;
		for node in &self.hydrology {
			phi = phi.min(node.phi(p));
		}
		if phi.is_finite() {
			return phi;
		}
		// Empty complex: distance to the leaf AABB (still Lipschitz-ish for MC).
		let dx = (self.bounds.min.x - x).max(x - self.bounds.max.x).max(0.0);
		let dz = (self.bounds.min.y - z).max(z - self.bounds.max.y).max(0.0);
		(dx * dx + dz * dz).sqrt()
	}

	/// Pocket water SDF: outside carve → distance to carve; inside → half-space below \(W\).
	///
	/// Terrain occludes subterranean volume; the free surface stays flat at \(W\)
	/// across the carve (not a slab against \(h\)).
	pub fn water_distance(&self, p: Vec3, _terrain_height: f32) -> f32 {
		let Some(w) = self.surface_at(p.x, p.z) else {
			return self.approximate_distance_to_carve(p.x, p.z);
		};
		p.y - w
	}

	/// Build a [`WaterFill`] that delegates SDF / \(W\) to this complex.
	pub fn water_fill(&self) -> WaterFill {
		WaterFill::from_hydro(Arc::new(self.clone()))
	}

	pub fn compile(&self) -> CompiledWatershed {
		let complex = Arc::new(self.clone());
		let fills = if complex.is_empty() {
			Vec::new()
		} else {
			vec![WaterFill::from_hydro(Arc::clone(&complex))]
		};
		CompiledWatershed { bounds: self.bounds, seed: self.seed, complex, fills }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::backfill::{BasinBackfill, HydroBackfill};
	use crate::primitive::hydro::{
		Ellipse, HydroElevation, HydroFootprint, HydroPrimitive, RadialBowl, ReachProfile,
		ReachSegment,
	};
	use crate::primitive::node::HydroNode;
	use crate::primitive::parameters::HydroParams;
	use terrain_stamps::RegionNoise;

	#[test]
	fn empty_complex_compiles_empty() -> anyhow::Result<()> {
		let c = HydroComplex::new(Bounds2::from_xz(0.0, 0.0, 10.0, 10.0), 1);
		let out = c.compile();
		assert!(out.is_empty());
		assert!(out.fills.is_empty());
		Ok(())
	}

	#[test]
	fn node_basin_backfill_raises_interior() -> anyhow::Result<()> {
		let node = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse(Ellipse {
					center: Vec2::ZERO,
					radii: Vec2::splat(8.0),
					rotation: 0.0,
				}),
				elevation: HydroElevation::Radial(RadialBowl { surface: 40.0, center_depth: 3.0 }),
				influence_pad: 4.0,
			},
			HydroParams::default(),
			12.0,
		)
		.with_backfill(HydroBackfill::Basin(BasinBackfill {
			noise: RegionNoise::from_seed(1, 0.08, 4.0),
			fade: 2.0,
			add_only: true,
		}));
		let complex =
			HydroComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 3).with_hydro(vec![node]);
		let base = 40.0;
		let mut max_raise = 0.0_f32;
		for &(x, z) in &[(0.0, 0.0), (2.0, 1.0), (-3.0, 2.0), (1.5, -2.5), (-1.0, -1.0)] {
			let bare = {
				let nodes = complex.nodes_intersecting(Vec2::new(x, z));
				HydroNode::elevation_blend_without_backfill(&nodes, base, Vec2::new(x, z))
			};
			let full = complex.modify_elevation(base, x, z);
			max_raise = max_raise.max(full - bare);
		}
		anyhow::ensure!(max_raise > 0.05, "interior should raise somewhere, max Δ={max_raise}");
		let h_out = complex.modify_elevation(base, 35.0, 0.0);
		anyhow::ensure!((h_out - base).abs() < 1e-3, "far field identity, got {h_out}");
		Ok(())
	}

	#[test]
	fn pitched_neighbor_rim_does_not_dry_upstream_carve() -> anyhow::Result<()> {
		// Colinear reaches with a steep drop. Mid of the upstream segment sits in
		// the downstream rim band; fill W must not soft-min that lower surface.
		let half_w = 8.0;
		let rim_w = 25.0;
		let depth = 4.0;
		let mut params = HydroParams::default();
		params.rim.width = rim_w;
		params.apron.width = 8.0;
		let a = Vec2::new(0.0, 0.0);
		let b = Vec2::new(40.0, 0.0);
		let c = Vec2::new(80.0, 0.0);
		let upstream = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment { a, b, half_width: half_w }),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: 100.0,
					surface_b: 80.0,
					center_depth: depth,
				}),
				influence_pad: rim_w + 8.0,
			},
			params.clone(),
			rim_w + 8.0,
		);
		let downstream = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment { a: b, b: c, half_width: half_w }),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: 80.0,
					surface_b: 60.0,
					center_depth: depth,
				}),
				influence_pad: rim_w + 8.0,
			},
			params,
			rim_w + 8.0,
		);
		let complex = HydroComplex::new(Bounds2::from_xz(-20.0, -40.0, 100.0, 40.0), 1)
			.with_hydro(vec![upstream, downstream]);
		let mid = Vec2::new(20.0, 0.0);
		let w = complex.surface_at(mid.x, mid.y).expect("surface");
		assert!(w > 85.0, "fill W should track upstream carve grade, got {w}");
		let h = complex.modify_elevation(100.0, mid.x, mid.y);
		let fill = complex.water_fill();
		assert!(
			fill.wet_y_span_at(mid.x, mid.y, h).is_some(),
			"pitched mid-channel should stay wet: W={w} h={h}"
		);
		let d = fill.distance(bevy_math::Vec3::new(mid.x, (h + w) * 0.5, mid.y), h);
		assert!(d < 0.0, "half-space interior should be negative, got {d}");
		Ok(())
	}

	#[test]
	fn water_exterior_uses_carve_distance_not_sentinel() -> anyhow::Result<()> {
		let node = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse(Ellipse {
					center: Vec2::ZERO,
					radii: Vec2::splat(10.0),
					rotation: 0.0,
				}),
				elevation: HydroElevation::Radial(RadialBowl { surface: 40.0, center_depth: 3.0 }),
				influence_pad: 20.0,
			},
			HydroParams::default(),
			20.0,
		);
		let complex =
			HydroComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 7).with_hydro(vec![node]);
		// Just outside the bowl: positive φ, finite, not a 1e6 cliff.
		let d = complex.approximate_distance_to_carve(12.0, 0.0);
		assert!(d > 0.0 && d < 20.0, "expected modest carve distance, got {d}");
		let far = complex.water_distance(Vec3::new(12.0, 38.0, 0.0), 36.0);
		assert!((far - d).abs() < 1e-3, "outside carve water_distance should equal carve approx");
		Ok(())
	}

	#[test]
	fn water_fill_probes_node_interior() -> anyhow::Result<()> {
		let node = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse(Ellipse {
					center: Vec2::new(30.0, -25.0),
					radii: Vec2::splat(8.0),
					rotation: 0.0,
				}),
				elevation: HydroElevation::Radial(RadialBowl { surface: 40.0, center_depth: 3.0 }),
				influence_pad: 12.0,
			},
			HydroParams::default(),
			12.0,
		);
		let fill = HydroComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 5)
			.with_hydro(vec![node])
			.water_fill();
		let probes = fill.wet_volume_probe_points();
		assert!(
			probes.iter().any(|p| (p.x - 30.0).abs() < 1e-3 && (p.y + 25.0).abs() < 1e-3),
			"expected lake-center probe, got {probes:?}"
		);
		Ok(())
	}
}
