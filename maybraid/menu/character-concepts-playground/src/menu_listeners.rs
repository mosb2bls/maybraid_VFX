use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{CharacterMenuEvent, MenuButton, ToggleSectionKey};
use character_creation_menus::{CharacterMenu, ConceptSpecies, MenuEvent, SectionId};

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
	match menu.species.value {
		ConceptSpecies::Braidman => {
			ConceptPreviewConfig::braidman_with_animation(menu.braidman_config(), menu.animation())
		}
		ConceptSpecies::Brenal => {
			ConceptPreviewConfig::brenal_with_animation(menu.brenal_config(), menu.animation())
		}
		ConceptSpecies::Caole => {
			ConceptPreviewConfig::caole_with_animation(menu.caole_config(), menu.animation())
		}
		ConceptSpecies::Epiphant => {
			ConceptPreviewConfig::epiphant_with_animation(menu.epiphant_config(), menu.animation())
		}
		ConceptSpecies::Hars => {
			ConceptPreviewConfig::hars_with_animation(menu.hars_config(), menu.animation())
		}
		ConceptSpecies::Yilter => {
			ConceptPreviewConfig::ylter_with_animation(menu.ylter_config(), menu.animation())
		}
		ConceptSpecies::Sonyak => {
			ConceptPreviewConfig::sonyak_with_animation(menu.sonyak_config(), menu.animation())
		}
		ConceptSpecies::Claber => {
			ConceptPreviewConfig::claber_with_animation(menu.claber_config(), menu.animation())
		}
		ConceptSpecies::Croconot => {
			ConceptPreviewConfig::croconot_with_animation(menu.croconot_config(), menu.animation())
		}
		ConceptSpecies::Brodler => {
			ConceptPreviewConfig::brodler_with_animation(menu.brodler_config(), menu.animation())
		}
		ConceptSpecies::Mygr => {
			ConceptPreviewConfig::mygr_with_animation(menu.mygr_config(), menu.animation())
		}
		ConceptSpecies::Dui => {
			ConceptPreviewConfig::dui_with_animation(menu.dui_config(), menu.animation())
		}
		ConceptSpecies::Lidder => {
			ConceptPreviewConfig::lidder_with_animation(menu.lidder_config(), menu.animation())
		}
		ConceptSpecies::Chupri => {
			ConceptPreviewConfig::chupri_with_animation(menu.chupri_config(), menu.animation())
		}
		ConceptSpecies::Brokker => {
			ConceptPreviewConfig::brokker_with_animation(menu.brokker_config(), menu.animation())
		}
		ConceptSpecies::Tipple => {
			ConceptPreviewConfig::tipple_with_animation(menu.tipple_config(), menu.animation())
		}
		ConceptSpecies::Topple => {
			ConceptPreviewConfig::topple_with_animation(menu.topple_config(), menu.animation())
		}
		ConceptSpecies::Kispar => {
			ConceptPreviewConfig::kispar_with_animation(menu.kispar_config(), menu.animation())
		}
		ConceptSpecies::Tapp => {
			ConceptPreviewConfig::tapp_with_animation(menu.tapp_config(), menu.animation())
		}
		ConceptSpecies::Kaller => {
			ConceptPreviewConfig::kaller_with_animation(menu.kaller_config(), menu.animation())
		}
		ConceptSpecies::Kappler => {
			ConceptPreviewConfig::kappler_with_animation(menu.kappler_config(), menu.animation())
		}
		ConceptSpecies::Wumbus => {
			ConceptPreviewConfig::wumbus_with_animation(menu.wumbus_config(), menu.animation())
		}
		ConceptSpecies::Lero => {
			ConceptPreviewConfig::lero_with_animation(menu.lero_config(), menu.animation())
		}
		ConceptSpecies::Spibmom => {
			ConceptPreviewConfig::spibmom_with_animation(menu.spibmom_config(), menu.animation())
		}
		ConceptSpecies::Grener => {
			ConceptPreviewConfig::grener_with_animation(menu.grener_config(), menu.animation())
		}
		ConceptSpecies::Thumplus => {
			ConceptPreviewConfig::thumplus_with_animation(menu.thumplus_config(), menu.animation())
		}
		ConceptSpecies::Mistler => {
			ConceptPreviewConfig::mistler_with_animation(menu.mistler_config(), menu.animation())
		}
		ConceptSpecies::Tuberwaber => ConceptPreviewConfig::tuberwaber_with_animation(
			menu.tuberwaber_config(),
			menu.animation(),
		),
	}
}

pub fn menu_from_preview_config(config: &ConceptPreviewConfig) -> CharacterMenu {
	match config {
		ConceptPreviewConfig::Braidman { config, animation } => {
			CharacterMenu::from_braidman(config, *animation)
		}
		ConceptPreviewConfig::Brenal { config, animation } => {
			CharacterMenu::from_brenal(config, *animation)
		}
		ConceptPreviewConfig::Caole { config, animation } => {
			CharacterMenu::from_caole(config, *animation)
		}
		ConceptPreviewConfig::Epiphant { config, animation } => {
			CharacterMenu::from_epiphant(config, *animation)
		}
		ConceptPreviewConfig::Hars { config, animation } => {
			CharacterMenu::from_hars(config, *animation)
		}
		ConceptPreviewConfig::Yilter { config, animation } => {
			CharacterMenu::from_ylter(config, *animation)
		}
		ConceptPreviewConfig::Sonyak { config, animation } => {
			CharacterMenu::from_sonyak(config, *animation)
		}
		ConceptPreviewConfig::Claber { config, animation } => {
			CharacterMenu::from_claber(config, *animation)
		}
		ConceptPreviewConfig::Croconot { config, animation } => {
			CharacterMenu::from_croconot(config, *animation)
		}
		ConceptPreviewConfig::Brodler { config, animation } => {
			CharacterMenu::from_brodler(config, *animation)
		}
		ConceptPreviewConfig::Mygr { config, animation } => {
			CharacterMenu::from_mygr(config, *animation)
		}
		ConceptPreviewConfig::Dui { config, animation } => {
			CharacterMenu::from_dui(config, *animation)
		}
		ConceptPreviewConfig::Lidder { config, animation } => {
			CharacterMenu::from_lidder(config, *animation)
		}
		ConceptPreviewConfig::Chupri { config, animation } => {
			CharacterMenu::from_chupri(config, *animation)
		}
		ConceptPreviewConfig::Brokker { config, animation } => {
			CharacterMenu::from_brokker(config, *animation)
		}
		ConceptPreviewConfig::Tipple { config, animation } => {
			CharacterMenu::from_tipple(config, *animation)
		}
		ConceptPreviewConfig::Topple { config, animation } => {
			CharacterMenu::from_topple(config, *animation)
		}
		ConceptPreviewConfig::Kispar { config, animation } => {
			CharacterMenu::from_kispar(config, *animation)
		}
		ConceptPreviewConfig::Tapp { config, animation } => {
			CharacterMenu::from_tapp(config, *animation)
		}
		ConceptPreviewConfig::Kaller { config, animation } => {
			CharacterMenu::from_kaller(config, *animation)
		}
		ConceptPreviewConfig::Kappler { config, animation } => {
			CharacterMenu::from_kappler(config, *animation)
		}
		ConceptPreviewConfig::Wumbus { config, animation } => {
			CharacterMenu::from_wumbus(config, *animation)
		}
		ConceptPreviewConfig::Lero { config, animation } => {
			CharacterMenu::from_lero(config, *animation)
		}
		ConceptPreviewConfig::Spibmom { config, animation } => {
			CharacterMenu::from_spibmom(config, *animation)
		}
		ConceptPreviewConfig::Grener { config, animation } => {
			CharacterMenu::from_grener(config, *animation)
		}
		ConceptPreviewConfig::Thumplus { config, animation } => {
			CharacterMenu::from_thumplus(config, *animation)
		}
		ConceptPreviewConfig::Mistler { config, animation } => {
			CharacterMenu::from_mistler(config, *animation)
		}
		ConceptPreviewConfig::Tuberwaber { config, animation } => {
			CharacterMenu::from_tuberwaber(config, *animation)
		}
	}
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

fn preview_species(species: ConceptSpecies) -> crate::preview::ConceptSpecies {
	match species {
		ConceptSpecies::Braidman => crate::preview::ConceptSpecies::Braidman,
		ConceptSpecies::Brenal => crate::preview::ConceptSpecies::Brenal,
		ConceptSpecies::Caole => crate::preview::ConceptSpecies::Caole,
		ConceptSpecies::Epiphant => crate::preview::ConceptSpecies::Epiphant,
		ConceptSpecies::Hars => crate::preview::ConceptSpecies::Hars,
		ConceptSpecies::Yilter => crate::preview::ConceptSpecies::Yilter,
		ConceptSpecies::Sonyak => crate::preview::ConceptSpecies::Sonyak,
		ConceptSpecies::Claber => crate::preview::ConceptSpecies::Claber,
		ConceptSpecies::Croconot => crate::preview::ConceptSpecies::Croconot,
		ConceptSpecies::Brodler => crate::preview::ConceptSpecies::Brodler,
		ConceptSpecies::Mygr => crate::preview::ConceptSpecies::Mygr,
		ConceptSpecies::Dui => crate::preview::ConceptSpecies::Dui,
		ConceptSpecies::Lidder => crate::preview::ConceptSpecies::Lidder,
		ConceptSpecies::Chupri => crate::preview::ConceptSpecies::Chupri,
		ConceptSpecies::Brokker => crate::preview::ConceptSpecies::Brokker,
		ConceptSpecies::Tipple => crate::preview::ConceptSpecies::Tipple,
		ConceptSpecies::Topple => crate::preview::ConceptSpecies::Topple,
		ConceptSpecies::Kispar => crate::preview::ConceptSpecies::Kispar,
		ConceptSpecies::Tapp => crate::preview::ConceptSpecies::Tapp,
		ConceptSpecies::Kaller => crate::preview::ConceptSpecies::Kaller,
		ConceptSpecies::Kappler => crate::preview::ConceptSpecies::Kappler,
		ConceptSpecies::Wumbus => crate::preview::ConceptSpecies::Wumbus,
		ConceptSpecies::Lero => crate::preview::ConceptSpecies::Lero,
		ConceptSpecies::Spibmom => crate::preview::ConceptSpecies::Spibmom,
		ConceptSpecies::Grener => crate::preview::ConceptSpecies::Grener,
		ConceptSpecies::Thumplus => crate::preview::ConceptSpecies::Thumplus,
		ConceptSpecies::Mistler => crate::preview::ConceptSpecies::Mistler,
		ConceptSpecies::Tuberwaber => crate::preview::ConceptSpecies::Tuberwaber,
	}
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
			let preview_species = preview_species(species);
			if config.species() != preview_species {
				reset_for_species_switch(
					preview_species,
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
