//! Owned simulation-suspension claims. Distinct from capture and from
//! install / remove. Releasing one token cannot release another owner's freeze.
//! Each claim names the subject whose participation it freezes.

use bevy::prelude::*;

use crate::actor::ActorTarget;
use crate::token::CapabilityToken;

/// What a suspension freezes. Capture is not a scope — it only redirects input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SuspensionScope {
	/// Player realization (movement, items). NPCs and installed motors stay.
	Gameplay,
	/// Broader world freeze. Pause does not use this.
	///
	/// Production: [`SuspensionStack::freezes`]`(Self::Simulation)` is the
	/// world-wide motor gate. No production writer currently installs this
	/// claim; hosts that need a full sim stop must acquire one explicitly.
	Simulation,
}

/// Whose participation a claim freezes. A session claim does not freeze NPCs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SuspensionSubject {
	/// The session's local player. NPCs are not this subject.
	SessionLocal,
	/// One actor, player or NPC.
	Actor(ActorTarget),
}

impl SuspensionSubject {
	pub const fn session_local() -> Self {
		Self::SessionLocal
	}

	pub const fn actor(target: ActorTarget) -> Self {
		Self::Actor(target)
	}

	pub const fn is_session(self) -> bool {
		matches!(self, Self::SessionLocal)
	}

	/// Whether this claim covers `actor` given the session's local assignment.
	pub fn covers(self, actor: ActorTarget, local: Option<ActorTarget>) -> bool {
		match self {
			Self::SessionLocal => local == Some(actor),
			Self::Actor(target) => target == actor,
		}
	}
}

/// One owner's freeze of one scope for one subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SuspensionClaim {
	token: CapabilityToken,
	scope: SuspensionScope,
	subject: SuspensionSubject,
}

impl SuspensionClaim {
	pub const fn new(
		token: CapabilityToken,
		scope: SuspensionScope,
		subject: SuspensionSubject,
	) -> Self {
		Self { token, scope, subject }
	}

	/// Session pause: freeze local-player realization, keep capabilities.
	pub const fn pause_gameplay() -> Self {
		Self::new(
			CapabilityToken::PAUSE,
			SuspensionScope::Gameplay,
			SuspensionSubject::SessionLocal,
		)
	}

	pub const fn token(self) -> CapabilityToken {
		self.token
	}

	pub const fn scope(self) -> SuspensionScope {
		self.scope
	}

	pub const fn subject(self) -> SuspensionSubject {
		self.subject
	}
}

/// Nested freezes. The owner that installed a claim is the only one that can
/// release it.
#[derive(Resource, Clone, Debug, Default)]
pub struct SuspensionStack {
	claims: Vec<SuspensionClaim>,
}

impl SuspensionStack {
	pub fn set(&mut self, claim: SuspensionClaim, present: bool) {
		self.pop_token(claim.token());
		if present {
			self.claims.push(claim);
		}
	}

	pub fn set_pause(&mut self, paused: bool) {
		self.set(SuspensionClaim::pause_gameplay(), paused);
	}

	/// Session teardown releases the pause token only. Other owners stay.
	pub fn release_play(&mut self) {
		self.pop_token(CapabilityToken::PAUSE);
	}

	pub fn pop_token(&mut self, token: CapabilityToken) -> Option<SuspensionClaim> {
		let index = self.claims.iter().rposition(|claim| claim.token() == token)?;
		Some(self.claims.remove(index))
	}

	/// Session-scoped freeze of `scope` (local player / world-wide Simulation).
	pub fn freezes(&self, scope: SuspensionScope) -> bool {
		self.claims.iter().any(|claim| {
			claim.scope() == scope
				&& (claim.subject().is_session() || scope == SuspensionScope::Simulation)
		})
	}

	/// Actor-scoped freeze: session-local claims cover the assigned local actor;
	/// explicit actor claims cover that actor only. Simulation covers everyone.
	pub fn freezes_actor(
		&self,
		actor: ActorTarget,
		scope: SuspensionScope,
		local: Option<ActorTarget>,
	) -> bool {
		if scope != SuspensionScope::Simulation && self.freezes(SuspensionScope::Simulation) {
			return true;
		}
		self.claims
			.iter()
			.any(|claim| claim.scope() == scope && claim.subject().covers(actor, local))
	}

	pub fn owns(&self, token: CapabilityToken) -> bool {
		self.claims.iter().any(|claim| claim.token() == token)
	}

	pub fn is_empty(&self) -> bool {
		self.claims.is_empty()
	}

	pub fn len(&self) -> usize {
		self.claims.len()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pause_freezes_gameplay_not_simulation() {
		let mut stack = SuspensionStack::default();
		stack.set_pause(true);
		assert!(stack.freezes(SuspensionScope::Gameplay));
		assert!(!stack.freezes(SuspensionScope::Simulation));
		assert!(stack.owns(CapabilityToken::PAUSE));
		stack.set_pause(false);
		assert!(!stack.freezes(SuspensionScope::Gameplay));
		assert!(stack.is_empty());
	}

	#[test]
	fn releasing_one_token_leaves_the_other() {
		let mut stack = SuspensionStack::default();
		stack.set(SuspensionClaim::pause_gameplay(), true);
		let other = CapabilityToken::from_bits(7);
		stack.set(
			SuspensionClaim::new(
				other,
				SuspensionScope::Simulation,
				SuspensionSubject::session_local(),
			),
			true,
		);
		assert_eq!(stack.len(), 2);
		assert!(stack.pop_token(CapabilityToken::PAUSE).is_some());
		assert!(!stack.freezes(SuspensionScope::Gameplay));
		assert!(stack.freezes(SuspensionScope::Simulation));
		assert!(stack.owns(other));
	}

	#[test]
	fn release_play_does_not_clear_foreign_tokens() {
		let mut stack = SuspensionStack::default();
		stack.set_pause(true);
		let other = CapabilityToken::from_bits(3);
		stack.set(
			SuspensionClaim::new(
				other,
				SuspensionScope::Simulation,
				SuspensionSubject::session_local(),
			),
			true,
		);
		stack.release_play();
		assert!(!stack.owns(CapabilityToken::PAUSE));
		assert!(stack.owns(other));
	}

	#[test]
	fn session_pause_does_not_freeze_an_unrelated_actor() {
		let local = ActorTarget::new(Entity::from_bits(1));
		let npc = ActorTarget::new(Entity::from_bits(2));
		let mut stack = SuspensionStack::default();
		stack.set_pause(true);
		assert!(stack.freezes_actor(local, SuspensionScope::Gameplay, Some(local)));
		assert!(!stack.freezes_actor(npc, SuspensionScope::Gameplay, Some(local)));
	}

	#[test]
	fn actor_claim_freezes_only_that_actor() {
		let npc = ActorTarget::new(Entity::from_bits(2));
		let other = ActorTarget::new(Entity::from_bits(3));
		let mut stack = SuspensionStack::default();
		stack.set(
			SuspensionClaim::new(
				CapabilityToken::from_bits(4),
				SuspensionScope::Gameplay,
				SuspensionSubject::actor(npc),
			),
			true,
		);
		assert!(stack.freezes_actor(npc, SuspensionScope::Gameplay, None));
		assert!(!stack.freezes_actor(other, SuspensionScope::Gameplay, None));
		assert!(!stack.freezes(SuspensionScope::Gameplay));
	}

	#[test]
	fn simulation_freezes_every_actor() {
		let local = ActorTarget::new(Entity::from_bits(1));
		let npc = ActorTarget::new(Entity::from_bits(2));
		let mut stack = SuspensionStack::default();
		stack.set(
			SuspensionClaim::new(
				CapabilityToken::from_bits(5),
				SuspensionScope::Simulation,
				SuspensionSubject::session_local(),
			),
			true,
		);
		assert!(stack.freezes(SuspensionScope::Simulation));
		assert!(stack.freezes_actor(local, SuspensionScope::Gameplay, Some(local)));
		assert!(stack.freezes_actor(npc, SuspensionScope::Gameplay, Some(local)));
	}
}
