use bevy::ecs::event::EntityEvent;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{
	BevyMenuSink, MenuSink, MenuThumbnailContext, RenderContext,
};
use character_creation_menus::SectionOpenState;
use character_ui_menu::{AssetThumbnailDisplay, MenuComponent, ThumbnailRequest};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText, GameCommandUiConfig};

use crate::{
	menu_listeners::CharacterMenuState, preview::ConceptPreviewConfig, thumbnail::ThumbnailCache,
};

pub use character_ui_menu::CameraFocus;

const PANEL_WIDTH: f32 = 360.0;
const PANEL_HEIGHT_PERCENT: f32 = 82.0;
const SCROLL_LINE_PX: f32 = 14.0;

#[derive(Resource, Debug)]
pub struct CreatorUiState {
	pub sections: SectionOpenState,
	pub hovered: Option<CameraFocus>,
	pub last_selected: Option<CameraFocus>,
	pub asset_thumbnails: AssetThumbnailDisplay,
	layout_revision: u64,
}

impl Default for CreatorUiState {
	fn default() -> Self {
		Self {
			sections: SectionOpenState::default(),
			hovered: None,
			last_selected: None,
			asset_thumbnails: AssetThumbnailDisplay::default(),
			layout_revision: 0,
		}
	}
}

impl CreatorUiState {
	pub fn focused_target(&self) -> Option<CameraFocus> {
		self.hovered.or(self.last_selected)
	}

	pub fn bump_layout_revision(&mut self) {
		self.layout_revision += 1;
	}

	pub(crate) fn toggle_section(&mut self, section: character_creation_menus::SectionId) {
		self.sections.toggle(section);
		self.bump_layout_revision();
	}
}

#[derive(Resource, Default)]
pub struct CreatorUiSyncState {
	layout_revision: u64,
	menu_dirty: bool,
}

#[derive(Component)]
pub struct CreatorUiRoot;

#[derive(Component)]
pub struct CreatorUiScrollViewport;

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct CreatorUiScroll {
	pub entity: Entity,
	pub delta: Vec2,
}

struct CachedThumbnails<'a> {
	cache: &'a ThumbnailCache,
}

impl MenuThumbnailContext for CachedThumbnails<'_> {
	fn image_for_asset(
		&mut self,
		_label: &'static str,
		asset_path: &'static str,
		color: Color,
		_camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>> {
		self.cache.cached_image(asset_path, color)
	}
}

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "character concepts - / cmd - F1 drawer - L look - WASD".into(),
		empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.12, 0.14, 0.18, 0.82),
		controls_hint: "F1 hide drawer - L lock look - help - Enter - up/down history".into(),
	}
}

pub fn drawer_config() -> GameCommandDrawerConfig {
	GameCommandDrawerConfig { open_at_start: false, ..GameCommandDrawerConfig::default() }
}

pub(crate) fn sync_command_status_text(
	config: Res<ConceptPreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = config.status_label();
}

pub fn setup_creator_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
	mut thumbnails: ResMut<ThumbnailCache>,
	menu_state: Res<CharacterMenuState>,
	ui_state: Res<CreatorUiState>,
	mut sync_state: ResMut<CreatorUiSyncState>,
) {
	sync_state.layout_revision = ui_state.layout_revision;
	sync_state.menu_dirty = false;
	let viewport = spawn_creator_ui_shell(&mut commands);
	rebuild_creator_ui_panel(
		&mut commands,
		asset_server.as_ref(),
		images.as_mut(),
		thumbnails.as_mut(),
		menu_state.as_ref(),
		ui_state.as_ref(),
		[viewport],
	);
}

pub fn sync_creator_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
	mut thumbnails: ResMut<ThumbnailCache>,
	menu_state: Res<CharacterMenuState>,
	ui_state: Res<CreatorUiState>,
	mut sync_state: ResMut<CreatorUiSyncState>,
	roots: Query<Entity, With<CreatorUiRoot>>,
	viewports: Query<Entity, With<CreatorUiScrollViewport>>,
) {
	let layout_changed = sync_state.layout_revision != ui_state.layout_revision;
	let menu_changed = sync_state.menu_dirty || menu_state.is_changed();
	if !layout_changed && !menu_changed && !roots.is_empty() {
		return;
	}
	sync_state.layout_revision = ui_state.layout_revision;
	sync_state.menu_dirty = false;
	if roots.is_empty() {
		spawn_creator_ui_shell(&mut commands);
	}
	rebuild_creator_ui_panel(
		&mut commands,
		asset_server.as_ref(),
		images.as_mut(),
		thumbnails.as_mut(),
		menu_state.as_ref(),
		ui_state.as_ref(),
		&viewports,
	);
}

pub fn refresh_creator_ui_display() {}

pub(crate) fn mark_menu_ui_dirty(sync_state: &mut CreatorUiSyncState) {
	sync_state.menu_dirty = true;
}

fn spawn_creator_ui_shell(commands: &mut Commands) -> Entity {
	let mut viewport = Entity::PLACEHOLDER;
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(10.0),
				right: Val::Px(10.0),
				width: Val::Px(PANEL_WIDTH),
				height: Val::Percent(PANEL_HEIGHT_PERCENT),
				padding: UiRect::all(Val::Px(10.0)),
				flex_direction: FlexDirection::Column,
				overflow: Overflow::clip(),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.84)),
			CreatorUiRoot,
		))
		.with_children(|shell| {
			viewport = shell
				.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_grow: 1.0,
						flex_shrink: 1.0,
						min_height: Val::Px(0.0),
						flex_direction: FlexDirection::Column,
						row_gap: Val::Px(8.0),
						overflow: Overflow::scroll_y(),
						..default()
					},
					ScrollPosition::default(),
					Pickable::default(),
					CreatorUiScrollViewport,
				))
				.id();
		});
	viewport
}

fn rebuild_creator_ui_panel(
	commands: &mut Commands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	menu_state: &CharacterMenuState,
	ui_state: &CreatorUiState,
	viewports: impl IntoIterator<Item = Entity>,
) {
	thumbnails.begin_ui_rebuild();
	let mut prewarm = Vec::new();
	for viewport in viewports {
		commands.entity(viewport).despawn_related::<Children>();
		commands.entity(viewport).with_children(|panel| {
			populate_creator_ui_panel(panel, thumbnails, menu_state, ui_state, &mut prewarm);
		});
	}
	if ui_state.asset_thumbnails != AssetThumbnailDisplay::None {
		crate::thumbnail::prewarm_thumbnail_requests(
			commands,
			images,
			asset_server,
			thumbnails,
			&prewarm,
		);
	}
}

fn populate_creator_ui_panel(
	panel: &mut ChildSpawnerCommands,
	thumbnails: &ThumbnailCache,
	menu_state: &CharacterMenuState,
	ui_state: &CreatorUiState,
	prewarm: &mut Vec<ThumbnailRequest>,
) {
	text(panel, "Character Concepts", 15.0, Color::WHITE);
	text(panel, "Typed menu primitives rendered through a shared Bevy layer.", 10.0, muted());
	let mut cached = CachedThumbnails { cache: thumbnails };
	let mut context = RenderContext {
		sections: &ui_state.sections,
		thumbnails: &mut cached,
		asset_thumbnails: ui_state.asset_thumbnails,
		prewarm,
	};
	BevyMenuSink.render_nodes(&menu_state.0.menu_nodes(), panel, &mut context);
}

fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
	parent.spawn((
		Text::new(value.to_string()),
		TextFont { font_size: FontSize::Px(size), ..default() },
		TextColor(color),
		Pickable::IGNORE,
	));
}

fn muted() -> Color {
	Color::srgba(0.72, 0.78, 0.86, 1.0)
}

pub fn send_creator_ui_scroll_events(
	mut mouse_wheel_reader: MessageReader<MouseWheel>,
	hover_map: Res<HoverMap>,
	mut commands: Commands,
) {
	for mouse_wheel in mouse_wheel_reader.read() {
		let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);
		if mouse_wheel.unit == MouseScrollUnit::Line {
			delta *= SCROLL_LINE_PX;
		}
		for pointer_map in hover_map.values() {
			for entity in pointer_map.keys().copied() {
				commands.trigger(CreatorUiScroll { entity, delta });
			}
		}
	}
}

pub fn on_creator_ui_scroll(
	mut scroll: On<CreatorUiScroll>,
	mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<CreatorUiScrollViewport>>,
) {
	let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
		return;
	};
	let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
	let delta = &mut scroll.delta;
	if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
		let max =
			if delta.x > 0. { scroll_position.x >= max_offset.x } else { scroll_position.x <= 0. };
		if !max {
			scroll_position.x += delta.x;
			delta.x = 0.;
		}
	}
	if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
		let max =
			if delta.y > 0. { scroll_position.y >= max_offset.y } else { scroll_position.y <= 0. };
		if !max {
			scroll_position.y += delta.y;
			delta.y = 0.;
		}
	}
	if *delta == Vec2::ZERO {
		scroll.propagate(false);
	}
}
