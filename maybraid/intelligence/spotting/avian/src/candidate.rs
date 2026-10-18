use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use bevy::prelude::*;
use spotting_intelligence::{SpotCandidate, SpotDirective, SpottedContact};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ProbeCandidate {
	pub(super) rank: SpotCandidate,
	pub(super) respot_interval_secs: f32,
}

impl ProbeCandidate {
	pub(super) fn new(
		subject: Entity,
		directive: SpotDirective,
		salience: f32,
		distance: f32,
		known: bool,
		available_samples: usize,
	) -> Self {
		Self {
			rank: SpotCandidate {
				subject,
				directive_priority: directive.priority,
				salience: if salience.is_finite() { salience } else { 0.0 },
				distance,
				known,
				max_samples: directive.max_samples_per_subject.min(available_samples),
			},
			respot_interval_secs: directive.respot_interval_secs.max(0.0),
		}
	}

	pub(super) fn merge(&mut self, other: Self) {
		if other.rank.directive_priority > self.rank.directive_priority {
			*self = other;
			return;
		}
		if other.rank.directive_priority == self.rank.directive_priority {
			self.rank.max_samples = self.rank.max_samples.max(other.rank.max_samples);
			self.respot_interval_secs = self.respot_interval_secs.min(other.respot_interval_secs);
		}
		self.rank.known |= other.rank.known;
	}
}

pub(super) fn merge_candidate(
	candidates: &mut BTreeMap<Entity, ProbeCandidate>,
	candidate: ProbeCandidate,
) {
	match candidates.entry(candidate.rank.subject) {
		Entry::Vacant(entry) => {
			entry.insert(candidate);
		}
		Entry::Occupied(mut entry) => entry.get_mut().merge(candidate),
	}
}

pub(super) fn blocks_respot(contact: &SpottedContact, now: f32, directive: SpotDirective) -> bool {
	contact.is_fresh(now, directive.freshness_secs) && !contact.is_due(now)
}
