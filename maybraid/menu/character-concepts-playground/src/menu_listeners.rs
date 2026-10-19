use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{CharacterMenuEvent, MenuButton, ToggleSectionKey};
use character_creation_menus::{CharacterMenu, MenuEvent, SectionId};

use crate::{
	camera_focus::{focus_debug_enabled, queue_camera_focus, PendingCameraFocus},
	focus_reference::FocusReferenceSyncState,
	preview::{ConceptPreviewConfig, ConceptPreviewSyncState, PreviewRespawnCooldown},
	species_session::{reset_for_species_switch, CameraFocusBootState, SpeciesSessionState},
};

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CharacterMenuState(pub CharacterMenu);

impl Default for CharacterMenuState {
	fn default() -> Self {
		Self(CharacterMenu::default())
	}
}

pub fn menu_to_preview_config(menu: &CharacterMenu) -> ConceptPreviewConfig {
	ConceptPreviewConfig::new(menu.species_config(), menu.animation())
}

pub fn menu_from_preview_config(config: &ConceptPreviewConfig) -> CharacterMenu {
	CharacterMenu::from_species_config(config.species_config(), config.animation())
}

pub fn init_character_menu_state(
	config: Res<ConceptPreviewConfig>,
	mut menu_state: ResMut<CharacterMenuState>,
) {
	menu_state.0 = menu_from_preview_config(&config);
}

pub fn sync_menu_state_from_config(
	config: Res<ConceptPreviewConfig>,
	mut menu_state: ResMut<CharacterMenuState>,
) {
	if !config.is_changed() {
		return;
	}
	menu_state.0 = menu_from_preview_config(&config);
}

pub fn dispatch_menu_interactions(
	mut menu_state: ResMut<CharacterMenuState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<CharacterMenu>>,
	mut section_interactions: Query<
		(&Interaction, &ToggleSectionKey),
		(Changed<Interaction>, With<Button>),
	>,
	mut menu_interactions: Query<
		(&Interaction, &MenuButton<MenuEvent>),
		(Changed<Interaction>, With<Button>, Without<ToggleSectionKey>),
	>,
	mut config: ResMut<ConceptPreviewConfig>,
	mut ui_state: ResMut<crate::ui::CreatorUiState>,
	mut ui_sync: ResMut<crate::ui::CreatorUiSyncState>,
	mut session: ResMut<SpeciesSessionState>,
	mut preview_sync: ResMut<ConceptPreviewSyncState>,
	mut focus_sync: ResMut<FocusReferenceSyncState>,
	mut respawn_cooldown: ResMut<PreviewRespawnCooldown>,
	mut pending_camera: ResMut<PendingCameraFocus>,
	mut camera_boot: ResMut<CameraFocusBootState>,
) {
	for (interaction, toggle) in &mut section_interactions {
		if *interaction != Interaction::Pressed {
			continue;
		}
		if let Some(section) = section_id_for_label(toggle.0) {
			ui_state.toggle_section(section);
		}
	}

	for (interaction, button) in &mut menu_interactions {
		if *interaction == Interaction::Hovered {
			if let Some(focus) = menu_state.0.camera_focus_for_event(button.0) {
				ui_state.hovered = Some(focus);
			}
			continue;
		}
		if *interaction != Interaction::Pressed {
			continue;
		}
		let event = button.0;
		if let MenuEvent::SetSpecies(species) = event {
			if config.species() != species {
				reset_for_species_switch(
					species,
					&mut session,
					&mut config,
					&mut ui_state,
					&mut preview_sync,
					&mut focus_sync,
					&mut respawn_cooldown,
					&mut pending_camera,
					&mut camera_boot,
				);
				menu_state.0 = menu_from_preview_config(&config);
				crate::ui::mark_menu_ui_dirty(&mut ui_sync);
				menu_events.write(CharacterMenuEvent::MenuUpdate(menu_state.0.clone()));
			}
			continue;
		}
		if let Some(focus) = menu_state.0.camera_focus_for_event(event) {
			ui_state.last_selected = Some(focus);
			menu_events.write(CharacterMenuEvent::CameraFocus(focus));
		}
		if !menu_state.0.apply(event) {
			continue;
		}
		crate::ui::mark_menu_ui_dirty(&mut ui_sync);
		menu_events.write(CharacterMenuEvent::MenuUpdate(menu_state.0.clone()));
		if focus_debug_enabled() {
			info!("[camera-focus] typed-ui event={event:?}");
		}
	}
}

pub fn on_character_menu_event(
	mut events: MessageReader<CharacterMenuEvent<CharacterMenu>>,
	mut config: ResMut<ConceptPreviewConfig>,
	mut preview_sync: ResMut<ConceptPreviewSyncState>,
	mut focus_sync: ResMut<FocusReferenceSyncState>,
	mut respawn_cooldown: ResMut<PreviewRespawnCooldown>,
	mut pending_camera: ResMut<PendingCameraFocus>,
	mut ui_state: ResMut<crate::ui::CreatorUiState>,
	mut ui_sync: ResMut<crate::ui::CreatorUiSyncState>,
) {
	for event in events.read() {
		match event {
			CharacterMenuEvent::MenuUpdate(menu) => {
				*config = menu_to_preview_config(menu);
				preview_sync.invalidate_live();
				focus_sync.invalidate_live();
				respawn_cooldown.frames_remaining = 0;
				crate::ui::mark_menu_ui_dirty(&mut ui_sync);
			}
			CharacterMenuEvent::CameraFocus(focus) => {
				queue_camera_focus(&mut pending_camera, *focus, "menu-event");
				ui_state.last_selected = Some(*focus);
			}
		}
	}
}

fn section_id_for_label(label: &'static str) -> Option<SectionId> {
	match label {
		"Presets" => Some(SectionId::Presets),
		"Head" => Some(SectionId::Head),
		"Body" => Some(SectionId::Body),
		"Head & Features" => Some(SectionId::HeadFeatures),
		"Hair" => Some(SectionId::Hair),
		"Clothing" => Some(SectionId::Clothing),
		"Weapons" => Some(SectionId::Weapons),
		"Skill Maps" => Some(SectionId::SkillMaps),
		"Loadout" => Some(SectionId::Loadout),
		"Animation" => Some(SectionId::Animation),
		_ => None,
	}
}
