//! Test helper other lanes can use against the published interface.
//!
//! Enable the `test-support` feature from a consumer's `dev-dependencies`.

use bevy::prelude::*;

use crate::activation::LubikanActivationRequest;
use crate::catalog::{LubikanCatalog, LubikanSelfConfig};
use crate::change::LubikanChargeChanged;
use crate::identity::LubikanDefinition;
use crate::ledger::{LubikanLedger, LubikanOwner};
use crate::lifetime::LubikanOwnerLifetime;
use crate::{LubikanInterfacePlugin, LubikansPlugin};

/// Register the shared messages and default Self catalog / config.
pub fn install_interface(app: &mut App) {
	if !app.is_plugin_added::<LubikanInterfacePlugin>() {
		app.add_plugins(LubikanInterfacePlugin);
	}
	app.init_resource::<LubikanSelfConfig>();
	if app.world().get_resource::<LubikanCatalog>().is_none() {
		let config = *app.world().resource::<LubikanSelfConfig>();
		app.insert_resource(LubikanCatalog::with_self(&config));
	}
}

/// Register the gameplay plugin (Self, owner attach, activation).
pub fn install_gameplay(app: &mut App) {
	if !app.is_plugin_added::<LubikansPlugin>() {
		app.add_plugins(LubikansPlugin);
	}
}

/// Spawn an owner that already has Self unlocked and equipped.
pub fn spawn_self_owner(world: &mut World) -> Entity {
	let config = world.get_resource::<LubikanSelfConfig>().copied().unwrap_or_default();
	if world.get_resource::<LubikanCatalog>().is_none() {
		world.insert_resource(LubikanCatalog::with_self(&config));
	}
	world.spawn((LubikanOwner, LubikanLedger::with_self(&config))).id()
}

/// Unlock an extra definition on `owner` so selection-switch tests have a
/// second distinct account.
pub fn unlock_on(
	world: &mut World,
	owner: Entity,
	definition: LubikanDefinition,
) -> Option<crate::LubikanAccountId> {
	if let Some(mut catalog) = world.get_resource_mut::<LubikanCatalog>() {
		catalog.register(definition);
	}
	world
		.get_mut::<LubikanLedger>(owner)
		.map(|mut ledger| ledger.ensure_owned(definition))
}

/// Owner plus the ledger snapshot taken at spawn time.
pub struct LubikanFixture {
	pub owner: Entity,
	pub config: LubikanSelfConfig,
}

impl LubikanFixture {
	pub fn spawn(world: &mut World) -> Self {
		let config = world.get_resource::<LubikanSelfConfig>().copied().unwrap_or_default();
		Self { owner: spawn_self_owner(world), config }
	}

	pub fn spawn_in(app: &mut App) -> Self {
		install_interface(app);
		Self::spawn(app.world_mut())
	}

	pub fn ledger<'a>(&self, world: &'a World) -> Option<&'a LubikanLedger> {
		world.get::<LubikanLedger>(self.owner)
	}

	pub fn ledger_mut<'a>(&self, world: &'a mut World) -> Option<Mut<'a, LubikanLedger>> {
		world.get_mut::<LubikanLedger>(self.owner)
	}

	pub fn unlock(
		&self,
		world: &mut World,
		definition: LubikanDefinition,
	) -> Option<crate::LubikanAccountId> {
		unlock_on(world, self.owner, definition)
	}

	pub fn write_change(world: &mut World, change: LubikanChargeChanged) {
		world.write_message(change);
	}

	pub fn request_equipped(world: &mut World, owner: Entity) {
		world.write_message(LubikanActivationRequest::equipped(owner));
	}

	pub fn write_lifetime(world: &mut World, lifetime: LubikanOwnerLifetime) {
		world.write_message(lifetime);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::identity::{LubikanAccountId, LubikanDefinitionId};
	use crate::{LubikanActivationRefused, LubikanActivationReport, LubikanActivationSession};
	use anyhow::{anyhow, Result};

	#[test]
	fn fixture_gives_consumers_a_self_owner() -> Result<()> {
		let mut app = App::new();
		let fixture = LubikanFixture::spawn_in(&mut app);
		let ledger = fixture.ledger(app.world()).ok_or_else(|| anyhow!("ledger"))?;
		if !ledger.selection().is_self() {
			return Err(anyhow!("new owners start on Self"));
		}
		assert_eq!(ledger.equipped().id(), LubikanAccountId::SELF);
		assert!(app.world().get_resource::<LubikanCatalog>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanChargeChanged>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationRequest>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationSession>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationRefused>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationReport>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanOwnerLifetime>>().is_some());
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(2), 15.0, 5.0);
		let id = fixture.unlock(app.world_mut(), extra).ok_or_else(|| anyhow!("unlock"))?;
		assert_ne!(id, LubikanAccountId::SELF);
		Ok(())
	}
}
