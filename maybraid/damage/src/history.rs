//! Bounded, queryable record of accepted [`DamageApplied`](crate::DamageApplied).

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{DamageActorId, DamageApplied, DamageCredit, ResponsibleOwner};

/// Closed interval on game time (`start <= at <= end`). [`TimeInterval::since`]
/// leaves the end open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeInterval {
	pub start: f32,
	pub end: Option<f32>,
}

impl TimeInterval {
	pub fn new(start: f32, end: f32) -> Self {
		Self { start: finite_time(start), end: Some(finite_time(end)) }
	}

	pub fn since(start: f32) -> Self {
		Self { start: finite_time(start), end: None }
	}

	pub fn contains(self, at: f32) -> bool {
		let at = finite_time(at);
		if at < self.start {
			return false;
		}
		match self.end {
			Some(end) => at <= end,
			None => true,
		}
	}
}

/// Time window and capacity for [`DamageHistory`].
///
/// An active healing session must latch whether damage occurred; it must not
/// rely on this window forever. Defaults cover a full Self heal (5% max HP / s)
/// plus a short lookback.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct DamageHistoryPolicy {
	pub retain_secs: f32,
	pub max_records: usize,
}

impl Default for DamageHistoryPolicy {
	fn default() -> Self {
		Self { retain_secs: 45.0, max_records: 256 }
	}
}

impl DamageHistoryPolicy {
	pub fn retain_secs(self) -> f32 {
		if self.retain_secs.is_finite() {
			self.retain_secs.max(0.0)
		} else {
			0.0
		}
	}

	pub fn max_records(self) -> usize {
		self.max_records.max(1)
	}
}

/// One accepted hit, keyed by durable identities rather than live entities.
///
/// [`Self::applied`] is the HP actually lost. [`Self::requested`] is the
/// scaled `DamageApplied.amount` and may exceed `applied` on overkill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageRecord {
	pub at: f32,
	pub recipient: DamageActorId,
	pub requested: f32,
	pub applied: f32,
	pub remaining: f32,
	pub point: Vec3,
	pub source: Option<DamageActorId>,
	pub owner: Option<DamageActorId>,
	pub credit: Option<DamageCredit>,
}

impl DamageRecord {
	/// Strictly positive HP actually lost.
	pub fn is_positive(self) -> bool {
		self.applied.is_finite() && self.applied > 0.0
	}

	pub fn is_zero_effect(self) -> bool {
		!self.is_positive()
	}

	pub fn is_overkill(self) -> bool {
		self.is_positive() && self.requested.is_finite() && self.applied < self.requested
	}

	pub fn in_interval(self, interval: TimeInterval) -> bool {
		interval.contains(self.at)
	}

	pub fn for_recipient(self, recipient: DamageActorId) -> bool {
		self.recipient == recipient
	}
}

/// Recent accepted damage. Oldest records evict first.
#[derive(Resource, Clone, Debug, Default)]
pub struct DamageHistory {
	records: VecDeque<DamageRecord>,
}

impl DamageHistory {
	pub fn len(&self) -> usize {
		self.records.len()
	}

	pub fn is_empty(&self) -> bool {
		self.records.is_empty()
	}

	pub fn records(&self) -> impl Iterator<Item = &DamageRecord> {
		self.records.iter()
	}

	pub fn clear(&mut self) {
		self.records.clear();
	}

	pub fn push(&mut self, record: DamageRecord) {
		self.records.push_back(record);
	}

	pub fn evict(&mut self, now: f32, policy: DamageHistoryPolicy) {
		let now = finite_time(now);
		let keep_after = now - policy.retain_secs();
		while self.records.front().is_some_and(|record| record.at < keep_after) {
			self.records.pop_front();
		}
		let max = policy.max_records();
		while self.records.len() > max {
			self.records.pop_front();
		}
	}

	pub fn for_recipient(&self, recipient: DamageActorId) -> impl Iterator<Item = &DamageRecord> {
		self.records.iter().filter(move |record| record.for_recipient(recipient))
	}

	pub fn for_recipient_in(
		&self,
		recipient: DamageActorId,
		interval: TimeInterval,
	) -> impl Iterator<Item = &DamageRecord> {
		self.for_recipient(recipient).filter(move |record| record.in_interval(interval))
	}

	/// Whether `recipient` took strictly positive HP loss in `interval`.
	pub fn received_positive_damage(
		&self,
		recipient: DamageActorId,
		interval: TimeInterval,
	) -> bool {
		self.for_recipient_in(recipient, interval).any(|record| record.is_positive())
	}

	/// Healing lookback: positive applied HP at or after `since`.
	pub fn received_positive_damage_since(&self, recipient: DamageActorId, since: f32) -> bool {
		self.received_positive_damage(recipient, TimeInterval::since(since))
	}
}

/// Drop the retained window. Session teardown and explicit resets write this.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct ClearDamageHistory;

pub fn maintain_damage_history(
	time: Res<Time>,
	policy: Res<DamageHistoryPolicy>,
	mut history: ResMut<DamageHistory>,
	mut applied: MessageReader<DamageApplied>,
	mut clear: MessageReader<ClearDamageHistory>,
	owners: Query<&ResponsibleOwner>,
	credits: Query<&DamageCredit>,
) {
	if clear.read().next().is_some() {
		history.clear();
	}
	let now = time.elapsed_secs();
	for event in applied.read() {
		history.push(snapshot_applied(now, event, &owners, &credits));
	}
	history.evict(now, *policy);
}

fn snapshot_applied(
	at: f32,
	event: &DamageApplied,
	owners: &Query<&ResponsibleOwner>,
	credits: &Query<&DamageCredit>,
) -> DamageRecord {
	let recipient = DamageActorId::from_entity(event.target);
	let source = event.source.map(DamageActorId::from_entity);
	let owner = event.source.and_then(|source| {
		owners.get(source).ok().map(|owner| DamageActorId::from_entity(owner.actor))
	});
	let responsible = event
		.source
		.and_then(|source| owners.get(source).ok().map(|owner| owner.actor).or(Some(source)));
	let credit = event
		.source
		.and_then(|source| credits.get(source).ok().copied())
		.or_else(|| responsible.and_then(|entity| credits.get(entity).ok().copied()));
	DamageRecord {
		at: finite_time(at),
		recipient,
		requested: event.amount,
		applied: event.applied,
		remaining: event.remaining,
		point: event.point,
		source,
		owner,
		credit,
	}
}

fn finite_time(time: f32) -> f32 {
	if time.is_finite() {
		time.max(0.0)
	} else {
		0.0
	}
}

#[cfg(test)]
mod tests;
