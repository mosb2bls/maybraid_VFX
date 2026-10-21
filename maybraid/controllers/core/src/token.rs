//! Owner identity for install, capture, and suspension claims.
//!
//! [#1442](https://github.com/ramate-io/maybraid/issues/1442) will supply
//! assignment tokens. Those convert into this handle; releasing one token
//! cannot release another owner's claim.

use bevy::prelude::*;

use crate::actor::ActorTarget;
use crate::ui::UiTarget;

/// Opaque owner of a capture or suspension claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapabilityToken(u64);

impl CapabilityToken {
	pub const CONSOLE: Self = Self::well_known(1);
	pub const MAP: Self = Self::well_known(2);
	pub const AWAITING: Self = Self::well_known(3);
	pub const TEXT_LOCK: Self = Self::well_known(4);
	pub const PAUSE: Self = Self::well_known(10);

	pub const fn from_bits(bits: u64) -> Self {
		Self(bits)
	}

	pub const fn well_known(tag: u8) -> Self {
		Self(u64::MAX - tag as u64)
	}

	pub const fn from_entity(entity: Entity) -> Self {
		Self(entity.to_bits())
	}

	pub const fn from_ui(target: UiTarget) -> Self {
		Self::from_entity(target.entity())
	}

	pub const fn from_actor(target: ActorTarget) -> Self {
		Self::from_entity(target.entity())
	}

	pub const fn bits(self) -> u64 {
		self.0
	}

	/// Temporary: menu dispatch still addresses [`UiTarget`] until #1442
	/// tokens carry assignment. One call site to delete.
	pub const fn as_ui_target(self) -> UiTarget {
		UiTarget::new(Entity::from_bits(self.0))
	}

	/// Temporary: actor addressing still uses [`ActorTarget`] until #1442
	/// assignment tokens land. One conversion to delete.
	pub const fn as_actor_target(self) -> ActorTarget {
		ActorTarget::new(Entity::from_bits(self.0))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn well_known_tokens_are_distinct() {
		assert_ne!(CapabilityToken::CONSOLE, CapabilityToken::TEXT_LOCK);
		assert_ne!(CapabilityToken::MAP, CapabilityToken::PAUSE);
		assert_ne!(CapabilityToken::AWAITING, CapabilityToken::CONSOLE);
	}

	#[test]
	fn entity_tokens_round_trip_ui_and_actor() {
		let entity = Entity::from_bits(42);
		let token = CapabilityToken::from_entity(entity);
		assert_eq!(token.as_ui_target().entity(), entity);
		assert_eq!(token.as_actor_target().entity(), entity);
		assert_eq!(CapabilityToken::from_ui(UiTarget::new(entity)), token);
	}
}
