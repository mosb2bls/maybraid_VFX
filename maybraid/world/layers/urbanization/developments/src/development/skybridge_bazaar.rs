//! [`SkybridgeBazaar`]: towers joined by skybridges on one terrace.

mod skybridge;

pub use skybridge::Skybridge;

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use buildings::{
	CardinalFace, Confines, ConnectingHall, Fit, MappedOpening, MappedOpeningQuad, SingleHighrise,
};
use material_ref::MaterialRef;
use procedural_common::{Bounds2, NoiseParams, SeededHash};

use super::ring_fort::RING_FORT_MAX_FOOTPRINT;
use super::root_hash;
use super::shepherds_village::fit::{fit_shepherds_building, ShepherdsBuildingKind};
use super::shepherds_village::ShepherdsVillageBuilding;
use crate::plan::yaw_about_xz;
use crate::scatter::{bounds_intersect, ScatterChoice, ScatterRecipe};
use crate::{
	DevelopmentFinish, DevelopmentFinishRole, PlacedBuilding, Terrace, TerraceDevelopment,
	TerraceEnvelope,
};

/// Towers, occupied skybridges, and a market at ground level.
#[derive(Debug, Clone, PartialEq)]
pub struct SkybridgeBazaar {
	pub bounds: Aabb3d,
	pub towers: Vec<PlacedBuilding<SingleHighrise>>,
	pub bridges: Vec<PlacedBuilding<Skybridge>>,
	pub market: Vec<ShepherdsVillageBuilding>,
}

impl TerraceDevelopment for SkybridgeBazaar {
	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: 160.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(220.0),
			min_height: 64.0,
			max_height: 96.0,
			rotates: false,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::Connector, false)
	}

	fn fit_terrace(terrace: &Terrace) -> Option<Self> {
		Some(
			Self::fit(terrace.cell, &terrace.confines(), terrace.noise())?
				.with_bridge_material(terrace.finish.wall.clone()),
		)
	}
}

impl SkybridgeBazaar {
	pub fn with_tower_material(mut self, wall: MaterialRef) -> Self {
		for tower in &mut self.towers {
			tower.building = tower.building.clone().with_wall_material(wall.clone());
		}
		self
	}

	pub fn with_bridge_material(mut self, material: MaterialRef) -> Self {
		for bridge in &mut self.bridges {
			bridge.building = bridge.building.clone().with_material(material.clone());
		}
		self
	}

	/// Three towers joined by skybridges over a hut market, inside `confines`
	/// on `cell`.
	pub fn fit(cell: Aabb3d, confines: &Confines, noise: NoiseParams) -> Option<Self> {
		let center = confines.center_xz();
		let y = confines.bounds.min.y;
		let extent = confines.footprint();
		let spacing = (extent.x * 0.28).clamp(42.0, 60.0);
		let tower_foot = Vec2::splat((spacing * 0.6).clamp(30.0, 37.5));
		let root = root_hash(cell, noise);
		let tower_material =
			DevelopmentFinish::pick_for_role(root, DevelopmentFinishRole::Highrise, false).wall;
		let bridge_material =
			DevelopmentFinish::pick_for_role(root, DevelopmentFinishRole::Connector, false).wall;
		let mut towers = Vec::new();
		for (index, x) in [-spacing, 0.0, spacing].into_iter().enumerate() {
			let tower_center =
				center + Vec2::new(x, if index == 1 { extent.y * 0.12 } else { 0.0 });
			let bounds = Aabb3d::from_min_max(
				Vec3::new(
					tower_center.x - tower_foot.x * 0.5,
					y,
					tower_center.y - tower_foot.y * 0.5,
				),
				Vec3::new(
					tower_center.x + tower_foot.x * 0.5,
					confines.bounds.max.y,
					tower_center.y + tower_foot.y * 0.5,
				),
			);
			let (building, _) =
				SingleHighrise::fit_to_confines(&Confines::from_bounds(bounds), noise).ok()?;
			towers.push(PlacedBuilding {
				center_xz: tower_center,
				yaw: 0.0,
				footprint: tower_foot,
				ground_height: y,
				building: building.with_wall_material(tower_material.clone()),
			});
		}

		let mut bridges = Vec::new();
		for (pair_index, pair) in towers.windows(2).enumerate() {
			let a = &pair[0];
			let b = &pair[1];
			let common_storeys = a.building.storey_count().min(b.building.storey_count());
			let bridge_storey = bridge_storey(root, pair_index, common_storeys)?;
			let direction_a = facing_cardinal(a, b.center_xz - a.center_xz);
			let direction_b = facing_cardinal(b, a.center_xz - b.center_xz);
			let end_a =
				mapped_bridge_endpoint(a, bridge_storey, direction_a)?.widened(0.6).raised(0.75);
			let end_b =
				mapped_bridge_endpoint(b, bridge_storey, direction_b)?.widened(0.6).raised(0.75);
			let hall = ConnectingHall::rough_stone(end_a, end_b);
			let bridge = Skybridge::new(hall, bridge_storey, [direction_a, direction_b])
				.with_material(bridge_material.clone());
			let bounds = bridge.bounds;
			let bridge_center =
				Vec2::new((bounds.min.x + bounds.max.x) * 0.5, (bounds.min.z + bounds.max.z) * 0.5);
			bridges.push(PlacedBuilding {
				center_xz: bridge_center,
				yaw: 0.0,
				footprint: Vec2::new(bounds.max.x - bounds.min.x, bounds.max.z - bounds.min.z),
				ground_height: bounds.min.y,
				building: bridge,
			});
		}

		let recipe = ScatterRecipe {
			grid_side: 5,
			min_count: 10,
			max_count: 16,
			cell_inset: (cell.max.x - confines.bounds.max.x).abs() + 24.0,
			jitter: 7.0,
			clearance: 1.5,
			choices: vec![ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: 1.0,
				min_footprint: 5.0,
				max_footprint: 8.0,
			}],
		};
		let market = scatter_shepherds(cell, y, noise, &recipe)
			.into_iter()
			.filter(|building| {
				towers
					.iter()
					.all(|tower| building.center_xz.distance(tower.center_xz) > tower_foot.x)
			})
			.collect();

		Some(Self { bounds: confines.bounds, towers, bridges, market })
	}
}

fn bridge_storey(root: SeededHash, pair_index: usize, common_storeys: usize) -> Option<usize> {
	let first = 2;
	let count = common_storeys.checked_sub(4)?;
	let offset = (root.unit(701 + pair_index as u32) * count as f32).floor() as usize;
	Some(first + offset.min(count.saturating_sub(1)))
}

fn facing_cardinal(tower: &PlacedBuilding<SingleHighrise>, world_direction: Vec2) -> CardinalFace {
	let local = rotate_xz(world_direction, -tower.yaw);
	if local.x.abs() >= local.y.abs() {
		if local.x >= 0.0 {
			CardinalFace::East
		} else {
			CardinalFace::West
		}
	} else if local.y >= 0.0 {
		CardinalFace::North
	} else {
		CardinalFace::South
	}
}

fn mapped_bridge_endpoint(
	tower: &PlacedBuilding<SingleHighrise>,
	storey: usize,
	direction: CardinalFace,
) -> Option<MappedOpening> {
	let mapped = *tower.building.mapped_bridge_passage(storey, direction)?;
	let transform = yaw_about_xz(tower.center_xz, tower.yaw);
	let (bl, br, tl, tr) = mapped.endpoint_corners();
	Some(MappedOpening::new(
		MappedOpeningQuad::new(
			transform.transform_point(bl),
			transform.transform_point(br),
			transform.transform_point(tl),
			transform.transform_point(tr),
		),
		rotate_xz(mapped.orientation, tower.yaw),
	))
}

fn rotate_xz(vector: Vec2, yaw: f32) -> Vec2 {
	let (sin, cos) = yaw.sin_cos();
	Vec2::new(cos * vector.x + sin * vector.y, -sin * vector.x + cos * vector.y)
}

fn scatter_shepherds(
	cell: Aabb3d,
	height: f32,
	noise: NoiseParams,
	recipe: &ScatterRecipe<ShepherdsBuildingKind>,
) -> Vec<ShepherdsVillageBuilding> {
	let root = root_hash(cell, noise);
	let plan = recipe.plan(cell, root);
	let mut occupied: Vec<Bounds2> = Vec::new();
	let mut buildings = Vec::new();
	for candidate in plan.candidates {
		if buildings.len() >= plan.target_count {
			break;
		}
		let collision = recipe.collision_bounds(&candidate);
		if occupied.iter().copied().any(|bounds| bounds_intersect(bounds, collision)) {
			continue;
		}
		let hash = SeededHash::new(
			root.seed.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0xA24B_AED5)),
		);
		let mut local_noise = noise;
		local_noise.seed = noise.seed.wrapping_add(candidate.slot as i32 * 97);
		if let Some(building) = fit_shepherds_building(
			candidate.kind,
			candidate.center,
			candidate.yaw,
			candidate.footprint,
			height,
			hash,
			local_noise,
		) {
			occupied.push(collision);
			buildings.push(building);
		}
	}
	buildings
}

#[cfg(test)]
mod tests {
	use building_components::BuildingComponents;
	use lod::scene::LodSceneLevel;

	use super::*;

	fn cell() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0))
	}

	fn confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(50.0, 10.0, 50.0),
			Vec3::new(250.0, 90.0, 250.0),
		))
	}

	#[test]
	fn bazaar_bridges_join_adjacent_towers_at_mapped_passages() -> anyhow::Result<()> {
		let bazaar = SkybridgeBazaar::fit(cell(), &confines(), NoiseParams::default())
			.ok_or_else(|| anyhow::anyhow!("bazaar did not fit"))?;
		assert_eq!(bazaar.towers.len(), 3);
		assert_eq!(bazaar.bridges.len(), 2);
		assert!(!bazaar.market.is_empty());
		for (index, bridge) in bazaar.bridges.iter().enumerate() {
			let pair = &bazaar.towers[index..=index + 1];
			assert!(bridge.building.storey < pair[0].building.storey_count());
			assert!(bridge.building.storey < pair[1].building.storey_count());
			assert_eq!(bridge.building.directions, [CardinalFace::East, CardinalFace::West]);
			let expected_a =
				mapped_bridge_endpoint(&pair[0], bridge.building.storey, CardinalFace::East)
					.ok_or_else(|| anyhow::anyhow!("left tower endpoint was not mapped"))?
					.widened(0.6)
					.raised(0.75);
			let expected_b =
				mapped_bridge_endpoint(&pair[1], bridge.building.storey, CardinalFace::West)
					.ok_or_else(|| anyhow::anyhow!("right tower endpoint was not mapped"))?
					.widened(0.6)
					.raised(0.75);
			assert_eq!(bridge.building.endpoints(), (expected_a, expected_b));
			assert!(!bridge.building.panel_nodes_for_level(LodSceneLevel::High).is_empty());
			assert!(bridge.building.floor_nodes_for_level(LodSceneLevel::High).is_empty());
		}
		Ok(())
	}

	#[test]
	fn skybridge_storeys_endpoints_and_materials_are_deterministic() -> anyhow::Result<()> {
		let noise = NoiseParams { seed: 83, ..NoiseParams::default() };
		let first = SkybridgeBazaar::fit(cell(), &confines(), noise)
			.ok_or_else(|| anyhow::anyhow!("first bazaar did not fit"))?;
		let second = SkybridgeBazaar::fit(cell(), &confines(), noise)
			.ok_or_else(|| anyhow::anyhow!("second bazaar did not fit"))?;
		assert_eq!(first.bridges, second.bridges);
		for bridge in &first.bridges {
			let material = bridge
				.building
				.material()
				.ok_or_else(|| anyhow::anyhow!("bridge material was not stamped"))?;
			let panels = bridge.building.panel_nodes_for_level(LodSceneLevel::High).flatten();
			assert!(!panels.is_empty());
			assert!(panels.iter().all(|panel| panel.material.as_ref() == Some(material)));
		}
		Ok(())
	}
}
