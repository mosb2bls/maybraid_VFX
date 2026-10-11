//! First-encounter item icons. Discovery stays in the host, capture in `screen-capture`.

mod host;

use bevy::prelude::*;
use character_items::{icon_catalog_key_for_label, Inventory, InventoryItem};
use character_persist::SaveRoot;
use firearms_components::firearm_preview_camera;
use host::{
	install_capture_hosts, mark_capture_hosts_ready, realize_capture_hosts, remember_item,
	strip_silhouette_shader_materials, HOST_CLOTHING, HOST_FIREARM,
};
use screen_capture::{
	CaptureCompleted, CaptureFailed, CaptureLighting, CaptureRequest, CaptureResolution,
	CaptureRevision, CaptureScene, CaptureSubject, CaptureView, ScreenCapturePlugin,
	ScreenCaptureSystems,
};

pub use host::ItemCaptureSubjects;
pub use screen_capture::{CaptureClient, CaptureId, CaptureMode};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;

/// Default inventory icon size.
pub const ITEM_ICON_RESOLUTION: CaptureResolution = CaptureResolution::ICON;

/// Clothing inspect camera (catalog thumbnail).
const CLOTHING_EYE: Vec3 = Vec3::new(0.0, 0.75, 2.5);
const CLOTHING_LOOK: Vec3 = Vec3::new(0.0, 0.65, 0.0);

/// Systems that request icons run before completed handles are stored.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemIconSystems {
	Discover,
	Store,
}

/// Handles for shaded and silhouette icons keyed by capture id.
#[derive(Resource, Default, Debug)]
pub struct ItemIconCache {
	pub shaded: HashMap<CaptureId, Handle<Image>>,
	pub silhouette: HashMap<CaptureId, Handle<Image>>,
	shaded_by_key: HashMap<u64, Handle<Image>>,
	silhouette_by_key: HashMap<u64, Handle<Image>>,
	failed: HashSet<(CaptureId, CaptureMode)>,
	/// Bumps when a handle is stored or a capture fails so hosts can rebuild UI.
	pub generation: u64,
}

impl ItemIconCache {
	pub fn shaded(&self, id: &CaptureId) -> Option<Handle<Image>> {
		self.shaded.get(id).cloned()
	}

	pub fn silhouette(&self, id: &CaptureId) -> Option<Handle<Image>> {
		self.silhouette.get(id).cloned()
	}

	pub fn shaded_for_key(&self, key: u64) -> Option<Handle<Image>> {
		self.shaded_by_key.get(&key).cloned()
	}

	pub fn silhouette_for_key(&self, key: u64) -> Option<Handle<Image>> {
		self.silhouette_by_key.get(&key).cloned()
	}

	pub fn silhouette_for_item(&self, item: &InventoryItem) -> Option<Handle<Image>> {
		self.silhouette(&CaptureId::new(item.capture_id_label()?))
	}

	pub fn shaded_for_item(&self, item: &InventoryItem) -> Option<Handle<Image>> {
		self.shaded(&CaptureId::new(item.capture_id_label()?))
	}

	pub fn failed(&self, id: &CaptureId, mode: CaptureMode) -> bool {
		self.failed.contains(&(id.clone(), mode))
	}

	pub fn insert_shaded(&mut self, id: CaptureId, image: Handle<Image>) {
		let key = icon_catalog_key_for_label(id.as_str());
		self.shaded_by_key.insert(key, image.clone());
		self.shaded.insert(id, image);
		self.generation = self.generation.wrapping_add(1);
	}

	pub fn insert_silhouette(&mut self, id: CaptureId, image: Handle<Image>) {
		let key = icon_catalog_key_for_label(id.as_str());
		self.silhouette_by_key.insert(key, image.clone());
		self.silhouette.insert(id, image);
		self.generation = self.generation.wrapping_add(1);
	}

	fn mark_failed(&mut self, id: CaptureId, mode: CaptureMode) {
		self.failed.insert((id, mode));
		self.generation = self.generation.wrapping_add(1);
	}
}

/// Capture ids already requested this process (per revision).
#[derive(Resource, Default, Debug)]
pub struct ItemIconDiscovery {
	requested: HashSet<CaptureId>,
	revision: CaptureRevision,
}

impl ItemIconDiscovery {
	pub fn requested_len(&self) -> usize {
		self.requested.len()
	}
}

/// Maybraid first-encounter wiring on top of [`ScreenCapturePlugin`].
pub struct ItemIconPlugin;

impl Plugin for ItemIconPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ScreenCapturePlugin>() {
			app.add_plugins(ScreenCapturePlugin::new(discover_icon_cache_dir()));
		}
		if app.get_sub_app(bevy::render::RenderApp).is_some() {
			install_capture_hosts(app);
		}
		app.init_resource::<ItemIconCache>()
			.init_resource::<ItemIconDiscovery>()
			.init_resource::<ItemCaptureSubjects>()
			.configure_sets(Update, ItemIconSystems::Store.after(ItemIconSystems::Discover))
			.add_systems(Update, realize_capture_hosts)
			.add_systems(
				Update,
				(store_completed_icons, store_failed_icons)
					.chain()
					.in_set(ItemIconSystems::Store),
			)
			.add_systems(
				PostUpdate,
				(strip_silhouette_shader_materials, mark_capture_hosts_ready)
					.chain()
					.before(ScreenCaptureSystems::Advance),
			);
	}
}

/// Writable icon cache: `MAYBRAID_ICON_CACHE`, else sibling of the save root.
pub fn discover_icon_cache_dir() -> PathBuf {
	if let Ok(path) = env::var("MAYBRAID_ICON_CACHE") {
		return PathBuf::from(path);
	}
	let saves = SaveRoot::discover().path;
	saves.parent().unwrap_or(&saves).join("icon-cache")
}

/// Visual capture subject for a catalog item, when it has a 3D scene.
#[derive(Clone, Debug)]
pub struct InventoryItemSubject {
	pub id: CaptureId,
	pub scene: CaptureScene,
	pub view: CaptureView,
}

impl InventoryItemSubject {
	pub fn from_item(item: &InventoryItem) -> Option<Self> {
		let id = CaptureId::new(item.capture_id_label()?);
		match item {
			InventoryItem::Firearm { .. } => {
				let (eye, look_at) = firearm_preview_camera(CaptureView::DEFAULT_FOV);
				Some(Self {
					id: id.clone(),
					scene: CaptureScene::host(HOST_FIREARM, id.as_str()),
					view: CaptureView::look_at(eye, look_at).perspective(CaptureView::DEFAULT_FOV),
				})
			}
			InventoryItem::Clothing { .. } => Some(Self {
				id: id.clone(),
				scene: CaptureScene::host(HOST_CLOTHING, id.as_str()),
				view: CaptureView::look_at(CLOTHING_EYE, CLOTHING_LOOK)
					.perspective(CaptureView::DEFAULT_FOV),
			}),
			InventoryItem::Grenade { .. } | InventoryItem::SkillMap { .. } => None,
		}
	}
}

impl CaptureSubject for InventoryItemSubject {
	fn capture_id(&self) -> CaptureId {
		self.id.clone()
	}

	fn capture_scene(&self) -> CaptureScene {
		self.scene.clone()
	}

	fn preferred_view(&self) -> CaptureView {
		self.view
	}
}

pub fn item_icon_requests(
	item: &InventoryItem,
	lighting: CaptureLighting,
	revision: CaptureRevision,
) -> Vec<CaptureRequest> {
	let Some(subject) = InventoryItemSubject::from_item(item) else {
		return Vec::new();
	};
	[CaptureMode::Shaded, CaptureMode::Silhouette]
		.into_iter()
		.map(|mode| {
			CaptureRequest::from_subject(&subject, mode, ITEM_ICON_RESOLUTION, lighting, revision)
		})
		.collect()
}

/// Enqueue first-encounter shaded + silhouette captures for bag items.
pub fn request_inventory_icons(
	inventory: &Inventory,
	discovery: &mut ItemIconDiscovery,
	subjects: &mut ItemCaptureSubjects,
	client: &mut CaptureClient,
) {
	let revision = CaptureRevision::CURRENT;
	if discovery.revision != revision {
		discovery.requested.clear();
		discovery.revision = revision;
	}
	for request in
		pending_inventory_icons(inventory, discovery, subjects, CaptureLighting::studio(), revision)
	{
		client.request(request);
	}
}

pub fn pending_inventory_icons(
	inventory: &Inventory,
	discovery: &mut ItemIconDiscovery,
	subjects: &mut ItemCaptureSubjects,
	lighting: CaptureLighting,
	revision: CaptureRevision,
) -> Vec<CaptureRequest> {
	let mut out = Vec::new();
	for item in &inventory.items {
		let Some(subject) = InventoryItemSubject::from_item(item) else {
			continue;
		};
		if !discovery.requested.insert(subject.capture_id()) {
			continue;
		}
		remember_item(subjects, item);
		out.extend(item_icon_requests(item, lighting, revision));
	}
	out
}

fn store_completed_icons(
	mut completed: MessageReader<CaptureCompleted>,
	mut cache: ResMut<ItemIconCache>,
) {
	for event in completed.read() {
		let id = event.key.id.clone();
		match event.key.mode {
			CaptureMode::Shaded => cache.insert_shaded(id, event.image.clone()),
			CaptureMode::Silhouette => cache.insert_silhouette(id, event.image.clone()),
		}
	}
}

fn store_failed_icons(mut failed: MessageReader<CaptureFailed>, mut cache: ResMut<ItemIconCache>) {
	for event in failed.read() {
		cache.mark_failed(event.key.id.clone(), event.key.mode);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, FirearmSpec, InventoryItem, ItemColor,
	};
	use screen_capture::{CaptureCacheKey, CaptureError, CaptureSource};

	#[test]
	fn firearm_variants_have_distinct_ids() -> anyhow::Result<()> {
		let bullpup = InventoryItem::firearm(FirearmMesh::Bullpup);
		let reltor = InventoryItem::firearm(FirearmMesh::Reltor);
		let a =
			InventoryItemSubject::from_item(&bullpup).ok_or_else(|| anyhow::anyhow!("bullpup"))?;
		let b =
			InventoryItemSubject::from_item(&reltor).ok_or_else(|| anyhow::anyhow!("reltor"))?;
		assert_ne!(a.capture_id(), b.capture_id());
		assert_eq!(a.capture_id(), a.capture_id());
		assert_eq!(
			a.capture_id().as_str(),
			bullpup.capture_id_label().ok_or_else(|| anyhow::anyhow!("label"))?
		);
		Ok(())
	}

	#[test]
	fn customized_look_changes_id() -> anyhow::Result<()> {
		let plain = InventoryItem::from_firearm_spec(FirearmSpec::from_mesh(FirearmMesh::Bullpup));
		let mut painted = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		painted.looks.body.material = character_items::FirearmMaterial::LavaVeins;
		let painted = InventoryItem::from_firearm_spec(painted);
		let a = InventoryItemSubject::from_item(&plain).ok_or_else(|| anyhow::anyhow!("plain"))?;
		let b =
			InventoryItemSubject::from_item(&painted).ok_or_else(|| anyhow::anyhow!("painted"))?;
		assert_ne!(a.capture_id(), b.capture_id());
		Ok(())
	}

	#[test]
	fn first_encounter_requests_both_modes() -> anyhow::Result<()> {
		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let requests =
			item_icon_requests(&item, CaptureLighting::studio(), CaptureRevision::CURRENT);
		assert_eq!(requests.len(), 2);
		assert_eq!(requests[0].mode, CaptureMode::Shaded);
		assert_eq!(requests[1].mode, CaptureMode::Silhouette);
		assert_eq!(requests[0].id, requests[1].id);
		Ok(())
	}

	#[test]
	fn clothing_is_capturable_grenades_are_not() -> anyhow::Result<()> {
		let clothing = InventoryItem::clothing(
			ClothingMesh::Pants,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert!(InventoryItemSubject::from_item(&clothing).is_some());
		assert!(InventoryItemSubject::from_item(&InventoryItem::standard_grenade()).is_none());
		Ok(())
	}

	#[test]
	fn discovery_dedupes_the_same_item() -> anyhow::Result<()> {
		let inventory = Inventory::with_starter_outfit(vec![
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::firearm(FirearmMesh::Bullpup),
		]);
		let mut discovery = ItemIconDiscovery::default();
		let mut subjects = ItemCaptureSubjects::default();
		let first = pending_inventory_icons(
			&inventory,
			&mut discovery,
			&mut subjects,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		let second = pending_inventory_icons(
			&inventory,
			&mut discovery,
			&mut subjects,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		assert_eq!(first.len(), 2);
		assert!(second.is_empty());
		assert_eq!(discovery.requested_len(), 1);
		assert!(subjects.get(&first[0].id).is_some());
		Ok(())
	}

	#[test]
	fn firearm_subject_is_a_generated_kit_host() -> anyhow::Result<()> {
		let item = InventoryItem::firearm(FirearmMesh::Silopup);
		let subject =
			InventoryItemSubject::from_item(&item).ok_or_else(|| anyhow::anyhow!("subject"))?;
		assert!(matches!(
			subject.capture_scene(),
			CaptureScene::Host { kind, .. } if kind == HOST_FIREARM
		));
		Ok(())
	}

	#[test]
	fn clothing_subject_is_a_preview_host() -> anyhow::Result<()> {
		let item = InventoryItem::clothing(
			ClothingMesh::Pants,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let subject =
			InventoryItemSubject::from_item(&item).ok_or_else(|| anyhow::anyhow!("subject"))?;
		assert!(matches!(
			subject.capture_scene(),
			CaptureScene::Host { kind, .. } if kind == HOST_CLOTHING
		));
		Ok(())
	}

	#[test]
	fn cache_lookups_stay_empty_until_ready() -> anyhow::Result<()> {
		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let key = item.icon_catalog_key().ok_or_else(|| anyhow::anyhow!("key"))?;
		let cache = ItemIconCache::default();
		assert!(cache.shaded_for_key(key).is_none());
		assert!(cache.silhouette_for_item(&item).is_none());
		assert!(cache.shaded_for_item(&item).is_none());
		Ok(())
	}

	#[test]
	fn completed_handles_are_keyed_for_inventory_and_hud() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ItemIconCache>()
			.add_message::<CaptureCompleted>();
		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let subject =
			InventoryItemSubject::from_item(&item).ok_or_else(|| anyhow::anyhow!("subject"))?;
		let request = CaptureRequest::from_subject(
			&subject,
			CaptureMode::Shaded,
			ITEM_ICON_RESOLUTION,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		let image = Handle::default();
		app.world_mut().write_message(CaptureCompleted {
			key: CaptureCacheKey::from_request(&request),
			image: image.clone(),
			source: CaptureSource::Cache,
		});
		app.add_systems(Update, store_completed_icons);
		app.update();

		let cache = app.world().resource::<ItemIconCache>();
		let key = item.icon_catalog_key().ok_or_else(|| anyhow::anyhow!("key"))?;
		assert!(cache.shaded_for_key(key).is_some());
		assert!(cache.shaded_for_item(&item).is_some());
		assert!(cache.silhouette_for_item(&item).is_none());
		assert_eq!(cache.generation, 1);
		Ok(())
	}

	#[test]
	fn failed_captures_leave_placeholders() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ItemIconCache>()
			.add_message::<CaptureFailed>();
		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let subject =
			InventoryItemSubject::from_item(&item).ok_or_else(|| anyhow::anyhow!("subject"))?;
		let request = CaptureRequest::from_subject(
			&subject,
			CaptureMode::Silhouette,
			ITEM_ICON_RESOLUTION,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		app.world_mut().write_message(CaptureFailed {
			key: CaptureCacheKey::from_request(&request),
			error: CaptureError::Readback,
		});
		app.add_systems(Update, store_failed_icons);
		app.update();

		let cache = app.world().resource::<ItemIconCache>();
		assert!(cache.silhouette_for_item(&item).is_none());
		assert!(cache.failed(&subject.capture_id(), CaptureMode::Silhouette));
		assert!(!cache.failed(&subject.capture_id(), CaptureMode::Shaded));
		Ok(())
	}
}
