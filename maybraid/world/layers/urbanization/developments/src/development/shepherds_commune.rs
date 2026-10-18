//! [`ShepherdsCommune`]: shepherds' buildings joined by graded lanes.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec2;
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, NoiseParams, SeededHash};

use super::raise_toward_peak;
use super::shepherds_village::fit::{
	fit_shepherds_building, sample_shepherds_footprint, sample_shepherds_kind, shepherds_recipe,
	ShepherdsBuildingKind,
};
use super::shepherds_village::ShepherdsVillageBuilding;
use crate::connectivity::{corridor_levels, ConnectivityGraph};
use crate::math::lerp;
use crate::plan::cell_salt;
use crate::scatter::{bounds_intersect, ScatterCandidate};
use crate::{ConnectedDevelopment, Development, DevelopmentEdge, PadParams, PadPlan, SiteGround};

/// Graded path connecting two commune pads.
#[derive(Debug, Clone, PartialEq)]
pub struct ShepherdsCommuneCorridor {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
}

/// One site in a Shepherds Commune connectivity graph.
#[derive(Debug, Clone, PartialEq)]
pub struct ShepherdsCommuneSite {
	pub position: Vec2,
	pub elevation: Option<f32>,
	pub building: Option<ShepherdsVillageBuilding>,
}

/// Shepherds Village laid out as a reusable connected development.
pub type ShepherdsCommune = ConnectedDevelopment<ShepherdsCommuneSite, ShepherdsCommuneCorridor>;

impl ShepherdsCommune {
	pub fn buildings(&self) -> impl Iterator<Item = &ShepherdsVillageBuilding> {
		self.nodes.iter().filter_map(|site| site.building.as_ref())
	}

	pub fn corridors(&self) -> impl Iterator<Item = &ShepherdsCommuneCorridor> {
		self.edges.iter().map(|edge| &edge.payload)
	}
}

const CELL_INSET: f32 = 32.0;
/// Capsule half-width before berm. With berm 2 this is a ~20 m flatten so a
/// `res_2=5` origin cell (~5 m pitch) actually samples the corridor.
const PATH_HALF_WIDTH: f32 = 8.0;
const MIN_PATH_LEN: f32 = 16.0;
const MIN_BUILDINGS: usize = 2;
/// Maximum tree-edge slope (rise/run) when BFS-assigning pad heights from the peak.
const MAX_PATH_GRADE: f32 = 0.15;
/// Keep one compact commune reasonably close to its highest site.
const MAX_COMMUNE_RELIEF: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
struct CommuneSite {
	hash: SeededHash,
	kind: ShepherdsBuildingKind,
	footprint: Vec2,
	yaw: f32,
}

impl Development for ShepherdsCommune {
	type Plan = Self;

	/// A hysteresis connectivity graph on `cell`, then graded lane pads over
	/// `ground`, then one building per connected keypoint.
	fn plan(ground: &mut impl SiteGround, cell: Aabb3d, seed: u32) -> Option<(Self, Vec<PadPlan>)> {
		let root = SeededHash::new(seed.wrapping_add(cell_salt(cell)));
		let walk = Bounds2::from_xz(
			cell.min.x + CELL_INSET,
			cell.min.z + CELL_INSET,
			cell.max.x - CELL_INSET,
			cell.max.z - CELL_INSET,
		);
		if walk.min.x >= walk.max.x || walk.min.y >= walk.max.y {
			return None;
		}

		let head = sample_endpoint(root, 11, walk);
		let toe = sample_endpoint(root, 17, walk);
		if head.distance(toe) < MIN_PATH_LEN {
			return None;
		}
		let degree = if root.unit(23) < 0.5 { 2 } else { 3 };
		let graph = HysteresisGraph::with_degree(
			degree,
			walk,
			root.seed.wrapping_add(29),
			head,
			toe,
			&HysteresisConfig {
				max_segments: 12,
				step_len: 18.0,
				snap_radius: 16.0,
				connect_radius: 28.0,
				..HysteresisConfig::default()
			},
		);
		let conn = ConnectivityGraph::from_hysteresis(&graph)?;

		// Resolve the actual site plans before elevation assignment so the terrain
		// sample covers each building's complete flatten + ease influence.
		let sites: Vec<CommuneSite> = conn
			.keypoints
			.iter()
			.enumerate()
			.map(|(i, _)| {
				let hash = SeededHash::new(
					root.seed.wrapping_add((i as u32 + 1).wrapping_mul(0x9E37_79B9)),
				);
				let kind = sample_shepherds_kind(hash);
				CommuneSite {
					hash,
					kind,
					footprint: sample_shepherds_footprint(hash, kind),
					yaw: conn.yaw_at(i),
				}
			})
			.collect();
		let mut natural_height = vec![None; conn.keypoints.len()];
		for (i, (p, site)) in conn.keypoints.iter().zip(&sites).enumerate() {
			natural_height[i] = ground.height_upper_on_rect(
				*p,
				PadParams::shepherds().influence_half(site.footprint * 0.5),
				site.yaw,
			);
		}
		let mut key_height = conn.assign_graded_heights(&natural_height, MAX_PATH_GRADE);
		raise_toward_peak(&mut key_height, MAX_COMMUNE_RELIEF);
		let ConnectivityGraph { keypoints, corridors } = conn;

		let recipe = shepherds_recipe();
		let mut pads = Vec::new();
		let mut kept_corridors = Vec::new();
		let mut connected = vec![false; keypoints.len()];
		for corridor in corridors {
			let Some(ha) = key_height[corridor.from_key] else {
				continue;
			};
			let Some(hb) = key_height[corridor.to_key] else {
				continue;
			};
			let path_len = corridor.arclength();
			if path_len < MIN_PATH_LEN {
				continue;
			}
			let levels = corridor_levels(&corridor.path, ha, hb);
			let pad = PadPlan::graded_polyline(
				&corridor.path,
				&levels,
				PATH_HALF_WIDTH,
				PadParams::path(),
			);
			if ground.hydro_overlaps(&pad) {
				continue;
			}
			pads.push(pad);
			kept_corridors.push(DevelopmentEdge::new(
				corridor.from_key,
				corridor.to_key,
				ShepherdsCommuneCorridor { path: corridor.path, levels },
			));
			connected[corridor.from_key] = true;
			connected[corridor.to_key] = true;
		}
		if kept_corridors.is_empty() {
			return None;
		}

		let mut buildings: Vec<Option<ShepherdsVillageBuilding>> =
			(0..keypoints.len()).map(|_| None).collect();
		let mut building_count = 0;
		let mut occupied = Vec::<Bounds2>::new();
		for (i, center) in keypoints.iter().copied().enumerate() {
			let Some(height) = key_height[i] else {
				continue;
			};
			if !connected[i] {
				continue;
			}
			let CommuneSite { hash, kind, footprint, yaw } = sites[i];
			let candidate = ScatterCandidate { slot: i, center, yaw, footprint, kind };
			let occupied_bounds = recipe.collision_bounds(&candidate);
			if occupied.iter().any(|b| bounds_intersect(*b, occupied_bounds)) {
				continue;
			}
			let coarse = PadPlan::building_skirt(
				center,
				footprint * 0.5,
				yaw,
				height,
				PadParams::shepherds(),
			);
			if ground.hydro_overlaps(&coarse) {
				continue;
			}
			let noise =
				NoiseParams { seed: seed as i32 ^ (i as i32 * 7919), ..NoiseParams::default() };
			let Some(placed) =
				fit_shepherds_building(kind, center, yaw, footprint, height, hash, noise)
			else {
				continue;
			};
			let pad = placed.pad_plan(PadParams::shepherds());
			if ground.hydro_overlaps(&pad) {
				continue;
			}
			buildings[i] = Some(placed);
			building_count += 1;
			pads.push(pad);
			occupied.push(occupied_bounds);
		}

		if building_count < MIN_BUILDINGS {
			return None;
		}
		let nodes = keypoints
			.into_iter()
			.zip(key_height)
			.zip(buildings)
			.map(|((position, elevation), building)| ShepherdsCommuneSite {
				position,
				elevation,
				building,
			})
			.collect();
		Some((Self::new(cell, nodes, kept_corridors), pads))
	}

	fn build(commune: &Self) -> Option<Self> {
		Some(commune.clone())
	}
}

fn sample_endpoint(hash: SeededHash, salt: u32, bounds: Bounds2) -> Vec2 {
	Vec2::new(
		lerp(bounds.min.x, bounds.max.x, hash.unit(salt)),
		lerp(bounds.min.y, bounds.max.y, hash.unit(salt.wrapping_add(1))),
	)
}

#[cfg(test)]
mod tests {
	use bevy_math::Vec3;

	use super::*;
	use crate::development::shepherds_village::HOUSE_MAX_FOOTPRINT;

	#[test]
	fn hysteresis_walk_stays_inside_the_cell_inset() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(200.0, 1.0, 200.0));
		let walk = Bounds2::from_xz(
			cell.min.x + CELL_INSET,
			cell.min.z + CELL_INSET,
			cell.max.x - CELL_INSET,
			cell.max.z - CELL_INSET,
		);
		let graph = HysteresisGraph::with_degree(
			2,
			walk,
			7,
			Vec2::new(40.0, 40.0),
			Vec2::new(160.0, 160.0),
			&HysteresisConfig::default(),
		);
		for p in &graph.nodes {
			anyhow::ensure!((CELL_INSET..=200.0 - CELL_INSET).contains(&p.x));
			anyhow::ensure!((CELL_INSET..=200.0 - CELL_INSET).contains(&p.y));
		}
		Ok(())
	}

	#[test]
	fn connecting_grade_is_narrower_than_a_house() -> anyhow::Result<()> {
		let flatten_half = PATH_HALF_WIDTH + PadParams::path().berm;
		anyhow::ensure!(
			flatten_half < HOUSE_MAX_FOOTPRINT * 0.5,
			"path flatten should not be as wide as a house"
		);
		anyhow::ensure!(
			flatten_half >= 8.0,
			"path core should span more than one ~5 m terrain sample"
		);
		Ok(())
	}
}
