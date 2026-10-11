//! Realize posed, painted inventory subjects into isolated capture roots.

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use character_items::{ClothingHost, ClothingMesh, InventoryItem, ItemColor};
use character_shaders::{ClothingShaderMaterial, FaceShaderMaterial};
use characters::{
	add_character_components_host, character_bounds, CharacterComponents,
	CharacterComponentsPlugin, ClothingLayer, ComponentsOnly, Layers, PartNode,
};
use firearm_user::GeneratedFirearm;
use firearms_components::{
	add_firearm_components_host, firearm_bounds, spawn_firearm_components, FirearmComponentsPlugin,
};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use lod::{
	LodLevelRootPending, LodLevelSpawnRequest, LodRefreshCorePlugin, LodRefreshDomain,
	LodRefreshMembership,
};
use material_ref::MaterialRefApplied;
use screen_capture::{CaptureHostReady, CaptureHostRoot, CaptureId, CaptureSilhouetteRoot};
use std::collections::HashMap;

pub const HOST_FIREARM: &str = "generated-firearm";
pub const HOST_CLOTHING: &str = "clothing-preview";

#[derive(Resource, Default, Debug)]
pub struct ItemCaptureSubjects {
	items: HashMap<CaptureId, InventoryItem>,
}

impl ItemCaptureSubjects {
	pub fn remember(&mut self, id: CaptureId, item: InventoryItem) {
		self.items.insert(id, item);
	}

	pub fn get(&self, id: &CaptureId) -> Option<&InventoryItem> {
		self.items.get(id)
	}
}

#[derive(Component)]
pub(crate) struct CaptureHostRealized;

/// Bind-pose garment with catalog material — same inspect path as the menu preview.
#[derive(Clone, PartialEq, Component)]
struct ClothingIconPreview {
	layer: ClothingLayer,
}

impl Default for ClothingIconPreview {
	fn default() -> Self {
		Self {
			layer: ClothingLayer::new(
				ClothingMesh::TankTop,
				ItemColor::Natural,
				ClothingHost::HUMANOID,
			),
		}
	}
}

impl CharacterComponents for ClothingIconPreview {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("clothing", vec![self.layer.preview_part_node()])
	}
}

pub fn install_capture_hosts(app: &mut App) {
	if !app.is_plugin_added::<LodRefreshCorePlugin>() {
		app.add_plugins(LodRefreshCorePlugin);
	}
	if !app.is_plugin_added::<FirearmComponentsPlugin>() {
		app.add_plugins(FirearmComponentsPlugin);
	}
	add_firearm_components_host::<GeneratedFirearm>(app);
	if !app.is_plugin_added::<CharacterComponentsPlugin>() {
		app.add_plugins(CharacterComponentsPlugin);
	}
	add_character_components_host::<ClothingIconPreview>(app);
}

pub fn remember_item(subjects: &mut ItemCaptureSubjects, item: &InventoryItem) {
	let Some(label) = item.capture_id_label() else {
		return;
	};
	subjects.remember(CaptureId::new(label), item.clone());
}

pub fn realize_capture_hosts(
	mut commands: Commands,
	subjects: Res<ItemCaptureSubjects>,
	hosts: Query<(Entity, &CaptureHostRoot), Without<CaptureHostRealized>>,
) {
	for (root, host) in &hosts {
		let Some(item) = subjects.get(&host.id) else {
			continue;
		};
		if spawn_item_host(&mut commands, root, item) {
			commands.entity(root).try_insert(CaptureHostRealized);
		}
	}
}

pub fn mark_capture_hosts_ready(
	mut commands: Commands,
	hosts: Query<
		Entity,
		(With<CaptureHostRoot>, With<CaptureHostRealized>, Without<CaptureHostReady>),
	>,
	children: Query<&Children>,
	pending_spawn: Query<(), With<LodLevelSpawnRequest>>,
	pending_root: Query<(), With<LodLevelRootPending>>,
	meshes: Query<Entity, With<Mesh3d>>,
	applied: Query<(), With<MaterialRefApplied>>,
	standard: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
	clothing: Query<(), With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: Query<(), With<MeshMaterial3d<FaceShaderMaterial>>>,
) {
	for root in &hosts {
		if host_tree_ready(
			root,
			&children,
			&pending_spawn,
			&pending_root,
			&meshes,
			&applied,
			&standard,
			&clothing,
			&faces,
		) {
			commands.entity(root).try_insert(CaptureHostReady);
		}
	}
}

/// Strip clothing/face shaders and keep the tree extractable. Shader systems
/// re-insert custom materials in Update; this runs again before readback.
pub fn prepare_silhouette_hosts(
	mut commands: Commands,
	roots: Query<Entity, With<CaptureSilhouetteRoot>>,
	children: Query<&Children>,
	clothing: Query<Entity, With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: Query<Entity, With<MeshMaterial3d<FaceShaderMaterial>>>,
) {
	for root in &roots {
		visit_descendants(root, &children, &mut |entity| {
			if clothing.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<ClothingShaderMaterial>>();
			}
			if faces.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<FaceShaderMaterial>>();
			}
		});
	}
}

pub fn keep_capture_hosts_visible(
	mut commands: Commands,
	roots: Query<Entity, With<CaptureHostRoot>>,
	children: Query<&Children>,
	hidden: Query<(), Or<(With<Disabled>, With<Visibility>)>>,
) {
	for root in &roots {
		visit_descendants(root, &children, &mut |entity| {
			if hidden.get(entity).is_err() {
				return;
			}
			commands.entity(entity).try_remove::<Disabled>();
			commands.entity(entity).try_insert(Visibility::Inherited);
		});
	}
}

fn spawn_item_host(commands: &mut Commands, root: Entity, item: &InventoryItem) -> bool {
	match item {
		InventoryItem::Firearm { spec, .. } => {
			let preview = GeneratedFirearm::from_spec(*spec);
			for entity in spawn_firearm_components(
				commands,
				&preview,
				Transform::IDENTITY,
				firearm_bounds(&preview),
			) {
				pin_capture_host(commands, entity, root);
			}
			true
		}
		InventoryItem::Clothing { mesh, material, .. } => {
			let preview = ClothingIconPreview {
				layer: ClothingLayer::new(*mesh, material.color, ClothingHost::HUMANOID)
					.with_material(material.id),
			};
			let entity = spawn_clothing_icon(commands, &preview);
			pin_capture_host(commands, entity, root);
			true
		}
		InventoryItem::Grenade { .. } | InventoryItem::SkillMap { .. } => false,
	}
}

fn pin_capture_host(commands: &mut Commands, entity: Entity, root: Entity) {
	commands.entity(entity).try_insert((
		ChildOf(root),
		LodRefreshMembership(LodRefreshDomain::of::<CaptureHostRoot>()),
	));
}

fn spawn_clothing_icon(commands: &mut Commands, preview: &ClothingIconPreview) -> Entity {
	let bounds = character_bounds(preview);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(preview.clone());
	commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				Transform::IDENTITY
				Visibility::default()
			},
		))
		.id()
}

fn host_tree_ready(
	root: Entity,
	children: &Query<&Children>,
	pending_spawn: &Query<(), With<LodLevelSpawnRequest>>,
	pending_root: &Query<(), With<LodLevelRootPending>>,
	meshes: &Query<Entity, With<Mesh3d>>,
	applied: &Query<(), With<MaterialRefApplied>>,
	standard: &Query<(), With<MeshMaterial3d<StandardMaterial>>>,
	clothing: &Query<(), With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: &Query<(), With<MeshMaterial3d<FaceShaderMaterial>>>,
) -> bool {
	let mut found_mesh = false;
	let mut pending = false;
	let mut unpainted = false;
	visit_descendants(root, children, &mut |entity| {
		if pending_spawn.get(entity).is_ok() || pending_root.get(entity).is_ok() {
			pending = true;
		}
		if meshes.get(entity).is_err() {
			return;
		}
		found_mesh = true;
		if applied.get(entity).is_err()
			&& standard.get(entity).is_err()
			&& clothing.get(entity).is_err()
			&& faces.get(entity).is_err()
		{
			unpainted = true;
		}
	});
	found_mesh && !pending && !unpainted
}

fn visit_descendants(root: Entity, children: &Query<&Children>, visit: &mut impl FnMut(Entity)) {
	visit(root);
	let Ok(kids) = children.get(root) else {
		return;
	};
	for child in kids.iter() {
		visit_descendants(child, children, visit);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{FirearmMesh, FirearmSpec};

	#[test]
	fn clothing_preview_uses_the_humanoid_fitted_mesh() {
		let layer = ClothingLayer::new(ClothingMesh::Pants, ItemColor::Red, ClothingHost::HUMANOID)
			.with_material(character_items::ClothingMaterial::Cloth);
		let preview = ClothingIconPreview { layer: layer.clone() };
		let nodes = preview.part_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 1);
		assert_eq!(layer.preview_part_node().scene, nodes[0].scene);
	}

	#[test]
	fn generated_firearm_is_the_capture_host() {
		let spec = FirearmSpec::from_mesh(FirearmMesh::Silopup);
		let item = InventoryItem::from_firearm_spec(spec);
		assert!(item.capture_id_label().is_some());
		let _ = GeneratedFirearm::from_spec(spec);
	}

	#[test]
	fn capture_hosts_use_an_isolated_lod_domain() {
		let domain = LodRefreshDomain::of::<CaptureHostRoot>();
		assert_eq!(domain, LodRefreshDomain::of::<CaptureHostRoot>());
		assert_ne!(domain, LodRefreshDomain::of::<GeneratedFirearm>());
	}
}
