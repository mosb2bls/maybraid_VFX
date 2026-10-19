use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{ThreatGroupId, ThreatId};

const MIN_STRENGTH: f32 = 1e-3;
const MAX_STRENGTH: f32 = 1_000_000.0;

/// Positive membership or antagonism strength, optionally decayed over time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AffiliationStrength {
	pub weight: f32,
	pub observed_at: f32,
	pub half_life: Option<f32>,
}

impl AffiliationStrength {
	pub fn permanent(weight: f32) -> Self {
		Self { weight: finite_weight(weight), observed_at: 0.0, half_life: None }
	}

	pub fn decaying(weight: f32, observed_at: f32, half_life: f32) -> Self {
		Self {
			weight: finite_weight(weight),
			observed_at: finite_time(observed_at),
			half_life: half_life.is_finite().then_some(half_life.max(0.0)),
		}
	}

	pub fn effective(self, now: f32) -> f32 {
		let elapsed = (finite_time(now) - self.observed_at).max(0.0);
		match self.half_life {
			Some(half_life) if half_life > 0.0 => self.weight * (-elapsed / half_life).exp2(),
			Some(_) if elapsed > 0.0 => 0.0,
			_ => self.weight,
		}
	}
}

/// What this actor belongs to, hunts, and spares.
///
/// Antagonism and mitigation are both directional observer beliefs matched
/// against the candidate's memberships. Net threat is max aggravation minus
/// max mitigation, clamped at zero.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct Affiliations {
	pub memberships: BTreeMap<ThreatGroupId, AffiliationStrength>,
	pub known_antagonists: BTreeMap<ThreatGroupId, AffiliationStrength>,
	pub known_allies: BTreeMap<ThreatGroupId, AffiliationStrength>,
}

impl Affiliations {
	pub fn with_self(id: ThreatId) -> Self {
		let mut affiliations = Self::default();
		affiliations.join(ThreatGroupId::individual(id), AffiliationStrength::permanent(1.0));
		affiliations
	}

	pub fn join(&mut self, group: ThreatGroupId, strength: AffiliationStrength) {
		self.memberships.insert(group, strength);
	}

	pub fn antagonize(&mut self, group: ThreatGroupId, strength: AffiliationStrength) {
		self.known_antagonists.insert(group, strength);
	}

	pub fn stop_antagonizing(&mut self, group: ThreatGroupId) {
		self.known_antagonists.remove(&group);
	}

	pub fn mitigate(&mut self, group: ThreatGroupId, strength: AffiliationStrength) {
		self.known_allies.insert(group, strength);
	}

	pub fn stop_mitigating(&mut self, group: ThreatGroupId) {
		self.known_allies.remove(&group);
	}

	/// Directional hostility of `self` toward the candidate's memberships.
	pub fn threat_weight(&self, candidate: &Self, now: f32) -> f32 {
		let aggravation = overlap_weight(&self.known_antagonists, &candidate.memberships, now);
		let mitigation = overlap_weight(&self.known_allies, &candidate.memberships, now);
		(aggravation - mitigation).max(0.0)
	}

	pub fn maintain(&mut self, now: f32) {
		self.memberships.retain(|_, strength| strength.effective(now) >= MIN_STRENGTH);
		self.known_antagonists
			.retain(|_, strength| strength.effective(now) >= MIN_STRENGTH);
		self.known_allies.retain(|_, strength| strength.effective(now) >= MIN_STRENGTH);
	}

	/// Replace `self` with `next` only when the affiliation snapshot differs.
	pub fn replace_if_changed(&mut self, next: &Self) {
		if self != next {
			*self = next.clone();
		}
	}
}

fn overlap_weight(
	beliefs: &BTreeMap<ThreatGroupId, AffiliationStrength>,
	memberships: &BTreeMap<ThreatGroupId, AffiliationStrength>,
	now: f32,
) -> f32 {
	memberships
		.iter()
		.filter_map(|(group, membership)| {
			let belief = beliefs.get(group)?;
			Some(membership.effective(now) * belief.effective(now))
		})
		.filter(|weight| weight.is_finite())
		.fold(0.0, f32::max)
}

fn finite_weight(weight: f32) -> f32 {
	if weight.is_finite() {
		weight.clamp(0.0, MAX_STRENGTH)
	} else {
		0.0
	}
}

fn finite_time(time: f32) -> f32 {
	if time.is_finite() {
		time.max(0.0)
	} else {
		0.0
	}
}
