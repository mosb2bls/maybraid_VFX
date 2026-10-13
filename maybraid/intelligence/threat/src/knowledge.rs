use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{Affiliations, ThreatId, ThreatRecord, ThreatSource};
use spotting_intelligence::SpottingUser;

/// Discovery cadence, retention, and bounded work for one recipient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreatDiscoveryPolicy {
	pub radius: f32,
	pub scan_interval_secs: f32,
	pub retained_scan_interval_secs: f32,
	pub retention_secs: f32,
	pub desired_threats: usize,
	pub candidates_per_scan: usize,
	pub max_known: usize,
	pub threat_threshold: f32,
	/// Idle plants keep `radius`. Damage / shared alert uses at least this.
	pub alert_radius: f32,
}

impl ThreatDiscoveryPolicy {
	/// High-band envelope. Idle spotting stays closer; alert fills this gap.
	pub const ALERT_RANGE: f32 = 200.0;
}

impl Default for ThreatDiscoveryPolicy {
	fn default() -> Self {
		Self {
			radius: 80.0,
			scan_interval_secs: 0.5,
			retained_scan_interval_secs: 3.0,
			retention_secs: 20.0,
			desired_threats: 8,
			candidates_per_scan: 24,
			max_known: 64,
			threat_threshold: 0.2,
			alert_radius: Self::ALERT_RANGE,
		}
	}
}

/// Directed inbox item from sessions, stimuli, sharing, or other systems.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct ThreatObservation {
	pub recipient: Entity,
	pub subject: ThreatId,
	pub source: ThreatSource,
	pub confidence: f32,
}

impl ThreatObservation {
	pub fn new(
		recipient: Entity,
		subject: ThreatId,
		source: ThreatSource,
		confidence: f32,
	) -> Self {
		Self { recipient, subject, source, confidence }
	}
}

/// Retained semantic threat knowledge. This is not visual contact memory.
#[derive(Clone, Debug, PartialEq)]
pub struct KnownThreat {
	pub id: ThreatId,
	pub entity: Option<Entity>,
	pub last_known_position: Vec3,
	pub salience: f32,
	pub threat_weight: f32,
	pub confidence: f32,
	pub sources: ThreatSource,
	pub first_observed_at: f32,
	pub last_confirmed_at: f32,
	pub memberships: Affiliations,
}

/// Per-recipient retained threats.
#[derive(Component, Clone, Debug, Default)]
pub struct ThreatKnowledge {
	known: BTreeMap<ThreatId, KnownThreat>,
}

impl ThreatKnowledge {
	pub fn observe(
		&mut self,
		record: &ThreatRecord,
		recipient_affiliations: &Affiliations,
		source: ThreatSource,
		confidence: f32,
		now: f32,
		threshold: f32,
	) -> Option<&KnownThreat> {
		if source.is_empty() || !confidence.is_finite() || !now.is_finite() {
			return None;
		}
		let threat_weight = recipient_affiliations.threat_weight(&record.affiliations, now);
		if threat_weight < threshold.max(0.0) {
			return None;
		}
		let confidence = confidence.clamp(0.0, 1.0);
		let known = self.known.entry(record.id).or_insert_with(|| KnownThreat {
			id: record.id,
			entity: Some(record.entity),
			last_known_position: record.position,
			salience: record.salience,
			threat_weight,
			confidence,
			sources: source,
			first_observed_at: now,
			last_confirmed_at: now,
			memberships: record.affiliations.clone(),
		});
		known.entity = Some(record.entity);
		known.last_known_position = record.position;
		known.salience = record.salience;
		known.threat_weight = threat_weight;
		known.confidence = known.confidence.max(confidence);
		known.sources.insert(source);
		known.last_confirmed_at = now;
		known.memberships = record.affiliations.clone();
		Some(known)
	}

	pub fn get(&self, id: ThreatId) -> Option<&KnownThreat> {
		self.known.get(&id)
	}

	pub fn iter(&self) -> impl Iterator<Item = &KnownThreat> {
		self.known.values()
	}

	pub fn len(&self) -> usize {
		self.known.len()
	}

	pub fn is_empty(&self) -> bool {
		self.known.is_empty()
	}

	/// Injury, incoming fire, or a pack-shared finding. Not a local scan alone.
	pub fn wants_alert_perception(&self) -> bool {
		self.iter()
			.any(|known| known.sources.intersects(ThreatSource::ALERT_PERCEPTION))
	}

	pub fn remove_source(&mut self, id: ThreatId, source: ThreatSource) -> bool {
		let Some(known) = self.known.get_mut(&id) else {
			return false;
		};
		known.sources.remove(source);
		if known.sources.is_empty() {
			self.known.remove(&id);
		}
		true
	}

	pub fn reconcile_registry(&mut self, registry: &crate::ThreatRegistry) {
		for known in self.known.values_mut() {
			if let Some(record) = registry.get(known.id) {
				known.entity = Some(record.entity);
				known.last_known_position = record.position;
				known.salience = record.salience;
				known.memberships = record.affiliations.clone();
			} else {
				known.entity = None;
			}
		}
	}

	pub fn maintain(
		&mut self,
		recipient_affiliations: &Affiliations,
		policy: ThreatDiscoveryPolicy,
		now: f32,
	) {
		let retention = policy.retention_secs.max(0.0);
		let threshold = policy.threat_threshold.max(0.0);
		self.known.retain(|_, known| {
			known.threat_weight = recipient_affiliations.threat_weight(&known.memberships, now);
			let retained = now - known.last_confirmed_at <= retention
				|| known.sources.intersects(ThreatSource::OBJECTIVE);
			retained && known.threat_weight >= threshold
		});
		while self.known.len() > policy.max_known {
			let Some(oldest) = self
				.known
				.values()
				.min_by(|a, b| {
					a.last_confirmed_at
						.total_cmp(&b.last_confirmed_at)
						.then_with(|| a.id.cmp(&b.id))
				})
				.map(|known| known.id)
			else {
				break;
			};
			self.known.remove(&oldest);
		}
	}
}

/// Installed local threat-discovery behavior.
#[derive(Component, Clone, Debug)]
pub struct ThreatIntelligenceUser {
	pub policy: ThreatDiscoveryPolicy,
	/// Idle `SpotDirective::range` snapshot. Zero until the first alert apply.
	pub idle_spotting_range: f32,
	pub(crate) next_scan_at: f32,
	pub(crate) next_forget_at: f32,
	pub(crate) sample_cursor: usize,
}

impl Default for ThreatIntelligenceUser {
	fn default() -> Self {
		Self {
			policy: ThreatDiscoveryPolicy::default(),
			idle_spotting_range: 0.0,
			next_scan_at: 0.0,
			next_forget_at: 0.0,
			sample_cursor: 0,
		}
	}
}

impl ThreatIntelligenceUser {
	pub fn new(policy: ThreatDiscoveryPolicy) -> Self {
		Self { policy, ..default() }
	}

	pub fn perception_radius(&self, knowledge: &ThreatKnowledge) -> f32 {
		if knowledge.wants_alert_perception() {
			self.policy.radius.max(self.policy.alert_radius)
		} else {
			self.policy.radius
		}
	}

	fn alert_spotting_range(&self, knowledge: &ThreatKnowledge, spotting: &SpottingUser) -> f32 {
		let idle = if self.idle_spotting_range > 0.0 {
			self.idle_spotting_range
		} else {
			spotting.directives.iter().map(|directive| directive.range).fold(0.0, f32::max)
		};
		if knowledge.wants_alert_perception() {
			idle.max(self.policy.alert_radius)
		} else {
			idle
		}
	}

	pub fn alert_spotting_needs_write(
		&self,
		knowledge: &ThreatKnowledge,
		spotting: &SpottingUser,
	) -> bool {
		let range = self.alert_spotting_range(knowledge, spotting);
		self.idle_spotting_range <= 0.0
			|| spotting.directives.iter().any(|directive| directive.range != range)
	}

	/// Stretch or restore spotting directives so hinted subjects can be probed.
	pub fn apply_alert_spotting(
		&mut self,
		knowledge: &ThreatKnowledge,
		spotting: &mut SpottingUser,
	) {
		if self.idle_spotting_range <= 0.0 {
			self.idle_spotting_range =
				spotting.directives.iter().map(|directive| directive.range).fold(0.0, f32::max);
		}
		let range = self.alert_spotting_range(knowledge, spotting);
		for directive in &mut spotting.directives {
			if directive.range != range {
				directive.range = range;
			}
		}
	}
}
