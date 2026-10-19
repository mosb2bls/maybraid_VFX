//! Distant-tile POI selection over an HCSG [`PoiOrigin`] scheme plus retained knowledge.

use std::collections::HashMap;

use bevy::prelude::*;
use lod::hcsg::HcsgStorage;
use meandering_intelligence::MeanderingIntelligenceUser;
use poi_intelligence::{
	begin_poi_goal, choose_poi, DistantTilePoiLookup, KnownPoi, PoiEntry, PoiGoal,
	PoiGoalCompleted, PoiGoalState, PoiIntelligenceUser, PoiKnowledge, PoiSource, PoiSystems,
	PoiVisitPolicy, PoiVisitState,
};

const DEFAULT_TILE_SIZE: f32 = 256.0;
const MAX_TILE_SIZE: f32 = 1_000.0;
const MAX_TILE_DISTANCE: u32 = 32_767;

/// Probes deterministic distant tiles and delegates a selected destination to `PoiGoal`.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct JourneyingIntelligenceUser {
	pub tile_size: f32,
	pub min_tile_distance: u32,
	pub max_tile_distance: u32,
	pub tile_probes: usize,
	pub visit_policy: PoiVisitPolicy,
	pub seed: u64,
	pub selection_interval: f32,
	pub empty_tile_retry_secs: f32,
	pub linger_secs: f32,
	selection_step: u64,
	next_selection_at: f32,
	empty_tiles: HashMap<IVec2, f32>,
}

impl Default for JourneyingIntelligenceUser {
	fn default() -> Self {
		Self {
			tile_size: 256.0,
			min_tile_distance: 2,
			max_tile_distance: 8,
			tile_probes: 8,
			visit_policy: PoiVisitPolicy::default(),
			seed: 0,
			selection_interval: 0.5,
			empty_tile_retry_secs: 30.0,
			linger_secs: 0.0,
			selection_step: 0,
			next_selection_at: 0.0,
			empty_tiles: HashMap::new(),
		}
	}
}

impl JourneyingIntelligenceUser {
	pub fn new(seed: u64) -> Self {
		Self { seed, ..default() }
	}

	fn cycle_is_full(&self, visits: &PoiVisitState) -> bool {
		matches!(
			self.visit_policy,
			PoiVisitPolicy::Cycle { roster_size, .. }
				if roster_size > 0 && visits.cycle_roster().len() >= roster_size
		)
	}

	fn distant_tile(&self, at: Vec3, step: u64) -> IVec2 {
		let origin = tile_of(at, normalized_tile_size(self.tile_size));
		let hash = splitmix64(self.seed.wrapping_add(step));
		let minimum = self.min_tile_distance.clamp(1, MAX_TILE_DISTANCE);
		let maximum = self.max_tile_distance.clamp(minimum, MAX_TILE_DISTANCE);
		let span = u64::from(maximum) - u64::from(minimum) + 1;
		let distance = (u64::from(minimum) + hash % span) as i32;
		origin + square_ring_offset(distance, hash >> 32)
	}

	fn select_from_distant_tile(
		&mut self,
		selection: TileSelection<'_>,
		tile_matches: &mut Vec<PoiEntry>,
	) -> Option<poi_intelligence::PoiId> {
		let TileSelection { at, user_entity, storage, lookup, learner, knowledge, visits, now } =
			selection;
		let empty_tile_retry_secs = self.empty_tile_retry_secs.max(0.0);
		self.empty_tiles
			.retain(|_, checked_at| now - *checked_at < empty_tile_retry_secs);
		let probes = self.tile_probes.max(1);
		let tile_size = normalized_tile_size(self.tile_size);
		for probe in 0..probes {
			let step = self.selection_step.wrapping_add(probe as u64);
			let tile = self.distant_tile(at, step);
			if self.empty_tiles.contains_key(&tile) {
				continue;
			}
			tile_matches.clear();
			if let (Some(storage), Some(lookup)) = (storage, lookup) {
				lookup.0.collect_matching_in_xz_tile(
					storage,
					tile,
					tile_size,
					at.y,
					&learner.interests,
					tile_matches,
				);
			}
			let mut candidates: Vec<_> = knowledge
				.matching(&learner.interests)
				.copied()
				.filter(|known| tile_of(known.position, tile_size) == tile)
				.collect();
			if tile_matches.is_empty() && candidates.is_empty() {
				self.empty_tiles.insert(tile, now);
				continue;
			}
			for entry in tile_matches.iter().copied() {
				if knowledge.get(entry.id).is_none() && !learner.try_take_learning_credit() {
					continue;
				}
				let source = entry.discovery_source();
				if let Some(known) = knowledge.observe(entry.observation(user_entity, source), now)
				{
					if !candidates.iter().any(|candidate| candidate.id == known.id) {
						candidates.push(known);
					}
				}
			}
			for known in knowledge.matching(&learner.interests).copied() {
				if visits.cycle_roster().contains(&known.id)
					&& !candidates.iter().any(|candidate| candidate.id == known.id)
				{
					candidates.push(known);
				}
			}
			if candidates.is_empty() {
				continue;
			}
			self.empty_tiles.remove(&tile);
			let center = tile_center(tile, tile_size);
			let selected = choose_poi(visits, self.visit_policy, &candidates, now, |known| {
				base_score(known, &learner.interests) / (1.0 + known.position.xz().distance(center))
			});
			if selected.is_some() {
				self.selection_step = step.wrapping_add(1);
				return selected;
			}
		}
		self.selection_step = self.selection_step.wrapping_add(probes as u64);
		None
	}
}

type JourneyingSelection<'a> = (
	Entity,
	&'a GlobalTransform,
	&'a mut JourneyingIntelligenceUser,
	&'a mut PoiIntelligenceUser,
	&'a mut PoiKnowledge,
	&'a mut PoiVisitState,
	Option<&'a mut PoiGoalState>,
);
type JourneyingFilter = (Without<PoiGoal>, Without<MeanderingIntelligenceUser>);

struct TileSelection<'a> {
	at: Vec3,
	user_entity: Entity,
	storage: Option<&'a HcsgStorage>,
	lookup: Option<&'a DistantTilePoiLookup>,
	learner: &'a mut PoiIntelligenceUser,
	knowledge: &'a mut PoiKnowledge,
	visits: &'a mut PoiVisitState,
	now: f32,
}

pub struct JourneyingIntelligencePlugin;

impl Plugin for JourneyingIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(record_journeying_completions, select_journeying_goals)
				.chain()
				.in_set(PoiSystems::Select),
		);
	}
}

pub fn record_journeying_completions(
	time: Res<Time>,
	mut completed: MessageReader<PoiGoalCompleted>,
	mut users: Query<
		&mut PoiVisitState,
		(With<JourneyingIntelligenceUser>, Without<MeanderingIntelligenceUser>),
	>,
) {
	let now = time.elapsed_secs();
	for event in completed.read() {
		if let Ok(mut visits) = users.get_mut(event.user) {
			visits.complete(event.target, now);
		}
	}
}

pub fn select_journeying_goals(
	time: Res<Time>,
	storage: Option<Res<HcsgStorage>>,
	lookup: Option<Res<DistantTilePoiLookup>>,
	mut users: Query<JourneyingSelection<'_>, JourneyingFilter>,
	mut commands: Commands,
	mut tile_matches: Local<Vec<PoiEntry>>,
) {
	let now = time.elapsed_secs();
	for (entity, transform, mut journeying, mut learner, mut knowledge, mut visits, mut state) in
		&mut users
	{
		if now < journeying.next_selection_at {
			continue;
		}
		journeying.next_selection_at = now + journeying.selection_interval.max(0.05);
		let all: Vec<_> = knowledge.matching(&learner.interests).copied().collect();
		if let PoiVisitPolicy::Cycle { roster_size, .. } = journeying.visit_policy {
			visits.reconcile_cycle(roster_size, |id| all.iter().any(|known| known.id == id));
		}
		let selected = if journeying.cycle_is_full(&visits) {
			choose_poi(&mut visits, journeying.visit_policy, &all, now, |known| {
				base_score(known, &learner.interests)
			})
		} else {
			journeying.select_from_distant_tile(
				TileSelection {
					at: transform.translation(),
					user_entity: entity,
					storage: storage.as_deref(),
					lookup: lookup.as_deref(),
					learner: &mut learner,
					knowledge: &mut knowledge,
					visits: &mut visits,
					now,
				},
				&mut tile_matches,
			)
		};
		let Some(id) = selected else {
			continue;
		};
		let Some(known) = knowledge.get(id).copied() else {
			continue;
		};
		knowledge.include_source(id, PoiSource::OBJECTIVE);
		begin_poi_goal(
			&mut commands,
			entity,
			known,
			now,
			journeying.linger_secs,
			state.as_deref_mut(),
			0,
		);
	}
}

fn base_score(known: KnownPoi, interests: &poi_intelligence::PoiInterests) -> f32 {
	interests.weight(known.kind).unwrap_or(0.0) * known.salience * known.confidence
}

fn tile_of(position: Vec3, tile_size: f32) -> IVec2 {
	let size = normalized_tile_size(tile_size);
	(position.xz() / size).floor().as_ivec2()
}

fn tile_center(tile: IVec2, tile_size: f32) -> Vec2 {
	(tile.as_vec2() + Vec2::splat(0.5)) * normalized_tile_size(tile_size)
}

fn normalized_tile_size(tile_size: f32) -> f32 {
	if tile_size.is_finite() {
		tile_size.clamp(1.0, MAX_TILE_SIZE)
	} else {
		DEFAULT_TILE_SIZE
	}
}

fn square_ring_offset(distance: i32, index: u64) -> IVec2 {
	let side = distance as u64 * 2;
	let index = index % (side * 4);
	let along = (index % side) as i32;
	match index / side {
		0 => IVec2::new(-distance + along, -distance),
		1 => IVec2::new(distance, -distance + along),
		2 => IVec2::new(distance - along, distance),
		_ => IVec2::new(-distance, distance - along),
	}
}

fn splitmix64(mut value: u64) -> u64 {
	value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
	value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
	use super::*;
	use lod::hcsg::universal_bounds;
	use poi_intelligence::{PoiCatalog, PoiEntry, PoiId, PoiInterests, PoiKind};

	#[test]
	fn distant_tiles_are_outside_the_local_origin() -> anyhow::Result<()> {
		let user =
			JourneyingIntelligenceUser { min_tile_distance: 3, max_tile_distance: 3, ..default() };
		for step in 0..32 {
			let tile = user.distant_tile(Vec3::ZERO, step);
			assert!(tile != IVec2::ZERO);
			assert_eq!(tile.x.abs().max(tile.y.abs()), 3);
		}
		Ok(())
	}

	#[test]
	fn tile_math_floors_negative_coordinates() -> anyhow::Result<()> {
		assert_eq!(tile_of(Vec3::new(-1.0, 0.0, -257.0), 256.0), IVec2::new(-1, -2));
		Ok(())
	}

	#[test]
	fn distant_tile_probe_learns_from_the_scheme() -> anyhow::Result<()> {
		let kind = PoiKind::new("test/destination");
		let mut world = World::new();
		let user_entity = world.spawn_empty().id();
		let mut journeying = JourneyingIntelligenceUser {
			min_tile_distance: 3,
			max_tile_distance: 3,
			tile_probes: 1,
			..default()
		};
		let tile = journeying.distant_tile(Vec3::ZERO, 0);
		let center = tile_center(tile, journeying.tile_size);
		let storage = HcsgStorage::default();
		storage.seed(
			PoiCatalog {
				entries: vec![PoiEntry {
					id: PoiId(7),
					kind,
					position: Vec3::new(center.x, 0.0, center.y),
					arrival_radius: 1.0,
					salience: 1.0,
					local: true,
					global: false,
				}],
			},
			universal_bounds(),
		);
		let lookup = DistantTilePoiLookup::default();
		let mut learner = PoiIntelligenceUser::new(PoiInterests::one(kind));
		let mut knowledge = PoiKnowledge::default();
		let mut visits = PoiVisitState::default();
		let mut tile_matches = Vec::new();
		let selected = journeying.select_from_distant_tile(
			TileSelection {
				at: Vec3::ZERO,
				user_entity,
				storage: Some(&storage),
				lookup: Some(&lookup),
				learner: &mut learner,
				knowledge: &mut knowledge,
				visits: &mut visits,
				now: 0.0,
			},
			&mut tile_matches,
		);
		assert_eq!(selected, Some(PoiId(7)));
		assert!(knowledge.get(PoiId(7)).is_some());
		Ok(())
	}
}
