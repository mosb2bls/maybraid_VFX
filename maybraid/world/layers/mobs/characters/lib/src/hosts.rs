//! Register nested character LodScene hosts (all species) plus fulfill plugins.

use bevy::prelude::*;
use character_shaders::CharacterShadersPlugin;
use lod::{add_lod_refresh_chunk_for, LodRefreshCorePlugin};
use material_ref::StandardMaterialRefPlugin;
use scene_ref::SceneRefPlugin;

use character_motion::{CharacterMotionPlugin, CharacterMotionSystems};

use crate::material_lib::CharacterMaterialRefPlugin;

use crate::plugin::{
	add_character_components_host, CharacterComponentsPlugin, CharacterHostSystems,
};
use crate::species::{
	braidman::Braidman, brenal::Brenal, brodler::Brodler, brokker::Brokker, caole::Caole,
	chupri::Chupri, claber::Claber, croconot::Croconot, dui::Dui, epiphant::Epiphant,
	grener::Grener, hars::Hars, kaller::Kaller, kappler::Kappler, kispar::Kispar, lero::Lero,
	lidder::Lidder, mistler::Mistler, mygr::Mygr, sonyak::Sonyak, spibmom::Spibmom, tapp::Tapp,
	thumplus::Thumplus, tipple::Tipple, topple::Topple, tuberwaber::Tuberwaber, wumbus::Wumbus,
	ylter::Yilter,
};
use crate::{Clothed, PartNode, RigNode};

/// Scene-ref, material-ref, LOD refresh, fixed-assembly drain, and every clothed species host.
///
/// World/mob/player runtime spawn uses [`crate::spawn_fixed_character_visual`].
/// Playgrounds that still spawn [`crate::ComponentsOnly`] via [`lod::LodScene::host`] add this once.
pub struct CharacterHostsPlugin;

impl Plugin for CharacterHostsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<CharacterShadersPlugin>() {
			app.add_plugins(CharacterShadersPlugin);
		}
		if !app.is_plugin_added::<CharacterMaterialRefPlugin>() {
			app.add_plugins(CharacterMaterialRefPlugin);
		}
		if !app.is_plugin_added::<StandardMaterialRefPlugin>() {
			app.add_plugins(StandardMaterialRefPlugin);
		}
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<CharacterComponentsPlugin>() {
			app.add_plugins(CharacterComponentsPlugin);
		}
		if !app.is_plugin_added::<CharacterMotionPlugin>() {
			app.add_plugins(CharacterMotionPlugin);
		}
		app.configure_sets(Update, CharacterMotionSystems::Anim.after(CharacterHostSystems::Pose));
		add_lod_refresh_chunk_for::<RigNode>(app);
		add_lod_refresh_chunk_for::<PartNode>(app);
		add_character_components_host::<Clothed<Braidman>>(app);
		add_character_components_host::<Clothed<Brenal>>(app);
		add_character_components_host::<Clothed<Brodler>>(app);
		add_character_components_host::<Clothed<Brokker>>(app);
		add_character_components_host::<Clothed<Caole>>(app);
		add_character_components_host::<Clothed<Chupri>>(app);
		add_character_components_host::<Clothed<Claber>>(app);
		add_character_components_host::<Clothed<Croconot>>(app);
		add_character_components_host::<Clothed<Dui>>(app);
		add_character_components_host::<Clothed<Epiphant>>(app);
		add_character_components_host::<Clothed<Grener>>(app);
		add_character_components_host::<Clothed<Hars>>(app);
		add_character_components_host::<Clothed<Kaller>>(app);
		add_character_components_host::<Clothed<Kappler>>(app);
		add_character_components_host::<Clothed<Kispar>>(app);
		add_character_components_host::<Clothed<Lero>>(app);
		add_character_components_host::<Clothed<Lidder>>(app);
		add_character_components_host::<Clothed<Mistler>>(app);
		add_character_components_host::<Clothed<Mygr>>(app);
		add_character_components_host::<Clothed<Sonyak>>(app);
		add_character_components_host::<Clothed<Spibmom>>(app);
		add_character_components_host::<Clothed<Tapp>>(app);
		add_character_components_host::<Clothed<Thumplus>>(app);
		add_character_components_host::<Clothed<Tipple>>(app);
		add_character_components_host::<Clothed<Topple>>(app);
		add_character_components_host::<Clothed<Tuberwaber>>(app);
		add_character_components_host::<Clothed<Wumbus>>(app);
		add_character_components_host::<Clothed<Yilter>>(app);
	}
}
