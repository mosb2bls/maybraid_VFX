//! Nested [`LodScene`] host registration for character recipes.

use bevy::app::SceneSpawnerSystems;
use bevy::prelude::*;
use lod::{add_lod_refresh_chunk_for, LodRefreshSystems, LodScene};
use rigs::{RigPlugin, RigSystems};

use crate::components::{CharacterComponents, ComponentsOnly};
use crate::skin::{
	fulfill_skin_ref_roots, invalidate_changed_skin_ref_roots, prune_duplicate_part_scenes,
	remap_part_skin_to_rig,
};
use crate::terrain_pitch::prepare_character_terrain_pitch;

/// Register chunk fulfill for a structural [`ComponentsOnly<C>`] host.
///
/// [`crate::RigNode`] / [`crate::PartNode`] stamp socket, skin, pose, and material
/// refs from [`lod::LodScene::host`] / [`lod::LodScene::scene_with_level`]. Each
/// species only adds its recipe type here (typically [`crate::Clothed<T>`]).
pub fn add_character_components_host<C>(app: &mut App)
where
	C: CharacterComponents + Send + Sync + 'static,
	ComponentsOnly<C>: Component + LodScene,
{
	add_lod_refresh_chunk_for::<ComponentsOnly<C>>(app);
}

/// Realize loop for nested character hosts: membership, bone map, refs, pose.
///
/// Shared armature work lives in [`RigSystems`]. [`Self::InvalidateRefs`] also
/// drops skin `*Applied`. [`Self::Fulfill`] remaps skin and prunes duplicate
/// GLB armatures after sockets. Clip mailbox lives in
/// [`character_motion::CharacterMotionSystems::Anim`].
pub type CharacterHostSystems = RigSystems;

/// Nested preview hosts still spawn as LodScene; runtime fixed assemblies drain first.
pub struct CharacterComponentsPlugin;

impl Plugin for CharacterComponentsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<RigPlugin>() {
			app.add_plugins(RigPlugin);
		}
		app.configure_sets(
			Update,
			CharacterHostSystems::Membership.after(LodRefreshSystems::Fulfill),
		);
		app.add_systems(
			Update,
			invalidate_changed_skin_ref_roots.in_set(CharacterHostSystems::InvalidateRefs),
		);
		app.add_systems(
			Update,
			(
				fulfill_skin_ref_roots,
				remap_part_skin_to_rig
					.after(fulfill_skin_ref_roots)
					.after(SceneSpawnerSystems::WorldInstanceSpawn),
				prune_duplicate_part_scenes.after(remap_part_skin_to_rig),
			)
				.in_set(CharacterHostSystems::Fulfill),
		);
		app.add_systems(
			Update,
			crate::fixed::drain_character_assembly.before(CharacterHostSystems::Membership),
		);
		app.add_systems(
			Update,
			crate::fixed::reveal_ready_fixed_members.after(SceneSpawnerSystems::WorldInstanceSpawn),
		);
		app.init_resource::<crate::fixed::CharacterAssemblyBudget>()
			.init_resource::<crate::fixed::CharacterAssemblyDiagnostics>();
		app.add_systems(Update, prepare_character_terrain_pitch.after(CharacterHostSystems::Pose));
	}
}
