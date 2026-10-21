//! Owned simulation-suspension claims. Distinct from capture and from
//! install / remove. Releasing one token cannot release another owner's freeze.

use bevy::prelude::*;

use crate::token::CapabilityToken;

/// What a suspension freezes. Capture is not a scope — it only redirects input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SuspensionScope {
	/// Player realization (movement, items). NPCs and installed motors stay.
	Gameplay,
	/// Broader world freeze. Pause does not use this.
	Simulation,
}

/// One owner's freeze of one scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SuspensionClaim {
	token: CapabilityToken,
	scope: SuspensionScope,
}

impl SuspensionClaim {
	pub const fn new(token: CapabilityToken, scope: SuspensionScope) -> Self {
		Self { token, scope }
	}

	/// Session pause: freeze player realization, keep capabilities.
	pub const fn pause_gameplay() -> Self {
		Self::new(CapabilityToken::PAUSE, SuspensionScope::Gameplay)
	}

	pub const fn token(self) -> CapabilityToken {
		self.token
	}

	pub const fn scope(self) -> SuspensionScope {
		self.scope
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

	pub fn freezes(&self, scope: SuspensionScope) -> bool {
		self.claims.iter().any(|claim| claim.scope() == scope)
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
		stack.set(SuspensionClaim::new(other, SuspensionScope::Simulation), true);
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
		stack.set(SuspensionClaim::new(other, SuspensionScope::Simulation), true);
		stack.release_play();
		assert!(!stack.owns(CapabilityToken::PAUSE));
		assert!(stack.owns(other));
	}
}
