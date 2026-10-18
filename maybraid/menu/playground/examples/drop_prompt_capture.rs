//! Headless capture of the inventory drop confirm card.
//!
//! ```bash
//! INVENTORY_DROP_CAPTURE_DIR=/tmp/inventory-drop-menu \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p menu-playground --example drop_prompt_capture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use bevy::text::Justify;
use menu_components::theme::{HEADER_FONT_SIZE, PANEL_ROW_GAP, TEXT_YELLOW};
use menu_components::{
	spawn_hud_action, spawn_hud_text, HudFonts, HudMenu, HudMenuItem, HudOverlayMenu,
};

fn assets_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Inventory drop confirm".into(),
						resolution: (960, 540).into(),
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
		.add_systems(Startup, spawn_card)
		.add_systems(Update, (capture_card, quit_after_shot).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource, Default)]
struct CaptureShots {
	confirm: bool,
}

fn shot_dir() -> PathBuf {
	std::env::var_os("INVENTORY_DROP_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/inventory-drop-menu"))
}

fn spawn_card(mut commands: Commands, assets: Res<AssetServer>, dir: Res<CaptureDir>) {
	let _ = std::fs::create_dir_all(&dir.0);
	commands.spawn(Camera2d);
	let fonts = HudFonts::load(assets.as_ref());
	commands
		.spawn((
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
		))
		.with_children(|root| {
			root.spawn((
				Node {
					width: Val::Px(720.0),
					max_width: Val::Percent(92.0),
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::FlexStart,
					row_gap: Val::Px(PANEL_ROW_GAP),
					..default()
				},
				Pickable::default(),
			))
			.with_children(|card| {
				spawn_hud_text(
					card,
					fonts.header(HEADER_FONT_SIZE * 0.42),
					"Drop Bullpup?",
					TEXT_YELLOW,
					Justify::Left,
				);
				let mut list = card.spawn((
					HudOverlayMenu,
					HudMenu::new(2),
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::FlexStart,
						row_gap: Val::Px(PANEL_ROW_GAP),
						..default()
					},
					Pickable::IGNORE,
				));
				let menu = list.id();
				list.with_children(|list| {
					spawn_hud_action(
						list,
						&fonts,
						"Drop",
						JustifyContent::FlexStart,
						HudMenuItem { index: 0, menu },
					);
					spawn_hud_action(
						list,
						&fonts,
						"Cancel",
						JustifyContent::FlexStart,
						HudMenuItem { index: 1, menu },
					);
				});
			});
		});
}

fn capture_card(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	mut shots: ResMut<CaptureShots>,
) {
	if !shots.confirm && time.elapsed_secs() >= 2.5 {
		let path = dir.0.join("01-drop-confirm.png");
		commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
		shots.confirm = true;
	}
}

fn request_exists(path: impl AsRef<Path>) -> bool {
	path.as_ref().is_file()
}

fn quit_after_shot(
	time: Res<Time>,
	shots: Res<CaptureShots>,
	dir: Res<CaptureDir>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
) {
	let path = dir.0.join("01-drop-confirm.png");
	if shots.confirm && request_exists(&path) && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| time.elapsed() > Duration::from_secs(20)
	{
		exit.write(AppExit::Success);
	}
}
