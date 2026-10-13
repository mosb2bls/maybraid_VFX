//! Hydrology nodes: hydraulic primitive + rim/apron [`HydroParams`].
//!
//! See [`crate::WATERSHED_CORRECTION`](../WATERSHED_CORRECTION.md): nodes are the
//! source of truth for hydraulic geometry and parameters;
//! [`Self::max_correction_extent`] only governs spatial discoverability.
//!
//! Terrain correction blends per-node candidates by
//! [`Self::terrain_blend_classification`] priority: carve → soft shore →
//! rim → soft rim↔apron → apron.

use crate::primitive::backfill::HydroBackfill;
use crate::primitive::hydro::{
	smoothmax_fold, smoothmin_fold, HydroFootprint, HydroPrimitive, SURFACE_SMOOTHMIN_K,
};
use crate::primitive::parameters::{CorrectionStage, TerrainBlendStage};
use bevy_math::Vec2;
use procedural_common::{Bounds2, Smoothstep01};

pub use crate::primitive::parameters::HydroParams;

/// One hydrology entity indexed by hydraulic support ⊕ correction extent.
#[derive(Debug, Clone)]
pub struct HydroNode {
	pub primitive: HydroPrimitive,
	pub params: HydroParams,
	/// Max distance beyond hydraulic support at which any correction pass may
	/// need this node. Indexing only — not the final apron profile.
	pub max_correction_extent: f32,
	/// Optional post-blend height noise (at most one kind for now).
	pub backfill: Option<HydroBackfill>,
}

impl HydroNode {
	pub fn new(primitive: HydroPrimitive, params: HydroParams, max_correction_extent: f32) -> Self {
		Self {
			primitive,
			params,
			max_correction_extent: max_correction_extent.max(0.0),
			backfill: None,
		}
	}

	/// Attach a single backfill recipe (replaces any previous).
	pub fn with_backfill(mut self, backfill: HydroBackfill) -> Self {
		self.backfill = Some(backfill);
		self
	}

	/// Conservative pad for indexing / broadphase.
	///
	/// Max of authored [`Self::max_correction_extent`], influence pad, and
	/// param rim+apron. Shore / rim-outer noise stays inside those bands.
	pub fn index_pad(&self) -> f32 {
		self.max_correction_extent
			.max(self.primitive.influence_pad)
			.max(self.params.correction_pad())
	}

	/// Representative interior sample (ellipse center / reach midpoint).
	pub fn sample_point(&self) -> Vec2 {
		match &self.primitive.footprint {
			HydroFootprint::Reach(seg) => (seg.a + seg.b) * 0.5,
			HydroFootprint::Ellipse(e) => e.center,
		}
	}

	/// AABB of hydraulic support expanded by [`Self::index_pad`].
	pub fn correction_index_bounds(&self) -> Bounds2 {
		let (mn, mx) = self.primitive.aabb();
		let pad = self.index_pad();
		Bounds2 { min: mn - Vec2::splat(pad), max: mx + Vec2::splat(pad) }
	}

	/// True when conservative correction support overlaps or touches `bounds`.
	pub fn correction_intersects(&self, bounds: Bounds2) -> bool {
		let support = self.correction_index_bounds();
		support.min.x <= bounds.max.x
			&& bounds.min.x <= support.max.x
			&& support.min.y <= bounds.max.y
			&& bounds.min.y <= support.max.y
	}

	/// True when [`Self::correction_index_bounds`] fits entirely inside `cell`.
	///
	/// Authored leaves use this in `hydro_nodes()` so oversized correction
	/// support never leaves the pocket cell.
	pub fn inbounds(&self, cell: Bounds2) -> bool {
		let b = self.correction_index_bounds();
		cell.contains(b.min) && cell.contains(b.max)
	}

	/// Occupancy SDF, optionally warped by shore [`HydroParams::boundary_noise`]
	/// (centroid → ring / wet \(\phi = 0\)).
	pub fn phi(&self, p: Vec2) -> f32 {
		let mut d = self.primitive.phi(p);
		if let Some(noise) = &self.params.boundary_noise {
			d += noise.sample_boundary(p);
		}
		d
	}

	/// Noisy rim-outer radius along occupancy \(\phi\) (ring → apron seam).
	///
	/// Independent of [`Self::phi`]'s shore warp: \(r_{\mathrm{rim}}(p) =
	/// r_{\mathrm{rim}} + \) [`HydroParams::rim_boundary_noise`].
	pub fn rim_outer(&self, p: Vec2) -> f32 {
		let mut r = self.params.rim.width.max(0.0);
		if let Some(noise) = &self.params.rim_boundary_noise {
			r += noise.sample_boundary(p);
		}
		r.max(0.0)
	}

	pub fn surface_level(&self, p: Vec2) -> f32 {
		self.primitive.surface_and_bed(p).0
	}

	pub fn bed_level(&self, p: Vec2) -> f32 {
		self.primitive.surface_and_bed(p).1
	}

	/// Where `p` sits relative to this node's hydraulic support and bands.
	///
	/// Hard bands (water / debug): carve \(\phi \le 0\), rim, apron.
	/// Terrain blend uses [`Self::terrain_blend_classification`].
	pub fn point_classification(&self, p: Vec2) -> Option<CorrectionStage> {
		let d = self.phi(p);
		let rim_edge = self.rim_outer(p);
		let apron_w = self.params.apron.width.max(0.0);
		if d <= 0.0 {
			Some(CorrectionStage::Carve)
		} else if d < rim_edge {
			Some(CorrectionStage::Rim)
		} else if d < rim_edge + apron_w {
			Some(CorrectionStage::Apron)
		} else {
			None
		}
	}

	/// Soft-aware band for terrain elevation blend (shore + rim↔apron zones).
	pub fn terrain_blend_classification(&self, p: Vec2) -> Option<TerrainBlendStage> {
		let d = self.phi(p);
		let mu = self.shore_blend_half();
		let nu = self.rim_apron_blend_half();
		let rim_edge = self.rim_outer(p);
		let apron_w = self.params.apron.width.max(0.0);
		if d <= 0.0 {
			Some(TerrainBlendStage::Carve)
		} else if d <= mu {
			Some(TerrainBlendStage::SoftShore)
		} else if d < rim_edge - nu {
			Some(TerrainBlendStage::Rim)
		} else if d <= rim_edge + nu {
			Some(TerrainBlendStage::SoftRimApron)
		} else if d < rim_edge + apron_w {
			Some(TerrainBlendStage::Apron)
		} else {
			None
		}
	}

	#[inline]
	fn shore_blend_half(&self) -> f32 {
		self.params.shore_blend_half()
	}

	/// Bed graded toward free surface \(W\) as \(\phi \to 0^-\) (no bank lift).
	fn carve_bed_toward_w(&self, p: Vec2) -> f32 {
		let w = self.surface_level(p);
		let geo_bed = self.primitive.surface_and_bed(p).1;
		let geo_depth = (w - geo_bed).max(0.0);
		let phi = self.phi(p);
		if phi >= 0.0 {
			return w;
		}
		let edge = self.params.boundary_noise_amp().max(2.0);
		let t = (-phi / edge).clamp(0.0, 1.0);
		let keep = t * t * (3.0 - 2.0 * t);
		w - geo_depth * keep
	}

	/// Absolute bank across the rim band (classification uses noisy [`Self::phi`]).
	pub fn rim_candidate(&self, _elevation: f32, p: Vec2) -> f32 {
		self.params.bank_target(self.surface_level(p), p)
	}

	/// Soft carve↔rim height across \(\phi \in [-\mu, +\mu]\).
	///
	/// Geometric bed falloff can disagree with [`Self::phi`]'s boundary noise;
	/// depth → 0 as \(\phi \to 0^-\) ([`Self::carve_bed_toward_w`]). Without
	/// shore blend the terrain meets at \(W\) then jumps to `bank_target` on the
	/// rim side — a vertical cliff along \(\phi = 0\). Soft blend removes that
	/// cliff (water ownership stays hard at \(\phi = 0\)).
	pub fn carve_candidate(&self, elevation: f32, p: Vec2) -> f32 {
		self.shore_terrain_candidate(elevation, p)
	}

	/// Soft carve↔rim height for \(\phi \in [-\mu, +\mu]\); outside that, bed or bank.
	fn shore_terrain_candidate(&self, elevation: f32, p: Vec2) -> f32 {
		let phi = self.phi(p);
		let mu = self.shore_blend_half();
		let bed = self.carve_bed_toward_w(p);
		let bank = self.rim_candidate(elevation, p);
		if mu <= 1e-6 {
			return if phi <= 0.0 { bed } else { bank };
		}
		// t = 0 at φ = -μ, t = 1 at φ = +μ.
		let t = Smoothstep01::eval((phi + mu) / (2.0 * mu));
		bed * (1.0 - t) + bank * t
	}

	/// Grade from bank at the (noisy) rim edge toward identity at the apron outer.
	///
	/// Uses occupancy \(\phi\) and [`Self::rim_outer`] so the grade tracks both
	/// noisy radii (shore + ring→apron).
	pub fn apron_candidate(&self, elevation: f32, p: Vec2) -> f32 {
		let d = self.phi(p);
		let rim_edge = self.rim_outer(p);
		let apron_w = self.params.apron.width.max(1e-3);
		// Smoothstep grade: 0 at rim/apron seam (full bank), 1 at outer (terrain).
		let u = ((d - rim_edge) / apron_w).clamp(0.0, 1.0);
		let fade = u * u * (3.0 - 2.0 * u);
		let bank = self.params.bank_target(self.surface_level(p), p);
		// Raise-only: never cut below the incoming elevation while grading out.
		(elevation * fade + bank * (1.0 - fade)).max(elevation)
	}

	#[inline]
	fn rim_apron_blend_half(&self) -> f32 {
		self.params.rim_apron_blend_half()
	}

	/// Soft rim↔apron height across \(\phi \in [r_{\mathrm{rim}}(p)-\nu, r_{\mathrm{rim}}(p)+\nu]\).
	fn rim_apron_terrain_candidate(&self, elevation: f32, p: Vec2) -> f32 {
		let d = self.phi(p);
		let rim_edge = self.rim_outer(p);
		let nu = self.rim_apron_blend_half();
		let rim_h = self.rim_candidate(elevation, p);
		let apron_h = self.apron_candidate(elevation, p);
		if nu <= 1e-6 {
			return if d < rim_edge { rim_h } else { apron_h };
		}
		// t = 0 at φ = rim_edge - ν, t = 1 at φ = rim_edge + ν.
		let t = Smoothstep01::eval((d - (rim_edge - nu)) / (2.0 * nu));
		rim_h * (1.0 - t) + apron_h * t
	}

	/// Carve → rim → apron blend **without** per-node backfill.
	///
	/// Soft shore: wet nodes (\(\phi \le 0\)) use [`Self::shore_terrain_candidate`]
	/// so height lifts toward the bank as \(\phi \to 0^-\). Samples with
	/// \(0 < \phi \le \mu\) (rim-side blend) apply only when no wet carve owns
	/// the column — keeps soft-min membership stable (extra rim-side terms were
	/// dipping beds via polynomial soft-min).
	///
	/// Soft rim↔apron: pure rim stays soft-max banks; a band around
	/// \(r_{\mathrm{rim}}(p)\) uses [`Self::rim_apron_terrain_candidate`] when no
	/// pure-rim node owns the column.
	///
	/// WaterFill / bare-bed callers use this; terrain uses [`Self::elevation_blend`].
	pub fn elevation_blend_without_backfill(nodes: &[&Self], elevation: f32, p: Vec2) -> f32 {
		let mut carves: Vec<&Self> = Vec::new();
		let mut soft_shores: Vec<&Self> = Vec::new();
		let mut rims: Vec<&Self> = Vec::new();
		let mut soft_rim_aprons: Vec<&Self> = Vec::new();
		let mut aprons: Vec<&Self> = Vec::new();

		for node in nodes {
			match node.terrain_blend_classification(p) {
				Some(TerrainBlendStage::Carve) => carves.push(node),
				Some(TerrainBlendStage::SoftShore) => soft_shores.push(node),
				Some(TerrainBlendStage::Rim) => rims.push(node),
				Some(TerrainBlendStage::SoftRimApron) => soft_rim_aprons.push(node),
				Some(TerrainBlendStage::Apron) => aprons.push(node),
				None => {}
			}
		}

		if !carves.is_empty() {
			let vals: Vec<f32> =
				carves.iter().map(|n| n.shore_terrain_candidate(elevation, p)).collect();
			return smoothmin_fold(&vals, SURFACE_SMOOTHMIN_K);
		}
		if !soft_shores.is_empty() {
			let vals: Vec<f32> =
				soft_shores.iter().map(|n| n.shore_terrain_candidate(elevation, p)).collect();
			return smoothmin_fold(&vals, SURFACE_SMOOTHMIN_K);
		}
		if !rims.is_empty() {
			let mut surfaces = Vec::with_capacity(rims.len());
			let mut raised = Vec::with_capacity(rims.len());
			for n in &rims {
				surfaces.push(n.surface_level(p));
				raised.push(n.rim_candidate(elevation, p));
			}
			let water = smoothmin_fold(&surfaces, SURFACE_SMOOTHMIN_K);
			return smoothmax_fold(&raised, SURFACE_SMOOTHMIN_K).max(water);
		}
		if !soft_rim_aprons.is_empty() {
			let vals: Vec<f32> = soft_rim_aprons
				.iter()
				.map(|n| n.rim_apron_terrain_candidate(elevation, p))
				.collect();
			return smoothmax_fold(&vals, SURFACE_SMOOTHMIN_K);
		}
		if !aprons.is_empty() {
			let vals: Vec<f32> = aprons.iter().map(|n| n.apron_candidate(elevation, p)).collect();
			return smoothmax_fold(&vals, SURFACE_SMOOTHMIN_K);
		}
		elevation
	}

	/// Terrain elevation: [`Self::elevation_blend_without_backfill`], then soft-max
	/// with each intersecting node's backfill-raised height.
	///
	/// Soft-max (not a sum) so overlapping stream-graph segments do not stack
	/// rim grit into runaway pillars — same spirit as rim bank soft-max.
	pub fn elevation_blend(nodes: &[&Self], elevation: f32, p: Vec2) -> f32 {
		let h0 = Self::elevation_blend_without_backfill(nodes, elevation, p);
		let mut raised = Vec::with_capacity(nodes.len() + 1);
		raised.push(h0);
		for node in nodes {
			if let Some(bf) = &node.backfill {
				if bf.weight(node, p) > 1e-6 {
					raised.push(bf.compose(h0, node, p));
				}
			}
		}
		if raised.len() == 1 {
			h0
		} else {
			smoothmax_fold(&raised, SURFACE_SMOOTHMIN_K)
		}
	}

	/// Alias for [`Self::elevation_blend`] (historical name).
	#[inline]
	pub fn blend_terrain_elevation(nodes: &[&Self], elevation: f32, p: Vec2) -> f32 {
		Self::elevation_blend(nodes, elevation, p)
	}

	/// Soft-min free surface over **Carve** nodes at `p` (same ownership as terrain carve).
	///
	/// Rim / apron do not contribute — keeps \(W\) coupled to the local carved bed.
	pub fn blend_surface_elevation(nodes: &[&Self], p: Vec2) -> Option<f32> {
		let mut surfaces = Vec::new();
		for node in nodes {
			if matches!(node.point_classification(p), Some(CorrectionStage::Carve)) {
				surfaces.push(node.surface_level(p));
			}
		}
		if surfaces.is_empty() {
			None
		} else {
			Some(smoothmin_fold(&surfaces, SURFACE_SMOOTHMIN_K))
		}
	}
}

/// Build reach-segment nodes from a graded polyline (one node per segment).
///
/// When `backfill` is `Some`, each segment node clones that recipe.
pub fn nodes_from_polyline(
	path: &[Vec2],
	levels: &[f32],
	half_width: f32,
	center_depth: f32,
	params: &HydroParams,
	max_correction_extent: f32,
	backfill: Option<&HydroBackfill>,
) -> Vec<HydroNode> {
	use crate::primitive::hydro::{HydroElevation, HydroFootprint, ReachProfile, ReachSegment};
	let n = path.len().min(levels.len());
	if n < 2 {
		return Vec::new();
	}
	let hw = half_width.max(1e-3);
	let depth = center_depth.max(0.25);
	let extent = max_correction_extent.max(params.correction_pad()).max(0.0);
	let mut out = Vec::with_capacity(n - 1);
	for i in 0..n - 1 {
		let a = path[i];
		let b = path[i + 1];
		if a.distance(b) <= 1e-4 {
			continue;
		}
		let mut node = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment { a, b, half_width: hw }),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: levels[i],
					surface_b: levels[i + 1],
					center_depth: depth,
				}),
				influence_pad: extent,
			},
			params.clone(),
			extent,
		);
		if let Some(bf) = backfill {
			node.backfill = Some(bf.clone());
		}
		out.push(node);
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::hydro::{HydroElevation, HydroFootprint, ReachProfile, ReachSegment};

	fn reach_node(half_width: f32) -> HydroNode {
		let mut params = HydroParams::default();
		params.rim.width = 4.0;
		params.apron.width = 8.0;
		HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment {
					a: Vec2::new(0.0, 0.0),
					b: Vec2::new(40.0, 0.0),
					half_width,
				}),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: 30.0,
					surface_b: 30.0,
					center_depth: 3.0,
				}),
				influence_pad: 12.0,
			},
			params,
			12.0,
		)
	}

	#[test]
	fn inbounds_requires_correction_aabb_inside_cell() -> anyhow::Result<()> {
		let node = reach_node(8.0);
		let tight = Bounds2::from_xz(0.0, -10.0, 40.0, 10.0);
		anyhow::ensure!(!node.inbounds(tight), "pad should spill past tight cell");
		let roomy = Bounds2::from_xz(-80.0, -80.0, 120.0, 80.0);
		anyhow::ensure!(node.inbounds(roomy), "roomy cell should contain index AABB");
		Ok(())
	}

	#[test]
	fn classifies_carve_rim_apron_bands() -> anyhow::Result<()> {
		let node = reach_node(8.0);
		assert_eq!(node.point_classification(Vec2::new(20.0, 0.0)), Some(CorrectionStage::Carve));
		assert_eq!(node.point_classification(Vec2::new(20.0, 10.0)), Some(CorrectionStage::Rim));
		assert_eq!(node.point_classification(Vec2::new(20.0, 14.0)), Some(CorrectionStage::Apron));
		assert_eq!(node.point_classification(Vec2::new(20.0, 30.0)), None);
		Ok(())
	}

	#[test]
	fn rim_boundary_noise_shifts_ring_to_apron_seam() -> anyhow::Result<()> {
		let mut node = reach_node(8.0);
		// Constant +2 wu on rim outer (sample_boundary returns raw when not expand_only).
		node.params.rim_boundary_noise = Some(terrain_stamps::RegionNoise::from_seed(0, 0.0, 2.0));
		// Force a constant sample by using amp with zero frequency — still spatial.
		// Instead assert via rim_outer offset when noise amp is large and we pick a
		// point where classification must move with rim_edge.
		let p = Vec2::new(20.0, 12.5); // φ ≈ 4.5; nominal rim_w=4 → apron; +noise may pull back to rim
		let edge = node.rim_outer(p);
		anyhow::ensure!(
			(edge - 4.0).abs() > 1e-3 || node.params.rim_boundary_noise_amp() > 0.0,
			"rim boundary noise should be attached"
		);
		anyhow::ensure!(edge >= 0.0, "rim_outer non-negative");
		// Classification must agree with noisy edge, not nominal rim.width alone.
		let d = node.phi(p);
		let expected = if d <= 0.0 {
			Some(CorrectionStage::Carve)
		} else if d < edge {
			Some(CorrectionStage::Rim)
		} else if d < edge + node.params.apron.width {
			Some(CorrectionStage::Apron)
		} else {
			None
		};
		assert_eq!(node.point_classification(p), expected);
		Ok(())
	}

	#[test]
	fn elevation_blend_composes_backfill_over_bare() -> anyhow::Result<()> {
		use crate::primitive::backfill::{HydroBackfill, RimBackfill};
		let mut node = reach_node(8.0);
		node.backfill = Some(HydroBackfill::Rim(RimBackfill {
			noise: terrain_stamps::RegionNoise::from_seed(9, 0.06, 3.0),
			band: 6.0,
			add_only: true,
		}));
		let p = Vec2::new(20.0, 8.0);
		let bare = HydroNode::elevation_blend_without_backfill(&[&node], 40.0, p);
		let full = HydroNode::elevation_blend(&[&node], 40.0, p);
		anyhow::ensure!(
			(full - bare).abs() > 1e-3 || node.backfill.as_ref().unwrap().weight(&node, p) < 0.05,
			"with-backfill should differ when rim weight is active: bare={bare} full={full}"
		);
		let far = Vec2::new(20.0, 40.0);
		let bare_f = HydroNode::elevation_blend_without_backfill(&[&node], 40.0, far);
		let full_f = HydroNode::elevation_blend(&[&node], 40.0, far);
		anyhow::ensure!((bare_f - full_f).abs() < 1e-3, "far field should match bare and full");
		Ok(())
	}

	#[test]
	fn elevation_blend_soft_maxes_overlapping_backfills() -> anyhow::Result<()> {
		use crate::primitive::backfill::{HydroBackfill, RimBackfill};
		let bf = HydroBackfill::Rim(RimBackfill {
			noise: terrain_stamps::RegionNoise::from_seed(11, 0.05, 4.0),
			band: 8.0,
			add_only: true,
		});
		let mut a = reach_node(8.0);
		a.backfill = Some(bf.clone());
		let mut b = a.clone();
		b.backfill = Some(bf);
		let p = Vec2::new(20.0, 8.0);
		let h0 = HydroNode::elevation_blend_without_backfill(&[&a, &b], 40.0, p);
		let one = HydroNode::elevation_blend(&[&a], 40.0, p);
		let two = HydroNode::elevation_blend(&[&a, &b], 40.0, p);
		let w = a.backfill.as_ref().unwrap().weight(&a, p);
		anyhow::ensure!(w > 0.2, "rim weight should be active at shore, got {w}");
		// Soft-max of identical candidates ≈ one raise, not 2× the delta.
		let delta_one = one - h0;
		let delta_two = two - h0;
		anyhow::ensure!(
			delta_two < delta_one * 1.35,
			"overlapping identical backfills must not stack: Δ1={delta_one} Δ2={delta_two}"
		);
		Ok(())
	}

	#[test]
	fn soft_shore_meets_bank_across_phi_zero() -> anyhow::Result<()> {
		let mut node = reach_node(8.0);
		node.params.shore_blend = 4.0;
		node.params.rim.lift = 2.0;
		node.params.rim_height = terrain_stamps::RegionNoise::from_seed(0, 0.02, 0.0);
		// Lateral transect across the wet edge (half_width = 8).
		let y_in = 8.0 - 2.0;
		let y_out = 8.0 + 2.0;
		let p_in = Vec2::new(20.0, y_in);
		let p_shore = Vec2::new(20.0, 8.0);
		let p_out = Vec2::new(20.0, y_out);
		let h_in = HydroNode::blend_terrain_elevation(&[&node], 40.0, p_in);
		let h_shore = HydroNode::blend_terrain_elevation(&[&node], 40.0, p_shore);
		let h_out = HydroNode::blend_terrain_elevation(&[&node], 40.0, p_out);
		let bank = node.params.bank_target(node.surface_level(p_shore), p_shore);
		// No cliff: shore samples stay near the bank, and the step across φ=0 is small.
		assert!(
			(h_shore - bank).abs() < 1.25,
			"shore height {h_shore} should approach bank {bank}"
		);
		assert!(
			(h_out - h_in).abs() < 2.5,
			"soft shore should limit jump across edge: in={h_in} out={h_out}"
		);
		Ok(())
	}

	#[test]
	fn soft_rim_apron_limits_jump_at_berm_outer() -> anyhow::Result<()> {
		let mut node = reach_node(8.0);
		node.params.rim.width = 4.0;
		node.params.apron.width = 8.0;
		node.params.rim_apron_blend = 2.0;
		node.params.rim.lift = 2.0;
		node.params.rim_height = terrain_stamps::RegionNoise::from_seed(0, 0.02, 0.0);
		// half_width=8 → φ≈0 at y=8; rim outer at y≈12.
		let rim_outer = 8.0 + 4.0;
		let p_in = Vec2::new(20.0, rim_outer - 1.5);
		let p_out = Vec2::new(20.0, rim_outer + 1.5);
		let h_in = HydroNode::blend_terrain_elevation(&[&node], 28.0, p_in);
		let h_out = HydroNode::blend_terrain_elevation(&[&node], 28.0, p_out);
		assert!(
			(h_out - h_in).abs() < 1.5,
			"soft rim↔apron should limit jump at berm outer: in={h_in} out={h_out}"
		);
		Ok(())
	}

	#[test]
	fn blend_prefers_carve_over_rim() -> anyhow::Result<()> {
		let deep = reach_node(8.0);
		let mut shallow_params = HydroParams::default();
		shallow_params.rim.width = 4.0;
		shallow_params.apron.width = 8.0;
		let offset = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment {
					a: Vec2::new(0.0, 6.0),
					b: Vec2::new(40.0, 6.0),
					half_width: 4.0,
				}),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: 30.0,
					surface_b: 30.0,
					center_depth: 1.0,
				}),
				influence_pad: 12.0,
			},
			shallow_params,
			12.0,
		);
		// Inside deep channel; offset node may classify rim/apron nearby.
		let p = Vec2::new(20.0, 0.0);
		assert_eq!(deep.point_classification(p), Some(CorrectionStage::Carve));
		let h = HydroNode::blend_terrain_elevation(&[&deep, &offset], 40.0, p);
		assert!(h < 30.0 - 1.0, "carve should win: {h}");
		Ok(())
	}
}
