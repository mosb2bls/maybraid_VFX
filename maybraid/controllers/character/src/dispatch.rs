//! D1: addressed equipped-item use. Inventory selection and the holster
//! commit stamp [`EquippedItemHandler`]; this system is the only Bind writer
//! of [`EquippedItemUse`] / [`EquippedPowerUse`].

use bevy::prelude::*;

use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};

use crate::intent::CharacterIntent;

/// Which UseItem interpreter owns this actor. Written by inventory / swap
/// commit, not by the pad.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EquippedItemHandler {
	#[default]
	None,
	Firearm,
	Grenade,
}

impl EquippedItemHandler {
	pub const fn accepts_use(self) -> bool {
		!matches!(self, Self::None)
	}

	pub const fn is_firearm(self) -> bool {
		matches!(self, Self::Firearm)
	}

	pub const fn is_grenade(self) -> bool {
		matches!(self, Self::Grenade)
	}
}

/// Addressed UseItem for the delivered actor's active handler.
///
/// Written every delivered frame so interpreters can tell **held**, **released**,
/// and **not delivered** (capture / suspend) apart. Absence of this message is
/// not a release.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct EquippedItemUse {
	actor: ActorTarget,
	handler: EquippedItemHandler,
	squeeze: f32,
	held: bool,
}

impl EquippedItemUse {
	pub const fn new(
		actor: ActorTarget,
		handler: EquippedItemHandler,
		squeeze: f32,
		held: bool,
	) -> Self {
		Self { actor, handler, squeeze, held }
	}

	pub const fn actor(self) -> ActorTarget {
		self.actor
	}

	pub const fn handler(self) -> EquippedItemHandler {
		self.handler
	}

	pub const fn squeeze(self) -> f32 {
		self.squeeze
	}

	pub const fn held(self) -> bool {
		self.held
	}

	pub fn addresses(self, entity: Entity) -> bool {
		self.actor.matches(entity)
	}
}

/// Addressed PowerUseItem. Lubikan activation consumes this, not the
/// broadcast intent.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquippedPowerUse {
	actor: ActorTarget,
}

impl EquippedPowerUse {
	pub const fn new(actor: ActorTarget) -> Self {
		Self { actor }
	}

	pub const fn actor(self) -> ActorTarget {
		self.actor
	}

	pub fn addresses(self, entity: Entity) -> bool {
		self.actor.matches(entity)
	}
}

/// Stamp [`EquippedItemUse`] / [`EquippedPowerUse`] for
/// [`InputRouter::delivered_actor`] only.
pub fn dispatch_equipped_item(
	router: Res<InputRouter>,
	mut intents: MessageReader<CharacterIntent>,
	handlers: Query<&EquippedItemHandler>,
	mut uses: MessageWriter<EquippedItemUse>,
	mut powers: MessageWriter<EquippedPowerUse>,
) {
	let mut squeeze = None;
	let mut power = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::UseItem(value) => squeeze = Some(value.max(squeeze.unwrap_or(0.0))),
			CharacterIntent::PowerUseItem => power = true,
			_ => {}
		}
	}
	let Some(actor) = router.delivered_actor(InputSource::local_pad()) else {
		return;
	};
	let handler = handlers.get(actor.entity()).ok().copied().unwrap_or_default();
	if handler.accepts_use() {
		let held = squeeze.is_some();
		uses.write(EquippedItemUse::new(actor, handler, squeeze.unwrap_or(0.0), held));
	}
	if power {
		powers.write(EquippedPowerUse::new(actor));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	fn actor(bits: u64) -> ActorTarget {
		ActorTarget::new(Entity::from_bits(bits))
	}

	fn write_intents(world: &mut World, intents: &[CharacterIntent]) {
		world
			.run_system_once({
				let intents = intents.to_vec();
				move |mut writer: MessageWriter<CharacterIntent>| {
					for intent in &intents {
						writer.write(*intent);
					}
				}
			})
			.expect("write");
	}

	#[test]
	fn dispatch_requires_delivery_and_a_handler() {
		let mut world = World::new();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<EquippedItemUse>>();
		world.init_resource::<Messages<EquippedPowerUse>>();
		let entity = world.spawn(EquippedItemHandler::Firearm).id();
		write_intents(&mut world, &[CharacterIntent::UseItem(0.9)]);
		world.run_system_once(dispatch_equipped_item).expect("undelivered");
		assert!(world.resource_mut::<Messages<EquippedItemUse>>().drain().next().is_none());

		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(entity));
		write_intents(&mut world, &[CharacterIntent::UseItem(0.9)]);
		world.run_system_once(dispatch_equipped_item).expect("delivered");
		let use_item = world
			.resource_mut::<Messages<EquippedItemUse>>()
			.drain()
			.next()
			.expect("use");
		assert!(use_item.addresses(entity));
		assert!(use_item.handler().is_firearm());
		assert!(use_item.held());
		assert!((use_item.squeeze() - 0.9).abs() < 1e-5);
	}

	#[test]
	fn delivered_release_is_an_explicit_not_held() {
		let mut world = World::new();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<EquippedItemUse>>();
		world.init_resource::<Messages<EquippedPowerUse>>();
		let entity = world.spawn(EquippedItemHandler::Grenade).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(entity));
		world.run_system_once(dispatch_equipped_item).expect("release");
		let use_item = world
			.resource_mut::<Messages<EquippedItemUse>>()
			.drain()
			.next()
			.expect("use");
		assert!(use_item.handler().is_grenade());
		assert!(!use_item.held());
	}

	#[test]
	fn capture_writes_no_use() {
		use maybraid_controllers_core::{CaptureKind, ScopeOwner, UiTarget};

		let mut world = World::new();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<EquippedItemUse>>();
		world.init_resource::<Messages<EquippedPowerUse>>();
		let entity = world.spawn(EquippedItemHandler::Firearm).id();
		{
			let mut router = world.resource_mut::<InputRouter>();
			router.assign_actor(InputSource::local_pad(), ActorTarget::new(entity));
			router.acquire_capture(
				InputSource::local_pad(),
				ScopeOwner::ui(UiTarget::new(Entity::from_bits(9))),
				CaptureKind::Overlay,
			);
		}
		write_intents(&mut world, &[CharacterIntent::UseItem(1.0)]);
		world.run_system_once(dispatch_equipped_item).expect("captured");
		assert!(world.resource_mut::<Messages<EquippedItemUse>>().drain().next().is_none());
	}

	#[test]
	fn none_handler_drops_use_but_keeps_power() {
		let mut world = World::new();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<EquippedItemUse>>();
		world.init_resource::<Messages<EquippedPowerUse>>();
		let entity = world.spawn(EquippedItemHandler::None).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(entity));
		write_intents(
			&mut world,
			&[CharacterIntent::UseItem(1.0), CharacterIntent::PowerUseItem],
		);
		world.run_system_once(dispatch_equipped_item).expect("power");
		assert!(world.resource_mut::<Messages<EquippedItemUse>>().drain().next().is_none());
		let power = world
			.resource_mut::<Messages<EquippedPowerUse>>()
			.drain()
			.next()
			.expect("power");
		assert!(power.addresses(entity));
		assert_eq!(power.actor(), actor(entity.to_bits()));
	}
}
