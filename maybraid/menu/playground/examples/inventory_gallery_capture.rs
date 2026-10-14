//! Headless capture of character-generation inventory and the character menu.
//!
//! ```bash
//! INVENTORY_GALLERY_CAPTURE_DIR=/tmp/inventory-gallery \
//!   INVENTORY_GALLERY_CAPTURE_TAG=before \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p menu-playground --example inventory_gallery_capture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};
use character_creation_menus::{CharacterMenu, MenuEvent};
use character_items::{
	ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor, SkillMapKind,
	SkillMapSpec,
};
use character_ui_menu::{AssetThumbnailDisplay, MenuComponent, MenuNode};
use maybraid_character_ui_menu_renderer::{
	MaybraidMenuSink, MenuJustify, MenuSink, NoThumbnails, RenderContext, find_overlay_node,
	render_overlay_body, spawn_overlay_shell,
};
use menu_components::theme::{
	HEADER_FONT_SIZE, PANEL_LABEL_FONT_SIZE, PANEL_ROW_GAP, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};
use menu_components::{
	HudFonts, HudMenu, HudMenuItem, spawn_grid_catalog_tile, spawn_hud_action, spawn_hud_text,
	spawn_tile_grid,
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
						title: "Inventory gallery capture".into(),
						resolution: (960, 1080).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path().to_string_lossy().into(), ..default() })
				.disable::<AudioPlugin>(),
		)
		.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.10)))
		.insert_resource(CaptureDir(shot_dir()))
		.insert_resource(CaptureTag(shot_tag()))
		.init_resource::<CaptureShots>()
		.add_systems(Startup, setup)
		.add_systems(Update, (capture_scene, advance_after_shot).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource)]
struct CaptureTag(String);

#[derive(Resource, Default)]
struct CaptureShots {
	scene: CaptureScene,
	requested: bool,
	shown_at: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CaptureScene {
	#[default]
	Grant,
	Menu,
	Modal,
	Done,
}

impl CaptureScene {
	fn next(self) -> Self {
		match self {
			Self::Grant => Self::Menu,
			Self::Menu => Self::Modal,
			Self::Modal | Self::Done => Self::Done,
		}
	}

	fn file_stem(self) -> &'static str {
		match self {
			Self::Grant => "01-character-generation-gallery",
			Self::Menu => "02-character-menu-inventory-entry",
			Self::Modal => "03-inventory-modal-headers",
			Self::Done => "done",
		}
	}
}

fn shot_dir() -> PathBuf {
	std::env::var_os("INVENTORY_GALLERY_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/inventory-gallery"))
}

fn shot_tag() -> String {
	std::env::var("INVENTORY_GALLERY_CAPTURE_TAG").unwrap_or_else(|_| "shot".into())
}

fn sample_items() -> Vec<InventoryItem> {
	vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Hawaiian, ItemColor::Blue),
		InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::firearm(FirearmMesh::Bullpup),
		InventoryItem::firearm(FirearmMesh::Reltor),
		InventoryItem::standard_grenade(),
		InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
	]
}

fn sample_menu() -> CharacterMenu {
	let mut menu = CharacterMenu::for_create(sample_items());
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
	let items = sample_items();
	commands
		.spawn((
			CaptureRoot,
			SpinRevealScreen,
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(18.0),
				padding: UiRect::all(Val::Px(32.0)),
				..default()
			},
			BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
		))
		.with_children(|root| {
			if capture_before() {
				spawn_sequential_card(root, fonts, &items[0], items.len());
			} else {
				spawn_all_item_gallery(root, fonts, &items);
			}
		});
}

fn capture_before() -> bool {
	std::env::var("INVENTORY_GALLERY_CAPTURE_TAG").ok().as_deref() == Some("before")
}

fn spawn_sequential_card(
	root: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	item: &InventoryItem,
	total: usize,
) {
	spawn_hud_text(
		root,
		fonts.header(HEADER_FONT_SIZE * 0.42),
		&item.name(),
		TEXT_YELLOW,
		bevy::text::Justify::Center,
	);
	if !item.catalog_detail().is_empty() {
		spawn_hud_text(
			root,
			fonts.item(PANEL_LABEL_FONT_SIZE),
			&item.catalog_detail(),
			TEXT_YELLOW_FAINT,
			bevy::text::Justify::Center,
		);
	}
	spawn_hud_text(
		root,
		fonts.item(PANEL_LABEL_FONT_SIZE),
		&format!("Clothing Item (1/{total})"),
		TEXT_YELLOW_FAINT,
		bevy::text::Justify::Center,
	);
	spawn_hud_action(
		root,
		fonts,
		"Next",
		JustifyContent::Center,
		HudMenuItem { index: 0, menu: Entity::PLACEHOLDER },
	);
}

fn spawn_all_item_gallery(
	root: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	items: &[InventoryItem],
) {
	spawn_hud_text(
		root,
		fonts.header(HEADER_FONT_SIZE * 0.42),
		"Inventory",
		TEXT_YELLOW,
		bevy::text::Justify::Center,
	);
	spawn_tile_grid(root, JustifyContent::Center, |grid| {
		for item in items {
			spawn_grid_catalog_tile(
				grid,
				fonts,
				&item.name(),
				&item.catalog_detail(),
				false,
				None,
				None,
				None,
				None,
				true,
				(),
			);
		}
	});
	spawn_hud_action(
		root,
		fonts,
		"Continue",
		JustifyContent::Center,
		HudMenuItem { index: 0, menu: Entity::PLACEHOLDER },
	);
}

fn spawn_menu_scene(commands: &mut Commands, fonts: &HudFonts) {
	if capture_before() {
		spawn_legacy_menu_scene(commands, fonts);
		return;
	}
	let menu = sample_menu();
	let nodes = menu.menu_nodes();
	commands
		.spawn((
			CaptureRoot,
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::FlexEnd,
				align_items: AlignItems::FlexStart,
				padding: UiRect::all(Val::Px(24.0)),
				..default()
			},
			BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
		))
		.with_children(|root| {
			root.spawn((
				Node {
					width: Val::Px(420.0),
					height: Val::Percent(92.0),
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::FlexEnd,
					row_gap: Val::Px(PANEL_ROW_GAP),
					padding: UiRect::all(Val::Px(16.0)),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|panel| {
				let mut list = panel.spawn((
					HudMenu::new(0),
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::FlexEnd,
						row_gap: Val::Px(PANEL_ROW_GAP),
						..default()
					},
					Pickable::IGNORE,
				));
				let hud = list.id();
				list.with_children(|panel| {
					paint_panel(panel, fonts, hud, &nodes);
				});
			});
		});
}

fn paint_panel(
	panel: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	hud: Entity,
	nodes: &[MenuNode<MenuEvent>],
) {
	let mut prewarm = Vec::new();
	let mut thumbnails = NoThumbnails;
	let mut context = RenderContext {
		fonts,
		thumbnails: &mut thumbnails,
		asset_thumbnails: AssetThumbnailDisplay::None,
		prewarm: &mut prewarm,
		hud_menu: hud,
		hud_item_count: 0,
		interactive: true,
		lock_appearance: false,
	};
	MaybraidMenuSink::new(MenuJustify::Right).render_nodes(nodes, panel, &mut context);
}

fn spawn_legacy_menu_scene(commands: &mut Commands, fonts: &HudFonts) {
	commands
		.spawn((
			CaptureRoot,
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::FlexEnd,
				align_items: AlignItems::FlexStart,
				padding: UiRect::all(Val::Px(24.0)),
				..default()
			},
			BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
		))
		.with_children(|root| {
			root.spawn((
				Node {
					width: Val::Px(420.0),
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::FlexEnd,
					row_gap: Val::Px(PANEL_ROW_GAP),
					padding: UiRect::all(Val::Px(16.0)),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|panel| {
				for label in ["Name", "Species", "Weapons", "Skill Maps", "Loadout"] {
					spawn_hud_text(
						panel,
						fonts.header(HEADER_FONT_SIZE * 0.32),
						label,
						TEXT_YELLOW,
						bevy::text::Justify::Right,
					);
				}
			});
		});
}

fn spawn_legacy_weapons_modal(commands: &mut Commands, fonts: &HudFonts) {
	let items = sample_items();
	let weapons: Vec<_> = items.iter().filter(|item| item.firearm_mesh().is_some()).collect();
	commands
		.spawn((
			CaptureRoot,
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.88)),
		))
		.with_children(|root| {
			root.spawn((
				Node {
					position_type: PositionType::Absolute,
					left: Val::Percent(6.0),
					top: Val::Percent(8.0),
					width: Val::Percent(62.0),
					height: Val::Percent(84.0),
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(PANEL_ROW_GAP),
					padding: UiRect::all(Val::Px(8.0)),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|panel| {
				spawn_hud_text(
					panel,
					fonts.header(HEADER_FONT_SIZE * 0.36),
					"Weapons",
					TEXT_YELLOW,
					bevy::text::Justify::Left,
				);
				spawn_tile_grid(panel, JustifyContent::FlexStart, |grid| {
					for item in weapons {
						spawn_grid_catalog_tile(
							grid,
							fonts,
							&item.name(),
							&item.catalog_detail(),
							true,
							None,
							None,
							None,
							None,
							false,
							(),
						);
					}
				});
			});
		});
}

fn spawn_modal_scene(commands: &mut Commands, fonts: &HudFonts) {
	if capture_before() {
		spawn_legacy_weapons_modal(commands, fonts);
		return;
	}
	let menu = sample_menu();
	let nodes = menu.menu_nodes();
	let overlay_key = find_overlay_node(&nodes, "Inventory")
		.map(|_| "Inventory")
		.or_else(|| find_overlay_node(&nodes, "Weapons").map(|_| "Weapons"))
		.unwrap_or("Weapons");
	let Some(node) = find_overlay_node(&nodes, overlay_key).cloned() else {
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
			viewport = spawn_overlay_shell(root, fonts, overlay_key, TEXT_YELLOW);
		});
	fill_overlay_viewport(commands, viewport, fonts, &node);
}

fn fill_overlay_viewport(
	commands: &mut Commands,
	viewport: Entity,
	fonts: &HudFonts,
	node: &MenuNode<MenuEvent>,
) {
	let Ok(mut entity) = commands.get_entity(viewport) else {
		return;
	};
	let mut prewarm = Vec::new();
	let mut thumbnails = NoThumbnails;
	entity.with_children(|body| {
		let mut context = RenderContext {
			fonts,
			thumbnails: &mut thumbnails,
			asset_thumbnails: AssetThumbnailDisplay::None,
			prewarm: &mut prewarm,
			hud_menu: viewport,
			hud_item_count: 0,
			interactive: true,
			lock_appearance: false,
		};
		render_overlay_body(node, body, &mut context, MenuJustify::Left);
	});
}

fn capture_scene(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	tag: Res<CaptureTag>,
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
	let path = dir.0.join(format!("{}-{}.png", shots.scene.file_stem(), tag.0));
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
	shots.requested = true;
}

#[allow(clippy::too_many_arguments)]
fn advance_after_shot(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	tag: Res<CaptureTag>,
	assets: Res<AssetServer>,
	mut shots: ResMut<CaptureShots>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
	roots: Query<Entity, With<CaptureRoot>>,
) {
	if shots.scene == CaptureScene::Done {
		if time.elapsed_secs() > 14.0 {
			exit.write(AppExit::Success);
		}
		return;
	}
	if !shots.requested {
		if time.elapsed_secs() > 20.0 {
			exit.write(AppExit::Success);
		}
		return;
	}
	let path = dir.0.join(format!("{}-{}.png", shots.scene.file_stem(), tag.0));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if !(ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| time.elapsed_secs() > 20.0)
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
		CaptureScene::Menu => spawn_menu_scene(&mut commands, &fonts),
		CaptureScene::Modal => spawn_modal_scene(&mut commands, &fonts),
		_ => {}
	}
}
