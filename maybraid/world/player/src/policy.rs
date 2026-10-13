//! What each generation mode asks of the player.
//!
//! Modes register a policy under their [`TypeId`]. Player systems look the
//! active mode up; they do not name a mode.

use std::any::TypeId;
use std::collections::HashMap;

use bevy::prelude::*;

/// Home spawn, waypoint retention, and whether a finished respawn ends the life.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModePlayerPolicy {
	pub home: Vec2,
	/// Persist and resume saved waypoints while this mode is active.
	pub keep_waypoints: bool,
	/// A respawn that finishes while this mode is still active replaces the body
	/// where it fell and signals [`PlayerLifeEnded`]. The next loadout dresses it.
	pub respawn_ends_life: bool,
	/// After world load, withhold the startup body and open the POI picker.
	pub pick_first_spawn: bool,
}

/// Policies registered by generation modes, keyed by the mode's [`TypeId`].
#[derive(Resource, Default, Clone, Debug)]
pub struct ModePlayerPolicies {
	by_mode: HashMap<TypeId, ModePlayerPolicy>,
}

impl ModePlayerPolicies {
	pub fn register(&mut self, mode: TypeId, policy: ModePlayerPolicy) {
		self.by_mode.insert(mode, policy);
	}

	pub fn get(&self, mode: TypeId) -> Option<ModePlayerPolicy> {
		self.by_mode.get(&mode).copied()
	}

	pub fn keeps_waypoints(&self, mode: Option<TypeId>) -> bool {
		mode.and_then(|id| self.get(id)).is_none_or(|policy| policy.keep_waypoints)
	}

	pub fn home(&self, mode: Option<TypeId>) -> Option<Vec2> {
		mode.and_then(|id| self.get(id)).map(|policy| policy.home)
	}

	pub fn respawn_ends_life(&self, mode: Option<TypeId>) -> bool {
		mode.and_then(|id| self.get(id)).is_some_and(|policy| policy.respawn_ends_life)
	}

	pub fn pick_first_spawn(&self, mode: Option<TypeId>) -> bool {
		mode.and_then(|id| self.get(id)).is_some_and(|policy| policy.pick_first_spawn)
	}
}

/// The mode a player respawn started in, and whether that mode ends a life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RespawnOrigin {
	mode: Option<TypeId>,
	ends_life: bool,
}

impl RespawnOrigin {
	pub fn began(mode: Option<TypeId>, ends_life: bool) -> Self {
		Self { mode, ends_life }
	}

	/// The mode that began this respawn is no longer active.
	pub fn abandoned(self, now: Option<TypeId>) -> bool {
		self.mode.is_some() && self.mode != now
	}

	/// Still in the mode, and that mode ends a life on respawn.
	pub fn ends_life(self, now: Option<TypeId>) -> bool {
		self.ends_life && !self.abandoned(now)
	}

	/// Leave replaced the mode before the timer finished.
	pub fn replace_immediately(self, now: Option<TypeId>) -> bool {
		self.abandoned(now)
	}

	/// Put the body back where it fell: the life is ending, or the mode was left.
	pub fn replace_in_place(self, now: Option<TypeId>) -> bool {
		self.ends_life(now) || self.abandoned(now)
	}
}

/// A respawn finished inside a mode whose policy ends the life.
/// The mode turns this into its own session message.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerLifeEnded;

/// The player lifecycle resolves a respawn here. A mode forwards [`PlayerLifeEnded`] after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerLifeSet {
	Resolve,
}

#[cfg(test)]
mod tests {
	use super::*;

	struct Home;
	struct Away;

	#[test]
	fn an_abandoned_respawn_replaces_immediately_and_ends_no_life() {
		let origin = RespawnOrigin::began(Some(TypeId::of::<Home>()), true);
		let away = Some(TypeId::of::<Away>());
		assert!(origin.abandoned(away));
		assert!(origin.replace_immediately(away));
		assert!(origin.replace_in_place(away));
		assert!(!origin.ends_life(away));
	}

	#[test]
	fn a_life_ending_mode_keeps_the_respawn_until_it_resolves() {
		let home = Some(TypeId::of::<Home>());
		let origin = RespawnOrigin::began(home, true);
		assert!(!origin.abandoned(home));
		assert!(!origin.replace_immediately(home));
		assert!(origin.ends_life(home));
		assert!(origin.replace_in_place(home));
	}

	#[test]
	fn a_mode_that_keeps_waypoints_reports_its_home() {
		let mut policies = ModePlayerPolicies::default();
		policies.register(
			TypeId::of::<Home>(),
			ModePlayerPolicy {
				home: Vec2::ZERO,
				keep_waypoints: true,
				respawn_ends_life: false,
				pick_first_spawn: true,
			},
		);
		let home = Some(TypeId::of::<Home>());
		assert!(policies.keeps_waypoints(home));
		assert_eq!(policies.home(home), Some(Vec2::ZERO));
		assert!(!policies.respawn_ends_life(home));
		assert!(policies.pick_first_spawn(home));
		assert!(policies.keeps_waypoints(None));
		assert!(policies.home(Some(TypeId::of::<Away>())).is_none());
		assert!(!policies.pick_first_spawn(Some(TypeId::of::<Away>())));
	}
}
