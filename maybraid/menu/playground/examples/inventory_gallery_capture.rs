//! Headless capture of the shared inventory layout (Starter Kit + modal).
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
use character_creation_menus::{overflow_inventory_items, CharacterMenu, InventoryLayout};
use character_ui_menu::MenuComponent;
use maybraid_character_ui_menu_renderer::{
	fill_inventory_layout, find_overlay_node, spawn_overlay_shell, spawn_starter_kit_shell,
	NoThumbnails,
};
use menu_components::theme::TEXT_YELLOW;
use menu_components::{spawn_hud_action, HudFonts, HudMenuItem, HudScrollViewport};
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
		.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.10)))
		.insert_resource(CaptureDir(shot_dir()))
		.init_resource::<CaptureShots>()
		.add_systems(Startup, setup)
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
	GrantTop,
	GrantScrolled,
	ModalTop,
	ModalScrolled,
	Done,
}

impl CaptureScene {
	fn next(self) -> Self {
		match self {
			Self::GrantTop => Self::GrantScrolled,
			Self::GrantScrolled => Self::ModalTop,
			Self::ModalTop => Self::ModalScrolled,
			Self::ModalScrolled | Self::Done => Self::Done,
		}
	}

	fn file_stem(self) -> &'static str {
		match self {
			Self::GrantTop => "starter_kit_top",
			Self::GrantScrolled => "starter_kit_scrolled",
			Self::ModalTop => "inventory_modal_top",
			Self::ModalScrolled => "inventory_modal_scrolled",
			Self::Done => "done",
		}
	}

	fn scrolled(self) -> bool {
		matches!(self, Self::GrantScrolled | Self::ModalScrolled)
	}
}

fn shot_dir() -> PathBuf {
	std::env::var_os("INVENTORY_GALLERY_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/inventory-gallery"))
}

fn sample_menu() -> CharacterMenu {
	let mut menu = CharacterMenu::for_create(overflow_inventory_items());
	menu.name = String::from("Misty");
	menu
}

#[derive(Component)]
struct CaptureRoot;

fn setup(mut commands: Commands, assets: Res<AssetServer>, dir: Res<CaptureDir>) {
	let _ = std::fs::create_dir_all(&dir.0);
	commands.spawn(Camera2d);
	let fonts = HudFonts::load(assets.as_ref());
	spawn_grant_scene(&mut commands, &fonts);
}

fn spawn_grant_scene(commands: &mut Commands, fonts: &HudFonts) {
	let layout = InventoryLayout::starter_kit(overflow_inventory_items());
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
			viewport = spawn_starter_kit_shell(root, fonts, layout.title, TEXT_YELLOW, ());
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
	let mut thumbs = NoThumbnails;
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

fn spawn_modal_scene(commands: &mut Commands, fonts: &HudFonts) {
	let menu = sample_menu();
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
			viewport = spawn_overlay_shell(root, fonts, "Inventory", TEXT_YELLOW);
		});
	let mut thumbs = NoThumbnails;
	fill_inventory_layout(commands, viewport, fonts, &node, true, None, &mut thumbs);
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
	mut shots: ResMut<CaptureShots>,
	roots: Query<(), With<CaptureRoot>>,
) {
	if shots.scene == CaptureScene::Done || shots.requested || roots.is_empty() {
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
		if time.elapsed_secs() > 24.0 {
			exit.write(AppExit::Success);
		}
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if !(ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| time.elapsed_secs() > 24.0)
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
	match shots.scene {
		CaptureScene::GrantScrolled => spawn_grant_scene(&mut commands, &fonts),
		CaptureScene::ModalTop | CaptureScene::ModalScrolled => {
			spawn_modal_scene(&mut commands, &fonts)
		}
		_ => {}
	}
}
