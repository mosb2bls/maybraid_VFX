//! Coordinate domain cleanup when an actor is retired. Each domain owns the
//! actual teardown; this adapter holds no persistent state.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use damage::DespawnAfter;
use firearm_user::FirearmUser;
use firearms::WeaponTrigger;
use grenade_user::GrenadeUser;
use maybraid_equipment::EquippedItemHandler;
use maybraid_session::{ActorRetired, SessionLifecycleSystems};
use tanz_user::{TanzExecution, TanzStack, TanzUser};

/// Apply domain teardown after hosts write [`ActorRetired`].
pub fn configure(app: &mut App) {
	app.add_systems(
		PostUpdate,
		apply_retired_actor_domains.in_set(SessionLifecycleSystems::Coordinate),
	);
}

fn apply_retired_actor_domains(
	mut retired: MessageReader<ActorRetired>,
	actors: Query<(
		Option<&FirearmUser>,
		Option<&GrenadeUser>,
		Option<&InventoryUser>,
		Has<TanzUser>,
	)>,
	mut commands: Commands,
) {
	for notice in retired.read() {
		let Ok((firearm, grenade, inventory, tanz)) = actors.get(notice.entity()) else {
			continue;
		};
		ActorDomainTeardown::new(notice.entity())
			.with_firearm(firearm)
			.with_grenade(grenade)
			.with_inventory(inventory)
			.with_tanz(tanz)
			.apply(&mut commands);
	}
}

/// Domain handles present on a retiring actor. World still owns motor strip,
/// vegetation markers, and loadout persistence.
#[derive(Clone, Copy, Debug)]
pub struct ActorDomainTeardown {
	entity: Entity,
	firearm: Option<FirearmUser>,
	grenade: Option<GrenadeUser>,
	inventory: Option<InventoryUser>,
	tanz: bool,
}

impl ActorDomainTeardown {
	pub const fn new(entity: Entity) -> Self {
		Self { entity, firearm: None, grenade: None, inventory: None, tanz: false }
	}

	pub fn with_firearm(mut self, firearm: Option<&FirearmUser>) -> Self {
		self.firearm = firearm.copied();
		self
	}

	pub fn with_grenade(mut self, grenade: Option<&GrenadeUser>) -> Self {
		self.grenade = grenade.copied();
		self
	}

	pub fn with_inventory(mut self, inventory: Option<&InventoryUser>) -> Self {
		self.inventory = inventory.copied();
		self
	}

	pub const fn with_tanz(mut self, tanz: bool) -> Self {
		self.tanz = tanz;
		self
	}

	pub const fn entity(self) -> Entity {
		self.entity
	}

	pub const fn firearm(self) -> Option<FirearmUser> {
		self.firearm
	}

	pub const fn grenade(self) -> Option<GrenadeUser> {
		self.grenade
	}

	pub const fn inventory(self) -> Option<InventoryUser> {
		self.inventory
	}

	/// Firearm trigger off + delayed kit despawn, grenade / bag despawn, and
	/// drop of item / Lubikan / Tanz markers. Does not strip world motors.
	pub fn apply(self, commands: &mut Commands) {
		if let Some(firearm) = self.firearm {
			commands
				.entity(firearm.held)
				.insert((WeaponTrigger(false), DespawnAfter::seconds(0.0)));
		}
		if let Some(grenade) = self.grenade {
			commands.entity(grenade.held).try_despawn();
		}
		if let Some(inventory) = self.inventory {
			commands.entity(inventory.bag).try_despawn();
		}
		let mut entity = commands.entity(self.entity);
		EquippedItemHandler::clear(&mut entity);
		entity.remove::<(
			FirearmUser,
			GrenadeUser,
			grenade_user::GrenadeThrow,
			InventoryUser,
			lubikans::LubikanOwner,
		)>();
		if self.tanz {
			entity.remove::<(TanzUser, TanzStack, TanzExecution)>();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn apply_cleans_item_domains_without_storing_state() {
		let mut world = World::new();
		let held = world.spawn(WeaponTrigger(true)).id();
		let bag = world.spawn_empty().id();
		let actor = world
			.spawn((
				FirearmUser::holding(held),
				InventoryUser::carrying(bag),
				EquippedItemHandler::Firearm,
				lubikans::LubikanOwner,
				TanzUser::default(),
				TanzStack::default(),
				TanzExecution::default(),
			))
			.id();
		let teardown = ActorDomainTeardown::new(actor)
			.with_firearm(world.get::<FirearmUser>(actor))
			.with_inventory(world.get::<InventoryUser>(actor))
			.with_tanz(true);
		teardown.apply(&mut world.commands());
		world.flush();
		assert_eq!(world.get::<WeaponTrigger>(held).map(|trigger| trigger.0), Some(false));
		assert!(world.get::<DespawnAfter>(held).is_some());
		assert!(!world.entities().contains(bag));
		assert!(world.get::<FirearmUser>(actor).is_none());
		assert!(world.get::<InventoryUser>(actor).is_none());
		assert!(world.get::<EquippedItemHandler>(actor).is_none());
		assert!(world.get::<lubikans::LubikanOwner>(actor).is_none());
		assert!(world.get::<TanzUser>(actor).is_none());
	}
}
