//! Shared Lubikan interface.
//!
//! Other lanes consume these types. Self is a normal definition; a missing
//! equipped selection resolves to Self. Charge and activation live in
//! gameplay — HUD construction must not create them.
//!
//! This crate does not depend on Bikanwarda.

mod activation;
mod catalog;
mod change;
mod charge;
mod identity;
mod ledger;
mod lifetime;
mod plugin;
mod selection;

#[cfg(any(test, feature = "test-support"))]
pub mod fixtures;

use bevy::prelude::*;

pub use activation::{
	LubikanActivation, LubikanActivationDenied, LubikanActivationGrant, LubikanActivationOutcome,
	LubikanActivationPhase, LubikanActivationRefused, LubikanActivationReport,
	LubikanActivationRequest, LubikanActivationSession, LubikanActivationSessionId,
	LubikanReadiness,
};
pub use catalog::{
	LubikanCatalog, LubikanSelfConfig, DEFAULT_SELF_ACTIVATION_COST, DEFAULT_SELF_CAPACITY,
};
pub use change::{LubikanChargeChanged, LubikanChargeReason};
pub use charge::{LubikanCharge, LubikanChargeMutation};
pub use identity::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};
pub use ledger::{LubikanAccount, LubikanAccountBinding, LubikanLedger, LubikanOwner};
pub use lifetime::{
	LubikanLifetimePolicy, LubikanOwnerLifetime, LubikanOwnerLifetimeKind, LubikanPersistence,
};
pub use plugin::{LubikanSessionStore, LubikansPlugin};
pub use selection::LubikanSelection;

/// Schedule sets other lanes order against. The interface plugin does not
/// run systems in these sets; gameplay plugins do.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LubikanSystems {
	/// Attach, snapshot, and restore owner ledgers.
	Owner,
	/// Sole owner of [`LubikanActivationRequest`]s: spend once and emit a
	/// [`LubikanActivationSession`], or emit [`LubikanActivationRefused`].
	Activate,
	/// Apply charge rules and emit [`LubikanChargeChanged`].
	Award,
}

/// Registers the shared messages. Does not spawn owners or insert Self into
/// a world — that is gameplay, and stays independent of HUD visibility.
pub struct LubikanInterfacePlugin;

impl Plugin for LubikanInterfacePlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<LubikanChargeChanged>()
			.add_message::<LubikanActivationRequest>()
			.add_message::<LubikanActivationSession>()
			.add_message::<LubikanActivationRefused>()
			.add_message::<LubikanActivationReport>()
			.add_message::<LubikanOwnerLifetime>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn interface_plugin_registers_shared_messages() {
		let mut app = App::new();
		app.add_plugins(LubikanInterfacePlugin);
		assert!(app.world().get_resource::<Messages<LubikanChargeChanged>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationRequest>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationSession>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationRefused>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationReport>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanOwnerLifetime>>().is_some());
	}
}
