//! First-encounter item icons. Discovery stays in Maybraid, capture in `screen-capture`.

use bevy::prelude::*;
use character_items::{Inventory, InventoryItem};
use character_persist::SaveRoot;
use firearms::firearm_preview_camera;
use screen_capture::{
	CaptureClient, CaptureCompleted, CaptureId, CaptureLighting, CaptureMode, CaptureRequest,
	CaptureResolution, CaptureRevision, CaptureScene, CaptureSubject, CaptureView,
	ScreenCapturePlugin,
};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;

use crate::weapon::WorldPlayerLoadout;

/// Default inventory icon size.
pub const ITEM_ICON_RESOLUTION: CaptureResolution = CaptureResolution::ICON;

/// Clothing inspect camera (catalog thumbnail).
const CLOTHING_EYE: Vec3 = Vec3::new(0.0, 0.75, 2.5);
const CLOTHING_LOOK: Vec3 = Vec3::new(0.0, 0.65, 0.0);

/// Handles for shaded and silhouette icons keyed by capture id.
#[derive(Resource, Default, Debug)]
pub struct ItemIconCache {
	pub shaded: HashMap<CaptureId, Handle<Image>>,
	pub silhouette: HashMap<CaptureId, Handle<Image>>,
}

impl ItemIconCache {
	pub fn shaded(&self, id: &CaptureId) -> Option<Handle<Image>> {
		self.shaded.get(id).cloned()
	}

	pub fn silhouette(&self, id: &CaptureId) -> Option<Handle<Image>> {
		self.silhouette.get(id).cloned()
	}
}

/// Capture ids already requested this process (per revision).
#[derive(Resource, Default, Debug)]
pub struct ItemIconDiscovery {
	requested: HashSet<CaptureId>,
	revision: CaptureRevision,
}

/// Maybraid first-encounter wiring on top of [`ScreenCapturePlugin`].
pub struct ItemIconPlugin;

impl Plugin for ItemIconPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ScreenCapturePlugin>() {
			app.add_plugins(ScreenCapturePlugin::new(discover_icon_cache_dir()));
		}
		app.init_resource::<ItemIconCache>()
			.init_resource::<ItemIconDiscovery>()
			.add_systems(Update, (discover_loadout_items, store_completed_icons));
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
		match item {
			InventoryItem::Firearm { spec, .. } => {
				let (eye, look_at) = firearm_preview_camera(CaptureView::DEFAULT_FOV);
				Some(Self {
					id: CaptureId::new(format!("firearm:{}", spec.identity_label())),
					scene: CaptureScene::scene_ref(spec.kit.body.path()),
					view: CaptureView::look_at(eye, look_at).perspective(CaptureView::DEFAULT_FOV),
				})
			}
			InventoryItem::Clothing { mesh, material, .. } => Some(Self {
				id: CaptureId::new(format!(
					"clothing:{}:{}:{}",
					mesh.label(),
					material.id.label(),
					material.color.label()
				)),
				scene: CaptureScene::scene_ref(mesh.path()),
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

fn discover_loadout_items(
	loadout: Option<Res<WorldPlayerLoadout>>,
	mut discovery: ResMut<ItemIconDiscovery>,
	mut client: CaptureClient,
) {
	let Some(loadout) = loadout else {
		return;
	};
	if !loadout.is_changed() && !discovery.requested.is_empty() {
		return;
	}
	let revision = CaptureRevision::CURRENT;
	if discovery.revision != revision {
		discovery.requested.clear();
		discovery.revision = revision;
	}
	for request in pending_inventory_icons(
		&loadout.inventory,
		&mut discovery,
		CaptureLighting::studio(),
		revision,
	) {
		client.request(request);
	}
}

fn pending_inventory_icons(
	inventory: &Inventory,
	discovery: &mut ItemIconDiscovery,
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
			CaptureMode::Shaded => {
				cache.shaded.insert(id, event.image.clone());
			}
			CaptureMode::Silhouette => {
				cache.silhouette.insert(id, event.image.clone());
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, FirearmSpec, InventoryItem, ItemColor,
	};

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
		let first = pending_inventory_icons(
			&inventory,
			&mut discovery,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		let second = pending_inventory_icons(
			&inventory,
			&mut discovery,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		assert_eq!(first.len(), 2);
		assert!(second.is_empty());
		assert_eq!(discovery.requested.len(), 1);
		Ok(())
	}
}
