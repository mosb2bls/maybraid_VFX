use bevy::prelude::*;
use intelligence_lod::{
	due_by_rank, staggered_interval, IntelligenceBand, IntelligenceLod, IntelligencePriority,
};

use crate::{
	GlobalPoi, LocalPoi, Poi, PoiDiscoverLimits, PoiId, PoiIntelligenceUser, PoiKnowledge,
	PoiObservation, PoiRegistry, PoiSource,
};

type PoiEntity<'a> = (Entity, &'a Poi, &'a GlobalTransform, Has<LocalPoi>, Has<GlobalPoi>);
type ChangedPoiEntity =
	Or<(Added<Poi>, Changed<Poi>, Changed<GlobalTransform>, Added<LocalPoi>, Added<GlobalPoi>)>;

/// Mirrors changed entity-backed POIs into the query-optimized registry.
pub fn sync_poi_registry(
	changed: Query<PoiEntity<'_>, ChangedPoiEntity>,
	current: Query<PoiEntity<'_>>,
	mut removed_pois: RemovedComponents<Poi>,
	mut removed_local: RemovedComponents<LocalPoi>,
	mut removed_global: RemovedComponents<GlobalPoi>,
	mut registry: ResMut<PoiRegistry>,
) {
	for (entity, poi, transform, local, global) in &changed {
		upsert(&mut registry, entity, *poi, transform.translation(), local, global);
	}

	let mut removed: Vec<_> = removed_pois
		.read()
		.chain(removed_local.read())
		.chain(removed_global.read())
		.collect();
	removed.sort();
	removed.dedup();
	for entity in removed {
		if let Ok((entity, poi, transform, local, global)) = current.get(entity) {
			upsert(&mut registry, entity, *poi, transform.translation(), local, global);
		} else {
			registry.remove_entity(entity);
		}
	}
}

fn upsert(
	registry: &mut PoiRegistry,
	entity: Entity,
	poi: Poi,
	position: Vec3,
	local: bool,
	global: bool,
) {
	if !position.is_finite() {
		warn!("ignoring POI {entity} with non-finite position {position}");
		registry.remove_entity(entity);
		return;
	}
	if !local && !global {
		registry.remove_entity(entity);
		return;
	}
	if let Err(error) = registry.upsert(entity, poi, position, local, global) {
		debug!("failed to index POI {entity} id={:?} at {position}: {error}", poi.id);
	}
}

/// Accepts directed findings from perception, scripting, sharing, or other systems.
pub fn ingest_poi_observations(
	time: Res<Time>,
	mut observations: MessageReader<PoiObservation>,
	mut users: Query<(&PoiIntelligenceUser, &mut PoiKnowledge)>,
) {
	let now = time.elapsed_secs();
	for observation in observations.read() {
		let Ok((user, mut knowledge)) = users.get_mut(observation.user) else {
			continue;
		};
		if user.interests.contains(observation.kind) {
			knowledge.observe(*observation, now);
		}
	}
}

/// Accrue everyone, forget on a staggered clock, then drain due scans.
///
/// Forget is not every learner every frame — retention is hundreds of seconds.
/// A scan still forgets for free. Far skips local and global scans unless a
/// fairness reserve is due. Global stays Mid / Near only. `skips` reset when a
/// scan body actually ran.
pub fn discover_pois(
	time: Res<Time>,
	registry: Res<PoiRegistry>,
	priority: Res<IntelligencePriority>,
	limits: Res<PoiDiscoverLimits>,
	mut users: Query<(
		Entity,
		&GlobalTransform,
		&mut PoiIntelligenceUser,
		&mut PoiKnowledge,
		Option<&mut IntelligenceLod>,
	)>,
	mut local_candidates: Local<Vec<PoiId>>,
	mut global_candidates: Local<Vec<PoiId>>,
) {
	let now = time.elapsed_secs();
	let delta = time.delta_secs();
	for (entity, _, mut user, mut knowledge, _) in &mut users {
		if user.learning_credit != user.next_learning_credit(delta) {
			user.accrue_learning(delta);
		}
		if now >= user.next_forget_at {
			forget_if_due(entity, now, &mut user, &mut knowledge);
		}
	}

	let mut due: Vec<Entity> = users
		.iter_mut()
		.filter_map(|(entity, _, user, _, _)| {
			(now >= user.next_local_scan_at || now >= user.next_global_scan_at).then_some(entity)
		})
		.collect();
	due_by_rank(&mut due, &priority);
	let fair = due.iter().copied().find(|entity| {
		users.get_mut(*entity).is_ok_and(|(_, _, _, _, lod)| {
			lod.as_deref().is_some_and(|lod| {
				lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
			})
		})
	});

	let mut remaining = limits.max_scans_per_tick;
	for entity in due {
		if remaining == 0 {
			break;
		}
		let Ok((_, transform, mut user, mut knowledge, mut lod)) = users.get_mut(entity) else {
			continue;
		};
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		if band == IntelligenceBand::Far && Some(entity) != fair {
			continue;
		}
		remaining -= 1;
		let position = transform.translation();
		let n = band.scale_count(user.policy.candidates_per_scan);
		knowledge.maintain(now, user.policy);
		user.next_forget_at = now + staggered_interval(FORGET_INTERVAL, entity, 2);
		if now >= user.next_local_scan_at {
			user.next_local_scan_at = now
				+ staggered_interval(user.policy.local_scan_interval, entity, 0)
					* band.interval_scale();
			registry.collect_local_matching(
				position,
				user.policy.local_radius,
				&user.interests,
				&mut local_candidates,
			);
			let cursor = user.local_cursor;
			learn_records(
				entity,
				&local_candidates,
				&registry,
				PoiSource::LOCAL_SCAN,
				now,
				cursor,
				n,
				&mut user,
				&mut knowledge,
			);
			user.local_cursor = user.local_cursor.wrapping_add(n);
		}
		if now >= user.next_global_scan_at && band != IntelligenceBand::Far {
			user.next_global_scan_at = now
				+ staggered_interval(user.policy.global_scan_interval, entity, 1)
					* band.interval_scale();
			registry.collect_global_matching(&user.interests, &mut global_candidates);
			let cursor = user.global_cursor;
			learn_records(
				entity,
				&global_candidates,
				&registry,
				PoiSource::GLOBAL_SCAN,
				now,
				cursor,
				n,
				&mut user,
				&mut knowledge,
			);
			user.global_cursor = user.global_cursor.wrapping_add(n);
		}
		if let Some(lod) = lod.as_deref_mut() {
			if lod.skips != 0 {
				lod.skips = 0;
			}
		}
	}
}

const FORGET_INTERVAL: f32 = 2.0;

fn forget_if_due(
	entity: Entity,
	now: f32,
	user: &mut PoiIntelligenceUser,
	knowledge: &mut PoiKnowledge,
) {
	if now < user.next_forget_at {
		return;
	}
	knowledge.maintain(now, user.policy);
	user.next_forget_at = now + staggered_interval(FORGET_INTERVAL, entity, 2);
}

fn learn_records(
	user_entity: Entity,
	ids: &[PoiId],
	registry: &PoiRegistry,
	source: PoiSource,
	now: f32,
	cursor: usize,
	budget: usize,
	user: &mut PoiIntelligenceUser,
	knowledge: &mut PoiKnowledge,
) {
	if ids.is_empty() {
		return;
	}
	let budget = budget.min(ids.len());
	for offset in 0..budget {
		let record = registry
			.get(ids[(cursor + offset) % ids.len()])
			.expect("collect_*_matching ids are indexed");
		if knowledge.get(record.id).is_none() && !user.try_take_learning_credit() {
			continue;
		}
		knowledge.observe(record.observation(user_entity, source), now);
	}
}
