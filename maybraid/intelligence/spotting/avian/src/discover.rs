use std::collections::BTreeMap;

use avian3d::prelude::{Collider, LinearVelocity, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use spotting_intelligence::{SpotDirective, SpotSubject, SpottingUser};

use crate::candidate::{blocks_respot, merge_candidate, ProbeCandidate};

fn directive_satisfied(
	directive: SpotDirective,
	now: f32,
	observer: Vec3,
	user: &SpottingUser,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	transforms: &TransformHelper,
) -> bool {
	directive.is_satisfied(
		now,
		user.contacts.values().filter_map(|contact| {
			let Ok((_, subject, _)) = subjects.get(contact.subject) else {
				return None;
			};
			let Ok(transform) = transforms.compute_global_transform(contact.subject) else {
				return None;
			};
			Some(spotting_intelligence::SpotContactView {
				contact,
				layers: subject.layers,
				distance: transform.translation().distance(observer),
			})
		}),
	)
}

pub(super) fn next_discovery_interval(user: &SpottingUser) -> f32 {
	user.directives
		.iter()
		.map(|directive| directive.discovery_interval_secs.max(0.0))
		.reduce(f32::min)
		.unwrap_or(0.25)
}

fn resolve_spot_subject_entity(
	entity: Entity,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	parents: &Query<&ChildOf>,
) -> Option<Entity> {
	let mut subject_entity = entity;
	loop {
		if subjects.get(subject_entity).is_ok() {
			return Some(subject_entity);
		}
		let Ok(parent) = parents.get(subject_entity) else {
			return None;
		};
		subject_entity = parent.parent();
	}
}

fn discover_from_broadphase(
	spotter_entity: Entity,
	user: &SpottingUser,
	now: f32,
	observer: Vec3,
	spatial: &SpatialQuery,
	animated_filter: &SpatialQueryFilter,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	parents: &Query<&ChildOf>,
	transforms: &TransformHelper,
	candidates: &mut BTreeMap<Entity, ProbeCandidate>,
) {
	for &directive in &user.directives {
		if directive.desired_count == 0
			|| directive_satisfied(directive, now, observer, user, subjects, transforms)
		{
			continue;
		}
		let range = directive.range.max(0.0);
		if range == 0.0 || !range.is_finite() {
			continue;
		}
		let sphere = Collider::sphere(range);
		for entity in
			spatial.shape_intersections(&sphere, observer, Quat::IDENTITY, animated_filter)
		{
			if entity == spotter_entity {
				continue;
			}
			let Some(subject_entity) = resolve_spot_subject_entity(entity, subjects, parents)
			else {
				continue;
			};
			if subject_entity == spotter_entity {
				continue;
			}
			let Ok((_, subject, _)) = subjects.get(subject_entity) else {
				continue;
			};
			let Ok(transform) = transforms.compute_global_transform(subject_entity) else {
				continue;
			};
			let distance = transform.translation().distance(observer);
			if !directive.matches(subject.layers, distance) {
				continue;
			}
			let known = user.contacts.get(&subject_entity);
			if known.is_some_and(|contact| blocks_respot(contact, now, directive)) {
				continue;
			}
			merge_candidate(
				candidates,
				ProbeCandidate::new(
					subject_entity,
					directive,
					subject.salience,
					distance,
					known.is_some(),
					subject.bounds.sample_count(),
				),
			);
		}
	}
}

fn discover_from_hints(
	spotter_entity: Entity,
	user: &SpottingUser,
	now: f32,
	observer: Vec3,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	transforms: &TransformHelper,
	candidates: &mut BTreeMap<Entity, ProbeCandidate>,
) {
	for (&subject_entity, hint) in &user.hints {
		if subject_entity == spotter_entity {
			continue;
		}
		let Ok((entity, subject, _)) = subjects.get(subject_entity) else {
			continue;
		};
		let Ok(transform) = transforms.compute_global_transform(entity) else {
			continue;
		};
		let distance = transform.translation().distance(observer);
		let known = user.contacts.get(&subject_entity);
		for &directive in &user.directives {
			if directive.desired_count == 0 || !directive.matches(subject.layers, distance) {
				continue;
			}
			if known.is_some_and(|contact| blocks_respot(contact, now, directive)) {
				continue;
			}
			let mut candidate = ProbeCandidate::new(
				subject_entity,
				directive,
				subject.salience,
				distance,
				known.is_some(),
				subject.bounds.sample_count(),
			);
			candidate.rank.directive_priority =
				candidate.rank.directive_priority.saturating_add(hint.priority());
			merge_candidate(candidates, candidate);
		}
	}
}

pub(super) fn discover_subjects(
	spotter_entity: Entity,
	user: &SpottingUser,
	now: f32,
	observer: Vec3,
	spatial: &SpatialQuery,
	animated_filter: &SpatialQueryFilter,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	parents: &Query<&ChildOf>,
	transforms: &TransformHelper,
	candidates: &mut BTreeMap<Entity, ProbeCandidate>,
) {
	discover_from_broadphase(
		spotter_entity,
		user,
		now,
		observer,
		spatial,
		animated_filter,
		subjects,
		parents,
		transforms,
		candidates,
	);
	discover_from_hints(spotter_entity, user, now, observer, subjects, transforms, candidates);
}

pub(super) fn merge_due_contacts(
	user: &SpottingUser,
	now: f32,
	observer: Vec3,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	transforms: &TransformHelper,
	candidates: &mut BTreeMap<Entity, ProbeCandidate>,
) {
	for contact in user.contacts.values().filter(|contact| contact.is_due(now)) {
		let Ok((entity, subject, _)) = subjects.get(contact.subject) else {
			continue;
		};
		let Ok(transform) = transforms.compute_global_transform(entity) else {
			continue;
		};
		let distance = transform.translation().distance(observer);
		for &directive in &user.directives {
			if !directive.matches(subject.layers, distance) {
				continue;
			}
			merge_candidate(
				candidates,
				ProbeCandidate::new(
					entity,
					directive,
					subject.salience,
					distance,
					true,
					subject.bounds.sample_count(),
				),
			);
		}
	}
}
