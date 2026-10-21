//! Explicit source → target addressing. No query-wide broadcast.

use crate::actor::ActorTarget;
use crate::source::InputSource;
use crate::ui::UiTarget;

/// Destination of a routed sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RouteTarget {
	Actor(ActorTarget),
	Ui(UiTarget),
}

impl RouteTarget {
	pub const fn actor(target: ActorTarget) -> Self {
		Self::Actor(target)
	}

	pub const fn ui(target: UiTarget) -> Self {
		Self::Ui(target)
	}

	pub const fn actor_entity(self) -> Option<bevy::prelude::Entity> {
		match self {
			Self::Actor(target) => Some(target.entity()),
			Self::Ui(_) => None,
		}
	}

	pub const fn ui_entity(self) -> Option<bevy::prelude::Entity> {
		match self {
			Self::Ui(target) => Some(target.entity()),
			Self::Actor(_) => None,
		}
	}
}

/// Sample or event with an explicit source and destination.
#[derive(Clone, Copy, Debug)]
pub struct Routed<T> {
	source: InputSource,
	target: RouteTarget,
	payload: T,
}

impl<T> Routed<T> {
	pub const fn new(source: InputSource, target: RouteTarget, payload: T) -> Self {
		Self { source, target, payload }
	}

	pub const fn to_ui(source: InputSource, target: UiTarget, payload: T) -> Self {
		Self::new(source, RouteTarget::ui(target), payload)
	}

	pub const fn to_actor(source: InputSource, target: ActorTarget, payload: T) -> Self {
		Self::new(source, RouteTarget::actor(target), payload)
	}

	pub const fn source(&self) -> InputSource {
		self.source
	}

	pub const fn target(&self) -> RouteTarget {
		self.target
	}

	pub const fn payload(&self) -> &T {
		&self.payload
	}

	pub fn into_payload(self) -> T {
		self.payload
	}

	pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Routed<U> {
		Routed { source: self.source, target: self.target, payload: map(self.payload) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Entity;

	#[test]
	fn routed_menu_nav_is_ui_local() {
		let menu = UiTarget::new(Entity::from_bits(11));
		let routed = Routed::to_ui(InputSource::local_pad(), menu, "down");
		assert_eq!(routed.source(), InputSource::local_pad());
		assert_eq!(routed.target().ui_entity(), Some(menu.entity()));
		assert_eq!(routed.target().actor_entity(), None);
		assert_eq!(*routed.payload(), "down");
	}

	#[test]
	fn routed_actor_sample_is_actor_local() {
		let actor = ActorTarget::new(Entity::from_bits(5));
		let routed = Routed::to_actor(InputSource::local_pad(), actor, 1u8);
		assert_eq!(routed.target().actor_entity(), Some(actor.entity()));
		assert_eq!(routed.into_payload(), 1);
	}
}
