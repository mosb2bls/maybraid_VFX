//! Idempotent plugin for Richmond development models.

use bevy::prelude::*;
use building_components::{
	apply_parent_confines, FurnitureWireframePlugin, LabelNode, LabelWireframePlugin,
	MassingSilhouettePlugin,
};
use building_physics::BuildingWalkColliderPlugin;
use building_shaders::{BuildingShadersPlugin, UrbanMaterialRefPlugin};
use buildings::wizards_tower::TowerSilhouettePlugin;
use lod::LodRefreshSystems;
use lod_lazy_refs::LodLazyRefsPlugin;
use scene_ref::SceneRefPlugin;

use crate::buildings_lod::register_developments_buildings_lod_plugin;
use crate::config::DevelopmentConfig;
use crate::place::DiscoverablePlace;
/// Registers SceneRef, urban MaterialRef, placeholder wireframes, building LOD, and walk colliders.
#[derive(Default)]
pub struct RichmondDevelopmentModelsPlugin;

/// Idempotent registration of [`RichmondDevelopmentModelsPlugin`].
pub fn register_richmond_plugin(app: &mut App) {
	if app.is_plugin_added::<RichmondDevelopmentModelsPlugin>() {
		return;
	}
	app.add_plugins(RichmondDevelopmentModelsPlugin);
}

impl Plugin for RichmondDevelopmentModelsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<LodLazyRefsPlugin>() {
			app.add_plugins(LodLazyRefsPlugin);
		}
		if !app.is_plugin_added::<BuildingShadersPlugin>() {
			app.add_plugins(BuildingShadersPlugin);
		}
		if !app.is_plugin_added::<UrbanMaterialRefPlugin>() {
			app.add_plugins(UrbanMaterialRefPlugin);
		}
		if !app.is_plugin_added::<FurnitureWireframePlugin>() {
			app.add_plugins(FurnitureWireframePlugin);
		}
		if !app.is_plugin_added::<LabelWireframePlugin>() {
			app.add_plugins(LabelWireframePlugin);
		}
		if !app.is_plugin_added::<TowerSilhouettePlugin>() {
			app.add_plugins(TowerSilhouettePlugin);
		}
		if !app.is_plugin_added::<MassingSilhouettePlugin>() {
			app.add_plugins(MassingSilhouettePlugin);
		}
		if !app.is_plugin_added::<BuildingWalkColliderPlugin>() {
			app.add_plugins(BuildingWalkColliderPlugin);
		}
		register_developments_buildings_lod_plugin(app);

		app.init_resource::<DevelopmentConfig>()
			.add_systems(Update, apply_parent_confines.after(LodRefreshSystems::Cull))
			.add_systems(Update, stamp_label_places);
	}
}

/// High usage-area labels become extra local places while those children exist.
///
/// When an ancestor host already has a development identity, the High place
/// inherits that host and gets a local id from the authored room pose.
#[allow(clippy::type_complexity)]
fn stamp_label_places(
	mut commands: Commands,
	added: Query<
		(Entity, &LabelNode, Option<&ChildOf>),
		(Added<LabelNode>, Without<DiscoverablePlace>),
	>,
	ancestors: Query<(Option<&ChildOf>, Option<&DiscoverablePlace>)>,
) {
	for (entity, node, child_of) in &added {
		let Some(mut place) = DiscoverablePlace::from_label_node(node) else {
			continue;
		};
		if let Some(host) = ancestor_host(child_of.map(ChildOf::parent), &ancestors) {
			place = place.with_identity(host, local_place_id(node));
		}
		commands.entity(entity).insert(place);
	}
}

fn ancestor_host(
	start: Option<Entity>,
	ancestors: &Query<(Option<&ChildOf>, Option<&DiscoverablePlace>)>,
) -> Option<lod::gen::Id> {
	let mut current = start;
	let mut guard = 0;
	while let Some(entity) = current {
		guard += 1;
		if guard > 32 {
			break;
		}
		let Ok((child_of, place)) = ancestors.get(entity) else {
			break;
		};
		if let Some(host) = place.and_then(|place| place.host) {
			return Some(host);
		}
		current = child_of.map(ChildOf::parent);
	}
	None
}

/// Durable room id from the authored usage-area pose, not the display label.
fn local_place_id(node: &LabelNode) -> u32 {
	let translation = node.placement.translation;
	let extents = node.geometry.extents();
	let mut h = 2_166_131_261u32;
	for bits in [
		translation.x.to_bits(),
		translation.y.to_bits(),
		translation.z.to_bits(),
		extents.x.to_bits(),
		extents.y.to_bits(),
		extents.z.to_bits(),
		node.placement.yaw.to_bits(),
	] {
		h = h.wrapping_mul(16_777_619) ^ bits;
	}
	h
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::{LabelGeometry, LabelStyle, Placement};

	#[test]
	fn high_lounge_label_receives_a_place() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_systems(Update, stamp_label_places);
		let lounge = app
			.world_mut()
			.spawn(LabelNode::new(
				LabelStyle::Gray,
				LabelGeometry::rectangle(bevy::math::Vec3::new(5.0, 3.0, 4.0)),
				"Lounge",
				Placement::default(),
			))
			.id();
		let furniture = app
			.world_mut()
			.spawn(LabelNode::new(
				LabelStyle::Blue,
				LabelGeometry::rectangle(bevy::math::Vec3::ONE),
				"Nightstand",
				Placement::default(),
			))
			.id();

		app.update();

		let place = app
			.world()
			.get::<DiscoverablePlace>(lounge)
			.ok_or_else(|| anyhow::anyhow!("lounge should become a place"))?;
		assert_eq!(place.label, crate::DiscoverablePlaceLabel::Lounge);
		assert!(!place.persistent);
		assert!(app.world().get::<DiscoverablePlace>(furniture).is_none());
		Ok(())
	}

	#[test]
	fn high_child_inherits_host_identity() -> anyhow::Result<()> {
		use lod::gen::Id;

		let mut app = App::new();
		app.add_systems(Update, stamp_label_places);
		let host_id = Id::from_cell(bevy::math::bounding::Aabb3d::from_min_max(
			bevy::math::Vec3::ZERO,
			bevy::math::Vec3::ONE,
		));
		let parent = app
			.world_mut()
			.spawn(
				DiscoverablePlace::host(crate::DiscoverablePlaceLabel::House, 8.0, 1.1)
					.with_identity(host_id, 4),
			)
			.id();
		let lounge = app
			.world_mut()
			.spawn((
				LabelNode::new(
					LabelStyle::Gray,
					LabelGeometry::rectangle(bevy::math::Vec3::new(5.0, 3.0, 4.0)),
					"Lounge",
					Placement::default(),
				),
				ChildOf(parent),
			))
			.id();

		app.update();

		let place = app
			.world()
			.get::<DiscoverablePlace>(lounge)
			.ok_or_else(|| anyhow::anyhow!("lounge should inherit host identity"))?;
		anyhow::ensure!(place.host == Some(host_id));
		anyhow::ensure!(place.local != 0);
		Ok(())
	}

	#[test]
	fn duplicate_lounge_labels_under_one_host_stay_distinct() -> anyhow::Result<()> {
		use lod::gen::Id;

		let mut app = App::new();
		app.add_systems(Update, stamp_label_places);
		let host_id = Id::from_cell(bevy::math::bounding::Aabb3d::from_min_max(
			bevy::math::Vec3::ZERO,
			bevy::math::Vec3::ONE,
		));
		let parent = app
			.world_mut()
			.spawn(
				DiscoverablePlace::host(crate::DiscoverablePlaceLabel::House, 8.0, 1.1)
					.with_identity(host_id, 4),
			)
			.id();
		let lounge_a = app
			.world_mut()
			.spawn((
				LabelNode::rectangle(
					LabelStyle::Gray,
					"Lounge",
					bevy::math::Vec3::new(0.0, 0.0, 0.0),
					bevy::math::Vec3::new(5.0, 3.0, 4.0),
					0.0,
				),
				ChildOf(parent),
			))
			.id();
		let lounge_b = app
			.world_mut()
			.spawn((
				LabelNode::rectangle(
					LabelStyle::Gray,
					"Lounge",
					bevy::math::Vec3::new(8.0, 0.0, 0.0),
					bevy::math::Vec3::new(5.0, 3.0, 4.0),
					0.0,
				),
				ChildOf(parent),
			))
			.id();

		app.update();

		let first = app
			.world()
			.get::<DiscoverablePlace>(lounge_a)
			.ok_or_else(|| anyhow::anyhow!("first lounge"))?;
		let second = app
			.world()
			.get::<DiscoverablePlace>(lounge_b)
			.ok_or_else(|| anyhow::anyhow!("second lounge"))?;
		anyhow::ensure!(first.host == Some(host_id));
		anyhow::ensure!(second.host == Some(host_id));
		anyhow::ensure!(first.local != second.local, "label text must not identify the room");
		Ok(())
	}
}
