//! Per-species build memory and runtime reset when switching species.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
	camera_focus::{queue_species_default_camera_focus, PendingCameraFocus},
	focus_reference::FocusReferenceSyncState,
	preview::{
		ConceptPreviewConfig, ConceptPreviewSyncState, ConceptSpecies, PreviewRespawnCooldown,
	},
	ui::CreatorUiState,
};

/// Remembers each species' last config so switching tabs restores prior builds.
#[derive(Resource, Debug, Clone, Default)]
pub struct SpeciesSessionState {
	saved: HashMap<ConceptSpecies, ConceptPreviewConfig>,
}

impl SpeciesSessionState {
	pub fn persist(&mut self, preview: &ConceptPreviewConfig) {
		self.saved.insert(preview.species(), preview.clone());
	}

	pub fn load(&self, species: ConceptSpecies) -> ConceptPreviewConfig {
		self.saved
			.get(&species)
			.cloned()
			.unwrap_or_else(|| ConceptPreviewConfig::default_for(species))
	}
}

/// Tracks which species default camera framing was last applied for.
#[derive(Resource, Default)]
pub struct CameraFocusBootState {
	pub applied_for: Option<ConceptSpecies>,
}

pub fn persist_species_session(
	config: Res<ConceptPreviewConfig>,
	mut session: ResMut<SpeciesSessionState>,
) {
	if config.is_changed() {
		session.persist(&config);
	}
}

pub fn invalidate_species_runtime(
	preview_sync: &mut ConceptPreviewSyncState,
	focus_sync: &mut FocusReferenceSyncState,
	cooldown: &mut PreviewRespawnCooldown,
) {
	preview_sync.invalidate();
	focus_sync.invalidate();
	cooldown.frames_remaining = 0;
}

pub fn reset_for_species_switch(
	species: ConceptSpecies,
	session: &mut SpeciesSessionState,
	config: &mut ConceptPreviewConfig,
	ui_state: &mut CreatorUiState,
	preview_sync: &mut ConceptPreviewSyncState,
	focus_sync: &mut FocusReferenceSyncState,
	cooldown: &mut PreviewRespawnCooldown,
	pending_camera: &mut PendingCameraFocus,
	camera_boot: &mut CameraFocusBootState,
) {
	session.persist(config);
	*config = session.load(species);
	invalidate_species_runtime(preview_sync, focus_sync, cooldown);
	ui_state.hovered = None;
	ui_state.last_selected = None;
	ui_state.bump_layout_revision();
	pending_camera.focus = None;
	pending_camera.resolved_target = None;
	pending_camera.focus_trigger = None;
	camera_boot.applied_for = None;
	queue_species_default_camera_focus(pending_camera, ui_state, config, "species-switch");
	camera_boot.applied_for = Some(species);
}

pub fn ensure_species_camera_focus(
	config: Res<ConceptPreviewConfig>,
	capture: Option<Res<crate::preview_capture::CaptureCameraLocked>>,
	mut ui_state: ResMut<CreatorUiState>,
	mut pending: ResMut<PendingCameraFocus>,
	mut boot: ResMut<CameraFocusBootState>,
) {
	if capture.is_some() {
		return;
	}
	let species = config.species();
	if boot.applied_for == Some(species) {
		return;
	}
	boot.applied_for = Some(species);
	queue_species_default_camera_focus(&mut pending, &mut ui_state, &config, "startup-default");
}

#[cfg(test)]
mod tests {
	use characters::{
		presets::GenderPreset,
		species::{
			brenal::{BrenalConfig, BrenalHornMesh},
			epiphant::EpiphantConfig,
		},
	};

	use super::*;
	use crate::animation::ConceptAnimation;

	#[test]
	fn load_uses_default_preview_until_persisted() {
		let session = SpeciesSessionState::default();
		for &species in ConceptSpecies::VALUES {
			let loaded = session.load(species);
			assert_eq!(loaded, ConceptPreviewConfig::default_for(species));
			assert_eq!(loaded.animation(), ConceptAnimation::default());
		}
	}

	#[test]
	fn persist_remembers_config_and_animation_independently() {
		let mut session = SpeciesSessionState::default();

		let mut brenal = BrenalConfig::default_preview();
		brenal.horns = BrenalHornMesh::HarrowedCrown;
		let brenal_preview =
			ConceptPreviewConfig::brenal_with_animation(brenal, ConceptAnimation::Gallop);
		session.persist(&brenal_preview);

		let mut epiphant = EpiphantConfig::default_preview();
		epiphant.gender = GenderPreset::Female;
		let epiphant_preview =
			ConceptPreviewConfig::epiphant_with_animation(epiphant, ConceptAnimation::EpiphantBray);
		session.persist(&epiphant_preview);

		assert_eq!(session.load(ConceptSpecies::Brenal), brenal_preview);
		assert_eq!(session.load(ConceptSpecies::Epiphant), epiphant_preview);
		assert_eq!(
			session.load(ConceptSpecies::Braidman),
			ConceptPreviewConfig::default_for(ConceptSpecies::Braidman)
		);
	}

	#[test]
	fn persist_overwrites_the_same_species() {
		let mut session = SpeciesSessionState::default();
		session.persist(&ConceptPreviewConfig::brenal_with_animation(
			BrenalConfig::default_preview(),
			ConceptAnimation::Run,
		));
		let mut brenal = BrenalConfig::default_preview();
		brenal.horns = BrenalHornMesh::HarrowedCrown;
		let updated = ConceptPreviewConfig::brenal_with_animation(brenal, ConceptAnimation::Gallop);
		session.persist(&updated);
		assert_eq!(session.load(ConceptSpecies::Brenal), updated);
	}
}
