//! Hierarchical world POIs over streamed Chico and Richmond scenes.

use std::collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

use bevy::prelude::*;
use chico::ChicoGroveHost;
use lod::LodScene;
use mob_characters::{LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use poi_intelligence::{
	GlobalPoi, LocalPoi, Poi, PoiId, PoiIntelligencePlugin, PoiKind, PoiRegistry, PoiSystems,
};
use richmond::{DiscoverablePlace, DiscoverablePlaceLabel};
use urbanization_layer_model::UrbanSetting;
use vegetation_components::VegetationInstance;

const LOCAL_VEGETATION_TILE: f32 = 160.0;
const VEGETATION_POI_SALT: u64 = 0x7665_6765_7461_7469;
const GROVE_POI_SALT: u64 = 0x6772_6f76_652d_706f;
const URBAN_POI_SALT: u64 = 0x7572_6261_6e2d_706f;
const INTERIOR_POI_SALT: u64 = 0x696e_7465_7269_6f72;

/// Expensive candidate resolutions allowed per frame.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldPoiDiscoveryBudget {
	pub scene_candidates_per_frame: usize,
	pub vegetation_candidates_per_frame: usize,
	pub local_places_per_building: usize,
}

impl Default for WorldPoiDiscoveryBudget {
	fn default() -> Self {
		Self {
			scene_candidates_per_frame: 24,
			vegetation_candidates_per_frame: 6,
			local_places_per_building: 8,
		}
	}
}

#[derive(Clone, Copy, Debug)]
enum PendingPoiTarget {
	Entity(Entity),
	Child { parent: Entity, translation: Vec3, name: &'static str },
}

#[derive(Clone, Copy, Debug)]
enum PendingPoiTier {
	Local,
	Global,
}

#[derive(Clone, Copy, Debug)]
struct PendingPoi {
	target: PendingPoiTarget,
	poi: Poi,
	tier: PendingPoiTier,
}

#[derive(Resource, Default)]
struct WorldPoiDiscoveryState {
	pending_pois: VecDeque<PendingPoi>,
	pending_vegetation: VecDeque<(IVec2, Entity)>,
	pending_vegetation_tiles: HashSet<IVec2>,
	vegetation_tiles: HashMap<IVec2, Entity>,
	vegetation_replacements: HashMap<IVec2, Entity>,
	high_places_by_host: HashMap<Entity, HashSet<Entity>>,
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldPoiSystems {
	Collect,
	Discover,
}

pub struct WorldPoiPlugin;

impl Plugin for WorldPoiPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PoiIntelligencePlugin>() {
			app.add_plugins(PoiIntelligencePlugin);
		}
		app.init_resource::<WorldPoiDiscoveryBudget>()
			.init_resource::<WorldPoiDiscoveryState>()
			.configure_sets(
				Update,
				(WorldPoiSystems::Collect, WorldPoiSystems::Discover)
					.chain()
					.before(PoiSystems::Index),
			)
			.add_systems(
				Update,
				(
					collect_vegetation_instances,
					queue_global_groves,
					queue_global_urban_settings,
					queue_local_places,
				)
					.in_set(WorldPoiSystems::Collect),
			)
			.add_systems(
				Update,
				(promote_pending_pois, discover_local_vegetation)
					.chain()
					.in_set(WorldPoiSystems::Discover),
			);
	}
}

fn collect_vegetation_instances(
	added: Query<(Entity, &VegetationInstance, &GlobalTransform), Added<VegetationInstance>>,
	instances: Query<(), With<VegetationInstance>>,
	mut state: ResMut<WorldPoiDiscoveryState>,
) {
	for (entity, instance, transform) in &added {
		let tile = vegetation_tile(transform.transform_point(instance.anchor));
		if state
			.vegetation_tiles
			.get(&tile)
			.is_some_and(|existing| instances.contains(*existing))
		{
			state.vegetation_replacements.entry(tile).or_insert(entity);
			continue;
		}
		if state.pending_vegetation_tiles.contains(&tile) {
			state.vegetation_replacements.entry(tile).or_insert(entity);
			continue;
		}
		state.vegetation_tiles.remove(&tile);
		state.pending_vegetation_tiles.insert(tile);
		state.pending_vegetation.push_back((tile, entity));
	}
}

/// Grove-host pin. Always-on: proxy UltraLow trees never spawn `VegetationInstance`.
fn queue_global_groves(
	groves: Query<(Entity, &ChicoGroveHost, &GlobalTransform), Added<ChicoGroveHost>>,
	mut state: ResMut<WorldPoiDiscoveryState>,
) {
	for (entity, grove, transform) in &groves {
		let bounds = grove.scene_bounds();
		let anchor = (bounds.min + bounds.max) * 0.5;
		let anchor: Vec3 = anchor.into();
		let world = transform.transform_point(anchor);
		let radius = ((bounds.max.x - bounds.min.x).max(bounds.max.z - bounds.min.z) * 0.5)
			.clamp(1.0, 256.0);
		let id = spatial_poi_id(GROVE_POI_SALT ^ u64::from(grove.layer.id_y().to_bits()), world);
		state.pending_pois.push_back(PendingPoi {
			target: PendingPoiTarget::Child {
				parent: entity,
				translation: anchor,
				name: "grove-global-poi",
			},
			poi: Poi::new(id, VEGETATION_POI).with_arrival_radius(radius).with_salience(1.25),
			tier: PendingPoiTier::Global,
		});
	}
}

fn queue_global_urban_settings(
	settings: Query<(Entity, &UrbanSetting), Added<UrbanSetting>>,
	mut state: ResMut<WorldPoiDiscoveryState>,
) {
	for (entity, setting) in &settings {
		state.pending_pois.push_back(PendingPoi {
			target: PendingPoiTarget::Entity(entity),
			poi: Poi::new(hashed_poi_id(URBAN_POI_SALT, setting.id), URBAN_POI)
				.with_arrival_radius(setting.arrival_radius)
				.with_salience(1.5),
			tier: PendingPoiTier::Global,
		});
	}
}

fn queue_local_places(
	areas: Query<(Entity, &DiscoverablePlace, &GlobalTransform), Added<DiscoverablePlace>>,
	parents: Query<&ChildOf>,
	places: Query<&DiscoverablePlace>,
	entities: Query<Entity>,
	budget: Res<WorldPoiDiscoveryBudget>,
	mut state: ResMut<WorldPoiDiscoveryState>,
) {
	prune_high_places(&mut state, &entities);
	for (entity, place, transform) in &areas {
		if !place.persistent {
			let Some(host) = persistent_host(entity, &parents, &places) else {
				queue_place(&mut state, entity, place, transform.translation());
				continue;
			};
			let admitted = state.high_places_by_host.entry(host).or_default();
			if !admitted.contains(&entity) && admitted.len() >= budget.local_places_per_building {
				continue;
			}
			admitted.insert(entity);
		}
		queue_place(&mut state, entity, place, transform.translation());
	}
}

fn queue_place(
	state: &mut WorldPoiDiscoveryState,
	entity: Entity,
	place: &DiscoverablePlace,
	translation: Vec3,
) {
	let kind = place_kind(place.label);
	let salience = place.salience.unwrap_or(place.label.default_salience());
	state.pending_pois.push_back(PendingPoi {
		target: PendingPoiTarget::Entity(entity),
		poi: Poi::new(
			spatial_poi_id(INTERIOR_POI_SALT ^ place.label.salt() ^ entity.to_bits(), translation),
			kind,
		)
		.with_arrival_radius(place.arrival_radius)
		.with_salience(salience),
		tier: PendingPoiTier::Local,
	});
}

fn place_kind(label: DiscoverablePlaceLabel) -> PoiKind {
	match label {
		DiscoverablePlaceLabel::Stall
		| DiscoverablePlaceLabel::Lounge
		| DiscoverablePlaceLabel::Market => SALOON_POI,
		_ => LOCAL_POI,
	}
}

fn persistent_host(
	entity: Entity,
	parents: &Query<&ChildOf>,
	places: &Query<&DiscoverablePlace>,
) -> Option<Entity> {
	if places.get(entity).is_ok_and(|place| place.persistent) {
		return Some(entity);
	}
	let mut current = entity;
	for _ in 0..8 {
		let parent = parents.get(current).ok()?.parent();
		if places.get(parent).is_ok_and(|place| place.persistent) {
			return Some(parent);
		}
		current = parent;
	}
	None
}

fn prune_high_places(state: &mut WorldPoiDiscoveryState, entities: &Query<Entity>) {
	for admitted in state.high_places_by_host.values_mut() {
		admitted.retain(|entity| entities.contains(*entity));
	}
	state
		.high_places_by_host
		.retain(|host, admitted| entities.contains(*host) && !admitted.is_empty());
}

fn promote_pending_pois(
	mut commands: Commands,
	budget: Res<WorldPoiDiscoveryBudget>,
	mut state: ResMut<WorldPoiDiscoveryState>,
	registry: Res<PoiRegistry>,
	entities: Query<()>,
) {
	let attempts = budget.scene_candidates_per_frame.min(state.pending_pois.len());
	let mut claimed = HashSet::new();
	for _ in 0..attempts {
		let Some(candidate) = state.pending_pois.pop_front() else {
			break;
		};
		let source = match candidate.target {
			PendingPoiTarget::Entity(entity) => entity,
			PendingPoiTarget::Child { parent, .. } => parent,
		};
		if !entities.contains(source) {
			continue;
		}
		if let Some(record) = registry.get(candidate.poi.id) {
			if matches!(candidate.target, PendingPoiTarget::Entity(entity) if entity == record.entity)
			{
				continue;
			}
			state.pending_pois.push_back(candidate);
			continue;
		}
		if !claimed.insert(candidate.poi.id) {
			state.pending_pois.push_back(candidate);
			continue;
		}
		match candidate.target {
			PendingPoiTarget::Entity(entity) => match candidate.tier {
				PendingPoiTier::Local => {
					commands.entity(entity).insert((candidate.poi, LocalPoi));
				}
				PendingPoiTier::Global => {
					commands.entity(entity).insert((candidate.poi, GlobalPoi));
				}
			},
			PendingPoiTarget::Child { parent, translation, name } => match candidate.tier {
				PendingPoiTier::Local => {
					commands.spawn((
						Name::new(name),
						ChildOf(parent),
						Transform::from_translation(translation),
						candidate.poi,
						LocalPoi,
					));
				}
				PendingPoiTier::Global => {
					commands.spawn((
						Name::new(name),
						ChildOf(parent),
						Transform::from_translation(translation),
						candidate.poi,
						GlobalPoi,
					));
				}
			},
		}
	}
}

fn discover_local_vegetation(
	mut commands: Commands,
	budget: Res<WorldPoiDiscoveryBudget>,
	mut state: ResMut<WorldPoiDiscoveryState>,
	instances: Query<(&VegetationInstance, &GlobalTransform)>,
) {
	let stale: Vec<_> = state
		.vegetation_tiles
		.iter()
		.filter_map(|(tile, entity)| (!instances.contains(*entity)).then_some(*tile))
		.collect();
	for tile in stale {
		state.vegetation_tiles.remove(&tile);
		if let Some(replacement) = state.vegetation_replacements.remove(&tile) {
			if instances.contains(replacement) && state.pending_vegetation_tiles.insert(tile) {
				state.pending_vegetation.push_back((tile, replacement));
			}
		}
	}
	for _ in 0..budget.vegetation_candidates_per_frame {
		let Some((tile, entity)) = state.pending_vegetation.pop_front() else {
			break;
		};
		state.pending_vegetation_tiles.remove(&tile);
		let Ok((instance, transform)) = instances.get(entity) else {
			continue;
		};
		let world = transform.transform_point(instance.anchor);
		if state
			.vegetation_tiles
			.get(&tile)
			.is_some_and(|existing| instances.contains(*existing))
		{
			state.vegetation_replacements.entry(tile).or_insert(entity);
			continue;
		}
		state.vegetation_tiles.insert(tile, entity);
		commands.spawn((
			Name::new("vegetation-local-poi"),
			ChildOf(entity),
			Transform::from_translation(instance.anchor),
			Poi::new(spatial_poi_id(VEGETATION_POI_SALT, world), VEGETATION_POI)
				.with_arrival_radius(instance.radius.clamp(0.5, 8.0))
				.with_salience(0.55),
			LocalPoi,
		));
	}
}

fn vegetation_tile(position: Vec3) -> IVec2 {
	(Vec2::new(position.x, position.z) / LOCAL_VEGETATION_TILE).floor().as_ivec2()
}

fn spatial_poi_id(salt: u64, position: Vec3) -> PoiId {
	hashed_poi_id(
		salt,
		(
			(position.x * 100.0).round() as i64,
			(position.y * 100.0).round() as i64,
			(position.z * 100.0).round() as i64,
		),
	)
}

fn hashed_poi_id(value_salt: u64, value: impl Hash) -> PoiId {
	let mut hasher = DefaultHasher::new();
	value_salt.hash(&mut hasher);
	value.hash(&mut hasher);
	PoiId(hasher.finish())
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn vegetation_tiles_are_stable_across_height() {
		assert_eq!(
			vegetation_tile(Vec3::new(49.0, 2.0, -1.0)),
			vegetation_tile(Vec3::new(49.0, 900.0, -1.0))
		);
		assert_ne!(
			vegetation_tile(Vec3::new(0.0, 0.0, 0.0)),
			vegetation_tile(Vec3::new(LOCAL_VEGETATION_TILE, 0.0, 0.0))
		);
	}

	#[test]
	fn semantic_salts_keep_equal_positions_distinct() {
		let at = Vec3::new(10.0, 2.0, 30.0);
		assert_ne!(spatial_poi_id(VEGETATION_POI_SALT, at), spatial_poi_id(GROVE_POI_SALT, at));
		assert_ne!(
			spatial_poi_id(INTERIOR_POI_SALT ^ DiscoverablePlaceLabel::House.salt(), at),
			spatial_poi_id(INTERIOR_POI_SALT ^ DiscoverablePlaceLabel::Lounge.salt(), at)
		);
	}

	#[test]
	fn stacked_storeys_at_the_same_pose_get_distinct_interior_ids() {
		let at = Vec3::new(20.0, 4.0, -8.0);
		let salt = INTERIOR_POI_SALT ^ DiscoverablePlaceLabel::Storey.salt();
		let first = Entity::from_bits(100);
		let second = Entity::from_bits(200);
		assert_ne!(
			spatial_poi_id(salt ^ first.to_bits(), at),
			spatial_poi_id(salt ^ second.to_bits(), at)
		);
	}

	#[test]
	fn lounge_stall_and_market_map_to_saloon() {
		assert_eq!(place_kind(DiscoverablePlaceLabel::Lounge), SALOON_POI);
		assert_eq!(place_kind(DiscoverablePlaceLabel::Stall), SALOON_POI);
		assert_eq!(place_kind(DiscoverablePlaceLabel::Market), SALOON_POI);
		assert_eq!(place_kind(DiscoverablePlaceLabel::House), LOCAL_POI);
		assert_eq!(place_kind(DiscoverablePlaceLabel::Storey), LOCAL_POI);
		assert_eq!(place_kind(DiscoverablePlaceLabel::Bedroom), LOCAL_POI);
	}

	#[test]
	fn high_rooms_are_capped_per_building_host() {
		let mut app = App::new();
		app.init_resource::<WorldPoiDiscoveryBudget>()
			.init_resource::<WorldPoiDiscoveryState>()
			.init_resource::<PoiRegistry>()
			.add_systems(Update, (queue_local_places, promote_pending_pois).chain());
		app.world_mut()
			.resource_mut::<WorldPoiDiscoveryBudget>()
			.local_places_per_building = 2;
		app.world_mut()
			.resource_mut::<WorldPoiDiscoveryBudget>()
			.scene_candidates_per_frame = 16;

		let host = app
			.world_mut()
			.spawn((
				DiscoverablePlace::host(DiscoverablePlaceLabel::House, 8.0, 1.1),
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::from_xyz(0.0, 0.0, 0.0),
			))
			.id();
		let rooms: Vec<_> = (0..4)
			.map(|index| {
				app.world_mut()
					.spawn((
						DiscoverablePlace::high(DiscoverablePlaceLabel::Bedroom, 3.0, 1.0),
						Transform::from_xyz(index as f32 * 4.0, 0.0, 0.0),
						GlobalTransform::from_xyz(index as f32 * 4.0, 0.0, 0.0),
						ChildOf(host),
					))
					.id()
			})
			.collect();

		app.update();

		assert!(app.world().get::<Poi>(host).is_some());
		assert_eq!(app.world().get::<Poi>(host).map(|poi| poi.kind), Some(LOCAL_POI));
		let promoted_rooms =
			rooms.iter().filter(|entity| app.world().get::<Poi>(**entity).is_some()).count();
		assert_eq!(promoted_rooms, 2);
	}

	#[test]
	fn market_host_promotes_a_saloon_place() {
		let mut app = App::new();
		app.init_resource::<WorldPoiDiscoveryBudget>()
			.init_resource::<WorldPoiDiscoveryState>()
			.init_resource::<PoiRegistry>()
			.add_systems(Update, (queue_local_places, promote_pending_pois).chain());
		let market = app
			.world_mut()
			.spawn((
				DiscoverablePlace::host(DiscoverablePlaceLabel::Market, 12.0, 1.25),
				Transform::from_xyz(20.0, 1.0, -8.0),
				GlobalTransform::from_xyz(20.0, 1.0, -8.0),
			))
			.id();

		app.update();

		let poi = app.world().get::<Poi>(market).copied();
		assert_eq!(poi.map(|poi| poi.kind), Some(SALOON_POI));
		assert!(app.world().get::<LocalPoi>(market).is_some());
	}

	#[test]
	fn replacement_waits_for_stable_id_owner_to_leave() -> Result<()> {
		let mut app = App::new();
		app.init_resource::<WorldPoiDiscoveryBudget>()
			.init_resource::<WorldPoiDiscoveryState>()
			.init_resource::<PoiRegistry>()
			.add_systems(Update, promote_pending_pois);
		let old = app.world_mut().spawn_empty().id();
		let replacement = app.world_mut().spawn_empty().id();
		let poi = Poi::new(PoiId(41), VEGETATION_POI);
		app.world_mut()
			.resource_mut::<PoiRegistry>()
			.upsert(old, poi, Vec3::ZERO, false, true)?;
		app.world_mut().resource_mut::<WorldPoiDiscoveryState>().pending_pois.push_back(
			PendingPoi {
				target: PendingPoiTarget::Entity(replacement),
				poi,
				tier: PendingPoiTier::Global,
			},
		);

		app.update();
		assert!(app.world().get::<Poi>(replacement).is_none());

		app.world_mut().resource_mut::<PoiRegistry>().remove_entity(old);
		app.update();
		assert_eq!(app.world().get::<Poi>(replacement), Some(&poi));
		Ok(())
	}

	#[test]
	fn one_drain_claims_a_stable_id_once() {
		let mut app = App::new();
		app.init_resource::<WorldPoiDiscoveryBudget>()
			.init_resource::<WorldPoiDiscoveryState>()
			.init_resource::<PoiRegistry>()
			.add_systems(Update, promote_pending_pois);
		let first = app.world_mut().spawn_empty().id();
		let second = app.world_mut().spawn_empty().id();
		let poi = Poi::new(PoiId(42), URBAN_POI);
		let mut state = app.world_mut().resource_mut::<WorldPoiDiscoveryState>();
		for entity in [first, second] {
			state.pending_pois.push_back(PendingPoi {
				target: PendingPoiTarget::Entity(entity),
				poi,
				tier: PendingPoiTier::Global,
			});
		}
		app.update();

		let promoted = [first, second]
			.into_iter()
			.filter(|entity| app.world().get::<Poi>(*entity).is_some())
			.count();
		assert_eq!(promoted, 1);
	}
}
