//! Realize posed, painted inventory subjects into isolated capture roots.

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use character_items::{ClothingHost, ClothingMesh, InventoryItem, ItemColor};
use character_shaders::{ClothingShaderMaterial, FaceShaderMaterial};
use characters::{
	add_character_components_host, character_bounds, spawn_fixed_character_assembly,
	species::braidman::BraidmanConfig, AnimClip, AnimMailbox, AnimProgress, AnimRef, AnimRefRoot,
	CharacterComponents, CharacterComponentsPlugin, CharacterHostsPlugin, CharacterMotionSystems,
	CharacterRecipe, CharacterRig, CharacterRigRole, ClothingLayer, ComponentsOnly, Layers,
	PartNode,
};
use firearm_user::GeneratedFirearm;
use firearms_components::{
	add_firearm_components_host, firearm_bounds, spawn_firearm_components, ActiveRigPose,
	FirearmComponentsPlugin, PoseApplied,
};
use grenades::{grenade_material, grenade_mesh, GrenadeMaterial, GrenadeMaterialPlugin};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use lod::{
	LodLevelRootPending, LodLevelSpawnRequest, LodRefreshCorePlugin, LodRefreshDomain,
	LodRefreshMembership,
};
use material_ref::MaterialRefApplied;
use screen_capture::{
	CaptureHostReady, CaptureHostRoot, CaptureId, CaptureSilhouetteRoot, HOST_REPRESENTATIVE_POSE,
};
use std::collections::HashMap;

pub const HOST_FIREARM: &str = "generated-firearm";
pub const HOST_CLOTHING: &str = "clothing-preview";
pub const HOST_GRENADE: &str = "grenade-preview";
pub const HOST_SKILL_MAP: &str = "skill-map-preview";

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

	pub fn items(&self) -> impl Iterator<Item = &InventoryItem> {
		self.items.values()
	}
}

/// Clip + progress for a [`HOST_REPRESENTATIVE_POSE`] capture host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseCaptureSpec {
	pub clip: AnimClip,
	pub progress: f32,
}

#[derive(Resource, Default, Debug)]
pub struct PoseCaptureSubjects {
	poses: HashMap<CaptureId, PoseCaptureSpec>,
}

impl PoseCaptureSubjects {
	pub fn remember(&mut self, id: CaptureId, spec: PoseCaptureSpec) {
		self.poses.insert(id, spec);
	}

	pub fn get(&self, id: &CaptureId) -> Option<PoseCaptureSpec> {
		self.poses.get(id).copied()
	}
}

/// Stamp the live clip onto a realized representative-pose host.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CapturePoseSpec {
	pub clip: AnimClip,
	pub progress: f32,
}

#[derive(Component)]
pub struct CaptureHostRealized;

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
	if !app.is_plugin_added::<CharacterHostsPlugin>() {
		app.add_plugins(CharacterHostsPlugin);
	}
	if !app.is_plugin_added::<GrenadeMaterialPlugin>() {
		app.add_plugins(GrenadeMaterialPlugin);
	}
	app.add_systems(Update, pose_representative_hosts.before(CharacterMotionSystems::Anim));
}

pub fn remember_item(subjects: &mut ItemCaptureSubjects, item: &InventoryItem) {
	let Some(label) = item.capture_id_label() else {
		return;
	};
	subjects.remember(CaptureId::new(label), item.clone());
}

pub fn realize_capture_hosts(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut grenade_materials: Option<ResMut<Assets<GrenadeMaterial>>>,
	subjects: Res<ItemCaptureSubjects>,
	poses: Res<PoseCaptureSubjects>,
	hosts: Query<(Entity, &CaptureHostRoot), Without<CaptureHostRealized>>,
) {
	for (root, host) in &hosts {
		if let Some(item) = subjects.get(&host.id) {
			if spawn_item_host(
				&mut commands,
				&mut meshes,
				grenade_materials.as_deref_mut(),
				root,
				item,
			) {
				commands.entity(root).try_insert(CaptureHostRealized);
			}
			continue;
		}
		if host.kind != HOST_REPRESENTATIVE_POSE {
			continue;
		}
		let Some(spec) = poses.get(&host.id) else {
			continue;
		};
		if spawn_pose_host(&mut commands, root) {
			commands.entity(root).try_insert((
				CaptureHostRealized,
				CapturePoseSpec { clip: spec.clip, progress: spec.progress },
			));
		}
	}
}

pub fn pose_representative_hosts(
	mut commands: Commands,
	hosts: Query<(Entity, &CapturePoseSpec), With<CaptureHostRealized>>,
	children: Query<&Children>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	for (root, spec) in &hosts {
		visit_descendants(root, &children, &mut |entity| {
			let Ok(rig) = rigs.get(entity) else {
				return;
			};
			if rig.role != CharacterRigRole::Body {
				return;
			}
			let desired = AnimRef::new(spec.clip);
			let needs = match anims.get(entity) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs {
				commands.entity(entity).try_insert(AnimRefRoot(desired));
			}
			commands.entity(entity).try_insert(AnimProgress(spec.progress));
		});
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
	grenades: Query<(), With<MeshMaterial3d<GrenadeMaterial>>>,
	poses: Query<(), With<ActiveRigPose>>,
	posed: Query<(), With<PoseApplied>>,
	pose_specs: Query<(), With<CapturePoseSpec>>,
	mailboxes: Query<&AnimMailbox>,
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
			&grenades,
			&poses,
			&posed,
			pose_specs.get(root).is_ok(),
			&mailboxes,
		) {
			commands.entity(root).try_insert(CaptureHostReady);
		}
	}
}

/// Strip clothing/face/grenade shaders and keep the tree extractable. Clothing
/// and face shaders restamp in Update (`MaterialRefApplied` still counts as
/// painted). Grenades do not, so they get a [`StandardMaterial`] stand-in;
/// capture restamps unlit gray through settle/readback.
pub fn prepare_silhouette_hosts(
	mut commands: Commands,
	mut standard_materials: Option<ResMut<Assets<StandardMaterial>>>,
	roots: Query<Entity, With<CaptureSilhouetteRoot>>,
	children: Query<&Children>,
	clothing: Query<Entity, With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: Query<Entity, With<MeshMaterial3d<FaceShaderMaterial>>>,
	grenades: Query<Entity, With<MeshMaterial3d<GrenadeMaterial>>>,
) {
	for root in &roots {
		visit_descendants(root, &children, &mut |entity| {
			if clothing.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<ClothingShaderMaterial>>();
			}
			if faces.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<FaceShaderMaterial>>();
			}
			if grenades.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<GrenadeMaterial>>();
				if let Some(materials) = standard_materials.as_mut() {
					commands.entity(entity).try_insert(MeshMaterial3d(materials.add(
						StandardMaterial { perceptual_roughness: 0.6, ..Default::default() },
					)));
				}
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

fn spawn_item_host(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	grenade_materials: Option<&mut Assets<GrenadeMaterial>>,
	root: Entity,
	item: &InventoryItem,
) -> bool {
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
				commands
					.entity(entity)
					.try_insert(ActiveRigPose { pose: preview.icon_pose().to_resolved() });
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
		InventoryItem::Grenade { .. } => {
			let Some(materials) = grenade_materials else {
				return false;
			};
			let entity = commands
				.spawn((
					Name::new("capture-grenade"),
					Mesh3d(meshes.add(grenade_mesh())),
					MeshMaterial3d(materials.add(grenade_material())),
					Transform::IDENTITY,
					Visibility::default(),
				))
				.id();
			pin_capture_host(commands, entity, root);
			true
		}
		InventoryItem::SkillMap { .. } => false,
	}
}

fn pin_capture_host(commands: &mut Commands, entity: Entity, root: Entity) {
	commands.entity(entity).try_insert((
		ChildOf(root),
		LodRefreshMembership(LodRefreshDomain::of::<CaptureHostRoot>()),
	));
}

fn spawn_pose_host(commands: &mut Commands, root: Entity) -> bool {
	// Same runtime assembly as in-game characters (not a nested LodScene host).
	// Capture-only apps may lack a unique LodViewer, which would stall LOD fulfill.
	let preview = BraidmanConfig::default_preview().clothed();
	let entity = spawn_fixed_character_assembly(commands, &preview, Transform::IDENTITY);
	pin_capture_host(commands, entity, root);
	true
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
	grenades: &Query<(), With<MeshMaterial3d<GrenadeMaterial>>>,
	poses: &Query<(), With<ActiveRigPose>>,
	posed: &Query<(), With<PoseApplied>>,
	require_mailbox: bool,
	mailboxes: &Query<&AnimMailbox>,
) -> bool {
	let mut found_mesh = false;
	let mut pending = false;
	let mut unpainted = false;
	let mut needs_pose = false;
	let mut pose_ready = false;
	let mut mailbox_ready = !require_mailbox;
	visit_descendants(root, children, &mut |entity| {
		if pending_spawn.get(entity).is_ok() || pending_root.get(entity).is_ok() {
			pending = true;
		}
		if poses.get(entity).is_ok() {
			needs_pose = true;
		}
		if posed.get(entity).is_ok() {
			pose_ready = true;
		}
		if require_mailbox && mailboxes.get(entity).is_ok_and(AnimMailbox::settled) {
			mailbox_ready = true;
		}
		if meshes.get(entity).is_err() {
			return;
		}
		found_mesh = true;
		if applied.get(entity).is_err()
			&& standard.get(entity).is_err()
			&& clothing.get(entity).is_err()
			&& faces.get(entity).is_err()
			&& grenades.get(entity).is_err()
		{
			unpainted = true;
		}
	});
	found_mesh && !pending && !unpainted && (!needs_pose || pose_ready) && mailbox_ready
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
	fn grenade_host_uses_the_thrown_mesh() {
		let item = InventoryItem::standard_grenade();
		assert!(item.capture_id_label().is_some());
		assert_eq!(HOST_GRENADE, "grenade-preview");
	}

	#[test]
	fn silhouette_prep_keeps_the_grenade_mesh_painted() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<GrenadeMaterial>()
			.add_systems(Update, prepare_silhouette_hosts);
		let grenade = app
			.world_mut()
			.resource_mut::<Assets<GrenadeMaterial>>()
			.add(grenade_material());
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Sphere::new(0.5));
		let root = app.world_mut().spawn(CaptureSilhouetteRoot).id();
		let child = app
			.world_mut()
			.spawn((Mesh3d(mesh), MeshMaterial3d(grenade), ChildOf(root)))
			.id();
		app.update();
		assert!(app.world().get::<MeshMaterial3d<GrenadeMaterial>>(child).is_none());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(child).is_some());
	}

	#[test]
	fn capture_hosts_use_an_isolated_lod_domain() {
		let domain = LodRefreshDomain::of::<CaptureHostRoot>();
		assert_eq!(domain, LodRefreshDomain::of::<CaptureHostRoot>());
		assert_ne!(domain, LodRefreshDomain::of::<GeneratedFirearm>());
	}

	#[test]
	fn representative_pose_host_kind_is_stable() {
		assert_eq!(HOST_REPRESENTATIVE_POSE, "representative-pose");
	}
}
