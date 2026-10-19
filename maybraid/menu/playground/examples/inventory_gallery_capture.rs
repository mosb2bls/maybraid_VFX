//! Headless capture of the shared inventory layout (Starter Kit + modal).
//!
//! The overflow bag is a **fixture** for the scroll demo. It is not the
//! generated Starter Kit. Real generation still grants three clothing items.
//!
//! ```bash
//! INVENTORY_GALLERY_CAPTURE_DIR=/tmp/inventory-gallery \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p menu-playground --example inventory_gallery_capture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use character_creation_menus::{CharacterMenu, InventoryLayout, STARTER_KIT_LABEL};
use character_items::{
	self_lubikan_catalog_key, ClothingMaterial, ClothingMesh, FirearmMesh, Inventory,
	InventoryItem, ItemColor, ItemRng, SkillMapKind, SkillMapSpec,
};
use character_ui_menu::MenuComponent;
use characters::StandaloneCharacterMaterialRefPlugin;
use item_icons::{
	request_inventory_icons, CaptureCacheRoot, CaptureClient, CaptureCompleted, CaptureFailed,
	CaptureHostReady, CaptureHostRealized, CaptureHostRoot, ItemCaptureSubjects, ItemIconCache,
	ItemIconDiscovery, ItemIconPlugin,
};
use maybraid_character_ui_menu_renderer::{
	fill_inventory_layout, fill_inventory_loading, find_overlay_node, spawn_overlay_shell,
	spawn_starter_kit_shell, MenuThumbnailContext,
};
use menu_components::theme::TEXT_YELLOW;
use menu_components::{
	spawn_header_line, spawn_hud_action, HudFonts, HudMenuItem, HudScrollViewport,
	InputPromptDevice,
};
use menu_screens::SpinRevealScreen;

fn assets_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Inventory layout capture".into(),
						resolution: (960, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path().to_string_lossy().into(), ..default() })
				.disable::<AudioPlugin>(),
		)
		.add_plugins(StandaloneCharacterMaterialRefPlugin)
		.add_plugins(ItemIconPlugin)
		.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.10)))
		.insert_resource(CaptureDir(shot_dir()))
		.init_resource::<CaptureShots>()
		.add_systems(Startup, (setup, allow_slow_braidman_icon, request_icon))
		.add_systems(Update, (log_capture_outcome, log_host_progress))
		.add_systems(Update, (apply_capture_scroll, capture_scene, advance_after_shot).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource, Default)]
struct CaptureShots {
	scene: CaptureScene,
	requested: bool,
	shown_at: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CaptureScene {
	#[default]
	Loading,
	WaitingForIcon,
	RealStarterKit,
	OverflowFixtureTop,
	OverflowFixtureScrolled,
	ModalInventory,
	ModalLubikan,
	IconCloseup,
	Done,
}

impl CaptureScene {
	fn next(self) -> Self {
		match self {
			Self::Loading => Self::WaitingForIcon,
			Self::WaitingForIcon => Self::RealStarterKit,
			Self::RealStarterKit => Self::OverflowFixtureTop,
			Self::OverflowFixtureTop => Self::OverflowFixtureScrolled,
			Self::OverflowFixtureScrolled => Self::ModalInventory,
			Self::ModalInventory => Self::ModalLubikan,
			Self::ModalLubikan => Self::IconCloseup,
			Self::IconCloseup | Self::Done => Self::Done,
		}
	}

	fn file_stem(self) -> &'static str {
		match self {
			Self::Loading => "inventory_loading",
			Self::WaitingForIcon => "waiting",
			Self::RealStarterKit => "real_starter_kit",
			Self::OverflowFixtureTop => "overflow_fixture_top",
			Self::OverflowFixtureScrolled => "overflow_fixture_scrolled",
			Self::ModalInventory => "inventory_modal",
			Self::ModalLubikan => "inventory_modal_lubikan",
			Self::IconCloseup => "self_lubikan_icon_closeup",
			Self::Done => "done",
		}
	}

	fn scrolled(self) -> bool {
		matches!(self, Self::OverflowFixtureScrolled | Self::ModalLubikan)
	}
}

struct CatalogThumbs<'a> {
	icons: Option<&'a ItemIconCache>,
}

impl MenuThumbnailContext for CatalogThumbs<'_> {
	fn image_for_asset(
		&mut self,
		_label: &'static str,
		_asset_path: &'static str,
		_color: Color,
		_camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>> {
		None
	}

	fn image_for_key(&mut self, key: u64) -> Option<Handle<Image>> {
		let icons = self.icons?;
		if key == self_lubikan_catalog_key() {
			icons.silhouette_for_key(key)
		} else {
			icons.catalog_for_key(key)
		}
	}

	fn tint_for_key(&mut self, key: u64) -> Color {
		if key == self_lubikan_catalog_key() {
			TEXT_YELLOW
		} else {
			Color::WHITE
		}
	}
}

fn shot_dir() -> PathBuf {
	std::env::var_os("INVENTORY_GALLERY_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/inventory-gallery"))
}

/// Generated Starter Kit: three clothing, two firearms, one grenade, one map.
fn real_starter_items() -> Vec<InventoryItem> {
	let mut rng = ItemRng::from_seed(42);
	rng.random_starter_loadout()
}

/// Scroll-demo fixture. Not granted by generation. Lives only in this example
/// and in `#[cfg(test)]` overflow coverage.
fn overflow_fixture_items() -> Vec<InventoryItem> {
	let mut items: Vec<InventoryItem> = ClothingMesh::VALUES
		.iter()
		.copied()
		.map(|mesh| InventoryItem::clothing(mesh, ClothingMaterial::Cloth, ItemColor::Natural))
		.collect();
	items.extend(FirearmMesh::VALUES.iter().copied().map(InventoryItem::firearm));
	items.push(InventoryItem::standard_grenade());
	items.push(InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)));
	items.push(InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)));
	items
}

fn sample_menu(items: Vec<InventoryItem>) -> CharacterMenu {
	let mut menu = CharacterMenu::for_create(items);
	menu.name = String::from("Misty");
	menu
}

#[derive(Component)]
struct CaptureRoot;

fn setup(mut commands: Commands, dir: Res<CaptureDir>) {
	let _ = std::fs::create_dir_all(&dir.0);
	commands.spawn(Camera2d);
}

fn allow_slow_braidman_icon(mut cache: ResMut<CaptureCacheRoot>) {
	cache.max_wait_frames = 3600;
}

fn request_icon(
	mut discovery: ResMut<ItemIconDiscovery>,
	mut subjects: ResMut<ItemCaptureSubjects>,
	mut client: CaptureClient,
) {
	for inventory in capture_icon_inventories() {
		request_inventory_icons(&inventory, &mut discovery, &mut subjects, &mut client);
	}
}

fn capture_icon_inventories() -> [Inventory; 2] {
	[
		Inventory::with_starter_outfit(real_starter_items()),
		Inventory::with_starter_outfit(overflow_fixture_items()),
	]
}

fn log_capture_outcome(
	mut failed: MessageReader<CaptureFailed>,
	mut completed: MessageReader<CaptureCompleted>,
) {
	for event in failed.read() {
		warn!(id = event.key.id.as_str(), error = %event.error, "item icon capture failed");
	}
	for event in completed.read() {
		info!(id = event.key.id.as_str(), "item icon capture completed");
	}
}

fn log_host_progress(
	time: Res<Time>,
	mut last: Local<u32>,
	hosts: Query<(
		Entity,
		&CaptureHostRoot,
		Option<&CaptureHostRealized>,
		Option<&CaptureHostReady>,
	)>,
	children: Query<&Children>,
	meshes: Query<Entity, With<Mesh3d>>,
) {
	let secs = time.elapsed_secs() as u32;
	if secs == *last || secs % 3 != 0 {
		return;
	}
	*last = secs;
	for (entity, host, realized, ready) in &hosts {
		let mut mesh_count = 0_u32;
		fn walk(
			entity: Entity,
			children: &Query<&Children>,
			meshes: &Query<Entity, With<Mesh3d>>,
			count: &mut u32,
		) {
			if meshes.get(entity).is_ok() {
				*count += 1;
			}
			if let Ok(kids) = children.get(entity) {
				for child in kids.iter() {
					walk(child, children, meshes, count);
				}
			}
		}
		walk(entity, &children, &meshes, &mut mesh_count);
		info!(
			kind = %host.kind,
			realized = realized.is_some(),
			ready = ready.is_some(),
			mesh_count,
			"capture host progress"
		);
	}
}

fn icon_ready(cache: &ItemIconCache) -> bool {
	capture_icon_inventories().iter().all(|inventory| {
		InventoryLayout::starter_kit(inventory.items.clone())
			.captures
			.iter()
			.all(|need| cache.filled_or_failed(need.key, need.silhouette))
	})
}

const SCENE_DEADLINE: Duration = Duration::from_secs(48);

fn scene_deadline_hit(shown_at: Option<Duration>, now: Duration) -> bool {
	shown_at.is_some_and(|at| now > at + SCENE_DEADLINE)
}

fn spawn_grant_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	items: Vec<InventoryItem>,
	title: &str,
	icons: Option<&ItemIconCache>,
) {
	let layout = InventoryLayout::starter_kit(items);
	let mut viewport = Entity::PLACEHOLDER;
	let mut footer = Entity::PLACEHOLDER;
	commands
		.spawn((
			CaptureRoot,
			SpinRevealScreen,
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				..default()
			},
			BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
		))
		.with_children(|root| {
			viewport = spawn_starter_kit_shell(root, fonts, title, TEXT_YELLOW, ());
			footer = root
				.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::Center,
						padding: UiRect::bottom(Val::Px(16.0)),
						..default()
					},
					Pickable::IGNORE,
				))
				.id();
		});
	let mut thumbs = CatalogThumbs { icons };
	let count =
		fill_inventory_layout(commands, viewport, fonts, &layout.node, false, None, &mut thumbs);
	if let Ok(mut footer) = commands.get_entity(footer) {
		footer.with_children(|footer| {
			spawn_hud_action(
				footer,
				fonts,
				"Continue",
				JustifyContent::Center,
				HudMenuItem { index: count, menu: viewport },
			);
		});
	}
}

fn spawn_modal_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	assets: &AssetServer,
	icons: Option<&ItemIconCache>,
) {
	let menu = sample_menu(real_starter_items());
	let nodes = menu.menu_nodes();
	let Some(node) = find_overlay_node(&nodes, "Inventory").cloned() else {
		return;
	};
	let mut viewport = Entity::PLACEHOLDER;
	commands
		.spawn((
			CaptureRoot,
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
		))
		.with_children(|root| {
			viewport = spawn_overlay_shell(
				root,
				fonts,
				"Inventory",
				TEXT_YELLOW,
				Some((assets, InputPromptDevice::Keyboard)),
			);
		});
	let mut thumbs = CatalogThumbs { icons };
	fill_inventory_layout(commands, viewport, fonts, &node, true, None, &mut thumbs);
}

fn spawn_icon_closeup(commands: &mut Commands, fonts: &HudFonts, image: Option<Handle<Image>>) {
	commands
		.spawn((
			CaptureRoot,
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(20.0),
				..default()
			},
			BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
		))
		.with_children(|root| {
			spawn_header_line(root, fonts, "Self Lubikan", None, 42.0, TEXT_YELLOW);
			root.spawn((
				Node { width: Val::Px(384.0), height: Val::Px(384.0), ..default() },
				match image {
					Some(image) => ImageNode { image, color: TEXT_YELLOW, ..default() },
					None => ImageNode::default(),
				},
				BackgroundColor(Color::srgb(0.12, 0.13, 0.16)),
			));
		});
}

fn spawn_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	assets: &AssetServer,
	scene: CaptureScene,
	icons: Option<&ItemIconCache>,
) {
	match scene {
		CaptureScene::RealStarterKit => {
			spawn_grant_scene(commands, fonts, real_starter_items(), STARTER_KIT_LABEL, icons);
		}
		CaptureScene::OverflowFixtureTop | CaptureScene::OverflowFixtureScrolled => {
			spawn_grant_scene(
				commands,
				fonts,
				overflow_fixture_items(),
				"Overflow fixture (not the starter grant)",
				icons,
			);
		}
		CaptureScene::ModalInventory | CaptureScene::ModalLubikan => {
			spawn_modal_scene(commands, fonts, assets, icons);
		}
		CaptureScene::IconCloseup => {
			let image =
				icons.and_then(|cache| cache.silhouette_for_key(self_lubikan_catalog_key()));
			spawn_icon_closeup(commands, fonts, image);
		}
		CaptureScene::Loading => {
			let mut viewport = Entity::PLACEHOLDER;
			commands
				.spawn((
					CaptureRoot,
					SpinRevealScreen,
					Node {
						width: Val::Percent(100.0),
						height: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						..default()
					},
					BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
				))
				.with_children(|root| {
					viewport =
						spawn_starter_kit_shell(root, fonts, STARTER_KIT_LABEL, TEXT_YELLOW, ());
				});
			fill_inventory_loading(commands, viewport, fonts);
		}
		CaptureScene::WaitingForIcon | CaptureScene::Done => {}
	}
}

fn apply_capture_scroll(
	shots: Res<CaptureShots>,
	mut viewports: Query<(&ComputedNode, &mut ScrollPosition), With<HudScrollViewport>>,
) {
	for (computed, mut scroll) in &mut viewports {
		let max = ((computed.content_size().y - computed.size().y)
			* computed.inverse_scale_factor())
		.max(0.0);
		scroll.y = if shots.scene.scrolled() { max } else { 0.0 };
	}
}

fn capture_scene(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	assets: Res<AssetServer>,
	cache: Res<ItemIconCache>,
	mut shots: ResMut<CaptureShots>,
	roots: Query<(), With<CaptureRoot>>,
) {
	if shots.scene == CaptureScene::Done || shots.requested {
		return;
	}
	if shots.scene == CaptureScene::WaitingForIcon {
		if icon_ready(&cache) || time.elapsed_secs() > 90.0 {
			let have = cache.silhouette_for_key(self_lubikan_catalog_key()).is_some();
			info!(have, secs = time.elapsed_secs(), "self-lubikan icon wait finished");
			shots.scene = CaptureScene::RealStarterKit;
			let fonts = HudFonts::load(assets.as_ref());
			spawn_scene(&mut commands, &fonts, assets.as_ref(), shots.scene, Some(cache.as_ref()));
		}
		return;
	}
	if roots.is_empty() {
		if shots.scene == CaptureScene::Loading {
			let fonts = HudFonts::load(assets.as_ref());
			spawn_scene(&mut commands, &fonts, assets.as_ref(), shots.scene, Some(cache.as_ref()));
		}
		return;
	}
	if shots.shown_at.is_none() {
		shots.shown_at = Some(time.elapsed());
	}
	if shots
		.shown_at
		.is_none_or(|at| time.elapsed() <= at + Duration::from_millis(2200))
	{
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
	shots.requested = true;
}

#[allow(clippy::too_many_arguments)]
fn advance_after_shot(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	assets: Res<AssetServer>,
	cache: Res<ItemIconCache>,
	mut shots: ResMut<CaptureShots>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
	roots: Query<Entity, With<CaptureRoot>>,
) {
	if shots.scene == CaptureScene::Done {
		if time.elapsed_secs() > 16.0 {
			exit.write(AppExit::Success);
		}
		return;
	}
	if !shots.requested {
		if shots.scene != CaptureScene::WaitingForIcon
			&& scene_deadline_hit(shots.shown_at, time.elapsed())
		{
			exit.write(AppExit::Success);
		}
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if !(ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| scene_deadline_hit(shots.shown_at, time.elapsed()))
	{
		return;
	}
	for entity in &roots {
		commands.entity(entity).try_despawn();
	}
	shots.requested = false;
	shots.shown_at = None;
	*ready_at = None;
	shots.scene = shots.scene.next();
	if shots.scene == CaptureScene::Done {
		exit.write(AppExit::Success);
		return;
	}
	let fonts = HudFonts::load(assets.as_ref());
	spawn_scene(&mut commands, &fonts, assets.as_ref(), shots.scene, Some(cache.as_ref()));
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::SELF_LUBIKAN_CAPTURE_LABEL;
	use item_icons::CaptureId;

	#[test]
	fn gallery_waits_for_starter_and_overflow_captures() {
		let mut cache = ItemIconCache::default();
		assert!(!icon_ready(&cache));
		cache.insert_silhouette(CaptureId::new(SELF_LUBIKAN_CAPTURE_LABEL), Handle::default());
		assert!(
			!icon_ready(&cache),
			"clothing / firearm / grenade captures must enter the wait set"
		);
		for inventory in capture_icon_inventories() {
			for item in &inventory.items {
				if matches!(item, InventoryItem::SkillMap { .. } | InventoryItem::Tanz { .. }) {
					continue;
				}
				let Some(label) = item.capture_id_label() else {
					continue;
				};
				cache.insert_shaded(CaptureId::new(label), Handle::default());
			}
		}
		assert!(icon_ready(&cache));
	}

	#[test]
	fn scene_deadlines_use_shown_at_not_total_app_time() {
		let shown_at = Some(Duration::from_secs(50));
		assert!(!scene_deadline_hit(shown_at, Duration::from_secs(60)));
		assert!(scene_deadline_hit(shown_at, Duration::from_secs(99)));
		assert!(!scene_deadline_hit(None, Duration::from_secs(90)));
	}
}
