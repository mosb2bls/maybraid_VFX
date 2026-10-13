use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use avian3d::prelude::{LinearVelocity, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use spotting_intelligence::{SpotCandidate, SpotSubject, SpottedContact, SpottingUser};

use crate::candidate::ProbeCandidate;
use crate::clear_segment;

pub(super) fn probe_candidate_visibility(
	user: &mut SpottingUser,
	now: f32,
	observer: Vec3,
	discovery_sample_cursor: usize,
	candidates: &BTreeMap<Entity, ProbeCandidate>,
	ranked: Vec<SpotCandidate>,
	grants: Vec<usize>,
	spatial: &SpatialQuery,
	fixed_filter: &SpatialQueryFilter,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	transforms: &TransformHelper,
) {
	for (candidate, sample_budget) in ranked.into_iter().zip(grants) {
		if sample_budget == 0 {
			continue;
		}
		let Some(policy) = candidates.get(&candidate.subject) else {
			continue;
		};
		let Ok((entity, subject, velocity)) = subjects.get(candidate.subject) else {
			continue;
		};
		let Ok(transform) = transforms.compute_global_transform(entity) else {
			continue;
		};
		let samples = subject.bounds.samples(observer, transform.translation());
		if samples.is_empty() {
			continue;
		}
		let sample_count =
			sample_budget.min(subject.bounds.sample_count()).min(candidate.max_samples);
		let sample_offset = user.contacts.get(&entity).map_or_else(
			|| discovery_sample_cursor.wrapping_add(entity.to_bits() as usize) % samples.len(),
			|contact| {
				usize::try_from(contact.consecutive_failures).unwrap_or(usize::MAX) % samples.len()
			},
		);
		let mut visible_point = None;
		let mut visible_head = None;
		for index in 0..sample_count {
			let sample = samples[(sample_offset + index) % samples.len()];
			if !clear_segment(observer, sample.point, spatial, fixed_filter) {
				continue;
			}
			if sample.feature.is_head() {
				visible_head = visible_head.or(Some(sample.point));
			} else {
				visible_point = visible_point.or(Some(sample.point));
			}
		}
		let visible_point = visible_point.or(visible_head);
		if let Some(visible_point) = visible_point {
			let velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
			match user.contacts.entry(entity) {
				Entry::Vacant(entry) => {
					entry.insert(SpottedContact::new(
						entity,
						transform.translation(),
						velocity,
						visible_point,
						visible_head,
						now,
						policy.respot_interval_secs,
					));
				}
				Entry::Occupied(mut entry) => entry.get_mut().note_success(
					transform.translation(),
					velocity,
					visible_point,
					visible_head,
					now,
					policy.respot_interval_secs,
				),
			}
		} else if let Some(contact) = user.contacts.get_mut(&entity) {
			contact.note_failure(now, policy.respot_interval_secs);
		}
	}
}
