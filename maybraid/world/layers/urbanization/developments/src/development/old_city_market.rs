//! [`OldCityMarket`]: stall clusters on stone terraces joined by graded lanes.

mod terrace;

pub use terrace::{OldCityMarketTerrace, OldCityMarketTier, MARKET_PLATFORM_HEIGHT};

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, NoiseParams, SeededHash};

use super::raise_toward_peak;
use super::shepherds_village::fit::{fit_shepherds_building_for_role, ShepherdsBuildingKind};
use super::shepherds_village::ShepherdsVillageBuilding;
use crate::connectivity::{corridor_levels, ConnectivityCorridor, ConnectivityGraph};
use crate::finish::DevelopmentFinishRole;
use crate::math::lerp;
use crate::plan::cell_salt;
use crate::scatter::{bounds_intersect, ScatterChoice, ScatterRecipe};
use crate::{
	ConnectedDevelopment, Development, DevelopmentEdge, PadParams, PadPlan, PlacedBuilding,
	SiteGround,
};

/// One graph site with a common platform and several independently posed stalls.
#[derive(Debug, Clone, PartialEq)]
pub struct OldCityMarketSite {
	pub position: Vec2,
	pub elevation: f32,
	pub tier: OldCityMarketTier,
	pub terrace: PlacedBuilding<OldCityMarketTerrace>,
	pub buildings: Vec<ShepherdsVillageBuilding>,
}

/// Graded market lane connecting two site terraces.
#[derive(Debug, Clone, PartialEq)]
pub struct OldCityMarketCorridor {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub half_width: f32,
}

/// A market whose stall clusters are joined by a deterministic lane graph.
pub type OldCityMarket = ConnectedDevelopment<OldCityMarketSite, OldCityMarketCorridor>;

impl OldCityMarket {
	pub fn buildings(&self) -> impl Iterator<Item = &ShepherdsVillageBuilding> {
		self.nodes.iter().flat_map(|site| site.buildings.iter())
	}

	pub fn terraces(&self) -> impl Iterator<Item = &PlacedBuilding<OldCityMarketTerrace>> {
		self.nodes.iter().map(|site| &site.terrace)
	}

	pub fn corridors(&self) -> impl Iterator<Item = &OldCityMarketCorridor> {
		self.edges.iter().map(|edge| &edge.payload)
	}

	pub fn stall_count(&self) -> usize {
		self.nodes.iter().map(|site| site.buildings.len()).sum()
	}
}

const MIN_MARKET_EXTENT: f32 = 210.0;
const MAX_MARKET_EXTENT: f32 = 236.0;
const PLAN_EDGE_INSET: f32 = 24.0;
const PATH_HALF_WIDTH: f32 = 3.5;
const MIN_PATH_LEN: f32 = 12.0;
const MAX_PATH_GRADE: f32 = 0.12;
const MAX_MARKET_RELIEF: f32 = 6.0;
const SITE_SEPARATION: f32 = 1.5;
const CORE_CLEARANCE: f32 = 3.0;
const MIN_MARKET_STALLS: usize = 24;
/// Half-range of stall yaw about the nearest corridor quarter-turn.
const STALL_YAW_JITTER: f32 = 0.35;

#[derive(Debug, Clone, Copy)]
struct MarketSitePlan {
	tier: OldCityMarketTier,
	center: Vec2,
	footprint: Vec2,
	yaw: f32,
}

#[derive(Debug)]
struct KeptCorridor {
	corridor: ConnectivityCorridor,
	levels: Vec<f32>,
	pad: PadPlan,
}

impl Development for OldCityMarket {
	type Plan = Self;

	/// Hysteresis lanes on `cell` joining multi-stall terrace clusters, each
	/// terrace and lane its own pad over `ground`.
	fn plan(ground: &mut impl SiteGround, cell: Aabb3d, seed: u32) -> Option<(Self, Vec<PadPlan>)> {
		let root = SeededHash::new(seed.wrapping_add(cell_salt(cell)));
		lay_out_in(market_bounds(cell, root), root, seed as i32, ground)
	}

	fn build(market: &Self) -> Option<Self> {
		Some(market.clone())
	}
}

fn market_bounds(cell: Aabb3d, root: SeededHash) -> Aabb3d {
	let available = Vec2::new(cell.max.x - cell.min.x, cell.max.z - cell.min.z);
	let max_x = (available.x - 2.0 * PLAN_EDGE_INSET).max(0.0);
	let max_z = (available.y - 2.0 * PLAN_EDGE_INSET).max(0.0);
	let extent = Vec2::new(
		lerp(MIN_MARKET_EXTENT, MAX_MARKET_EXTENT, root.unit(311)).min(max_x),
		lerp(MIN_MARKET_EXTENT, MAX_MARKET_EXTENT, root.unit(313)).min(max_z),
	);
	let center = Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5);
	Aabb3d::from_min_max(
		Vec3::new(center.x - extent.x * 0.5, cell.min.y, center.y - extent.y * 0.5),
		Vec3::new(center.x + extent.x * 0.5, cell.max.y, center.y + extent.y * 0.5),
	)
}

fn lay_out_in(
	bounds: Aabb3d,
	root: SeededHash,
	noise_seed: i32,
	ground: &mut impl SiteGround,
) -> Option<(OldCityMarket, Vec<PadPlan>)> {
	for attempt in 0..3u32 {
		let attempt_root =
			SeededHash::new(root.seed.wrapping_add(attempt.wrapping_mul(0x9E37_79B9)));
		if let Some(market) = try_lay_out(bounds, attempt_root, noise_seed, ground) {
			return Some(market);
		}
	}
	None
}

fn try_lay_out(
	bounds: Aabb3d,
	root: SeededHash,
	noise_seed: i32,
	ground: &mut impl SiteGround,
) -> Option<(OldCityMarket, Vec<PadPlan>)> {
	let plan_bounds = Bounds2::from_xz(bounds.min.x, bounds.min.z, bounds.max.x, bounds.max.z);
	if plan_bounds.extent().min_element() < MIN_MARKET_EXTENT - 1.0 {
		return None;
	}
	let walk = Bounds2 {
		min: plan_bounds.min + Vec2::splat(PLAN_EDGE_INSET),
		max: plan_bounds.max - Vec2::splat(PLAN_EDGE_INSET),
	};
	let head = Vec2::new(
		lerp(walk.min.x, walk.max.x, 0.08 + 0.14 * root.unit(17)),
		lerp(walk.min.y, walk.max.y, 0.18 + 0.16 * root.unit(19)),
	);
	let toe = Vec2::new(
		lerp(walk.min.x, walk.max.x, 0.78 + 0.14 * root.unit(23)),
		lerp(walk.min.y, walk.max.y, 0.66 + 0.18 * root.unit(29)),
	);
	let graph = HysteresisGraph::with_degree(
		3,
		walk,
		root.seed.wrapping_add(31),
		head,
		toe,
		&HysteresisConfig {
			max_segments: 7,
			step_len: 40.0,
			snap_radius: 22.0,
			connect_radius: 34.0,
			..HysteresisConfig::default()
		},
	);
	let conn = ConnectivityGraph::from_hysteresis(&graph)?;
	if conn.keypoints.len() < 3 {
		return None;
	}

	let adjacency = conn.undirected_adjacency();
	let core = (0..conn.keypoints.len()).max_by(|&a, &b| {
		adjacency[a]
			.len()
			.cmp(&adjacency[b].len())
			.then_with(|| {
				let center = plan_bounds.center();
				conn.keypoints[b]
					.distance_squared(center)
					.total_cmp(&conn.keypoints[a].distance_squared(center))
			})
			.then_with(|| b.cmp(&a))
	})?;
	let site_plans: Vec<MarketSitePlan> = conn
		.keypoints
		.iter()
		.enumerate()
		.map(|(index, center)| {
			let tier = if index == core {
				OldCityMarketTier::Dense
			} else if adjacency[index].len() >= 3 {
				OldCityMarketTier::Medium
			} else {
				OldCityMarketTier::Sparse
			};
			MarketSitePlan {
				tier,
				center: *center,
				footprint: terrace_footprint(tier),
				yaw: conn.yaw_at(index),
			}
		})
		.collect();

	let mut order: Vec<usize> = (0..site_plans.len()).collect();
	order.sort_by(|&a, &b| {
		(b == core)
			.cmp(&(a == core))
			.then_with(|| adjacency[b].len().cmp(&adjacency[a].len()))
			.then_with(|| a.cmp(&b))
	});
	let mut retained = vec![false; site_plans.len()];
	let mut occupied_terraces = Vec::<Bounds2>::new();
	for index in order {
		let site = site_plans[index];
		let half = site.footprint * 0.5 + Vec2::splat(SITE_SEPARATION);
		let site_bounds = Bounds2 { min: site.center - half, max: site.center + half };
		if !bounds_contains(plan_bounds, site_bounds)
			|| occupied_terraces
				.iter()
				.copied()
				.any(|occupied| bounds_intersect(occupied, site_bounds))
		{
			continue;
		}
		retained[index] = true;
		occupied_terraces.push(site_bounds);
	}
	if !retained[core] {
		return None;
	}

	let mut natural_height = vec![None; site_plans.len()];
	for (index, site) in site_plans.iter().copied().enumerate() {
		if !retained[index] {
			continue;
		}
		let half = PadParams::market().influence_half(site.footprint * 0.5);
		let coarse = PadPlan::building_skirt(
			site.center,
			site.footprint * 0.5,
			0.0,
			0.0,
			PadParams::market(),
		);
		if ground.hydro_overlaps(&coarse) {
			retained[index] = false;
			continue;
		}
		natural_height[index] = ground.height_upper_on_rect(site.center, half, 0.0);
		if natural_height[index].is_none() {
			retained[index] = false;
		}
	}
	if !retained[core] {
		return None;
	}

	let mut key_height = conn.assign_graded_heights(&natural_height, MAX_PATH_GRADE);
	raise_toward_peak(&mut key_height, MAX_MARKET_RELIEF);
	let mut kept_corridors = Vec::<KeptCorridor>::new();
	for corridor in &conn.corridors {
		if !retained[corridor.from_key] || !retained[corridor.to_key] {
			continue;
		}
		let Some(height_a) = key_height[corridor.from_key] else {
			continue;
		};
		let Some(height_b) = key_height[corridor.to_key] else {
			continue;
		};
		if corridor.arclength() < MIN_PATH_LEN {
			continue;
		}
		let levels = corridor_levels(&corridor.path, height_a, height_b);
		let pad =
			PadPlan::graded_polyline(&corridor.path, &levels, PATH_HALF_WIDTH, PadParams::path());
		if pad.is_empty() || ground.hydro_overlaps(&pad) {
			continue;
		}
		kept_corridors.push(KeptCorridor { corridor: corridor.clone(), levels, pad });
	}
	if kept_corridors
		.iter()
		.filter(|kept| kept.corridor.from_key == core || kept.corridor.to_key == core)
		.count()
		< 2
	{
		return None;
	}
	let connected = core_component(site_plans.len(), core, &kept_corridors);
	kept_corridors
		.retain(|kept| connected[kept.corridor.from_key] && connected[kept.corridor.to_key]);

	let mut pads: Vec<PadPlan> = kept_corridors.iter().map(|kept| kept.pad.clone()).collect();

	let mut remap = vec![usize::MAX; site_plans.len()];
	let mut nodes = Vec::new();
	let mut dense_stall_count = None;
	for index in 0..site_plans.len() {
		if !connected[index] {
			continue;
		}
		let site = site_plans[index];
		let Some(height) = key_height[index] else {
			continue;
		};
		let buildings =
			fit_site_buildings(site, height, root, noise_seed, market_recipe(site.tier));
		let tier_min = market_tier_range(site.tier).0;
		if buildings.len() < tier_min {
			return None;
		}
		if index == core {
			dense_stall_count = Some(buildings.len());
		}
		let terrace_building = OldCityMarketTerrace::new(site.center, site.footprint, height);
		let terrace = PlacedBuilding {
			center_xz: site.center,
			yaw: 0.0,
			footprint: site.footprint,
			ground_height: height,
			building: terrace_building,
		};
		pads.push(PadPlan::building_skirt(
			site.center,
			site.footprint * 0.5,
			0.0,
			height,
			PadParams::market(),
		));
		remap[index] = nodes.len();
		nodes.push(OldCityMarketSite {
			position: site.center,
			elevation: height,
			tier: site.tier,
			terrace,
			buildings,
		});
	}
	if dense_stall_count.is_none_or(|count| count < market_tier_range(OldCityMarketTier::Dense).0) {
		return None;
	}

	let edges: Vec<_> = kept_corridors
		.into_iter()
		.filter_map(|kept| {
			let from = *remap.get(kept.corridor.from_key)?;
			let to = *remap.get(kept.corridor.to_key)?;
			if from == usize::MAX || to == usize::MAX {
				return None;
			}
			Some(DevelopmentEdge::new(
				from,
				to,
				OldCityMarketCorridor {
					path: kept.corridor.path,
					levels: kept.levels,
					half_width: PATH_HALF_WIDTH,
				},
			))
		})
		.collect();
	let market = OldCityMarket::new(bounds, nodes, edges);
	if !market.topology_is_valid()
		|| market.edges.is_empty()
		|| market.nodes.len() < 3
		|| market.stall_count() < MIN_MARKET_STALLS
	{
		return None;
	}
	Some((market, pads))
}

fn fit_site_buildings(
	site: MarketSitePlan,
	height: f32,
	root: SeededHash,
	noise_seed: i32,
	recipe: ScatterRecipe<ShepherdsBuildingKind>,
) -> Vec<ShepherdsVillageBuilding> {
	let half = site.footprint * 0.5;
	let bounds = Aabb2d { min: site.center - half, max: site.center + half };
	let site_seed = root.seed
		^ site.center.x.to_bits().rotate_left(11)
		^ site.center.y.to_bits().rotate_left(23);
	let site_hash = SeededHash::new(site_seed);
	let plan = recipe.plan_in_bounds(bounds, site_hash);
	let terrace_bounds = Bounds2 { min: bounds.min, max: bounds.max };
	let mut occupied = Vec::<Bounds2>::new();
	let mut buildings = Vec::with_capacity(plan.target_count);
	for candidate in plan.candidates {
		if buildings.len() >= plan.target_count {
			break;
		}
		let yaw_hash = SeededHash::new(
			site_hash
				.seed
				.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0x51ED_270B)),
		);
		// Reserve space on a corridor quarter-turn so dense pads stay viable, then
		// crook the authored stall for an organic market look.
		let pack_yaw =
			(site.yaw / std::f32::consts::FRAC_PI_2).round() * std::f32::consts::FRAC_PI_2;
		let visual_yaw = pack_yaw + (yaw_hash.unit(4) - 0.5) * 2.0 * STALL_YAW_JITTER;
		let mut pack_candidate = candidate.clone();
		pack_candidate.yaw = pack_yaw;
		let collision = recipe.collision_bounds(&pack_candidate);
		if !bounds_contains(terrace_bounds, collision)
			|| pack_candidate.center.distance(site.center) < CORE_CLEARANCE
			|| occupied.iter().copied().any(|occupied| bounds_intersect(occupied, collision))
		{
			continue;
		}
		let hash = SeededHash::new(
			site_hash
				.seed
				.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0xA24B_AED5)),
		);
		let noise = NoiseParams {
			seed: noise_seed
				.wrapping_add(candidate.slot as i32 * 97)
				.wrapping_add(site_seed as i32),
			..NoiseParams::default()
		};
		let Some(building) = fit_shepherds_building_for_role(
			candidate.kind,
			candidate.center,
			visual_yaw,
			candidate.footprint,
			height + MARKET_PLATFORM_HEIGHT,
			hash,
			noise,
			DevelopmentFinishRole::OldCityMarket,
		) else {
			continue;
		};
		occupied.push(collision);
		buildings.push(building);
	}
	buildings
}

fn market_recipe(tier: OldCityMarketTier) -> ScatterRecipe<ShepherdsBuildingKind> {
	let (grid_side, min_count, max_count, hut_max, larger_weight) = match tier {
		OldCityMarketTier::Dense => (6, 14, 22, 5.0, 0.04),
		OldCityMarketTier::Medium => (5, 7, 12, 5.5, 0.07),
		OldCityMarketTier::Sparse => (3, 2, 5, 6.5, 0.12),
	};
	ScatterRecipe {
		grid_side,
		min_count,
		max_count,
		cell_inset: 3.0,
		jitter: 0.25,
		clearance: 0.25,
		choices: vec![
			ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: 1.0 - larger_weight,
				min_footprint: 4.0,
				max_footprint: hut_max,
			},
			ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: larger_weight,
				min_footprint: 6.0,
				max_footprint: 8.0,
			},
		],
	}
}

fn market_tier_range(tier: OldCityMarketTier) -> (usize, usize) {
	match tier {
		OldCityMarketTier::Dense => (14, 22),
		OldCityMarketTier::Medium => (7, 12),
		OldCityMarketTier::Sparse => (2, 5),
	}
}

fn terrace_footprint(tier: OldCityMarketTier) -> Vec2 {
	match tier {
		OldCityMarketTier::Dense => Vec2::splat(38.0),
		OldCityMarketTier::Medium => Vec2::splat(32.0),
		OldCityMarketTier::Sparse => Vec2::splat(24.0),
	}
}

fn bounds_contains(outer: Bounds2, inner: Bounds2) -> bool {
	inner.min.x >= outer.min.x
		&& inner.min.y >= outer.min.y
		&& inner.max.x <= outer.max.x
		&& inner.max.y <= outer.max.y
}

fn core_component(node_count: usize, core: usize, corridors: &[KeptCorridor]) -> Vec<bool> {
	let mut connected = vec![false; node_count];
	if core >= node_count {
		return connected;
	}
	connected[core] = true;
	loop {
		let mut changed = false;
		for kept in corridors {
			let from = kept.corridor.from_key;
			let to = kept.corridor.to_key;
			if connected[from] && !connected[to] {
				connected[to] = true;
				changed = true;
			} else if connected[to] && !connected[from] {
				connected[from] = true;
				changed = true;
			}
		}
		if !changed {
			break;
		}
	}
	connected
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ground::tests::FlatGround;
	use crate::scatter::ScatterCandidate;
	use crate::PadForm;

	fn representative_bounds() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(224.0, 40.0, 224.0))
	}

	fn flat_market(seed: u32) -> anyhow::Result<(OldCityMarket, Vec<PadPlan>)> {
		lay_out_in(
			representative_bounds(),
			SeededHash::new(seed),
			seed as i32,
			&mut FlatGround { height: 12.0, wet: false },
		)
		.ok_or_else(|| anyhow::anyhow!("representative market should fit"))
	}

	#[test]
	fn representative_market_has_dense_connected_clusters() -> anyhow::Result<()> {
		let (market, pads) = flat_market(73)?;
		assert!(market.topology_is_valid());
		assert!(market.nodes.len() >= 3);
		assert!(!market.edges.is_empty());
		assert!(market.stall_count() >= MIN_MARKET_STALLS);
		let dense = market
			.nodes
			.iter()
			.enumerate()
			.find(|(_, site)| site.tier == OldCityMarketTier::Dense)
			.ok_or_else(|| anyhow::anyhow!("market should contain a dense core"))?;
		assert!((14..=22).contains(&dense.1.buildings.len()));
		assert!(market.incident_edges(dense.0).count() >= 2);
		assert!(market.nodes.iter().all(|site| {
			let (min, max) = market_tier_range(site.tier);
			(min..=max).contains(&site.buildings.len())
		}));
		assert!(pads.len() >= market.nodes.len() + market.edges.len());
		Ok(())
	}

	#[test]
	fn stalls_stay_on_their_terrace_without_overlap() -> anyhow::Result<()> {
		let (market, _) = flat_market(73)?;
		let mut occupied_market = Vec::new();
		let mut terraces = Vec::new();
		for site in &market.nodes {
			let footprint = terrace_footprint(site.tier);
			let terrace = Bounds2 {
				min: site.position - footprint * 0.5,
				max: site.position + footprint * 0.5,
			};
			assert!(!terraces.iter().copied().any(|other| bounds_intersect(other, terrace)));
			terraces.push(terrace);
			let mut occupied = Vec::new();
			let recipe = market_recipe(site.tier);
			for building in &site.buildings {
				let pack_yaw = (building.yaw / std::f32::consts::FRAC_PI_2).round()
					* std::f32::consts::FRAC_PI_2;
				let candidate = ScatterCandidate {
					slot: 0,
					center: building.center_xz,
					yaw: pack_yaw,
					footprint: building.footprint,
					kind: ShepherdsBuildingKind::Hut,
				};
				let collision = recipe.collision_bounds(&candidate);
				assert!(bounds_contains(terrace, collision));
				assert!(!occupied.iter().copied().any(|other| bounds_intersect(other, collision)));
				assert!(!occupied_market
					.iter()
					.copied()
					.any(|other| bounds_intersect(other, collision)));
				occupied.push(collision);
				occupied_market.push(collision);
			}
		}
		Ok(())
	}

	#[test]
	fn market_stalls_use_varied_yaw() -> anyhow::Result<()> {
		let (market, _) = flat_market(73)?;
		let stall_yaws: Vec<f32> = market.buildings().map(|building| building.yaw).collect();
		anyhow::ensure!(!stall_yaws.is_empty(), "market should place stalls");
		let unique_bins = stall_yaws
			.iter()
			.map(|yaw| (yaw.rem_euclid(std::f32::consts::TAU) / 0.2).floor() as i32)
			.collect::<std::collections::BTreeSet<_>>();
		anyhow::ensure!(
			unique_bins.len() >= 3,
			"expected stall yaw variety, got {} bins from {:?}",
			unique_bins.len(),
			stall_yaws
		);
		let all_quarter_turned = stall_yaws.iter().all(|yaw| {
			let phase = yaw.rem_euclid(std::f32::consts::FRAC_PI_2);
			phase < 1e-3 || (std::f32::consts::FRAC_PI_2 - phase) < 1e-3
		});
		anyhow::ensure!(!all_quarter_turned, "stalls should not all snap to quarter turns");
		Ok(())
	}

	#[test]
	fn market_layout_is_deterministic_and_uses_site_and_corridor_pads() -> anyhow::Result<()> {
		let (first, first_pads) = flat_market(73)?;
		let (second, second_pads) = flat_market(73)?;
		assert_eq!(first, second);
		assert_eq!(first_pads, second_pads);
		let forms = || first_pads.iter().flat_map(|pad| &pad.forms);
		assert!(forms().any(|form| matches!(form, PadForm::Flatten { .. })));
		assert!(forms().any(|form| matches!(form, PadForm::Grade { .. })));
		let bounds = representative_bounds();
		let inside = |p: Vec2| {
			p.x >= bounds.min.x && p.y >= bounds.min.z && p.x <= bounds.max.x && p.y <= bounds.max.z
		};
		assert!(forms().all(|form| match *form {
			PadForm::Flatten { center, .. } => inside(center),
			PadForm::Grade { a, b, .. } => inside(a) && inside(b),
		}));
		Ok(())
	}

	#[test]
	fn hydro_overlap_rejects_market_sites() {
		let market = lay_out_in(
			representative_bounds(),
			SeededHash::new(73),
			73,
			&mut FlatGround { height: 12.0, wet: true },
		);
		assert!(market.is_none());
	}

	#[test]
	fn representative_confines_are_usually_viable() {
		for side in [212.0, 224.0, 236.0] {
			let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(side, 40.0, side));
			let viable = (0..16)
				.filter(|seed| {
					lay_out_in(
						bounds,
						SeededHash::new(*seed),
						*seed as i32,
						&mut FlatGround { height: 12.0, wet: false },
					)
					.is_some()
				})
				.count();
			assert!(viable >= 10, "{side} m confines produced only {viable}/16 viable markets");
		}
	}
}
