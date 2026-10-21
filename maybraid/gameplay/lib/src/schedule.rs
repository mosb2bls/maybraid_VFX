//! Cross-domain schedule constraints. Domain-internal ordering stays in the
//! domain crate.

use bevy::prelude::*;
use lubikans::LubikanSystems;
use maybraid_character_controller::CharacterControlSystems;
use maybraid_controllers_core::ControllerPipeline;
use maybraid_equipment::EquipmentSystems;
use maybraid_session::{SessionCapabilitySystems, WorldGameplayEnabled};
use player::PlayerSystems;

/// Order player intent and equipment against session / Lubikan owner.
pub fn configure(app: &mut App) {
	app.configure_sets(
		Update,
		PlayerSystems::Intent
			.after(CharacterControlSystems)
			.after(SessionCapabilitySystems)
			.before(ControllerPipeline::Motor),
	);
	app.configure_sets(
		Update,
		(
			EquipmentSystems::SelectCategory
				.after(CharacterControlSystems)
				.after(SessionCapabilitySystems)
				.after(LubikanSystems::Owner)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
			EquipmentSystems::BeginHolster
				.after(CharacterControlSystems)
				.after(SessionCapabilitySystems)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
			EquipmentSystems::CycleQueue
				.after(CharacterControlSystems)
				.after(SessionCapabilitySystems)
				.after(LubikanSystems::Owner)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
			EquipmentSystems::CommitHolster
				.after(CharacterControlSystems)
				.after(SessionCapabilitySystems)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
		),
	);
}
