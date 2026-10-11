use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::place::ArrivalDisk;
use crate::{PoiId, PoiInterests, PoiKind, PoiLearningPolicy, PoiSource, MAX_POI_ARRIVAL_RADIUS};

/// Latest remembered snapshot of one POI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownPoi {
	pub id: PoiId,
	pub entity: Option<Entity>,
	pub kind: PoiKind,
	pub position: Vec3,
	pub arrival_radius: f32,
	pub salience: f32,
	pub confidence: f32,
	pub sources: PoiSource,
	pub first_observed_at: f32,
	pub last_observed_at: f32,
}

impl KnownPoi {
	pub fn arrival_disk(self) -> ArrivalDisk {
		ArrivalDisk::new(self.position, self.arrival_radius)
	}
}

/// An inbox item produced by a non-POI discovery system.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PoiObservation {
	pub user: Entity,
	pub id: PoiId,
	pub entity: Option<Entity>,
	pub kind: PoiKind,
	pub position: Vec3,
	pub arrival_radius: f32,
	pub salience: f32,
	pub confidence: f32,
	pub source: PoiSource,
}

impl PoiObservation {
	pub fn external(user: Entity, id: PoiId, kind: PoiKind, position: Vec3) -> Self {
		Self {
			user,
			id,
			entity: None,
			kind,
			position,
			arrival_radius: 1.0,
			salience: 1.0,
			confidence: 1.0,
			source: PoiSource::EXTERNAL,
		}
	}
}

/// Source-owned, retained POI knowledge for one intelligence user.
#[derive(Component, Clone, Debug, Default)]
pub struct PoiKnowledge {
	known: BTreeMap<PoiId, KnownPoi>,
}

impl PoiKnowledge {
	pub fn observe(&mut self, observation: PoiObservation, now: f32) -> Option<KnownPoi> {
		if observation.source.is_empty()
			|| !observation.position.is_finite()
			|| !observation.arrival_radius.is_finite()
			|| !observation.salience.is_finite()
			|| !observation.confidence.is_finite()
			|| !now.is_finite()
		{
			return None;
		}
		let confidence = observation.confidence.clamp(0.0, 1.0);
		let arrival_radius = observation.arrival_radius.clamp(0.0, MAX_POI_ARRIVAL_RADIUS);
		let salience = observation.salience.max(0.0);
		if let Some(known) = self.known.get_mut(&observation.id) {
			known.entity = observation.entity.or(known.entity);
			known.kind = observation.kind;
			known.position = observation.position;
			known.arrival_radius = arrival_radius;
			known.salience = salience;
			known.confidence = known.confidence.max(confidence);
			known.sources.insert(observation.source);
			known.last_observed_at = now;
			return Some(*known);
		}

		let known = KnownPoi {
			id: observation.id,
			entity: observation.entity,
			kind: observation.kind,
			position: observation.position,
			arrival_radius,
			salience,
			confidence,
			sources: observation.source,
			first_observed_at: now,
			last_observed_at: now,
		};
		self.known.insert(known.id, known);
		Some(known)
	}

	pub fn remove_source(&mut self, id: PoiId, source: PoiSource) -> bool {
		let Some(known) = self.known.get_mut(&id) else {
			return false;
		};
		known.sources.remove(source);
		if known.sources.is_empty() {
			self.known.remove(&id);
		}
		true
	}

	pub fn include_source(&mut self, id: PoiId, source: PoiSource) -> bool {
		let Some(known) = self.known.get_mut(&id) else {
			return false;
		};
		let previous = known.sources;
		known.sources.insert(source);
		known.sources != previous
	}

	pub fn get(&self, id: PoiId) -> Option<&KnownPoi> {
		self.known.get(&id)
	}

	pub fn iter(&self) -> impl Iterator<Item = &KnownPoi> {
		self.known.values()
	}

	pub fn matching<'a>(
		&'a self,
		interests: &'a PoiInterests,
	) -> impl Iterator<Item = &'a KnownPoi> + 'a {
		self.known.values().filter(|known| interests.contains(known.kind))
	}

	pub fn len(&self) -> usize {
		self.known.len()
	}

	pub fn is_empty(&self) -> bool {
		self.known.is_empty()
	}

	pub fn maintain(&mut self, now: f32, policy: PoiLearningPolicy) {
		let retention = policy.retention_secs.max(0.0);
		self.known.retain(|_, known| {
			known.sources.intersects(policy.durable_sources)
				|| now - known.last_observed_at <= retention
		});

		while self.known.len() > policy.max_known {
			let Some(oldest) = self
				.known
				.values()
				.min_by(|a, b| {
					a.last_observed_at.total_cmp(&b.last_observed_at).then_with(|| a.id.cmp(&b.id))
				})
				.map(|known| known.id)
			else {
				break;
			};
			self.known.remove(&oldest);
		}
	}
}

/// Scan policy and mutable token-bucket state for one POI learner.
#[derive(Component, Clone, Debug)]
pub struct PoiIntelligenceUser {
	pub interests: PoiInterests,
	pub policy: PoiLearningPolicy,
	pub(crate) next_local_scan_at: f32,
	pub(crate) next_global_scan_at: f32,
	pub(crate) next_forget_at: f32,
	pub(crate) learning_credit: f32,
	pub(crate) local_cursor: usize,
	pub(crate) global_cursor: usize,
}

impl PoiIntelligenceUser {
	pub fn new(interests: PoiInterests) -> Self {
		Self {
			interests,
			policy: PoiLearningPolicy::default(),
			next_local_scan_at: 0.0,
			next_global_scan_at: 0.0,
			next_forget_at: 0.0,
			learning_credit: 1.0,
			local_cursor: 0,
			global_cursor: 0,
		}
	}

	pub fn with_policy(mut self, policy: PoiLearningPolicy) -> Self {
		self.policy = policy;
		self
	}

	pub(crate) fn next_learning_credit(&self, delta_secs: f32) -> f32 {
		let cap = self.policy.candidates_per_scan.max(1) as f32;
		(self.learning_credit + delta_secs.max(0.0) * self.policy.learning_rate_per_second.max(0.0))
			.min(cap)
	}

	pub(crate) fn accrue_learning(&mut self, delta_secs: f32) {
		let next = self.next_learning_credit(delta_secs);
		if self.learning_credit != next {
			self.learning_credit = next;
		}
	}

	pub fn try_take_learning_credit(&mut self) -> bool {
		if self.learning_credit < 1.0 {
			return false;
		}
		self.learning_credit -= 1.0;
		true
	}
}
