use std::collections::HashSet;

use bevy::prelude::*;
use intelligence_lod::{
	staggered_interval, IntelligenceBand, IntelligenceDueEntities, IntelligenceLod,
	IntelligencePriority,
};
use spotting_intelligence::{SpottingHintSource, SpottingUser};

use crate::{
	Affiliations, ThreatDiscoverLimits, ThreatId, ThreatIntelligenceUser, ThreatKnowledge,
	ThreatObservation, ThreatRegistry, ThreatSource, ThreatSubject,
};

type ThreatEntity<'a> = (
	Entity,
	Ref<'a, ThreatSubject>,
	Ref<'a, Affiliations>,
	&'a GlobalTransform,
	Option<&'a IntelligenceLod>,
);
type ChangedThreatEntity = Or<(
	Added<ThreatSubject>,
	Changed<ThreatSubject>,
	Changed<Affiliations>,
	Changed<GlobalTransform>,
)>;
type ThreatRecipients<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static ThreatSubject,
		&'static GlobalTransform,
		Ref<'static, Affiliations>,
		&'static mut ThreatIntelligenceUser,
		&'static mut ThreatKnowledge,
		Option<&'static mut IntelligenceLod>,
	),
>;

/// Mirrors live semantic threat subjects into the local Gimme index.
///
/// Identity changes always upsert. Translation-only updates are quantized by
/// [`IntelligenceLod`] (missing = Near). Walking Mid/Far plants do not rebuild
/// the index every frame.
pub fn sync_threat_registry(
	changed: Query<ThreatEntity<'_>, ChangedThreatEntity>,
	current: Query<ThreatEntity<'_>>,
	mut removed_subjects: RemovedComponents<ThreatSubject>,
	mut removed_affiliations: RemovedComponents<Affiliations>,
	mut registry: ResMut<ThreatRegistry>,
) {
	for (entity, subject, affiliations, transform, lod) in &changed {
		let identity_changed = subject.is_changed() || affiliations.is_changed();
		let position = transform.translation();
		if !identity_changed {
			let band = IntelligenceLod::band_or_near(lod);
			if let Some(existing) = registry.get_entity(entity) {
				if position.distance(existing.position) < move_quantum(band) {
					continue;
				}
			}
		}
		upsert(&mut registry, entity, *subject, &affiliations, position);
	}
	let mut removed: Vec<_> = removed_subjects.read().chain(removed_affiliations.read()).collect();
	removed.sort();
	removed.dedup();
	for entity in removed {
		if let Ok((entity, subject, affiliations, transform, _)) = current.get(entity) {
			upsert(&mut registry, entity, *subject, &affiliations, transform.translation());
		} else {
			registry.remove_entity(entity);
		}
	}
}

fn move_quantum(band: IntelligenceBand) -> f32 {
	match band {
		IntelligenceBand::Near => 1.0,
		IntelligenceBand::Mid => 4.0,
		IntelligenceBand::Far => 16.0,
	}
}

fn upsert(
	registry: &mut ThreatRegistry,
	entity: Entity,
	subject: ThreatSubject,
	affiliations: &Affiliations,
	position: Vec3,
) {
	if let Err(error) = registry.upsert(entity, subject, affiliations, position) {
		warn!("failed to index threat subject {entity}: {error}");
	}
}

/// Applies directed non-spatial findings through the same affiliation gate as scans.
pub fn ingest_threat_observations(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	mut observations: MessageReader<ThreatObservation>,
	mut recipients: Query<(&Affiliations, &ThreatIntelligenceUser, &mut ThreatKnowledge)>,
) {
	let now = time.elapsed_secs();
	for observation in observations.read() {
		let Some(record) = registry.get(observation.subject) else {
			continue;
		};
		if record.entity == observation.recipient {
			continue;
		}
		let Ok((affiliations, user, mut knowledge)) = recipients.get_mut(observation.recipient)
		else {
			continue;
		};
		knowledge.observe(
			record,
			affiliations,
			observation.source,
			observation.confidence,
			now,
			user.policy.threat_threshold,
		);
	}
}

/// Forget on a staggered clock, then drain due scans in shared viewer order.
///
/// Far skips the scan body unless a fairness reserve is due. `skips` reset when
/// a scan actually ran. Missing lod is Near (local player).
pub fn discover_threats(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	priority: Res<IntelligencePriority>,
	limits: Res<ThreatDiscoverLimits>,
	mut recipients: ThreatRecipients,
	mut local_candidates: Local<Vec<ThreatId>>,
	mut due_entities: Local<IntelligenceDueEntities>,
) {
	let now = time.elapsed_secs();
	for (entity, _, _, affiliations, mut user, mut knowledge, _) in &mut recipients {
		if affiliations.is_changed() {
			user.next_scan_at = 0.0;
		}
		if now >= user.next_forget_at {
			forget_if_due(entity, now, &affiliations, &mut user, &mut knowledge, &registry);
		}
	}

	due_entities.clear();
	for (entity, _, _, _, user, _, _) in recipients.iter_mut() {
		if now >= user.next_scan_at {
			due_entities.push(entity);
		}
	}
	let due = due_entities.finish(&priority);
	let fair = due.iter().copied().find(|entity| {
		recipients.get_mut(*entity).is_ok_and(|(_, _, _, _, _, _, lod)| {
			lod.as_deref().is_some_and(|lod| {
				lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
			})
		})
	});

	let mut remaining = limits.max_scans_per_tick;
	for &entity in due {
		if remaining == 0 {
			break;
		}
		let Ok((entity, identity, transform, affiliations, mut user, mut knowledge, mut lod)) =
			recipients.get_mut(entity)
		else {
			continue;
		};
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		if band == IntelligenceBand::Far && Some(entity) != fair {
			continue;
		}
		remaining -= 1;
		knowledge.reconcile_registry(&registry);
		knowledge.maintain(&affiliations, user.policy, now);
		user.next_forget_at = now + staggered_interval(FORGET_INTERVAL, entity, 2);
		registry.collect_local(
			transform.translation(),
			user.perception_radius(&knowledge),
			&mut local_candidates,
		);
		let count = local_candidates.len();
		let budget = band.scale_count(user.policy.candidates_per_scan).min(count);
		let mut taken = 0;
		let mut offset = 0;
		while taken < budget && offset < count {
			let record = registry
				.get(local_candidates[(user.sample_cursor + offset) % count])
				.expect("collect_local ids are indexed");
			offset += 1;
			if record.entity == entity || record.id == identity.id {
				continue;
			}
			knowledge.observe(
				record,
				&affiliations,
				ThreatSource::LOCAL_SCAN,
				1.0,
				now,
				user.policy.threat_threshold,
			);
			taken += 1;
		}
		user.sample_cursor = user.sample_cursor.wrapping_add(offset.max(1));
		let interval = if knowledge.len() >= user.policy.desired_threats {
			user.policy.retained_scan_interval_secs
		} else {
			user.policy.scan_interval_secs
		};
		user.next_scan_at = now + staggered_interval(interval, entity, 0) * band.interval_scale();
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
	affiliations: &Affiliations,
	user: &mut ThreatIntelligenceUser,
	knowledge: &mut ThreatKnowledge,
	registry: &ThreatRegistry,
) {
	if now < user.next_forget_at {
		return;
	}
	knowledge.reconcile_registry(registry);
	knowledge.maintain(affiliations, user.policy, now);
	user.next_forget_at = now + staggered_interval(FORGET_INTERVAL, entity, 2);
}

/// Reconciles retained threats into one independently-owned spotting hint source.
///
/// Only runs when knowledge changed this frame (scan, forget, or inbox).
pub fn export_threat_spotting_hints(
	mut recipients: Query<
		(&ThreatKnowledge, Option<&mut ThreatIntelligenceUser>, &mut SpottingUser),
		Changed<ThreatKnowledge>,
	>,
) {
	for (knowledge, user, mut spotting) in &mut recipients {
		if let Some(mut user) = user {
			if user.alert_spotting_needs_write(knowledge, &spotting) {
				user.apply_alert_spotting(knowledge, &mut spotting);
			}
		}
		let active: HashSet<Entity> = knowledge.iter().filter_map(|known| known.entity).collect();
		let retired: Vec<_> = spotting
			.hints
			.iter()
			.filter_map(|(entity, hint)| {
				(hint.has_source(SpottingHintSource::THREAT) && !active.contains(entity))
					.then_some(*entity)
			})
			.collect();
		for entity in retired {
			spotting.remove_hint_source(entity, SpottingHintSource::THREAT);
		}
		for known in knowledge.iter() {
			let Some(entity) = known.entity else {
				continue;
			};
			let priority = (known.threat_weight * known.confidence * 4.0)
				.round()
				.clamp(1.0, i32::MAX as f32) as i32;
			if spotting.has_hint_from(entity, SpottingHintSource::THREAT, priority) {
				continue;
			}
			spotting.hint_from(entity, SpottingHintSource::THREAT, priority);
		}
	}
}
