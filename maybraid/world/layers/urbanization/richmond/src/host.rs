//! Uniform presentation descriptors for every development building host.

use std::sync::Arc;

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{
	bsn, template_value, Commands, CommandsSceneExt, Entity, Transform, Visibility,
};
use bevy::scene::Scene;
use building_components::{
	building_bounds, building_components_host_for, BuildingComponents, FurnitureNode,
	FurnitureUsageNode,
};
use building_physics::{BuildingWalkShapes, BUILDING_FRICTION};
use buildings::wizards_tower::WizardsTower;
use buildings::{
	ConnectingStairwell, MixedUseLesHallesStorey, PitchedRoof, RectangularPitchedRoofComplex,
};
use lod::gen::Id;
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::LodSceneLevel;
use urbanization_developments::{
	yaw_about_xz, CircularTower, GalleryColonnade, GalleryTerrace, LesHalles,
	MixedUseLesHallesHost, OldCityMarketTerrace, RingFort, RingFortHost, ShepherdsBuilding,
	ShepherdsCommune, ShepherdsHouse, ShepherdsHut, ShepherdsVillage, SingleHighrise, Skybridge,
	TempleSanctum, TrazaloidTower,
};

use crate::place::{DiscoverablePlace, DiscoverablePlaceLabel};
use crate::BuiltDevelopment;

#[derive(Debug, Clone)]
pub enum DevelopmentHost {
	LesHallesStorey(Arc<MixedUseLesHallesStorey>, Transform),
	LesHallesStairwell(Box<ConnectingStairwell>, Transform),
	LesHallesRoof(Box<PitchedRoof>, Transform),
	ShepherdsHouse(Arc<ShepherdsHouse>, Transform),
	ShepherdsHut(Arc<ShepherdsHut>, Transform),
	OldCityMarketTerrace(Arc<OldCityMarketTerrace>, Transform),
	RingFortCircularTower(Arc<CircularTower>, Transform),
	RingFortTrazaloidTower(Arc<TrazaloidTower>, Transform),
	RingFortGalleryTerrace(Box<GalleryTerrace>, Transform),
	RingFortGalleryColonnade(Box<GalleryColonnade>, Transform),
	RingFortGalleryRoof(Box<RectangularPitchedRoofComplex>, Transform),
	SingleHighrise(Arc<buildings::SingleHighrise>, Transform),
	TempleSanctum(Arc<TempleSanctum>, Transform),
	WizardsTower(Arc<WizardsTower>, Transform),
	SkybridgeHall(Arc<Skybridge>, Transform),
}

impl DevelopmentHost {
	/// Always-on place pin for this host. Roofs and circulation are omitted.
	pub fn discoverable_place(&self) -> Option<DiscoverablePlace> {
		Some(match self {
			Self::LesHallesStorey(_, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Storey,
				8.0,
				DiscoverablePlaceLabel::Storey.default_salience(),
			),
			Self::ShepherdsHouse(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::House,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::House.default_salience(),
			),
			Self::ShepherdsHut(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Hut,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::Hut.default_salience(),
			),
			Self::OldCityMarketTerrace(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Market,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::Market.default_salience(),
			),
			Self::RingFortCircularTower(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Tower,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::Tower.default_salience(),
			),
			Self::RingFortTrazaloidTower(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Tower,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::Tower.default_salience(),
			),
			Self::RingFortGalleryTerrace(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Storey,
				arrival_from_building(building.as_ref()),
				DiscoverablePlaceLabel::Storey.default_salience(),
			),
			Self::SingleHighrise(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Highrise,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::Highrise.default_salience(),
			),
			Self::TempleSanctum(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Sanctum,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::Sanctum.default_salience(),
			),
			Self::WizardsTower(building, _) => {
				let bounds = building.scene_bounds();
				DiscoverablePlace::host(
					DiscoverablePlaceLabel::Tower,
					arrival_from_bounds(bounds),
					DiscoverablePlaceLabel::Tower.default_salience(),
				)
			}
			Self::SkybridgeHall(building, _) => DiscoverablePlace::host(
				DiscoverablePlaceLabel::Skybridge,
				arrival_from_building(&**building),
				DiscoverablePlaceLabel::Skybridge.default_salience(),
			),
			Self::LesHallesStairwell(_, _)
			| Self::LesHallesRoof(_, _)
			| Self::RingFortGalleryColonnade(_, _)
			| Self::RingFortGalleryRoof(_, _) => return None,
		})
	}

	/// World transform of the development host (building-local furniture composes under this).
	pub fn transform(&self) -> Transform {
		match self {
			Self::LesHallesStorey(_, transform)
			| Self::LesHallesStairwell(_, transform)
			| Self::LesHallesRoof(_, transform)
			| Self::ShepherdsHouse(_, transform)
			| Self::ShepherdsHut(_, transform)
			| Self::OldCityMarketTerrace(_, transform)
			| Self::RingFortCircularTower(_, transform)
			| Self::RingFortTrazaloidTower(_, transform)
			| Self::RingFortGalleryTerrace(_, transform)
			| Self::RingFortGalleryColonnade(_, transform)
			| Self::RingFortGalleryRoof(_, transform)
			| Self::SingleHighrise(_, transform)
			| Self::TempleSanctum(_, transform)
			| Self::WizardsTower(_, transform)
			| Self::SkybridgeHall(_, transform) => *transform,
		}
	}

	/// Building-local bounds; compose with [`Self::transform`] for world space.
	pub fn local_bounds(&self) -> Aabb3d {
		match self {
			Self::LesHallesStorey(building, _) => building_bounds(building.as_ref()),
			Self::LesHallesStairwell(building, _) => building_bounds(building.as_ref()),
			Self::LesHallesRoof(building, _) => building_bounds(building.as_ref()),
			Self::ShepherdsHouse(building, _) => building_bounds(building.as_ref()),
			Self::ShepherdsHut(building, _) => building_bounds(building.as_ref()),
			Self::OldCityMarketTerrace(building, _) => building_bounds(building.as_ref()),
			Self::RingFortCircularTower(building, _) => building_bounds(building.as_ref()),
			Self::RingFortTrazaloidTower(building, _) => building_bounds(building.as_ref()),
			Self::RingFortGalleryTerrace(building, _) => building_bounds(building.as_ref()),
			Self::RingFortGalleryColonnade(building, _) => building_bounds(building.as_ref()),
			Self::RingFortGalleryRoof(building, _) => building_bounds(building.as_ref()),
			Self::SingleHighrise(building, _) => building_bounds(building.as_ref()),
			Self::TempleSanctum(building, _) => building_bounds(building.as_ref()),
			Self::WizardsTower(building, _) => building.scene_bounds(),
			Self::SkybridgeHall(building, _) => building_bounds(building.as_ref()),
		}
	}

	/// High-LOD furniture slots on this host (Richmond packer IR).
	pub fn furniture_nodes(&self) -> Vec<FurnitureNode> {
		match self {
			Self::LesHallesStorey(building, _) => furniture_of(building.as_ref()),
			Self::LesHallesStairwell(building, _) => furniture_of(building.as_ref()),
			Self::LesHallesRoof(building, _) => furniture_of(building.as_ref()),
			Self::ShepherdsHouse(building, _) => furniture_of(building.as_ref()),
			Self::ShepherdsHut(building, _) => furniture_of(building.as_ref()),
			Self::OldCityMarketTerrace(building, _) => furniture_of(building.as_ref()),
			Self::RingFortCircularTower(building, _) => furniture_of(building.as_ref()),
			Self::RingFortTrazaloidTower(building, _) => furniture_of(building.as_ref()),
			Self::RingFortGalleryTerrace(building, _) => furniture_of(building.as_ref()),
			Self::RingFortGalleryColonnade(building, _) => furniture_of(building.as_ref()),
			Self::RingFortGalleryRoof(building, _) => furniture_of(building.as_ref()),
			Self::SingleHighrise(building, _) => furniture_of(building.as_ref()),
			Self::TempleSanctum(building, _) => furniture_of(building.as_ref()),
			Self::WizardsTower(building, _) => furniture_of(building.as_ref()),
			Self::SkybridgeHall(building, _) => furniture_of(building.as_ref()),
		}
	}

	/// High-LOD usage regions on this host (expanded by furniture-usage-areas).
	pub fn furniture_usage_nodes(&self) -> Vec<FurnitureUsageNode> {
		match self {
			Self::LesHallesStorey(building, _) => furniture_usage_of(building.as_ref()),
			Self::LesHallesStairwell(building, _) => furniture_usage_of(building.as_ref()),
			Self::LesHallesRoof(building, _) => furniture_usage_of(building.as_ref()),
			Self::ShepherdsHouse(building, _) => furniture_usage_of(building.as_ref()),
			Self::ShepherdsHut(building, _) => furniture_usage_of(building.as_ref()),
			Self::OldCityMarketTerrace(building, _) => furniture_usage_of(building.as_ref()),
			Self::RingFortCircularTower(building, _) => furniture_usage_of(building.as_ref()),
			Self::RingFortTrazaloidTower(building, _) => furniture_usage_of(building.as_ref()),
			Self::RingFortGalleryTerrace(building, _) => furniture_usage_of(building.as_ref()),
			Self::RingFortGalleryColonnade(building, _) => furniture_usage_of(building.as_ref()),
			Self::RingFortGalleryRoof(building, _) => furniture_usage_of(building.as_ref()),
			Self::SingleHighrise(building, _) => furniture_usage_of(building.as_ref()),
			Self::TempleSanctum(building, _) => furniture_usage_of(building.as_ref()),
			Self::WizardsTower(building, _) => furniture_usage_of(building.as_ref()),
			Self::SkybridgeHall(building, _) => furniture_usage_of(building.as_ref()),
		}
	}

	pub fn place_local_id(&self) -> u32 {
		match self {
			Self::LesHallesStorey(_, _) => 1,
			Self::LesHallesStairwell(_, _) => 2,
			Self::LesHallesRoof(_, _) => 3,
			Self::ShepherdsHouse(_, _) => 4,
			Self::ShepherdsHut(_, _) => 5,
			Self::OldCityMarketTerrace(_, _) => 6,
			Self::RingFortCircularTower(_, _) => 7,
			Self::RingFortTrazaloidTower(_, _) => 8,
			Self::RingFortGalleryTerrace(_, _) => 9,
			Self::RingFortGalleryColonnade(_, _) => 10,
			Self::RingFortGalleryRoof(_, _) => 11,
			Self::SingleHighrise(_, _) => 12,
			Self::TempleSanctum(_, _) => 13,
			Self::WizardsTower(_, _) => 14,
			Self::SkybridgeHall(_, _) => 15,
		}
	}

	pub fn spawn(
		&self,
		commands: &mut Commands,
		host_id: Option<Id>,
		lod_ref: &LodRef,
	) -> Vec<Entity> {
		vec![commands.spawn_scene(self.scene(host_id, lod_ref)).id()]
	}

	/// The pending LOD host for this building, with its walk colliders and
	/// place pin; it streams its own levels wherever it is spawned.
	///
	/// `lod_ref` is the world-space viewer so the first pending band matches
	/// the host pose, not the world origin.
	pub fn scene(&self, host_id: Option<Id>, lod_ref: &LodRef) -> Box<dyn Scene> {
		let host = match self {
			Self::LesHallesStorey(building, transform) => host(building, *transform, lod_ref),
			Self::LesHallesStairwell(building, transform) => {
				host(building.as_ref(), *transform, lod_ref)
			}
			Self::LesHallesRoof(building, transform) => {
				host(building.as_ref(), *transform, lod_ref)
			}
			Self::ShepherdsHouse(building, transform) => host(building, *transform, lod_ref),
			Self::ShepherdsHut(building, transform) => host(building, *transform, lod_ref),
			Self::OldCityMarketTerrace(building, transform) => host(building, *transform, lod_ref),
			Self::RingFortCircularTower(building, transform) => host(building, *transform, lod_ref),
			Self::RingFortTrazaloidTower(building, transform) => {
				host(building, *transform, lod_ref)
			}
			Self::RingFortGalleryTerrace(building, transform) => {
				host(building.as_ref(), *transform, lod_ref)
			}
			Self::RingFortGalleryColonnade(building, transform) => {
				host(building.as_ref(), *transform, lod_ref)
			}
			Self::RingFortGalleryRoof(building, transform) => {
				host(building.as_ref(), *transform, lod_ref)
			}
			Self::SingleHighrise(building, transform) => host(building, *transform, lod_ref),
			Self::TempleSanctum(building, transform) => host(building, *transform, lod_ref),
			Self::WizardsTower(building, transform) => {
				wizards_tower_host(building, *transform, lod_ref)
			}
			Self::SkybridgeHall(building, transform) => host(building, *transform, lod_ref),
		};
		let Some(mut place) = self.discoverable_place() else {
			return host;
		};
		if let Some(id) = host_id {
			place = place.with_identity(id, self.place_local_id());
		}
		Box::new((host, bsn! { template_value(place) }))
	}
}

fn furniture_of(building: &impl BuildingComponents) -> Vec<FurnitureNode> {
	building.furniture_nodes_for_level(LodSceneLevel::High).flatten()
}

fn furniture_usage_of(building: &impl BuildingComponents) -> Vec<FurnitureUsageNode> {
	building.furniture_usage_nodes_for_level(LodSceneLevel::High).flatten()
}

fn arrival_from_building(building: &impl BuildingComponents) -> f32 {
	arrival_from_bounds(building_bounds(building))
}

fn arrival_from_bounds(bounds: bevy::math::bounding::Aabb3d) -> f32 {
	let xz = (bounds.max.x - bounds.min.x).max(bounds.max.z - bounds.min.z);
	(xz * 0.35).clamp(6.0, 24.0)
}

pub trait DevelopmentHosts {
	fn hosts(&self) -> Vec<DevelopmentHost>;
}

impl DevelopmentHosts for BuiltDevelopment {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		match self {
			Self::LesHalles(development) => development.hosts(),
			Self::ShepherdsVillage(development) => development.hosts(),
			Self::ShepherdsCommune(development) => development.hosts(),
			Self::RingFort(development) => development.hosts(),
			Self::TempleComplex(development) => {
				let mut hosts = shepherd_building_hosts(&development.halls);
				hosts.push(DevelopmentHost::TempleSanctum(
					Arc::new(development.sanctum.building.clone()),
					yaw_about_xz(development.sanctum.center_xz, development.sanctum.yaw),
				));
				hosts
			}
			Self::SingleHighrise(development) => vec![single_highrise_host(development)],
			Self::SuburbanHomes(development) => shepherd_building_hosts(development.buildings()),
			Self::WizardsTower(development) => vec![DevelopmentHost::WizardsTower(
				Arc::new(development.building.tower.clone()),
				yaw_about_xz(development.center_xz, development.yaw),
			)],
			Self::SkybridgeBazaar(development) => {
				let mut hosts = shepherd_building_hosts(&development.market);
				hosts.extend(development.towers.iter().map(single_highrise_host));
				hosts.extend(development.bridges.iter().map(|placed| {
					DevelopmentHost::SkybridgeHall(
						Arc::new(placed.building.clone()),
						yaw_about_xz(placed.center_xz, placed.yaw),
					)
				}));
				hosts
			}
			Self::OldCityMarket(development) => {
				let mut hosts = shepherd_building_hosts(development.buildings());
				hosts.extend(development.terraces().map(|terrace| {
					DevelopmentHost::OldCityMarketTerrace(
						Arc::new(terrace.building.clone()),
						yaw_about_xz(terrace.center_xz, terrace.yaw),
					)
				}));
				hosts
			}
		}
	}
}

impl DevelopmentHosts for LesHalles {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		let transform = yaw_about_xz(self.center_xz, self.yaw);
		self.building
			.hosts()
			.into_iter()
			.map(|host| match host {
				MixedUseLesHallesHost::Storey(storey) => {
					DevelopmentHost::LesHallesStorey(Arc::new(storey), transform)
				}
				MixedUseLesHallesHost::Stairwell(stairwell) => {
					DevelopmentHost::LesHallesStairwell(Box::new(stairwell), transform)
				}
				MixedUseLesHallesHost::Roof(roof) => {
					DevelopmentHost::LesHallesRoof(Box::new(roof), transform)
				}
			})
			.collect()
	}
}

impl DevelopmentHosts for ShepherdsVillage {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		shepherd_building_hosts(&self.buildings)
	}
}

impl DevelopmentHosts for ShepherdsCommune {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		shepherd_building_hosts(self.buildings())
	}
}

impl DevelopmentHosts for RingFort {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		let transform = yaw_about_xz(self.center_xz, self.yaw);
		self.building
			.hosts()
			.into_iter()
			.filter_map(|host| match host {
				RingFortHost::Ring(host) => match *host {
					MixedUseLesHallesHost::Storey(storey) => {
						Some(DevelopmentHost::LesHallesStorey(Arc::new(storey), transform))
					}
					MixedUseLesHallesHost::Stairwell(stairwell) => {
						Some(DevelopmentHost::LesHallesStairwell(Box::new(stairwell), transform))
					}
					MixedUseLesHallesHost::Roof(_) => None,
				},
				RingFortHost::Circular(tower) => {
					Some(DevelopmentHost::RingFortCircularTower(tower, transform))
				}
				RingFortHost::Trazaloid(tower) => {
					Some(DevelopmentHost::RingFortTrazaloidTower(tower, transform))
				}
				RingFortHost::Terrace(terrace) => {
					Some(DevelopmentHost::RingFortGalleryTerrace(Box::new(terrace), transform))
				}
				RingFortHost::TerraceStairwell(stairwell)
				| RingFortHost::KeepStairwell(stairwell) => {
					Some(DevelopmentHost::LesHallesStairwell(Box::new(stairwell), transform))
				}
				RingFortHost::GalleryColonnade(colonnade) => {
					Some(DevelopmentHost::RingFortGalleryColonnade(Box::new(colonnade), transform))
				}
				RingFortHost::GalleryRoof(roof) => {
					Some(DevelopmentHost::RingFortGalleryRoof(Box::new(roof), transform))
				}
			})
			.collect()
	}
}

fn shepherd_building_hosts<'a>(
	buildings: impl IntoIterator<Item = &'a urbanization_developments::ShepherdsVillageBuilding>,
) -> Vec<DevelopmentHost> {
	buildings
		.into_iter()
		.map(|placed| {
			let transform = yaw_about_xz(placed.center_xz, placed.yaw);
			match &placed.building {
				ShepherdsBuilding::House(house) => {
					DevelopmentHost::ShepherdsHouse(house.clone(), transform)
				}
				ShepherdsBuilding::Hut(hut) => {
					DevelopmentHost::ShepherdsHut(hut.clone(), transform)
				}
			}
		})
		.collect()
}

fn single_highrise_host(placed: &SingleHighrise) -> DevelopmentHost {
	DevelopmentHost::SingleHighrise(
		Arc::new(placed.building.clone()),
		yaw_about_xz(placed.center_xz, placed.yaw),
	)
}

fn wizards_tower_host(
	building: &Arc<WizardsTower>,
	transform: Transform,
	lod_ref: &LodRef,
) -> Box<dyn Scene> {
	let bounds = building.scene_bounds();
	let mut tower = building.as_ref().clone();
	tower.host_transform = transform;
	let level = tower.scene_lod_level(lod_ref);
	let host = (
		lod_host_scene_pending(level, bounds),
		bsn! {
			template_value(transform)
			Visibility::default()
			template(move |_ctx| Ok(tower.clone()))
		},
	);
	with_walk_colliders(host, building.as_ref())
}

fn host<T>(building: &T, transform: Transform, lod_ref: &LodRef) -> Box<dyn Scene>
where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	let host =
		building_components_host_for(building, transform, building_bounds(building), Some(lod_ref));
	with_walk_colliders(host, building)
}

fn with_walk_colliders(
	host: impl Scene + 'static,
	building: &impl BuildingComponents,
) -> Box<dyn Scene> {
	match BuildingWalkShapes::of(building, BUILDING_FRICTION) {
		Some(shapes) => Box::new((host, bsn! { template(move |_ctx| Ok(shapes.clone())) })),
		None => Box::new(host),
	}
}

#[cfg(test)]
mod tests {
	use bevy::math::bounding::Aabb3d;
	use bevy::math::{Vec2, Vec3};
	use buildings::{Confines, Fit};
	use procedural_common::NoiseParams;
	use urbanization_developments::{
		Development, OldCityMarket, PadPlan, PlacedBuilding, SiteGround, SkybridgeBazaar,
		SuburbanHomes,
	};

	use super::{DevelopmentHost, DevelopmentHosts};
	use crate::cell::DevelopmentExtent;
	use crate::place::DiscoverablePlaceLabel;
	use crate::BuiltDevelopment;

	struct DryFlat;

	impl SiteGround for DryFlat {
		fn height_at(&mut self, _x: f32, _z: f32) -> Option<f32> {
			Some(12.0)
		}

		fn hydro_overlaps(&mut self, _pad: &PadPlan) -> bool {
			false
		}
	}

	#[test]
	fn market_hosts_every_stall_and_terrace() -> anyhow::Result<()> {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let market = (0..16)
			.find_map(|seed| OldCityMarket::plan(&mut DryFlat, cell, seed))
			.map(|(market, _)| market)
			.ok_or_else(|| anyhow::anyhow!("a market should fit on flat dry ground"))?;
		let hosts = BuiltDevelopment::OldCityMarket(Box::new(market.clone())).hosts();
		assert_eq!(hosts.len(), market.stall_count() + market.nodes.len());
		Ok(())
	}

	#[test]
	fn placed_single_highrise_emits_exactly_one_host() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::new(-30.0, 0.0, -30.0), Vec3::new(30.0, 64.0, 30.0));
		let confines = Confines::from_bounds(cell);
		let (building, _) =
			buildings::SingleHighrise::fit_to_confines(&confines, NoiseParams::default())?;
		let development = BuiltDevelopment::SingleHighrise(Box::new(PlacedBuilding {
			center_xz: Vec2::ZERO,
			yaw: 0.0,
			footprint: Vec2::splat(60.0),
			ground_height: 0.0,
			building,
		}));
		let hosts = development.hosts();
		assert_eq!(hosts.len(), 1);
		assert!(matches!(hosts[0], DevelopmentHost::SingleHighrise(..)));
		let place = hosts[0]
			.discoverable_place()
			.ok_or_else(|| anyhow::anyhow!("highrise should emit a place"))?;
		assert_eq!(place.label, DiscoverablePlaceLabel::Highrise);
		assert!(place.persistent);
		Ok(())
	}

	#[test]
	fn suburban_and_skybridge_hosts_stay_bounded_and_include_new_buildings() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let suburban_confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(45.0, 10.0, 45.0),
			Vec3::new(255.0, 26.0, 255.0),
		));
		let suburban = SuburbanHomes::fit(
			cell,
			&suburban_confines,
			NoiseParams { seed: 29, ..NoiseParams::default() },
		)
		.ok_or_else(|| anyhow::anyhow!("suburban neighborhood did not fit"))?;
		let expected = suburban.buildings().count();
		let hosts = BuiltDevelopment::SuburbanHomes(Box::new(suburban)).hosts();
		assert_eq!(hosts.len(), expected);
		assert!(hosts.len() <= 13);
		assert!(hosts.iter().all(|host| matches!(
			host,
			DevelopmentHost::ShepherdsHouse(..) | DevelopmentHost::ShepherdsHut(..)
		)));
		assert!(hosts.iter().all(|host| {
			host.discoverable_place().is_some_and(|place| {
				place.persistent
					&& matches!(
						place.label,
						DiscoverablePlaceLabel::House | DiscoverablePlaceLabel::Hut
					)
			})
		}));

		let bazaar_confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(50.0, 10.0, 50.0),
			Vec3::new(250.0, 90.0, 250.0),
		));
		let bazaar = SkybridgeBazaar::fit(cell, &bazaar_confines, NoiseParams::default())
			.ok_or_else(|| anyhow::anyhow!("skybridge bazaar did not fit"))?;
		let expected = bazaar.market.len() + bazaar.towers.len() + bazaar.bridges.len();
		let hosts = BuiltDevelopment::SkybridgeBazaar(Box::new(bazaar)).hosts();
		assert_eq!(hosts.len(), expected);
		assert!(hosts.len() <= 21);
		assert_eq!(
			hosts
				.iter()
				.filter(|host| matches!(host, DevelopmentHost::SkybridgeHall(..)))
				.count(),
			2
		);
		assert!(hosts.iter().any(|host| {
			host.discoverable_place()
				.is_some_and(|place| place.label == DiscoverablePlaceLabel::Skybridge)
		}));
		Ok(())
	}

	#[test]
	fn house_and_market_hosts_emit_persistent_places() -> anyhow::Result<()> {
		use bevy::prelude::Transform;
		use urbanization_developments::{OldCityMarketTerrace, ShepherdsHouse};

		let house_confines =
			Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(16.0, 8.0, 16.0)));
		let (house, _) = ShepherdsHouse::fit_to_confines(
			&house_confines,
			NoiseParams { seed: 7, ..NoiseParams::default() },
		)?;
		let house_host =
			DevelopmentHost::ShepherdsHouse(std::sync::Arc::new(house), Transform::IDENTITY);
		let house_place = house_host
			.discoverable_place()
			.ok_or_else(|| anyhow::anyhow!("house should emit a place"))?;
		assert_eq!(house_place.label, DiscoverablePlaceLabel::House);
		assert!(house_place.persistent);

		let terrace = OldCityMarketTerrace::new(Vec2::ZERO, Vec2::splat(24.0), 0.0);
		let market_host = DevelopmentHost::OldCityMarketTerrace(
			std::sync::Arc::new(terrace),
			Transform::IDENTITY,
		);
		let market_place = market_host
			.discoverable_place()
			.ok_or_else(|| anyhow::anyhow!("market terrace should emit a place"))?;
		assert_eq!(market_place.label, DiscoverablePlaceLabel::Market);
		assert!(market_place.persistent);
		Ok(())
	}

	#[test]
	fn les_halles_storey_hosts_fixed_walk_colliders() -> anyhow::Result<()> {
		use bevy::prelude::Transform;
		use urbanization_developments::MixedUseLesHallesDevelopment;

		let bounds =
			Aabb3d::from_min_max(Vec3::new(-18.0, 0.0, -18.0), Vec3::new(18.0, 10.0, 18.0));
		let confines = Confines::from_bounds(bounds);
		let (dev, _) = MixedUseLesHallesDevelopment::fit_to_confines(
			&confines,
			NoiseParams { seed: 1337, ..NoiseParams::default() },
		)?;
		let storey = dev
			.tower
			.floors
			.first()
			.ok_or_else(|| anyhow::anyhow!("expected a Les Halles storey"))?;

		let mut app = bevy::app::App::new();
		app.add_plugins((bevy::asset::AssetPlugin::default(), bevy::scene::ScenePlugin));
		let host = DevelopmentHost::LesHallesStorey(
			std::sync::Arc::new(storey.clone()),
			Transform::IDENTITY,
		);
		let identity = Transform::IDENTITY;
		let bounds = host.local_bounds();
		let lod_ref = lod::lod_ref::LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		let entity = host.spawn(&mut app.world_mut().commands(), None, &lod_ref)[0];
		app.update();

		let n = app
			.world()
			.get::<super::BuildingWalkShapes>(entity)
			.map(|spec| spec.shapes.len())
			.unwrap_or(0);
		assert!(n > 0, "Les Halles storey should queue Fixed walk shapes, got {n}");
		Ok(())
	}
}
