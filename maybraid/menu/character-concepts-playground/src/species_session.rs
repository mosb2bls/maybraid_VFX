//! Per-species build memory and runtime reset when switching species.

use bevy::prelude::*;
use characters::species::{
	braidman::BraidmanConfig, brenal::BrenalConfig, brodler::BrodlerConfig, brokker::BrokkerConfig,
	caole::CaoleConfig, chupri::ChupriConfig, claber::ClaberConfig, croconot::CroconotConfig,
	dui::DuiConfig, epiphant::EpiphantConfig, grener::GrenerConfig, hars::HarsConfig,
	kaller::KallerConfig, kappler::KapplerConfig, kispar::KisparConfig, lero::LeroConfig,
	lidder::LidderConfig, mistler::MistlerConfig, mygr::MygrConfig, sonyak::SonyakConfig,
	spibmom::SpibmomConfig, tapp::TappConfig, thumplus::ThumplusConfig, tipple::TippleConfig,
	topple::ToppleConfig, tuberwaber::TuberwaberConfig, wumbus::WumbusConfig, ylter::YilterConfig,
};

use crate::{
	camera_focus::{queue_species_default_camera_focus, PendingCameraFocus},
	focus_reference::FocusReferenceSyncState,
	preview::{
		ConceptPreviewConfig, ConceptPreviewSyncState, ConceptSpecies, PreviewRespawnCooldown,
	},
	ui::CreatorUiState,
};

/// Remembers each species' last config so switching tabs restores prior builds.
#[derive(Resource, Debug, Clone)]
pub struct SpeciesSessionState {
	pub braidman: BraidmanConfig,
	pub brenal: BrenalConfig,
	pub caole: CaoleConfig,
	pub epiphant: EpiphantConfig,
	pub hars: HarsConfig,
	pub ylter: YilterConfig,
	pub sonyak: SonyakConfig,
	pub claber: ClaberConfig,
	pub croconot: CroconotConfig,
	pub brodler: BrodlerConfig,
	pub mygr: MygrConfig,
	pub dui: DuiConfig,
	pub lidder: LidderConfig,
	pub chupri: ChupriConfig,
	pub brokker: BrokkerConfig,
	pub tipple: TippleConfig,
	pub topple: ToppleConfig,
	pub kispar: KisparConfig,
	pub tapp: TappConfig,
	pub kaller: KallerConfig,
	pub kappler: KapplerConfig,
	pub wumbus: WumbusConfig,
	pub lero: LeroConfig,
	pub spibmom: SpibmomConfig,
	pub grener: GrenerConfig,
	pub thumplus: ThumplusConfig,
	pub mistler: MistlerConfig,
	pub tuberwaber: TuberwaberConfig,
	pub caole_animation: crate::animation::ConceptAnimation,
	pub epiphant_animation: crate::animation::ConceptAnimation,
	pub hars_animation: crate::animation::ConceptAnimation,
	pub ylter_animation: crate::animation::ConceptAnimation,
	pub sonyak_animation: crate::animation::ConceptAnimation,
	pub brenal_animation: crate::animation::ConceptAnimation,
	pub claber_animation: crate::animation::ConceptAnimation,
	pub croconot_animation: crate::animation::ConceptAnimation,
	pub braidman_animation: crate::animation::ConceptAnimation,
	pub brodler_animation: crate::animation::ConceptAnimation,
	pub mygr_animation: crate::animation::ConceptAnimation,
	pub dui_animation: crate::animation::ConceptAnimation,
	pub lidder_animation: crate::animation::ConceptAnimation,
	pub chupri_animation: crate::animation::ConceptAnimation,
	pub brokker_animation: crate::animation::ConceptAnimation,
	pub tipple_animation: crate::animation::ConceptAnimation,
	pub topple_animation: crate::animation::ConceptAnimation,
	pub kispar_animation: crate::animation::ConceptAnimation,
	pub tapp_animation: crate::animation::ConceptAnimation,
	pub kaller_animation: crate::animation::ConceptAnimation,
	pub kappler_animation: crate::animation::ConceptAnimation,
	pub wumbus_animation: crate::animation::ConceptAnimation,
	pub lero_animation: crate::animation::ConceptAnimation,
	pub spibmom_animation: crate::animation::ConceptAnimation,
	pub grener_animation: crate::animation::ConceptAnimation,
	pub thumplus_animation: crate::animation::ConceptAnimation,
	pub mistler_animation: crate::animation::ConceptAnimation,
	pub tuberwaber_animation: crate::animation::ConceptAnimation,
}

impl Default for SpeciesSessionState {
	fn default() -> Self {
		Self {
			braidman: BraidmanConfig::default_preview(),
			brenal: BrenalConfig::default_preview(),
			caole: CaoleConfig::default_preview(),
			epiphant: EpiphantConfig::default_preview(),
			hars: HarsConfig::default_preview(),
			ylter: YilterConfig::default_preview(),
			sonyak: SonyakConfig::default_preview(),
			claber: ClaberConfig::default_preview(),
			croconot: CroconotConfig::default_preview(),
			brodler: BrodlerConfig::default_preview(),
			mygr: MygrConfig::default_preview(),
			dui: DuiConfig::default_preview(),
			lidder: LidderConfig::default_preview(),
			chupri: ChupriConfig::default_preview(),
			brokker: BrokkerConfig::default_preview(),
			tipple: TippleConfig::default_preview(),
			topple: ToppleConfig::default_preview(),
			kispar: KisparConfig::default_preview(),
			tapp: TappConfig::default_preview(),
			kaller: KallerConfig::default_preview(),
			kappler: KapplerConfig::default_preview(),
			wumbus: WumbusConfig::default_preview(),
			lero: LeroConfig::default_preview(),
			spibmom: SpibmomConfig::default_preview(),
			grener: GrenerConfig::default_preview(),
			thumplus: ThumplusConfig::default_preview(),
			mistler: MistlerConfig::default_preview(),
			tuberwaber: TuberwaberConfig::default_preview(),
			brenal_animation: crate::animation::ConceptAnimation::default(),
			caole_animation: crate::animation::ConceptAnimation::default(),
			epiphant_animation: crate::animation::ConceptAnimation::default(),
			hars_animation: crate::animation::ConceptAnimation::default(),
			ylter_animation: crate::animation::ConceptAnimation::default(),
			sonyak_animation: crate::animation::ConceptAnimation::default(),
			claber_animation: crate::animation::ConceptAnimation::default(),
			croconot_animation: crate::animation::ConceptAnimation::default(),
			braidman_animation: crate::animation::ConceptAnimation::default(),
			brodler_animation: crate::animation::ConceptAnimation::default(),
			mygr_animation: crate::animation::ConceptAnimation::default(),
			dui_animation: crate::animation::ConceptAnimation::default(),
			lidder_animation: crate::animation::ConceptAnimation::default(),
			chupri_animation: crate::animation::ConceptAnimation::default(),
			brokker_animation: crate::animation::ConceptAnimation::default(),
			tipple_animation: crate::animation::ConceptAnimation::default(),
			topple_animation: crate::animation::ConceptAnimation::default(),
			kispar_animation: crate::animation::ConceptAnimation::default(),
			tapp_animation: crate::animation::ConceptAnimation::default(),
			kaller_animation: crate::animation::ConceptAnimation::default(),
			kappler_animation: crate::animation::ConceptAnimation::default(),
			wumbus_animation: crate::animation::ConceptAnimation::default(),
			lero_animation: crate::animation::ConceptAnimation::default(),
			spibmom_animation: crate::animation::ConceptAnimation::default(),
			grener_animation: crate::animation::ConceptAnimation::default(),
			thumplus_animation: crate::animation::ConceptAnimation::default(),
			mistler_animation: crate::animation::ConceptAnimation::default(),
			tuberwaber_animation: crate::animation::ConceptAnimation::default(),
		}
	}
}

impl SpeciesSessionState {
	pub fn persist(&mut self, config: &ConceptPreviewConfig) {
		match config {
			ConceptPreviewConfig::Braidman { config, animation } => {
				self.braidman.clone_from(config);
				self.braidman_animation = *animation;
			}
			ConceptPreviewConfig::Brenal { config, animation } => {
				self.brenal.clone_from(config);
				self.brenal_animation = *animation;
			}
			ConceptPreviewConfig::Caole { config, animation } => {
				self.caole.clone_from(config);
				self.caole_animation = *animation;
			}
			ConceptPreviewConfig::Epiphant { config, animation } => {
				self.epiphant.clone_from(config);
				self.epiphant_animation = *animation;
			}
			ConceptPreviewConfig::Hars { config, animation } => {
				self.hars.clone_from(config);
				self.hars_animation = *animation;
			}
			ConceptPreviewConfig::Yilter { config, animation } => {
				self.ylter.clone_from(config);
				self.ylter_animation = *animation;
			}
			ConceptPreviewConfig::Sonyak { config, animation } => {
				self.sonyak.clone_from(config);
				self.sonyak_animation = *animation;
			}
			ConceptPreviewConfig::Claber { config, animation } => {
				self.claber.clone_from(config);
				self.claber_animation = *animation;
			}
			ConceptPreviewConfig::Croconot { config, animation } => {
				self.croconot.clone_from(config);
				self.croconot_animation = *animation;
			}
			ConceptPreviewConfig::Brodler { config, animation } => {
				self.brodler.clone_from(config);
				self.brodler_animation = *animation;
			}
			ConceptPreviewConfig::Mygr { config, animation } => {
				self.mygr.clone_from(config);
				self.mygr_animation = *animation;
			}
			ConceptPreviewConfig::Dui { config, animation } => {
				self.dui.clone_from(config);
				self.dui_animation = *animation;
			}
			ConceptPreviewConfig::Lidder { config, animation } => {
				self.lidder.clone_from(config);
				self.lidder_animation = *animation;
			}
			ConceptPreviewConfig::Chupri { config, animation } => {
				self.chupri.clone_from(config);
				self.chupri_animation = *animation;
			}
			ConceptPreviewConfig::Brokker { config, animation } => {
				self.brokker.clone_from(config);
				self.brokker_animation = *animation;
			}
			ConceptPreviewConfig::Tipple { config, animation } => {
				self.tipple.clone_from(config);
				self.tipple_animation = *animation;
			}
			ConceptPreviewConfig::Topple { config, animation } => {
				self.topple.clone_from(config);
				self.topple_animation = *animation;
			}
			ConceptPreviewConfig::Kispar { config, animation } => {
				self.kispar.clone_from(config);
				self.kispar_animation = *animation;
			}
			ConceptPreviewConfig::Tapp { config, animation } => {
				self.tapp.clone_from(config);
				self.tapp_animation = *animation;
			}
			ConceptPreviewConfig::Kaller { config, animation } => {
				self.kaller.clone_from(config);
				self.kaller_animation = *animation;
			}
			ConceptPreviewConfig::Kappler { config, animation } => {
				self.kappler.clone_from(config);
				self.kappler_animation = *animation;
			}
			ConceptPreviewConfig::Wumbus { config, animation } => {
				self.wumbus.clone_from(config);
				self.wumbus_animation = *animation;
			}
			ConceptPreviewConfig::Lero { config, animation } => {
				self.lero.clone_from(config);
				self.lero_animation = *animation;
			}
			ConceptPreviewConfig::Spibmom { config, animation } => {
				self.spibmom.clone_from(config);
				self.spibmom_animation = *animation;
			}
			ConceptPreviewConfig::Grener { config, animation } => {
				self.grener.clone_from(config);
				self.grener_animation = *animation;
			}
			ConceptPreviewConfig::Thumplus { config, animation } => {
				self.thumplus.clone_from(config);
				self.thumplus_animation = *animation;
			}
			ConceptPreviewConfig::Mistler { config, animation } => {
				self.mistler.clone_from(config);
				self.mistler_animation = *animation;
			}
			ConceptPreviewConfig::Tuberwaber { config, animation } => {
				self.tuberwaber.clone_from(config);
				self.tuberwaber_animation = *animation;
			}
		}
	}

	pub fn load(&self, species: ConceptSpecies) -> ConceptPreviewConfig {
		match species {
			ConceptSpecies::Braidman => ConceptPreviewConfig::braidman_with_animation(
				self.braidman.clone(),
				self.braidman_animation,
			),
			ConceptSpecies::Brenal => ConceptPreviewConfig::brenal_with_animation(
				self.brenal.clone(),
				self.brenal_animation,
			),
			ConceptSpecies::Caole => {
				ConceptPreviewConfig::caole_with_animation(self.caole.clone(), self.caole_animation)
			}
			ConceptSpecies::Epiphant => ConceptPreviewConfig::epiphant_with_animation(
				self.epiphant.clone(),
				self.epiphant_animation,
			),
			ConceptSpecies::Hars => {
				ConceptPreviewConfig::hars_with_animation(self.hars.clone(), self.hars_animation)
			}
			ConceptSpecies::Yilter => {
				ConceptPreviewConfig::ylter_with_animation(self.ylter.clone(), self.ylter_animation)
			}
			ConceptSpecies::Sonyak => ConceptPreviewConfig::sonyak_with_animation(
				self.sonyak.clone(),
				self.sonyak_animation,
			),
			ConceptSpecies::Claber => ConceptPreviewConfig::claber_with_animation(
				self.claber.clone(),
				self.claber_animation,
			),
			ConceptSpecies::Croconot => ConceptPreviewConfig::croconot_with_animation(
				self.croconot.clone(),
				self.croconot_animation,
			),
			ConceptSpecies::Brodler => ConceptPreviewConfig::brodler_with_animation(
				self.brodler.clone(),
				self.brodler_animation,
			),
			ConceptSpecies::Mygr => {
				ConceptPreviewConfig::mygr_with_animation(self.mygr.clone(), self.mygr_animation)
			}
			ConceptSpecies::Dui => {
				ConceptPreviewConfig::dui_with_animation(self.dui.clone(), self.dui_animation)
			}
			ConceptSpecies::Lidder => ConceptPreviewConfig::lidder_with_animation(
				self.lidder.clone(),
				self.lidder_animation,
			),
			ConceptSpecies::Chupri => ConceptPreviewConfig::chupri_with_animation(
				self.chupri.clone(),
				self.chupri_animation,
			),
			ConceptSpecies::Brokker => ConceptPreviewConfig::brokker_with_animation(
				self.brokker.clone(),
				self.brokker_animation,
			),
			ConceptSpecies::Tipple => ConceptPreviewConfig::tipple_with_animation(
				self.tipple.clone(),
				self.tipple_animation,
			),
			ConceptSpecies::Topple => ConceptPreviewConfig::topple_with_animation(
				self.topple.clone(),
				self.topple_animation,
			),
			ConceptSpecies::Kispar => ConceptPreviewConfig::kispar_with_animation(
				self.kispar.clone(),
				self.kispar_animation,
			),
			ConceptSpecies::Tapp => {
				ConceptPreviewConfig::tapp_with_animation(self.tapp.clone(), self.tapp_animation)
			}
			ConceptSpecies::Kaller => ConceptPreviewConfig::kaller_with_animation(
				self.kaller.clone(),
				self.kaller_animation,
			),
			ConceptSpecies::Kappler => ConceptPreviewConfig::kappler_with_animation(
				self.kappler.clone(),
				self.kappler_animation,
			),
			ConceptSpecies::Wumbus => ConceptPreviewConfig::wumbus_with_animation(
				self.wumbus.clone(),
				self.wumbus_animation,
			),
			ConceptSpecies::Lero => {
				ConceptPreviewConfig::lero_with_animation(self.lero.clone(), self.lero_animation)
			}
			ConceptSpecies::Spibmom => ConceptPreviewConfig::spibmom_with_animation(
				self.spibmom.clone(),
				self.spibmom_animation,
			),
			ConceptSpecies::Grener => ConceptPreviewConfig::grener_with_animation(
				self.grener.clone(),
				self.grener_animation,
			),
			ConceptSpecies::Thumplus => ConceptPreviewConfig::thumplus_with_animation(
				self.thumplus.clone(),
				self.thumplus_animation,
			),
			ConceptSpecies::Mistler => ConceptPreviewConfig::mistler_with_animation(
				self.mistler.clone(),
				self.mistler_animation,
			),
			ConceptSpecies::Tuberwaber => ConceptPreviewConfig::tuberwaber_with_animation(
				self.tuberwaber.clone(),
				self.tuberwaber_animation,
			),
		}
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
	mut ui_state: ResMut<CreatorUiState>,
	mut pending: ResMut<PendingCameraFocus>,
	mut boot: ResMut<CameraFocusBootState>,
) {
	let species = config.species();
	if boot.applied_for == Some(species) {
		return;
	}
	boot.applied_for = Some(species);
	queue_species_default_camera_focus(&mut pending, &mut ui_state, &config, "startup-default");
}
