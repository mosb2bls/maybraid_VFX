//! Enter / stay / exit session state for one occupying actor.

use std::collections::BTreeMap;

use bevy::prelude::{Entity, Message, Vec3};

use crate::identity::{OccupancyIdentity, OccupancyMatchKey, OccupiedRegion};
use crate::rule::{ReentryPolicy, RuleId};
use crate::timer::{OccupancyTimer, TimerKey};

/// One interval tick for a matching occupancy rule.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct OccupancyAward {
	pub recipient: Entity,
	pub rule: RuleId,
	pub cell: OccupancyMatchKey,
	pub amount: f32,
	pub origin: Vec3,
}

/// Occupancy change. Stay is implicit: the actor remains in the last identity.
#[derive(Message, Clone, Debug, PartialEq)]
pub enum OccupancyEvent {
	Enter { actor: Entity, region: OccupiedRegion },
	Exit { actor: Entity, region: OccupiedRegion },
}

/// Per-actor occupancy and timer book-keeping.
#[derive(Clone, Debug, Default)]
pub struct OccupancySession {
	pub identity: Option<OccupancyIdentity>,
	timers: BTreeMap<TimerKey, OccupancyTimer>,
	persisted: BTreeMap<TimerKey, OccupancyTimer>,
	/// Closed sessions never award. Used for retiring bodies and ended play.
	closed: bool,
}

impl OccupancySession {
	pub fn timer(&self, key: TimerKey) -> Option<OccupancyTimer> {
		self.timers.get(&key).copied()
	}

	pub fn timer_count(&self) -> usize {
		self.timers.len()
	}

	/// True after [`Self::close`]. Closed sessions skip identity and awards.
	pub fn is_closed(&self) -> bool {
		self.closed
	}

	/// End this occupancy session. Timers and identity are dropped so a
	/// retiring body or torn-down play session cannot keep accruing.
	pub fn close(&mut self) {
		self.identity = None;
		self.timers.clear();
		self.persisted.clear();
		self.closed = true;
	}

	pub fn bind_timer(&mut self, key: TimerKey, reentry: ReentryPolicy) -> &mut OccupancyTimer {
		if self.timers.contains_key(&key) {
			return self.timers.entry(key).or_default();
		}
		let initial = match reentry {
			ReentryPolicy::Persist => self.persisted.remove(&key).unwrap_or_default(),
			ReentryPolicy::Reset => OccupancyTimer::default(),
		};
		self.timers.insert(key, initial);
		self.timers.entry(key).or_default()
	}

	pub fn drop_unoccupied(
		&mut self,
		identity: &OccupancyIdentity,
		reentry_for: impl Fn(TimerKey) -> ReentryPolicy,
	) {
		let keys: Vec<TimerKey> = self.timers.keys().copied().collect();
		for key in keys {
			if key.cell.occupies(identity) {
				continue;
			}
			if let Some(timer) = self.timers.remove(&key) {
				if reentry_for(key) == ReentryPolicy::Persist {
					self.persisted.insert(key, timer);
				}
			}
		}
	}

	pub fn sync_identity(&mut self, actor: Entity, next: OccupancyIdentity) -> Vec<OccupancyEvent> {
		let previous = self.identity.replace(next);
		let Some(previous) = previous else {
			return next
				.regions()
				.into_iter()
				.flatten()
				.map(|region| OccupancyEvent::Enter { actor, region })
				.collect();
		};
		let mut events = Vec::new();
		for region in previous.regions().into_iter().flatten() {
			if !next.regions().into_iter().flatten().any(|current| current == region) {
				events.push(OccupancyEvent::Exit { actor, region });
			}
		}
		for region in next.regions().into_iter().flatten() {
			if !previous.regions().into_iter().flatten().any(|old| old == region) {
				events.push(OccupancyEvent::Enter { actor, region });
			}
		}
		events
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{Entity, Vec3};

	use super::*;
	use crate::identity::OccupancyIdentity;

	#[test]
	fn first_resolve_enters_forest_and_grove() -> anyhow::Result<()> {
		let mut session = OccupancySession::default();
		let events = session
			.sync_identity(Entity::from_bits(1), OccupancyIdentity::from_position(Vec3::ZERO));
		assert_eq!(events.len(), 2);
		assert!(events.iter().any(|event| matches!(
			event,
			OccupancyEvent::Enter { region: OccupiedRegion::Forest(_), .. }
		)));
		assert!(events.iter().any(|event| matches!(
			event,
			OccupancyEvent::Enter { region: OccupiedRegion::Grove(_), .. }
		)));
		Ok(())
	}

	#[test]
	fn leaving_a_grove_emits_exit_then_enter() -> anyhow::Result<()> {
		let mut session = OccupancySession::default();
		let actor = Entity::from_bits(2);
		session.sync_identity(actor, OccupancyIdentity::from_position(Vec3::ZERO));
		let events = session
			.sync_identity(actor, OccupancyIdentity::from_position(Vec3::new(150.0, 0.0, 0.0)));
		assert!(events.iter().any(|event| matches!(event, OccupancyEvent::Exit { .. })));
		assert!(events.iter().any(|event| matches!(event, OccupancyEvent::Enter { .. })));
		assert_eq!(
			session
				.identity
				.map(|identity| identity.grove)
				.ok_or_else(|| anyhow::anyhow!("session should keep identity"))?,
			OccupancyIdentity::from_position(Vec3::new(150.0, 0.0, 0.0)).grove
		);
		Ok(())
	}

	#[test]
	fn close_drops_timers_and_blocks_rebind() -> anyhow::Result<()> {
		let mut session = OccupancySession::default();
		session.sync_identity(Entity::from_bits(3), OccupancyIdentity::from_position(Vec3::ZERO));
		session.close();
		assert!(session.is_closed());
		assert!(session.identity.is_none());
		assert_eq!(session.timer_count(), 0);
		Ok(())
	}
}
