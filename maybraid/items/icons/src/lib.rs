//! First-encounter item icons. Discovery stays in the host, capture in `screen-capture`.

mod entry;
mod host;
mod prune;

use bevy::prelude::*;
use character_items::{Inventory, InventoryItem, SELF_LUBIKAN_CAPTURE_LABEL};
use character_persist::SaveRoot;
use characters::AnimClip;
use firearms_components::firearm_preview_camera;
use host::{
	install_capture_hosts, keep_capture_hosts_visible, mark_capture_hosts_ready,
	prepare_silhouette_hosts, realize_capture_hosts, remember_item, rest_pose_self_lubikan_skins,
	HOST_CLOTHING, HOST_FIREARM,
};
use screen_capture::{
	CaptureFraming, CaptureLighting, CaptureRequest, CaptureResolution, CaptureRevision,
	CaptureScene, CaptureSubject, CaptureView, ScreenCapturePlugin, ScreenCaptureSystems,
};

pub use entry::{IconCatalogProgress, IconEntry, IconEntryStatus, ItemIconCache};
pub use host::{
	CaptureHostRealized, CapturePoseSpec, ItemCaptureSubjects, PoseCaptureSpec,
	PoseCaptureSubjects, HOST_GRENADE, HOST_SELF_LUBIKAN, HOST_SKILL_MAP,
};
pub use prune::{
	kept_icon_file_names, prune_icon_cache, prune_icon_cache_to_saved_inventories,
	RequestPruneItemIconCache,
};
pub use screen_capture::{
	CaptureCacheKey, CaptureCacheRoot, CaptureClient, CaptureCompleted, CaptureFailed,
	CaptureHostReady, CaptureHostRoot, CaptureId, CaptureMode, CaptureTicket, RepresentativePose,
	HOST_REPRESENTATIVE_POSE,
};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

/// Default inventory icon size.
pub const ITEM_ICON_RESOLUTION: CaptureResolution = CaptureResolution::ICON;
/// Landscape HUD silhouette. Weapons are wide; a square PNG either pads or clips.
pub const FIREARM_SILHOUETTE_RESOLUTION: CaptureResolution = CaptureResolution::new(256, 112);
/// Square HUD / catalog charge. The in-world grenade is a sphere.
pub const GRENADE_SILHOUETTE_RESOLUTION: CaptureResolution = CaptureResolution::new(128, 128);
/// Tanz / representative-pose HUD silhouette. Matches the retired 128px assets.
pub const POSE_ICON_RESOLUTION: CaptureResolution = CaptureResolution::new(128, 128);
/// Isolated pose hosts wait on GLB load + mailbox settle. World streaming and
/// lavapipe both need more than the default 180-frame capture timeout.
const POSE_CAPTURE_MAX_WAIT_FRAMES: u32 = 1800;

/// Clothing inspect camera (catalog thumbnail).
const CLOTHING_EYE: Vec3 = Vec3::new(0.0, 0.75, 2.5);
const CLOTHING_LOOK: Vec3 = Vec3::new(0.0, 0.65, 0.0);

/// Full-body Braidman plus chest-front medical cross.
const SELF_LUBIKAN_EYE: Vec3 = Vec3::new(0.0, 1.0, 3.8);
const SELF_LUBIKAN_LOOK: Vec3 = Vec3::new(0.0, 0.95, 0.0);

/// Systems that request icons run before completed handles are stored.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemIconSystems {
	Discover,
	Store,
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
		if let Some(mut root) = app.world_mut().get_resource_mut::<CaptureCacheRoot>() {
			root.max_wait_frames = root.max_wait_frames.max(POSE_CAPTURE_MAX_WAIT_FRAMES);
		}
		if app.get_sub_app(bevy::render::RenderApp).is_some() {
			install_capture_hosts(app);
		}
		app.init_resource::<ItemIconCache>()
			.init_resource::<ItemIconDiscovery>()
			.init_resource::<ItemCaptureSubjects>()
			.init_resource::<PoseCaptureSubjects>()
			.configure_sets(Update, ItemIconSystems::Store.after(ItemIconSystems::Discover))
			.add_systems(Update, realize_capture_hosts)
			.add_systems(Update, prune::prune_icon_cache_on_request)
			.add_systems(
				Update,
				(store_completed_icons, store_failed_icons, log_pose_icon_events)
					.chain()
					.in_set(ItemIconSystems::Store),
			)
			.add_systems(
				PostUpdate,
				(
					keep_capture_hosts_visible,
					rest_pose_self_lubikan_skins,
					prepare_silhouette_hosts,
					mark_capture_hosts_ready,
				)
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
			InventoryItem::Grenade { .. } => Some(Self {
				id: id.clone(),
				scene: CaptureScene::host(HOST_GRENADE, id.as_str()),
				view: CaptureView::look_at(Vec3::new(1.2, 0.9, 1.2), Vec3::ZERO)
					.perspective(CaptureView::DEFAULT_FOV),
			}),
			InventoryItem::SkillMap { .. } => Some(Self {
				id: id.clone(),
				scene: CaptureScene::image(Handle::default()),
				view: CaptureView::unused(),
			}),
			InventoryItem::Tanz { .. } => None,
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
			CaptureRequest::from_subject(
				&subject,
				mode,
				resolution_for(item, mode),
				lighting,
				revision,
			)
			.with_file_stem(item.name())
			.with_framing(framing_for(item))
		})
		.collect()
}

fn resolution_for(item: &InventoryItem, mode: CaptureMode) -> CaptureResolution {
	match (item, mode) {
		(InventoryItem::Firearm { .. }, CaptureMode::Silhouette) => FIREARM_SILHOUETTE_RESOLUTION,
		(InventoryItem::Grenade { .. }, CaptureMode::Silhouette) => GRENADE_SILHOUETTE_RESOLUTION,
		_ => ITEM_ICON_RESOLUTION,
	}
}

fn framing_for(item: &InventoryItem) -> CaptureFraming {
	match item {
		InventoryItem::SkillMap { .. } => CaptureFraming::Authored,
		_ => CaptureFraming::ICON,
	}
}

/// Enqueue first-encounter shaded + silhouette captures for bag items.
pub fn request_inventory_icons(
	inventory: &Inventory,
	discovery: &mut ItemIconDiscovery,
	subjects: &mut ItemCaptureSubjects,
	client: &mut CaptureClient,
	cache: &mut ItemIconCache,
) {
	let revision = CaptureRevision::CURRENT;
	if discovery.revision != revision {
		discovery.requested.clear();
		discovery.revision = revision;
	}
	request_self_lubikan_icon(discovery, client, cache);
	for request in
		pending_inventory_icons(inventory, discovery, subjects, CaptureLighting::studio(), revision)
	{
		cache.remember_ticket(&client.request(request));
	}
}

/// Scene-capture the Self Lubikan tile: Braidman silhouette + medical cross.
pub fn request_self_lubikan_icon(
	discovery: &mut ItemIconDiscovery,
	client: &mut CaptureClient,
	cache: &mut ItemIconCache,
) {
	let revision = CaptureRevision::CURRENT;
	if discovery.revision != revision {
		discovery.requested.clear();
		discovery.revision = revision;
	}
	let subject = SelfLubikanSubject;
	if !discovery.requested.insert(subject.capture_id()) {
		return;
	}
	cache.remember_ticket(&client.request(self_lubikan_icon_request(revision)));
}

/// Silhouette request for the shared Self Lubikan catalog tile.
pub(crate) fn self_lubikan_icon_request(revision: CaptureRevision) -> CaptureRequest {
	CaptureRequest::from_subject(
		&SelfLubikanSubject,
		CaptureMode::Silhouette,
		ITEM_ICON_RESOLUTION,
		CaptureLighting::studio(),
		revision,
	)
	.with_file_stem("self-lubikan")
	.with_framing(CaptureFraming::ICON)
}

#[derive(Clone, Debug)]
struct SelfLubikanSubject;

impl CaptureSubject for SelfLubikanSubject {
	fn capture_id(&self) -> CaptureId {
		CaptureId::new(SELF_LUBIKAN_CAPTURE_LABEL)
	}

	fn capture_scene(&self) -> CaptureScene {
		CaptureScene::host(HOST_SELF_LUBIKAN, SELF_LUBIKAN_CAPTURE_LABEL)
	}

	fn preferred_view(&self) -> CaptureView {
		CaptureView::look_at(SELF_LUBIKAN_EYE, SELF_LUBIKAN_LOOK)
			.perspective(CaptureView::DEFAULT_FOV)
	}
}

/// Silhouette request for a [`RepresentativePose`] still. Hosts spawn the
/// default nude Braidman and scrub the live clip to `progress`.
pub fn pose_icon_request(subject: &impl RepresentativePose) -> CaptureRequest {
	CaptureRequest::from_subject(
		subject,
		CaptureMode::Silhouette,
		POSE_ICON_RESOLUTION,
		CaptureLighting::studio(),
		CaptureRevision::CURRENT,
	)
	.with_file_stem(subject.pose_label().as_ref())
	.with_framing(CaptureFraming::ICON)
}

pub fn request_pose_icon(
	subject: &impl RepresentativePose,
	clip: AnimClip,
	progress: f32,
	discovery: &mut ItemIconDiscovery,
	poses: &mut PoseCaptureSubjects,
	client: &mut CaptureClient,
	cache: &mut ItemIconCache,
) {
	let revision = CaptureRevision::CURRENT;
	if discovery.revision != revision {
		discovery.requested.clear();
		discovery.revision = revision;
	}
	let id = subject.capture_id();
	if !discovery.requested.insert(id.clone()) {
		return;
	}
	poses.remember(id, PoseCaptureSpec { clip, progress });
	cache.remember_ticket(&client.request(pose_icon_request(subject)));
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
		if matches!(item, InventoryItem::SkillMap { .. } | InventoryItem::Tanz { .. }) {
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
		cache.apply_completed(event);
	}
}

fn store_failed_icons(mut failed: MessageReader<CaptureFailed>, mut cache: ResMut<ItemIconCache>) {
	for event in failed.read() {
		cache.apply_failed(event);
	}
}

fn log_pose_icon_events(
	mut completed: MessageReader<CaptureCompleted>,
	mut failed: MessageReader<CaptureFailed>,
) {
	for event in completed.read() {
		if !event.key.id.as_str().starts_with("pose:") {
			continue;
		}
		info!(
			id = %event.key.id.as_str(),
			source = ?event.source,
			file = %event.key.file_name(),
			"pose icon ready"
		);
	}
	for event in failed.read() {
		if !event.key.id.as_str().starts_with("pose:") {
			continue;
		}
		warn!(
			id = %event.key.id.as_str(),
			error = %event.error,
			file = %event.key.file_name(),
			"pose icon failed"
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{
		icon_catalog_key_for_label, ClothingMaterial, ClothingMesh, FirearmMesh, FirearmSpec,
		InventoryItem, ItemColor,
	};
	use screen_capture::{CaptureCacheKey, CaptureError, CaptureFraming, CaptureSource};

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
		let stem = screen_capture::sanitize_file_stem(&item.name());
		assert!(!stem.is_empty());
		assert_eq!(requests[0].file_stem, stem);
		assert!(CaptureCacheKey::from_request(&requests[0]).file_name().starts_with(&stem));
		assert_eq!(requests[0].framing, CaptureFraming::ICON);
		assert_eq!(requests[0].resolution, ITEM_ICON_RESOLUTION);
		assert_eq!(requests[1].resolution, FIREARM_SILHOUETTE_RESOLUTION);
		Ok(())
	}

	#[test]
	fn clothing_grenades_and_skill_maps_are_capturable() -> anyhow::Result<()> {
		use character_items::{SkillMapKind, SkillMapSpec};
		use screen_capture::CaptureScene;

		let clothing = InventoryItem::clothing(
			ClothingMesh::Pants,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert!(InventoryItemSubject::from_item(&clothing).is_some());

		let grenade = InventoryItem::standard_grenade();
		let grenade_subject =
			InventoryItemSubject::from_item(&grenade).ok_or_else(|| anyhow::anyhow!("grenade"))?;
		assert!(matches!(
			grenade_subject.capture_scene(),
			CaptureScene::Host { kind, .. } if kind == HOST_GRENADE
		));
		let grenade_requests =
			item_icon_requests(&grenade, CaptureLighting::studio(), CaptureRevision::CURRENT);
		assert_eq!(grenade_requests[1].resolution, GRENADE_SILHOUETTE_RESOLUTION);

		let map = InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1));
		let map_subject =
			InventoryItemSubject::from_item(&map).ok_or_else(|| anyhow::anyhow!("skill map"))?;
		assert!(matches!(map_subject.capture_scene(), CaptureScene::Image(_)));
		assert_eq!(map_subject.preferred_view(), CaptureView::unused());
		let map_requests =
			item_icon_requests(&map, CaptureLighting::studio(), CaptureRevision::CURRENT);
		assert_eq!(map_requests[0].framing, CaptureFraming::Authored);
		assert_eq!(
			CaptureCacheKey::from_request(&map_requests[0]),
			CaptureCacheKey::from_request(
				&CaptureRequest::from_image_handle(
					map_subject.capture_id(),
					Handle::default(),
					CaptureMode::Shaded,
					ITEM_ICON_RESOLUTION,
					CaptureRevision::CURRENT,
				)
				.with_file_stem(map.name())
			)
		);
		Ok(())
	}

	#[test]
	fn skill_maps_are_remembered_not_enqueued() -> anyhow::Result<()> {
		use character_items::{SkillMapKind, SkillMapSpec};

		let item = InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1));
		let inventory = Inventory::with_starter_outfit(vec![item.clone()]);
		let mut discovery = ItemIconDiscovery::default();
		let mut subjects = ItemCaptureSubjects::default();
		let pending = pending_inventory_icons(
			&inventory,
			&mut discovery,
			&mut subjects,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		assert!(pending.is_empty());
		assert_eq!(discovery.requested_len(), 1);
		let id = CaptureId::new(item.capture_id_label().ok_or_else(|| anyhow::anyhow!("label"))?);
		assert!(subjects.get(&id).is_some());
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
	fn self_lubikan_icon_is_a_silhouette_host() -> anyhow::Result<()> {
		use character_items::self_lubikan_catalog_key;

		let subject = SelfLubikanSubject;
		assert_eq!(subject.capture_id(), CaptureId::new(SELF_LUBIKAN_CAPTURE_LABEL));
		assert!(matches!(
			subject.capture_scene(),
			CaptureScene::Host { kind, .. } if kind == HOST_SELF_LUBIKAN
		));
		let request = self_lubikan_icon_request(CaptureRevision::CURRENT);
		assert_eq!(request.mode, CaptureMode::Silhouette);
		assert_eq!(request.framing, CaptureFraming::ICON);
		assert_eq!(request.resolution, ITEM_ICON_RESOLUTION);
		assert_eq!(
			icon_catalog_key_for_label(SELF_LUBIKAN_CAPTURE_LABEL),
			self_lubikan_catalog_key()
		);
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
		assert_eq!(
			cache.entry(&CaptureCacheKey::from_request(&request)).map(IconEntry::status),
			Some(IconEntryStatus::Ready)
		);
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
		assert_eq!(
			cache.entry(&CaptureCacheKey::from_request(&request)).map(IconEntry::status),
			Some(IconEntryStatus::Failed)
		);
		Ok(())
	}
}
