//! World gameplay gates for equipment selection. Category and queue ownership
//! live in `maybraid-equipment`.

use bevy::prelude::*;
use game_commands::command::TextEntryFocus;
use lubikans::LubikanSystems;
use maybraid_character_controller::CharacterControlSystems;
use maybraid_equipment::{EquipmentPlugin, EquipmentSystems};
use maybraid_world_controller::SessionCapabilitySystems;

use crate::control::WorldGameplayEnabled;
use crate::map_view::WorldMapView;

pub use maybraid_equipment::ActiveInventoryCyclingCategory;

pub(crate) fn gameplay_cycling_allowed(
	gameplay: &WorldGameplayEnabled,
	text_focus: &TextEntryFocus,
	map: Option<&WorldMapView>,
) -> bool {
	gameplay.accepts_character_input(text_focus.0, map.is_some_and(|map| map.open))
}

pub(crate) fn configure(app: &mut App) {
	if !app.is_plugin_added::<EquipmentPlugin>() {
		app.add_plugins(EquipmentPlugin);
	}
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
