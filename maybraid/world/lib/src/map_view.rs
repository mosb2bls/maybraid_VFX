//! Overhead map camera and place-name pins. One `Camera3d`, not a minimap.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::text::FontSize;
use durham::DurhamSurface;
use game_commands::command::TextEntryFocus;
use geneva::{LanguageOverlay, NameKey, NamedOverlay};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use menu_components::{
	BARLOW_BLACK, BONES_ICON, ITEM_FONT_SIZE, MAP_ARROW_ICON, MAP_GROVE_ICON, MAP_HOUSE_ICON,
	MAP_MOUNTAIN_ICON, MAP_TOWN_ICON, MAP_TREE_ICON, MAP_WATER_ICON, NOTO_SANS_REGULAR,
	TEXT_SALMON, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};
use mob_characters::{LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use player::CameraFollow;
use player_camera::{
	CameraController, CameraLookSuppressed, CameraPovLocked, FollowCamera, PlayerCameraSystems,
};
use poi_intelligence::{PoiId, PoiKind, PoiRecord, PoiRegistry};
use richmond::DiscoverablePlace;
use world_player::{Player as VegetationPlayer, PlayerLifeSet, PlaygroundMode};

use crate::control::{InventoryEditCameraFollow, WorldGameplayEnabled};
use crate::player_lifecycle::WorldPlayerRespawnState;
use combat_hud::ScreenPin;

use crate::ui::HUD_MARGIN;

pub const DEFAULT_MAP_HEIGHT: f32 = 420.0;
const MIN_MAP_HEIGHT: f32 = 80.0;
const MAX_MAP_HEIGHT: f32 = 2_400.0;
const MAP_PIN_LIMIT: usize = 48;
const MAX_REGION_LABELS: usize = 2;
const MAX_FEATURE_LABELS: usize = 6;
const MAX_POI_LABELS: usize = 10;
const REGION_LABEL_PX: f32 = 28.0;
const FEATURE_LABEL_PX: f32 = 16.0;
const POI_LABEL_PX: f32 = 12.0;
const PLAYER_MARKER_PX: f32 = 14.0;
const DEATH_BONES_PX: f32 = 32.0;
const SELECTED_POI_LABEL_PX: f32 = 18.0;
const MAP_MARK_PX: f32 = 22.0;
const MAP_MARK_GAP: f32 = 6.0;
const MAP_ARROW_PX: f32 = 18.0;
const LABEL_SCREEN_GUTTER: f32 = 72.0;
/// Pull a label onto the frame only when its true projection is this close.
const EDGE_PULL_PX: f32 = 96.0;
const SELECTION_RING_PX: f32 = 46.0;
const SELECTION_DOT_PX: f32 = 10.0;
const SPAWN_KNOB_PX: f32 = 12.0;
const SPAWN_KNOB_ACTIVE_PX: f32 = 16.0;
const GIZMO_LIFT: f32 = 16.0;
const POI_NAME_RADIUS: f32 = 160.0;
const PICKER_TITLE: &str = "Pick Respawn Point";
const PICKER_TITLE_GUTTER: f32 = 48.0;
const SPAWN_KNOB_GRAY: Color = Color::srgb(0.58, 0.56, 0.52);

/// Overhead view of the current location. Focus moves with spawn-location picks only.
#[derive(Resource, Debug, PartialEq)]
pub struct WorldMapView {
	pub open: bool,
	pub focus: Vec2,
	pub height: f32,
	pub close_locked: bool,
	/// Death picker / new body: stamp third person instead of restoring the last POV.
	pub begin_life: bool,
}

impl Default for WorldMapView {
	fn default() -> Self {
		Self {
			open: false,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		}
	}
}

impl WorldMapView {
	pub fn open_at(&mut self, focus: Vec2, close_locked: bool) {
		self.open = true;
		self.focus = focus;
		self.height = self.height.clamp(MIN_MAP_HEIGHT, MAX_MAP_HEIGHT);
		if self.height < MIN_MAP_HEIGHT + 1.0 {
			self.height = DEFAULT_MAP_HEIGHT;
		}
		self.close_locked = close_locked;
	}

	pub fn close(&mut self) {
		if self.close_locked {
			self.begin_life = true;
		}
		self.open = false;
		self.close_locked = false;
	}

	pub fn request_begin_life(&mut self) {
		self.begin_life = true;
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum WorldMapSet {
	Toggle,
}

#[derive(Resource, Clone)]
struct MapLabelFont(Handle<Font>);

#[derive(Component)]
struct MapNameHud;

#[derive(Component)]
struct RespawnPickerTitle;

#[derive(Component)]
struct MapRespawnSelection;

#[derive(Component)]
struct MapSpawnKnob {
	id: PoiId,
}

#[derive(Component)]
struct MapNamePin {
	target: MapPinTarget,
}

#[derive(Component)]
struct MapPlayerMarker;

#[derive(Component)]
struct MapDeathBones;

#[derive(Component)]
struct MapTypeMark {
	target: MapPinTarget,
}

#[derive(Component)]
struct MapEdgeArrow {
	target: MapPinTarget,
}

#[derive(Resource, Clone)]
struct MapBonesIcon(Handle<Image>);

#[derive(Resource, Clone)]
struct MapMarkIcons {
	tree: Handle<Image>,
	grove: Handle<Image>,
	house: Handle<Image>,
	town: Handle<Image>,
	mountain: Handle<Image>,
	water: Handle<Image>,
	arrow: Handle<Image>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapMarkKind {
	Tree,
	Grove,
	House,
	Town,
	Mountain,
	Water,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapPinTarget {
	Name(NameKey),
	Poi(PoiId),
}

#[derive(Bundle)]
struct MapNamePinBundle {
	name: Name,
	pin: MapNamePin,
	node: Node,
	text: Text,
	font: TextFont,
	layout: TextLayout,
	color: TextColor,
	shadow: TextShadow,
	pickable: Pickable,
	visibility: Visibility,
}

pub struct WorldMapViewPlugin;

impl Plugin for WorldMapViewPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldMapView>()
			.init_resource::<LanguageOverlay>()
			.init_resource::<MapPresentation>()
			.configure_sets(
				Update,
				WorldMapSet::Toggle
					.after(CharacterControlSystems)
					.before(PlayerCameraSystems::Look),
			)
			.add_systems(Startup, spawn_map_name_hud)
			.add_systems(
				Update,
				(toggle_map_view, sync_map_camera_locks, pan_map_view)
					.chain()
					.in_set(WorldMapSet::Toggle),
			)
			.add_systems(
				Update,
				stamp_map_camera
					.after(WorldMapSet::Toggle)
					.after(PlayerLifeSet::Resolve)
					.before(PlayerCameraSystems::Look),
			)
			.add_systems(
				Update,
				(
					prepare_map_presentation,
					sync_map_name_pins,
					sync_map_type_marks,
					sync_map_edge_arrows,
					sync_map_player_marker,
					sync_map_death_bones,
					sync_respawn_picker_title,
					sync_respawn_spawn_knobs,
					sync_respawn_selection_marker,
					draw_highlighted_poi,
				)
					.chain()
					.after(PlayerCameraSystems::Apply),
			);
	}
}

fn spawn_map_name_hud(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(MapLabelFont(assets.load(NOTO_SANS_REGULAR)));
	commands.insert_resource(MapBonesIcon(assets.load(BONES_ICON)));
	commands.insert_resource(MapMarkIcons {
		tree: assets.load(MAP_TREE_ICON),
		grove: assets.load(MAP_GROVE_ICON),
		house: assets.load(MAP_HOUSE_ICON),
		town: assets.load(MAP_TOWN_ICON),
		mountain: assets.load(MAP_MOUNTAIN_ICON),
		water: assets.load(MAP_WATER_ICON),
		arrow: assets.load(MAP_ARROW_ICON),
	});
	commands.spawn((
		Name::new("map-name-hud"),
		MapNameHud,
		Node {
			position_type: PositionType::Absolute,
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			..default()
		},
		Pickable::IGNORE,
		Visibility::Hidden,
	));
	commands.spawn((
		Name::new("respawn-picker-title"),
		RespawnPickerTitle,
		Node {
			position_type: PositionType::Absolute,
			width: Val::Percent(100.0),
			padding: UiRect::new(
				Val::Px(PICKER_TITLE_GUTTER),
				Val::Px(PICKER_TITLE_GUTTER),
				Val::Px(PICKER_TITLE_GUTTER),
				Val::Px(0.0),
			),
			justify_content: JustifyContent::Center,
			..default()
		},
		Text::new(PICKER_TITLE),
		TextFont {
			font: assets.load(BARLOW_BLACK).into(),
			font_size: FontSize::Px(ITEM_FONT_SIZE),
			..default()
		},
		TextColor(TEXT_YELLOW),
		TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.06, 0.05, 0.04, 0.72) },
		TextLayout::new(Justify::Center, bevy::text::LineBreak::NoWrap),
		Pickable::IGNORE,
		Visibility::Hidden,
		GlobalZIndex(i32::MAX - 2),
	));
}

pub(crate) fn toggle_map_view(
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	mut map: ResMut<WorldMapView>,
	mut intents: MessageReader<CharacterIntent>,
	players: Query<&Transform, With<VegetationPlayer>>,
	followers: Query<&Transform, (With<CameraFollow>, Without<Camera3d>)>,
) {
	if text_focus.0 || *mode != PlaygroundMode::Character {
		return;
	}
	let mut open = false;
	let mut close = false;
	for intent in intents.read() {
		match intent {
			CharacterIntent::ToggleMap if map.open => close = true,
			CharacterIntent::ToggleMap | CharacterIntent::OpenMap => open = true,
			CharacterIntent::CloseMap => close = true,
			_ => {}
		}
	}
	if map.close_locked {
		return;
	}
	if close && map.open {
		map.close();
		return;
	}
	if !open || map.open || !gameplay.0 {
		return;
	}
	let Some(xz) = players
		.iter()
		.next()
		.or_else(|| followers.iter().next())
		.map(|transform| transform.translation.xz())
	else {
		return;
	};
	map.open_at(xz, false);
}

fn sync_map_camera_locks(
	map: Res<WorldMapView>,
	edit: Option<Res<InventoryEditCameraFollow>>,
	mut locked: Option<ResMut<CameraPovLocked>>,
	mut suppressed: Option<ResMut<CameraLookSuppressed>>,
) {
	let hold = map.open || edit.is_some_and(|edit| edit.0);
	if let Some(locked) = locked.as_deref_mut() {
		locked.0 = hold;
	}
	if let Some(suppressed) = suppressed.as_deref_mut() {
		suppressed.0 = map.open;
	}
}

fn pan_map_view(
	time: Res<Time>,
	mut map: ResMut<WorldMapView>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if !map.open {
		return;
	}
	let mut zoom_in = 0.0;
	let mut zoom_out = 0.0;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Focus(value) => zoom_in += value,
			CharacterIntent::Ads(value) => zoom_out += value,
			_ => {}
		}
	}
	let zoom = (zoom_in - zoom_out).clamp(-1.0, 1.0);
	map.height =
		(map.height * (1.0 - zoom * 0.6 * time.delta_secs())).clamp(MIN_MAP_HEIGHT, MAX_MAP_HEIGHT);
}

fn stamp_map_camera(
	mut map: ResMut<WorldMapView>,
	surface: DurhamSurface,
	mut cameras: Query<&mut CameraController, With<FollowCamera>>,
) {
	let Ok(ground) = surface.height_or_fallback(map.focus) else {
		return;
	};
	let begin_life = !map.open && map.begin_life;
	if begin_life {
		map.begin_life = false;
	}
	for mut controller in &mut cameras {
		if map.open {
			controller.enter_map(map.focus, map.height, ground);
		} else if begin_life {
			controller.begin_life();
		} else {
			controller.exit_map();
		}
	}
}

fn place_name_keys(
	places: &Query<(Entity, &DiscoverablePlace, &GlobalTransform)>,
) -> HashMap<Entity, NameKey> {
	places
		.iter()
		.map(|(entity, place, transform)| {
			let xz = transform.translation().xz();
			let key = match place.host {
				Some(host) => NameKey::Place { host, local: place.local },
				None => NameKey::ProvisionalPlace {
					qx: xz.x.round() as i32,
					qz: xz.y.round() as i32,
					label: place.label.salt() as u32,
				},
			};
			(entity, key)
		})
		.collect()
}

const MAP_PIN_QUANT_M: f32 = 8.0;

#[derive(Clone, PartialEq, Eq)]
struct MapPrepKey {
	overlay_epoch: u64,
	focus: IVec2,
	height_q: i32,
	highlighted: Option<PoiId>,
	picker: bool,
	cam: IVec3,
	yaw_q: i32,
	viewport: Option<(IVec2, IVec2)>,
	poi_membership: u64,
}

fn quantize_map_axis(value: f32) -> i32 {
	(value / MAP_PIN_QUANT_M).round() as i32
}

type MapCameraQuery<'w, 's> = Query<'w, 's, (&'static Camera, &'static GlobalTransform), (With<Camera3d>, With<FollowCamera>)>;

fn prepare_map_presentation(
	map: Res<WorldMapView>,
	overlay: Res<LanguageOverlay>,
	registry: Option<Res<PoiRegistry>>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	camera: MapCameraQuery,
	surface: DurhamSurface,
	places: Query<(Entity, &DiscoverablePlace, &GlobalTransform)>,
	mut presentation: ResMut<MapPresentation>,
	mut last_key: Local<Option<MapPrepKey>>,
) {
	if !map.open {
		if presentation.open {
			*presentation = MapPresentation::default();
		}
		*last_key = None;
		return;
	}
	let highlighted = pending.as_deref().and_then(|state| state.pending.as_ref()?.highlighted);
	let picker = picker_prompt_visible(&map);
	let (cam, yaw_q, viewport) = if let Ok((camera, camera_transform)) = camera.single() {
		let translation = camera_transform.translation();
		let forward = camera_transform.forward();
		(
			IVec3::new(
				quantize_map_axis(translation.x),
				quantize_map_axis(translation.y),
				quantize_map_axis(translation.z),
			),
			((forward.x.atan2(forward.z).to_degrees() / 5.0).round() as i32).rem_euclid(72),
			camera.logical_viewport_rect().map(|rect| {
				(
					IVec2::new(rect.min.x.round() as i32, rect.min.y.round() as i32),
					IVec2::new(rect.max.x.round() as i32, rect.max.y.round() as i32),
				)
			}),
		)
	} else {
		(IVec3::ZERO, 0, None)
	};
	let key = MapPrepKey {
		overlay_epoch: overlay.epoch,
		focus: IVec2::new(quantize_map_axis(map.focus.x), quantize_map_axis(map.focus.y)),
		height_q: quantize_map_axis(map.height),
		highlighted,
		picker,
		cam,
		yaw_q,
		viewport,
		poi_membership: registry
			.as_ref()
			.map(|registry| registry.membership_revision())
			.unwrap_or(0),
	};
	if last_key.as_ref() == Some(&key) {
		return;
	}
	*last_key = Some(key);
	let named = place_name_keys(&places);
	let wanted =
		map_pin_targets(&map, &overlay, registry.as_deref(), pending.as_deref(), Some(&named));
	let projected = if let Ok((camera, camera_transform)) = camera.single() {
		let viewport = camera.logical_viewport_rect();
		wanted
			.into_iter()
			.map(|wanted| {
				let projected = pin_world(&surface, wanted.xz)
					.ok()
					.and_then(|world| project_map_pin(camera, camera_transform, world));
				let label = projected.and_then(|(screen, on_screen)| {
					pin_label_screen(
						screen,
						on_screen,
						viewport,
						&wanted,
						picker,
						pin_is_kept(&wanted, highlighted),
					)
				});
				PresentedMapPin { wanted, label }
			})
			.collect()
	} else {
		wanted
			.into_iter()
			.map(|wanted| PresentedMapPin { wanted, label: None })
			.collect()
	};
	*presentation = MapPresentation { open: true, highlighted, pins: projected };
}

fn sync_map_name_pins(
	presentation: Res<MapPresentation>,
	fonts: Res<MapLabelFont>,
	hud: Query<Entity, With<MapNameHud>>,
	mut pins: Query<(
		Entity,
		&MapNamePin,
		&mut Node,
		&mut Text,
		&mut TextFont,
		&mut TextColor,
		&mut Visibility,
	)>,
	mut commands: Commands,
	mut root: Query<&mut Visibility, (With<MapNameHud>, Without<MapNamePin>)>,
) {
	let Ok(mut hud_vis) = root.single_mut() else {
		return;
	};
	if !presentation.open {
		*hud_vis = Visibility::Hidden;
		for (_, _, _, _, _, _, mut visibility) in &mut pins {
			*visibility = Visibility::Hidden;
		}
		return;
	}
	*hud_vis = Visibility::Inherited;
	let Ok(hud) = hud.single() else {
		return;
	};
	let highlighted = presentation.highlighted;
	let mut assigned = Vec::new();
	for (pin_entity, pin, mut node, mut text, mut font, mut color, mut visibility) in &mut pins {
		let Some(presented) =
			presentation.pins.iter().find(|pin_wanted| pin_wanted.wanted.id == pin.target)
		else {
			commands.entity(pin_entity).despawn();
			continue;
		};
		let Some((screen, _)) = presented.label else {
			*visibility = Visibility::Hidden;
			continue;
		};
		place_map_pin(&mut node, screen, &presented.wanted, highlighted);
		if text.0 != presented.wanted.label {
			text.0 = presented.wanted.label.clone();
		}
		let next_font = map_label_text_font(&fonts, presented.wanted.size);
		if font.font != next_font.font || font.font_size != next_font.font_size {
			*font = next_font;
		}
		color.0 = label_ink(presented.wanted.id, highlighted);
		*visibility = Visibility::Visible;
		assigned.push(presented.wanted.id);
	}
	for presented in &presentation.pins {
		if assigned.contains(&presented.wanted.id) {
			continue;
		}
		let Some((screen, _)) = presented.label else {
			continue;
		};
		commands.entity(hud).with_children(|root| {
			root.spawn(MapNamePinBundle {
				name: Name::new("map-name-pin"),
				pin: MapNamePin { target: presented.wanted.id },
				node: map_pin_node(screen, &presented.wanted, highlighted),
				text: Text::new(presented.wanted.label.clone()),
				font: map_label_text_font(&fonts, presented.wanted.size),
				layout: map_label_layout(),
				color: TextColor(label_ink(presented.wanted.id, highlighted)),
				shadow: map_label_shadow(),
				pickable: Pickable::IGNORE,
				visibility: Visibility::Visible,
			});
		});
	}
}

fn map_label_text_font(font: &MapLabelFont, size: f32) -> TextFont {
	TextFont { font: font.0.clone().into(), font_size: FontSize::Px(size), ..default() }
}

fn map_label_layout() -> TextLayout {
	TextLayout::new(Justify::Center, bevy::text::LineBreak::WordBoundary)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapLabelKind {
	Region,
	Feature,
	Poi,
}

fn map_label_kind(target: MapPinTarget) -> MapLabelKind {
	match target {
		MapPinTarget::Poi(_) => MapLabelKind::Poi,
		MapPinTarget::Name(NameKey::Region { .. }) => MapLabelKind::Region,
		MapPinTarget::Name(NameKey::Place { .. } | NameKey::ProvisionalPlace { .. }) => {
			MapLabelKind::Poi
		}
		MapPinTarget::Name(_) => MapLabelKind::Feature,
	}
}

fn map_label_size(kind: MapLabelKind) -> f32 {
	match kind {
		MapLabelKind::Region => REGION_LABEL_PX,
		MapLabelKind::Feature => FEATURE_LABEL_PX,
		MapLabelKind::Poi => POI_LABEL_PX,
	}
}

#[derive(Clone, Debug)]
struct MapPinWanted {
	id: MapPinTarget,
	xz: Vec2,
	extent: Rect,
	label: String,
	size: f32,
	mark: Option<MapMarkKind>,
}

#[derive(Clone)]
struct PresentedMapPin {
	wanted: MapPinWanted,
	label: Option<(Vec2, Option<Vec2>)>,
}

#[derive(Resource, Default)]
struct MapPresentation {
	open: bool,
	highlighted: Option<PoiId>,
	pins: Vec<PresentedMapPin>,
}

fn map_pin_targets(
	map: &WorldMapView,
	overlay: &LanguageOverlay,
	registry: Option<&PoiRegistry>,
	pending: Option<&WorldPlayerRespawnState>,
	named: Option<&HashMap<Entity, NameKey>>,
) -> Vec<MapPinWanted> {
	let view = map_view_rect(map);
	let mut wanted = Vec::new();
	let highlighted = pending.and_then(|state| state.pending.as_ref()?.highlighted);
	if let Some(pending) = pending.and_then(|state| state.pending.as_ref()) {
		if pending.map_opened {
			if let Some(registry) = registry {
				for id in &pending.candidates {
					let Some(record) = registry.get(*id) else {
						continue;
					};
					let xz = record.position.xz();
					let selected = Some(*id) == highlighted;
					if !selected && !near_view_rect(xz, view) {
						continue;
					}
					let name_key = named.and_then(|names| names.get(&record.entity)).copied();
					wanted.push(MapPinWanted {
						id: MapPinTarget::Poi(*id),
						xz,
						extent: Rect::from_center_size(xz, Vec2::splat(12.0)),
						label: label_for_poi(record, overlay, name_key),
						size: if selected {
							SELECTED_POI_LABEL_PX
						} else {
							map_label_size(MapLabelKind::Poi)
						},
						mark: mark_for_poi(record, overlay, name_key),
					});
				}
			}
		}
	}
	let mut names: Vec<_> = overlay
		.names
		.iter()
		.filter(|name| name_visible(name.key, map.height))
		.filter_map(|name| {
			let kind = map_label_kind(MapPinTarget::Name(name.key));
			let xz = label_anchor(name, view, kind)?;
			Some((name, kind, xz))
		})
		.collect();
	names.sort_by(|(a, _, a_xz), (b, _, b_xz)| {
		name_rank(a.key)
			.cmp(&name_rank(b.key))
			.then_with(|| a_xz.distance(map.focus).total_cmp(&b_xz.distance(map.focus)))
	});
	let mut regions = 0;
	let mut features = 0;
	let mut pois = 0;
	for (name, kind, xz) in names {
		let at_cap = match kind {
			MapLabelKind::Region => regions >= MAX_REGION_LABELS,
			MapLabelKind::Feature => features >= MAX_FEATURE_LABELS,
			MapLabelKind::Poi => pois >= MAX_POI_LABELS,
		};
		if at_cap {
			continue;
		}
		if matches!(kind, MapLabelKind::Poi) && wanted.iter().any(|pin| pin.xz.distance(xz) < 8.0) {
			continue;
		}
		match kind {
			MapLabelKind::Region => regions += 1,
			MapLabelKind::Feature => features += 1,
			MapLabelKind::Poi => pois += 1,
		}
		wanted.push(MapPinWanted {
			id: MapPinTarget::Name(name.key),
			xz,
			extent: name.extent,
			label: map_name_label(&name.surface, &name.english),
			size: map_label_size(kind),
			mark: mark_for_name(name),
		});
	}
	wanted.truncate(MAP_PIN_LIMIT);
	resolve_label_collisions(wanted, view, highlighted.map(MapPinTarget::Poi))
}

fn map_view_rect(map: &WorldMapView) -> Rect {
	let half = (map.height * 1.05).clamp(90.0, 2_200.0);
	Rect::from_center_size(map.focus, Vec2::splat(half * 2.0))
}

fn near_view_rect(xz: Vec2, view: Rect) -> bool {
	let pad = view.size() * 0.04;
	Rect { min: view.min - pad, max: view.max + pad }.contains(xz)
}

fn label_anchor(name: &NamedOverlay, view: Rect, kind: MapLabelKind) -> Option<Vec2> {
	match kind {
		MapLabelKind::Poi => view.contains(name.xz).then_some(name.xz),
		MapLabelKind::Region => {
			let hit = view.intersect(name.extent);
			if rect_empty(hit) {
				return None;
			}
			Some(title_band(hit))
		}
		MapLabelKind::Feature => {
			let hit = view.intersect(name.extent);
			if rect_empty(hit) {
				return None;
			}
			if swallows_view(hit, view) {
				return None;
			}
			if hit.contains(name.xz) {
				Some(name.xz)
			} else {
				Some(clamp_into_rect(name.xz, inset_rect(hit, 0.12)))
			}
		}
	}
}

fn title_band(hit: Rect) -> Vec2 {
	let height = hit.height();
	if !height.is_finite() || height < 8.0 {
		return hit.center();
	}
	let pad_y = (height * 0.16).max(8.0).min(height * 0.5);
	let lo = hit.min.y + 4.0;
	let hi = hit.max.y - 4.0;
	if lo > hi {
		return hit.center();
	}
	Vec2::new(hit.center().x, (hit.max.y - pad_y).clamp(lo, hi))
}

fn swallows_view(hit: Rect, view: Rect) -> bool {
	hit.width() * hit.height() >= view.width() * view.height() * 0.45
}

fn inset_rect(rect: Rect, fraction: f32) -> Rect {
	let pad = rect.size() * fraction.clamp(0.0, 0.4);
	let min = rect.min + pad;
	let max = rect.max - pad;
	if min.x < max.x && min.y < max.y {
		Rect { min, max }
	} else {
		rect
	}
}

fn rect_empty(rect: Rect) -> bool {
	rect.width() <= 0.0 || rect.height() <= 0.0
}

fn clamp_into_rect(point: Vec2, rect: Rect) -> Vec2 {
	Vec2::new(point.x.clamp(rect.min.x, rect.max.x), point.y.clamp(rect.min.y, rect.max.y))
}

fn resolve_label_collisions(
	mut wanted: Vec<MapPinWanted>,
	view: Rect,
	keep: Option<MapPinTarget>,
) -> Vec<MapPinWanted> {
	if let Some(keep) = keep {
		wanted.sort_by_key(|pin| if pin.id == keep { 0u8 } else { 1 });
	}
	let mut kept = Vec::with_capacity(wanted.len());
	for pin in wanted {
		let keep_pin = keep == Some(pin.id);
		let sep = label_separation(&pin, view);
		if keep_pin
			|| kept.iter().all(|other: &MapPinWanted| {
				pin.xz.distance(other.xz) >= sep.max(label_separation(other, view))
			}) {
			kept.push(pin);
			continue;
		}
		if let Some(xz) = nudge_label(&pin, &kept, view, sep) {
			kept.push(MapPinWanted { xz, ..pin });
		}
	}
	kept
}

fn label_separation(pin: &MapPinWanted, view: Rect) -> f32 {
	let side = view.width().min(view.height());
	match map_label_kind(pin.id) {
		MapLabelKind::Region => (side * 0.28).max(48.0),
		MapLabelKind::Feature => (side * 0.16).max(28.0),
		MapLabelKind::Poi => (side * 0.12).max(20.0),
	}
}

fn nudge_label(pin: &MapPinWanted, kept: &[MapPinWanted], view: Rect, sep: f32) -> Option<Vec2> {
	let room = allowed_label_rect(pin, view, sep);
	if rect_empty(room) {
		return None;
	}
	let dirs = [
		Vec2::X,
		Vec2::NEG_X,
		Vec2::Y,
		Vec2::NEG_Y,
		Vec2::new(1.0, 1.0),
		Vec2::new(-1.0, 1.0),
		Vec2::new(1.0, -1.0),
		Vec2::new(-1.0, -1.0),
	];
	for dir in dirs {
		let candidate = clamp_into_rect(pin.xz + dir.normalize() * sep, room);
		if kept.iter().all(|other: &MapPinWanted| {
			candidate.distance(other.xz) >= sep.max(label_separation(other, view))
		}) {
			return Some(candidate);
		}
	}
	None
}

fn allowed_label_rect(pin: &MapPinWanted, view: Rect, sep: f32) -> Rect {
	match map_label_kind(pin.id) {
		MapLabelKind::Poi => {
			view.intersect(Rect::from_center_size(pin.xz, Vec2::splat((sep * 2.0).max(24.0))))
		}
		MapLabelKind::Region | MapLabelKind::Feature => {
			let hit = view.intersect(pin.extent);
			if rect_empty(hit) {
				view
			} else {
				hit
			}
		}
	}
}

fn title_case(text: &str) -> String {
	text.split_whitespace()
		.filter(|word| !word.is_empty())
		.map(|word| {
			let mut chars = word.chars();
			match chars.next() {
				Some(first) => {
					let mut titled = first.to_uppercase().collect::<String>();
					titled.push_str(chars.as_str());
					titled
				}
				None => String::new(),
			}
		})
		.collect::<Vec<_>>()
		.join(" ")
}

fn map_name_label(surface: &str, english: &[String]) -> String {
	let native = title_case(surface.trim());
	let gloss = title_case(
		&english
			.iter()
			.map(|word| word.trim())
			.filter(|word| !word.is_empty())
			.collect::<Vec<_>>()
			.join(" "),
	);
	if gloss.is_empty() {
		native
	} else {
		format!("{native}\n{gloss}")
	}
}

fn name_visible(key: NameKey, height: f32) -> bool {
	match map_label_kind(MapPinTarget::Name(key)) {
		MapLabelKind::Region => true,
		MapLabelKind::Feature => height >= 160.0,
		MapLabelKind::Poi => height <= 1_200.0,
	}
}

fn name_rank(key: NameKey) -> u8 {
	match map_label_kind(MapPinTarget::Name(key)) {
		MapLabelKind::Region => 0,
		MapLabelKind::Feature => 1,
		MapLabelKind::Poi => 2,
	}
}

pub(crate) fn label_for_poi(
	poi: &PoiRecord,
	overlay: &LanguageOverlay,
	named: Option<NameKey>,
) -> String {
	overlay_name_for_poi(poi, overlay, named)
		.map(|name| map_name_label(&name.surface, &name.english))
		.unwrap_or_else(|| kind_label(poi.kind))
}

fn overlay_name_for_poi<'a>(
	poi: &PoiRecord,
	overlay: &'a LanguageOverlay,
	named: Option<NameKey>,
) -> Option<&'a NamedOverlay> {
	if let Some(key) = named {
		if let Some(name) = overlay.names.iter().find(|name| name.key == key) {
			return Some(name);
		}
	}
	let xz = poi.position.xz();
	let place_r = poi.arrival_radius.max(48.0);
	overlay
		.names
		.iter()
		.filter(|name| {
			name_covers_poi(name, xz, place_r) && name_kind_matches_poi(name.key, poi.kind)
		})
		.min_by(|a, b| {
			poi_name_rank(a.key)
				.cmp(&poi_name_rank(b.key))
				.then_with(|| a.xz.distance(xz).total_cmp(&b.xz.distance(xz)))
		})
}

fn name_kind_matches_poi(key: NameKey, kind: PoiKind) -> bool {
	if kind == VEGETATION_POI {
		matches!(key, NameKey::Grove(_) | NameKey::Forest(_))
	} else if kind == LOCAL_POI || kind == SALOON_POI || kind == URBAN_POI {
		matches!(
			key,
			NameKey::Place { .. }
				| NameKey::ProvisionalPlace { .. }
				| NameKey::Urban(_)
				| NameKey::UrbanLeaf(_)
		)
	} else {
		false
	}
}

fn name_covers_poi(name: &NamedOverlay, xz: Vec2, place_r: f32) -> bool {
	match name.key {
		NameKey::Place { .. } | NameKey::ProvisionalPlace { .. } => {
			name.xz.distance(xz) <= place_r || name.extent.contains(xz)
		}
		NameKey::Grove(_) => name.extent.contains(xz) || name.xz.distance(xz) <= POI_NAME_RADIUS,
		NameKey::Forest(_) | NameKey::Urban(_) | NameKey::UrbanLeaf(_) => name.extent.contains(xz),
		NameKey::Geographic(_) | NameKey::Region { .. } => false,
	}
}

fn poi_name_rank(key: NameKey) -> u8 {
	match key {
		NameKey::Place { .. } | NameKey::ProvisionalPlace { .. } => 0,
		NameKey::Grove(_) => 1,
		NameKey::Urban(_) | NameKey::UrbanLeaf(_) => 2,
		NameKey::Forest(_) => 3,
		NameKey::Geographic(_) | NameKey::Region { .. } => 4,
	}
}

fn kind_label(kind: PoiKind) -> String {
	let name = kind.name();
	let leaf = name.rsplit('/').next().unwrap_or(name);
	let leaf = leaf.rsplit("::").next().unwrap_or(leaf);
	title_case(leaf)
}

fn pin_world(surface: &DurhamSurface, xz: Vec2) -> Result<Vec3, lod::hcsg::Busy> {
	Ok(Vec3::new(xz.x, surface.height_or_fallback(xz)? + GIZMO_LIFT, xz.y))
}

fn label_ink(target: MapPinTarget, highlighted: Option<PoiId>) -> Color {
	if matches!(target, MapPinTarget::Poi(id) if Some(id) == highlighted) {
		return TEXT_YELLOW;
	}
	match map_label_kind(target) {
		MapLabelKind::Region => Color::srgba(0.10, 0.08, 0.06, 0.94),
		MapLabelKind::Feature => Color::srgba(0.14, 0.12, 0.09, 0.90),
		MapLabelKind::Poi => Color::srgba(0.16, 0.14, 0.11, 0.86),
	}
}

fn map_label_shadow() -> TextShadow {
	TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.98, 0.94, 0.86, 0.82) }
}

fn pin_width(size: f32, label: &str) -> f32 {
	let chars = label.lines().map(|line| line.chars().count()).max().unwrap_or(0);
	(size * 0.58 * chars as f32 + 12.0).clamp(72.0, 520.0)
}

fn pin_lines(label: &str) -> f32 {
	label.lines().count().max(1) as f32
}

fn pin_label_half(target: &MapPinWanted) -> Vec2 {
	let mark = if target.mark.is_some() { MAP_MARK_PX + MAP_MARK_GAP } else { 0.0 };
	Vec2::new(
		pin_width(target.size, &target.label) * 0.5 + mark,
		target.size * 0.85 * pin_lines(&target.label),
	)
}

fn pin_is_kept(target: &MapPinWanted, highlighted: Option<PoiId>) -> bool {
	matches!(target.id, MapPinTarget::Poi(id) if Some(id) == highlighted)
}

fn pin_label_screen(
	projected: Vec2,
	on_screen: bool,
	viewport: Option<Rect>,
	target: &MapPinWanted,
	picker: bool,
	keep: bool,
) -> Option<(Vec2, Option<Vec2>)> {
	let Some(viewport) = viewport else {
		return Some((projected, None));
	};
	comfortable_label_screen(projected, on_screen, viewport, pin_label_half(target), picker, keep)
}

fn comfortable_label_screen(
	projected: Vec2,
	on_screen: bool,
	viewport: Rect,
	half: Vec2,
	picker: bool,
	keep: bool,
) -> Option<(Vec2, Option<Vec2>)> {
	if !on_screen && !keep && !near_screen_edge(projected, viewport) {
		return None;
	}
	let top =
		if picker { PICKER_TITLE_GUTTER + ITEM_FONT_SIZE + 12.0 } else { LABEL_SCREEN_GUTTER };
	let min = viewport.min + Vec2::new(LABEL_SCREEN_GUTTER, top) + half;
	let max = viewport.max - Vec2::splat(LABEL_SCREEN_GUTTER) - half;
	let clamped = Vec2::new(
		projected.x.clamp(min.x.min(max.x), max.x.max(min.x)),
		projected.y.clamp(min.y.min(max.y), max.y.max(min.y)),
	);
	let delta = projected - clamped;
	if !on_screen || delta.length() > 8.0 {
		let dir = if delta.length() > 1e-3 {
			delta.normalize()
		} else {
			(projected - viewport.center()).normalize_or(Vec2::NEG_Y)
		};
		Some((clamped, Some(dir)))
	} else {
		Some((clamped, None))
	}
}

fn near_screen_edge(projected: Vec2, viewport: Rect) -> bool {
	Rect {
		min: viewport.min - Vec2::splat(EDGE_PULL_PX),
		max: viewport.max + Vec2::splat(EDGE_PULL_PX),
	}
	.contains(projected)
}

/// Screen position without the combat-HUD rim clamp.
fn project_map_pin(
	camera: &Camera,
	camera_transform: &GlobalTransform,
	world: Vec3,
) -> Option<(Vec2, bool)> {
	let rect = camera.logical_viewport_rect()?;
	let ndc = camera.world_to_ndc(camera_transform, world)?;
	let in_frustum = ndc.z > 0.0 && ndc.z < 1.0;
	let screen = (Vec2::new(ndc.x, -ndc.y) + Vec2::ONE) / 2.0 * rect.size() + rect.min;
	let on_screen = in_frustum
		&& screen.x >= rect.min.x
		&& screen.x <= rect.max.x
		&& screen.y >= rect.min.y
		&& screen.y <= rect.max.y;
	Some((screen, on_screen))
}

fn mark_for_name(name: &NamedOverlay) -> Option<MapMarkKind> {
	match name.key {
		NameKey::Grove(_) | NameKey::Forest(_) => Some(MapMarkKind::Grove),
		NameKey::Place { .. } | NameKey::ProvisionalPlace { .. } => Some(MapMarkKind::House),
		NameKey::Urban(_) | NameKey::UrbanLeaf(_) => Some(MapMarkKind::Town),
		NameKey::Geographic(_) => mark_from_english(&name.english),
		NameKey::Region { .. } => None,
	}
}

fn mark_for_poi(
	poi: &PoiRecord,
	overlay: &LanguageOverlay,
	named: Option<NameKey>,
) -> Option<MapMarkKind> {
	if let Some(name) = overlay_name_for_poi(poi, overlay, named) {
		if let Some(mark) = mark_for_name(name) {
			return Some(mark);
		}
	}
	let kind = poi.kind.name();
	if kind.contains("vegetation") || kind.contains("forage") {
		Some(MapMarkKind::Tree)
	} else if kind.contains("urban") || kind.contains("saloon") {
		Some(MapMarkKind::Town)
	} else if kind.contains("water") {
		Some(MapMarkKind::Water)
	} else {
		Some(MapMarkKind::House)
	}
}

fn mark_from_english(english: &[String]) -> Option<MapMarkKind> {
	const WATER: &[&str] = &[
		"lake",
		"loch",
		"tarn",
		"mere",
		"stream",
		"brook",
		"creek",
		"rivulet",
		"run",
		"pool",
		"pond",
		"bog",
		"marsh",
		"fen",
		"mire",
		"swamp",
		"water",
		"waters",
		"waterhole",
	];
	if english.iter().any(|word| WATER.contains(&word.as_str())) {
		Some(MapMarkKind::Water)
	} else {
		Some(MapMarkKind::Mountain)
	}
}

fn mark_image(icons: &MapMarkIcons, kind: MapMarkKind) -> Handle<Image> {
	match kind {
		MapMarkKind::Tree => icons.tree.clone(),
		MapMarkKind::Grove => icons.grove.clone(),
		MapMarkKind::House => icons.house.clone(),
		MapMarkKind::Town => icons.town.clone(),
		MapMarkKind::Mountain => icons.mountain.clone(),
		MapMarkKind::Water => icons.water.clone(),
	}
}

fn type_mark_screen(label: Vec2, target: &MapPinWanted, highlighted: Option<PoiId>) -> Vec2 {
	let width = pin_width(target.size, &target.label);
	let top = pin_label_top(label.y, target, highlighted);
	let height = target.size * pin_lines(&target.label);
	Vec2::new(label.x - width * 0.5 - MAP_MARK_GAP - MAP_MARK_PX * 0.5, top + height * 0.5)
}

fn arrow_rotation(dir: Vec2) -> f32 {
	dir.y.atan2(dir.x) + std::f32::consts::FRAC_PI_2
}

fn pin_label_top(screen_y: f32, target: &MapPinWanted, highlighted: Option<PoiId>) -> f32 {
	if matches!(target.id, MapPinTarget::Poi(_)) {
		let selected = matches!(target.id, MapPinTarget::Poi(id) if Some(id) == highlighted);
		let clearance =
			if selected { SELECTION_RING_PX * 0.5 + 8.0 } else { SPAWN_KNOB_PX * 0.5 + 6.0 };
		return screen_y + clearance;
	}
	screen_y - target.size * 0.85 * pin_lines(&target.label)
}

fn map_pin_node(screen: Vec2, target: &MapPinWanted, highlighted: Option<PoiId>) -> Node {
	let width = pin_width(target.size, &target.label);
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - width * 0.5),
		top: Val::Px(pin_label_top(screen.y, target, highlighted)),
		width: Val::Px(width),
		justify_content: JustifyContent::Center,
		..default()
	}
}

fn place_map_pin(node: &mut Node, screen: Vec2, target: &MapPinWanted, highlighted: Option<PoiId>) {
	let width = pin_width(target.size, &target.label);
	node.left = Val::Px(screen.x - width * 0.5);
	node.top = Val::Px(pin_label_top(screen.y, target, highlighted));
	node.width = Val::Px(width);
}

fn sync_map_type_marks(
	presentation: Res<MapPresentation>,
	icons: Res<MapMarkIcons>,
	hud: Query<Entity, With<MapNameHud>>,
	mut marks: Query<(Entity, &MapTypeMark, &mut Node, &mut ImageNode, &mut Visibility)>,
	mut commands: Commands,
) {
	if !presentation.open {
		for (entity, _, _, _, _) in &marks {
			commands.entity(entity).despawn();
		}
		return;
	}
	let Ok(hud) = hud.single() else {
		return;
	};
	let highlighted = presentation.highlighted;
	let mut assigned = Vec::new();
	for (entity, mark, mut node, mut image, mut visibility) in &mut marks {
		let Some(presented) = presentation
			.pins
			.iter()
			.find(|pin| pin.wanted.id == mark.target && pin.wanted.mark.is_some())
		else {
			commands.entity(entity).despawn();
			continue;
		};
		let Some((label, _)) = presented.label else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let screen = type_mark_screen(label, &presented.wanted, highlighted);
		place_type_mark(&mut node, screen);
		if let Some(kind) = presented.wanted.mark {
			image.image = mark_image(&icons, kind);
		}
		*visibility = Visibility::Visible;
		assigned.push(presented.wanted.id);
	}
	for presented in &presentation.pins {
		if assigned.contains(&presented.wanted.id) || presented.wanted.mark.is_none() {
			continue;
		}
		let Some(kind) = presented.wanted.mark else {
			continue;
		};
		let Some((label, _)) = presented.label else {
			continue;
		};
		let screen = type_mark_screen(label, &presented.wanted, highlighted);
		commands.entity(hud).with_children(|root| {
			root.spawn((
				Name::new("map-type-mark"),
				MapTypeMark { target: presented.wanted.id },
				type_mark_node(screen),
				ImageNode { image: mark_image(&icons, kind), ..default() },
				Pickable::IGNORE,
				Visibility::Visible,
				GlobalZIndex(i32::MAX - 8),
			));
		});
	}
}

fn type_mark_node(screen: Vec2) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - MAP_MARK_PX * 0.5),
		top: Val::Px(screen.y - MAP_MARK_PX * 0.5),
		width: Val::Px(MAP_MARK_PX),
		height: Val::Px(MAP_MARK_PX),
		..default()
	}
}

fn place_type_mark(node: &mut Node, screen: Vec2) {
	node.left = Val::Px(screen.x - MAP_MARK_PX * 0.5);
	node.top = Val::Px(screen.y - MAP_MARK_PX * 0.5);
}

fn sync_map_edge_arrows(
	presentation: Res<MapPresentation>,
	icons: Res<MapMarkIcons>,
	hud: Query<Entity, With<MapNameHud>>,
	mut arrows: Query<(Entity, &MapEdgeArrow, &mut Node, &mut UiTransform, &mut Visibility)>,
	mut commands: Commands,
) {
	if !presentation.open {
		for (entity, _, _, _, _) in &arrows {
			commands.entity(entity).despawn();
		}
		return;
	}
	let Ok(hud) = hud.single() else {
		return;
	};
	let mut assigned = Vec::new();
	for (entity, arrow, mut node, mut transform, mut visibility) in &mut arrows {
		let Some(presented) = presentation.pins.iter().find(|pin| pin.wanted.id == arrow.target)
		else {
			commands.entity(entity).despawn();
			continue;
		};
		let Some((label_screen, dir)) = presented.label else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let Some(dir) = dir else {
			*visibility = Visibility::Hidden;
			continue;
		};
		place_edge_arrow(&mut node, label_screen, dir);
		transform.rotation = Rot2::radians(arrow_rotation(dir));
		*visibility = Visibility::Visible;
		assigned.push(presented.wanted.id);
	}
	for presented in &presentation.pins {
		if assigned.contains(&presented.wanted.id) {
			continue;
		}
		let Some((label_screen, Some(dir))) = presented.label else {
			continue;
		};
		commands.entity(hud).with_children(|root| {
			root.spawn((
				Name::new("map-edge-arrow"),
				MapEdgeArrow { target: presented.wanted.id },
				edge_arrow_node(label_screen, dir),
				ImageNode {
					image: icons.arrow.clone(),
					color: label_ink(presented.wanted.id, None),
					..default()
				},
				UiTransform { rotation: Rot2::radians(arrow_rotation(dir)), ..default() },
				Pickable::IGNORE,
				Visibility::Visible,
				GlobalZIndex(i32::MAX - 6),
			));
		});
	}
}

fn edge_arrow_screen(label: Vec2, dir: Vec2) -> Vec2 {
	label + dir * (MAP_ARROW_PX * 0.9)
}

fn edge_arrow_node(label: Vec2, dir: Vec2) -> Node {
	let screen = edge_arrow_screen(label, dir);
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - MAP_ARROW_PX * 0.5),
		top: Val::Px(screen.y - MAP_ARROW_PX * 0.5),
		width: Val::Px(MAP_ARROW_PX),
		height: Val::Px(MAP_ARROW_PX),
		..default()
	}
}

fn place_edge_arrow(node: &mut Node, label: Vec2, dir: Vec2) {
	let screen = edge_arrow_screen(label, dir);
	node.left = Val::Px(screen.x - MAP_ARROW_PX * 0.5);
	node.top = Val::Px(screen.y - MAP_ARROW_PX * 0.5);
}

fn sync_map_player_marker(
	map: Res<WorldMapView>,
	players: Query<&Transform, With<VegetationPlayer>>,
	camera: MapCameraQuery,
	surface: DurhamSurface,
	hud: Query<Entity, With<MapNameHud>>,
	mut markers: Query<(&mut Node, &mut Visibility), With<MapPlayerMarker>>,
	mut commands: Commands,
) {
	if !map.open || picker_prompt_visible(&map) {
		hide_player_markers(&mut markers);
		return;
	}
	let Some(xz) = players.iter().next().map(|transform| transform.translation.xz()) else {
		hide_player_markers(&mut markers);
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		hide_player_markers(&mut markers);
		return;
	};
	let Some((screen, _)) = pin_world(&surface, xz)
		.ok()
		.and_then(|world| ScreenPin::project(camera, camera_transform, world, HUD_MARGIN))
	else {
		hide_player_markers(&mut markers);
		return;
	};
	if let Some((mut node, mut visibility)) = markers.iter_mut().next() {
		place_player_marker(&mut node, screen);
		*visibility = Visibility::Visible;
		return;
	}
	let Ok(hud) = hud.single() else {
		return;
	};
	commands.entity(hud).with_children(|root| {
		root.spawn((
			Name::new("map-player-marker"),
			MapPlayerMarker,
			player_marker_node(screen),
			BackgroundColor(TEXT_YELLOW),
			BorderColor::all(Color::srgba(0.08, 0.10, 0.14, 0.92)),
			Pickable::IGNORE,
			Visibility::Visible,
			GlobalZIndex(i32::MAX - 10),
		));
	});
}

fn hide_player_markers(markers: &mut Query<(&mut Node, &mut Visibility), With<MapPlayerMarker>>) {
	for (_, mut visibility) in markers.iter_mut() {
		*visibility = Visibility::Hidden;
	}
}

fn death_xz(pending: Option<&WorldPlayerRespawnState>) -> Option<Vec2> {
	pending
		.and_then(|state| state.pending.as_ref())
		.map(|pending| pending.death_at.xz())
}

#[cfg(test)]
fn player_map_xz(live: Option<&Transform>, death: Option<Vec2>) -> Option<Vec2> {
	live.map(|transform| transform.translation.xz()).or(death)
}

fn player_marker_node(screen: Vec2) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - PLAYER_MARKER_PX * 0.5),
		top: Val::Px(screen.y - PLAYER_MARKER_PX * 0.5),
		width: Val::Px(PLAYER_MARKER_PX),
		height: Val::Px(PLAYER_MARKER_PX),
		border: UiRect::all(Val::Px(2.0)),
		border_radius: BorderRadius::all(Val::Px(PLAYER_MARKER_PX * 0.5)),
		..default()
	}
}

fn place_player_marker(node: &mut Node, screen: Vec2) {
	node.left = Val::Px(screen.x - PLAYER_MARKER_PX * 0.5);
	node.top = Val::Px(screen.y - PLAYER_MARKER_PX * 0.5);
}

fn sync_map_death_bones(
	map: Res<WorldMapView>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	icon: Option<Res<MapBonesIcon>>,
	camera: Query<(&Camera, &GlobalTransform), (With<Camera3d>, With<FollowCamera>)>,
	surface: DurhamSurface,
	hud: Query<Entity, With<MapNameHud>>,
	mut markers: Query<(&mut Node, &mut Visibility), With<MapDeathBones>>,
	mut commands: Commands,
) {
	if !picker_prompt_visible(&map) || pending_is_first_life(pending.as_deref()) {
		hide_death_bones(&mut markers);
		return;
	}
	let Some(xz) = death_xz(pending.as_deref()) else {
		hide_death_bones(&mut markers);
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		hide_death_bones(&mut markers);
		return;
	};
	let Some((screen, _)) = pin_world(&surface, xz)
		.ok()
		.and_then(|world| ScreenPin::project(camera, camera_transform, world, HUD_MARGIN))
	else {
		hide_death_bones(&mut markers);
		return;
	};
	if let Some((mut node, mut visibility)) = markers.iter_mut().next() {
		place_death_bones(&mut node, screen);
		*visibility = Visibility::Visible;
		return;
	}
	let Some(icon) = icon else {
		return;
	};
	let Ok(hud) = hud.single() else {
		return;
	};
	commands.entity(hud).with_children(|root| {
		root.spawn((
			Name::new("map-death-bones"),
			MapDeathBones,
			death_bones_node(screen),
			ImageNode { image: icon.0.clone(), color: TEXT_SALMON, ..default() },
			Pickable::IGNORE,
			Visibility::Visible,
			GlobalZIndex(i32::MAX - 7),
		));
	});
}

fn hide_death_bones(markers: &mut Query<(&mut Node, &mut Visibility), With<MapDeathBones>>) {
	for (_, mut visibility) in markers.iter_mut() {
		*visibility = Visibility::Hidden;
	}
}

fn death_bones_node(screen: Vec2) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - DEATH_BONES_PX * 0.5),
		top: Val::Px(screen.y - DEATH_BONES_PX * 0.5),
		width: Val::Px(DEATH_BONES_PX),
		height: Val::Px(DEATH_BONES_PX),
		..default()
	}
}

fn place_death_bones(node: &mut Node, screen: Vec2) {
	node.left = Val::Px(screen.x - DEATH_BONES_PX * 0.5);
	node.top = Val::Px(screen.y - DEATH_BONES_PX * 0.5);
}

fn picker_prompt_visible(map: &WorldMapView) -> bool {
	map.open && map.close_locked
}

fn pending_is_first_life(pending: Option<&WorldPlayerRespawnState>) -> bool {
	pending
		.and_then(|state| state.pending.as_ref())
		.is_some_and(|pending| pending.first_life)
}

fn sync_respawn_picker_title(
	map: Res<WorldMapView>,
	mut titles: Query<&mut Visibility, With<RespawnPickerTitle>>,
) {
	let visible = picker_prompt_visible(&map);
	for mut visibility in &mut titles {
		*visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
	}
}

fn selected_poi_xz(
	pending: Option<&WorldPlayerRespawnState>,
	registry: Option<&PoiRegistry>,
) -> Option<Vec2> {
	let id = pending?.pending.as_ref()?.highlighted?;
	Some(registry?.get(id)?.position.xz())
}

fn picker_candidates(pending: Option<&WorldPlayerRespawnState>) -> &[PoiId] {
	pending
		.and_then(|state| state.pending.as_ref())
		.map(|pending| pending.candidates.as_slice())
		.unwrap_or(&[])
}

fn picker_highlighted(pending: Option<&WorldPlayerRespawnState>) -> Option<PoiId> {
	pending.and_then(|state| state.pending.as_ref()?.highlighted)
}

fn sync_respawn_spawn_knobs(
	map: Res<WorldMapView>,
	registry: Option<Res<PoiRegistry>>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	camera: Query<(&Camera, &GlobalTransform), (With<Camera3d>, With<FollowCamera>)>,
	surface: DurhamSurface,
	mut knobs: Query<(
		Entity,
		&MapSpawnKnob,
		&mut Node,
		&mut BackgroundColor,
		&mut BorderColor,
		&mut Visibility,
	)>,
	mut commands: Commands,
) {
	if !picker_prompt_visible(&map) {
		for (entity, _, _, _, _, _) in &knobs {
			commands.entity(entity).despawn();
		}
		return;
	}
	let Ok((camera, camera_transform)) = camera.single() else {
		for (_, _, _, _, _, mut visibility) in &mut knobs {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	let highlighted = picker_highlighted(pending.as_deref());
	let candidates = picker_candidates(pending.as_deref());
	let registry = registry.as_deref();
	let viewport = camera.logical_viewport_rect();
	let mut assigned = Vec::new();
	for (entity, knob, mut node, mut fill, mut border, mut visibility) in &mut knobs {
		if !candidates.contains(&knob.id) {
			commands.entity(entity).despawn();
			continue;
		}
		let Some(xz) = registry
			.and_then(|registry| registry.get(knob.id))
			.map(|record| record.position.xz())
		else {
			commands.entity(entity).despawn();
			continue;
		};
		let Some((projected, on_screen)) = pin_world(&surface, xz)
			.ok()
			.and_then(|world| project_map_pin(camera, camera_transform, world))
		else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let selected = Some(knob.id) == highlighted;
		let Some(screen) = map_marker_screen(projected, on_screen, viewport, selected) else {
			*visibility = Visibility::Hidden;
			continue;
		};
		paint_spawn_knob(&mut node, &mut fill, &mut border, screen, selected);
		*visibility = Visibility::Visible;
		assigned.push(knob.id);
	}
	for id in candidates {
		if assigned.contains(id) {
			continue;
		}
		let Some(xz) = registry
			.and_then(|registry| registry.get(*id))
			.map(|record| record.position.xz())
		else {
			continue;
		};
		let Some((projected, on_screen)) = pin_world(&surface, xz)
			.ok()
			.and_then(|world| project_map_pin(camera, camera_transform, world))
		else {
			continue;
		};
		let selected = Some(*id) == highlighted;
		let Some(screen) = map_marker_screen(projected, on_screen, viewport, selected) else {
			continue;
		};
		let (size, fill, border) = spawn_knob_look(selected);
		commands.spawn((
			Name::new("map-spawn-knob"),
			MapSpawnKnob { id: *id },
			spawn_knob_node(screen, size),
			BackgroundColor(fill),
			BorderColor::all(border),
			Pickable::IGNORE,
			Visibility::Visible,
			GlobalZIndex(i32::MAX - 9),
		));
	}
}

fn map_marker_screen(
	projected: Vec2,
	on_screen: bool,
	viewport: Option<Rect>,
	keep: bool,
) -> Option<Vec2> {
	if on_screen {
		return Some(projected);
	}
	if !keep && !viewport.is_some_and(|viewport| near_screen_edge(projected, viewport)) {
		return None;
	}
	let Some(viewport) = viewport else {
		return Some(projected);
	};
	let min = viewport.min + Vec2::splat(LABEL_SCREEN_GUTTER);
	let max = viewport.max - Vec2::splat(LABEL_SCREEN_GUTTER);
	Some(Vec2::new(
		projected.x.clamp(min.x.min(max.x), max.x.max(min.x)),
		projected.y.clamp(min.y.min(max.y), max.y.max(min.y)),
	))
}

fn spawn_knob_look(selected: bool) -> (f32, Color, Color) {
	if selected {
		(SPAWN_KNOB_ACTIVE_PX, TEXT_YELLOW, Color::srgba(0.08, 0.10, 0.14, 0.92))
	} else {
		(SPAWN_KNOB_PX, SPAWN_KNOB_GRAY, Color::srgba(0.22, 0.20, 0.18, 0.88))
	}
}

fn spawn_knob_node(screen: Vec2, size: f32) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - size * 0.5),
		top: Val::Px(screen.y - size * 0.5),
		width: Val::Px(size),
		height: Val::Px(size),
		border: UiRect::all(Val::Px(2.0)),
		border_radius: BorderRadius::all(Val::Px(size * 0.5)),
		..default()
	}
}

fn paint_spawn_knob(
	node: &mut Node,
	fill: &mut BackgroundColor,
	border: &mut BorderColor,
	screen: Vec2,
	selected: bool,
) {
	let (size, fill_color, border_color) = spawn_knob_look(selected);
	node.left = Val::Px(screen.x - size * 0.5);
	node.top = Val::Px(screen.y - size * 0.5);
	node.width = Val::Px(size);
	node.height = Val::Px(size);
	node.border_radius = BorderRadius::all(Val::Px(size * 0.5));
	fill.0 = fill_color;
	*border = BorderColor::all(border_color);
}

fn sync_respawn_selection_marker(
	map: Res<WorldMapView>,
	registry: Option<Res<PoiRegistry>>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	camera: Query<(&Camera, &GlobalTransform), (With<Camera3d>, With<FollowCamera>)>,
	surface: DurhamSurface,
	mut markers: Query<(&mut Node, &mut Visibility), With<MapRespawnSelection>>,
	mut commands: Commands,
) {
	let Some(xz) = selected_poi_xz(pending.as_deref(), registry.as_deref())
		.filter(|_| picker_prompt_visible(&map))
	else {
		hide_selection_markers(&mut markers);
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		hide_selection_markers(&mut markers);
		return;
	};
	let Some((projected, on_screen)) = pin_world(&surface, xz)
		.ok()
		.and_then(|world| project_map_pin(camera, camera_transform, world))
	else {
		hide_selection_markers(&mut markers);
		return;
	};
	let Some(screen) =
		map_marker_screen(projected, on_screen, camera.logical_viewport_rect(), true)
	else {
		hide_selection_markers(&mut markers);
		return;
	};
	if let Some((mut node, mut visibility)) = markers.iter_mut().next() {
		place_selection_marker(&mut node, screen);
		*visibility = Visibility::Visible;
		return;
	}
	let marker = commands
		.spawn((
			Name::new("map-respawn-selection"),
			MapRespawnSelection,
			selection_marker_node(screen),
			BackgroundColor(Color::srgba(1.0, 0.86, 0.22, 0.12)),
			BorderColor::all(TEXT_YELLOW),
			Pickable::IGNORE,
			Visibility::Visible,
			GlobalZIndex(i32::MAX - 8),
		))
		.id();
	commands.entity(marker).with_children(|root| {
		root.spawn((
			Node {
				width: Val::Px(SELECTION_DOT_PX),
				height: Val::Px(SELECTION_DOT_PX),
				border_radius: BorderRadius::all(Val::Px(SELECTION_DOT_PX * 0.5)),
				..default()
			},
			BackgroundColor(TEXT_YELLOW),
			Pickable::IGNORE,
		));
	});
}

fn hide_selection_markers(
	markers: &mut Query<(&mut Node, &mut Visibility), With<MapRespawnSelection>>,
) {
	for (_, mut visibility) in markers.iter_mut() {
		*visibility = Visibility::Hidden;
	}
}

fn selection_marker_node(screen: Vec2) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - SELECTION_RING_PX * 0.5),
		top: Val::Px(screen.y - SELECTION_RING_PX * 0.5),
		width: Val::Px(SELECTION_RING_PX),
		height: Val::Px(SELECTION_RING_PX),
		border: UiRect::all(Val::Px(3.0)),
		border_radius: BorderRadius::all(Val::Px(SELECTION_RING_PX * 0.5)),
		justify_content: JustifyContent::Center,
		align_items: AlignItems::Center,
		..default()
	}
}

fn place_selection_marker(node: &mut Node, screen: Vec2) {
	node.left = Val::Px(screen.x - SELECTION_RING_PX * 0.5);
	node.top = Val::Px(screen.y - SELECTION_RING_PX * 0.5);
}

fn draw_highlighted_poi(
	map: Res<WorldMapView>,
	registry: Option<Res<PoiRegistry>>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	players: Query<&Transform, With<VegetationPlayer>>,
	surface: DurhamSurface,
	mut gizmos: Gizmos,
) {
	if !map.open {
		return;
	}
	if !picker_prompt_visible(&map) {
		if let Some(xz) = players.iter().next().map(|transform| transform.translation.xz()) {
			if let Ok(world) = pin_world(&surface, xz) {
				gizmos.sphere(Isometry3d::from_translation(world), 2.2, TEXT_YELLOW);
			}
		}
	}
	let Some(id) = pending.as_deref().and_then(|state| state.pending.as_ref()?.highlighted) else {
		return;
	};
	let Some(record) = registry.and_then(|registry| registry.get(id).copied()) else {
		return;
	};
	let radius = (map.height * 0.045).max(record.arrival_radius).clamp(12.0, 48.0);
	let mut points = Vec::with_capacity(33);
	for index in 0..=32 {
		let angle = index as f32 / 32.0 * std::f32::consts::TAU;
		if let Ok(point) = pin_world(
			&surface,
			Vec2::new(
				record.position.x + angle.cos() * radius,
				record.position.z + angle.sin() * radius,
			),
		) {
			points.push(point);
		}
	}
	gizmos.linestrip(points, TEXT_YELLOW);
	if let Some(death) = death_xz(pending.as_deref()) {
		if let (Ok(from), Ok(to)) =
			(pin_world(&surface, death), pin_world(&surface, record.position.xz()))
		{
			gizmos.line(from, to, TEXT_YELLOW_FAINT);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use game_commands::command::TextEntryFocus;
	use player::CameraFollow;

	fn write_toggle(world: &mut World) {
		world.write_message(CharacterIntent::ToggleMap);
	}

	#[test]
	fn opening_the_map_keeps_camera_follow() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.init_resource::<WorldMapView>();
		world.init_resource::<Messages<CharacterIntent>>();
		let player = world
			.spawn((VegetationPlayer, CameraFollow, Transform::from_xyz(12.0, 3.0, -8.0)))
			.id();

		write_toggle(&mut world);
		world
			.run_system_once(toggle_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(crate::camera::sync_camera_mode)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let map = world.resource::<WorldMapView>();
		assert!(map.open);
		assert_eq!(map.focus, Vec2::new(12.0, -8.0));
		assert!(!map.close_locked);
		assert!(world.get::<CameraFollow>(player).is_some());
		Ok(())
	}

	#[test]
	fn locked_map_ignores_toggle() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: true,
			begin_life: false,
		});
		world.init_resource::<Messages<CharacterIntent>>();
		write_toggle(&mut world);
		world
			.run_system_once(toggle_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().close_locked);
		Ok(())
	}

	#[test]
	fn stick_intents_do_not_pan_the_map() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::new(10.0, 20.0),
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		});
		world.init_resource::<Messages<CharacterIntent>>();
		world.write_message(CharacterIntent::Move(Vec2::X));
		world.write_message(CharacterIntent::Look(Vec2::Y));
		world
			.run_system_once(pan_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.resource::<WorldMapView>().focus, Vec2::new(10.0, 20.0));
		Ok(())
	}

	#[test]
	fn dpad_down_closes_an_unlocked_map() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		});
		world.init_resource::<Messages<CharacterIntent>>();
		world.write_message(CharacterIntent::CloseMap);
		world
			.run_system_once(toggle_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<WorldMapView>().open);
		Ok(())
	}

	#[test]
	fn label_sizes_step_down_by_layer() {
		assert!(map_label_size(MapLabelKind::Region) > map_label_size(MapLabelKind::Feature));
		assert!(map_label_size(MapLabelKind::Feature) > map_label_size(MapLabelKind::Poi));
		assert!(map_label_size(MapLabelKind::Poi) >= 12.0);
		assert_eq!(
			map_label_kind(MapPinTarget::Name(NameKey::Region { ix: 0, iz: 0 })),
			MapLabelKind::Region
		);
	}

	#[test]
	fn poi_kind_fallback_is_a_plain_name() {
		assert_eq!(kind_label(PoiKind::new("mobs/vegetation")), "Vegetation");
		assert!(!kind_label(PoiKind::new("mobs/vegetation")).contains("PoiKind"));
	}

	fn test_poi(xz: Vec2) -> PoiRecord {
		PoiRecord {
			id: PoiId(1),
			entity: Entity::from_bits(1),
			kind: PoiKind::new("mobs/vegetation"),
			position: Vec3::new(xz.x, 4.0, xz.y),
			arrival_radius: 8.0,
			salience: 1.0,
			local: true,
			global: false,
		}
	}

	#[test]
	fn vegetation_poi_does_not_steal_a_nearby_place_name() {
		let poi = test_poi(Vec2::new(10.0, 6.0));
		let overlay = LanguageOverlay {
			names: vec![NamedOverlay {
				key: NameKey::ProvisionalPlace { qx: 0, qz: 0, label: 1 },
				surface: "oak stand".into(),
				english: vec!["green grove".into()],
				provisional: true,
				xz: Vec2::new(80.0, 90.0),
				extent: Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)),
			}],
			..Default::default()
		};
		assert_eq!(label_for_poi(&poi, &overlay, None), "Vegetation");
		assert_eq!(label_for_poi(&poi, &LanguageOverlay::default(), None), "Vegetation");
	}

	#[test]
	fn poi_labels_prefer_identity_then_compatible_cover() {
		let key = NameKey::ProvisionalPlace { qx: 10, qz: 6, label: 1 };
		let building = PoiRecord {
			id: PoiId(2),
			entity: Entity::from_bits(2),
			kind: LOCAL_POI,
			position: Vec3::new(10.0, 4.0, 6.0),
			arrival_radius: 8.0,
			salience: 1.0,
			local: true,
			global: false,
		};
		let overlay = LanguageOverlay {
			names: vec![
				NamedOverlay {
					key,
					surface: "amber house".into(),
					english: vec!["amber house".into()],
					provisional: true,
					xz: Vec2::new(10.0, 6.0),
					extent: Rect::from_center_size(Vec2::new(10.0, 6.0), Vec2::splat(8.0)),
				},
				NamedOverlay {
					key: NameKey::Grove(lod::gen::Id::Universal),
					surface: "oak stand".into(),
					english: vec!["green grove".into()],
					provisional: false,
					xz: Vec2::new(12.0, 6.0),
					extent: Rect::from_center_size(Vec2::new(12.0, 6.0), Vec2::splat(40.0)),
				},
			],
			..Default::default()
		};
		assert_eq!(label_for_poi(&building, &overlay, Some(key)), "Amber House\nAmber House");
		assert_eq!(
			label_for_poi(&test_poi(Vec2::new(10.0, 6.0)), &overlay, None),
			"Oak Stand\nGreen Grove"
		);
	}

	#[test]
	fn bilingual_label_stacks_title_case_english() {
		assert_eq!(map_name_label("ʃin", &["ridge".into()]), "Ʃin\nRidge");
		assert_eq!(map_name_label("red bush", &[String::new()]), "Red Bush");
		assert_eq!(map_name_label("oak grove", &["green wood".into()]), "Oak Grove\nGreen Wood");
	}

	#[test]
	fn region_label_sits_in_the_view_not_at_the_centroid() {
		let map = WorldMapView {
			open: true,
			focus: Vec2::new(100.0, 80.0),
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		};
		let overlay = LanguageOverlay {
			names: vec![NamedOverlay {
				key: NameKey::Region { ix: 0, iz: 0 },
				surface: "ʃin".into(),
				english: vec!["ridge".into()],
				provisional: true,
				xz: Vec2::splat(12_500.0),
				extent: Rect::from_corners(Vec2::ZERO, Vec2::splat(25_000.0)),
			}],
			..Default::default()
		};
		let wanted = map_pin_targets(&map, &overlay, None, None, None);
		assert_eq!(wanted.len(), 1);
		let view = map_view_rect(&map);
		assert!(view.contains(wanted[0].xz));
		assert!((wanted[0].xz.x - view.center().x).abs() < view.width() * 0.2);
		assert!(wanted[0].xz.y > view.center().y);
		assert_eq!(wanted[0].label, "Ʃin\nRidge");
	}

	#[test]
	fn overlapping_region_titles_do_not_stack() {
		let map = WorldMapView {
			open: true,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		};
		let overlay = LanguageOverlay {
			names: vec![
				NamedOverlay {
					key: NameKey::Region { ix: 0, iz: 0 },
					surface: "east".into(),
					english: Vec::new(),
					provisional: true,
					xz: Vec2::new(5_000.0, 0.0),
					extent: Rect::from_corners(
						Vec2::new(0.0, -10_000.0),
						Vec2::new(10_000.0, 10_000.0),
					),
				},
				NamedOverlay {
					key: NameKey::Region { ix: -1, iz: 0 },
					surface: "west".into(),
					english: Vec::new(),
					provisional: true,
					xz: Vec2::new(-5_000.0, 0.0),
					extent: Rect::from_corners(
						Vec2::new(-10_000.0, -10_000.0),
						Vec2::new(0.0, 10_000.0),
					),
				},
			],
			..Default::default()
		};
		let wanted = map_pin_targets(&map, &overlay, None, None, None);
		assert_eq!(wanted.len(), 2);
		assert!(wanted[0].xz.distance(wanted[1].xz) > 40.0);
	}

	#[test]
	fn closing_a_locked_map_requests_a_new_life_camera() {
		let mut map = WorldMapView { open: true, close_locked: true, ..default() };
		map.close();
		assert!(!map.open);
		assert!(!map.close_locked);
		assert!(map.begin_life);
	}

	#[test]
	fn spawn_labels_sit_below_the_ring() {
		let target = MapPinWanted {
			id: MapPinTarget::Poi(PoiId(1)),
			xz: Vec2::ZERO,
			extent: Rect::from_center_size(Vec2::ZERO, Vec2::splat(12.0)),
			label: "Grove".into(),
			size: SELECTED_POI_LABEL_PX,
			mark: Some(MapMarkKind::Tree),
		};
		let top = pin_label_top(100.0, &target, Some(PoiId(1)));
		assert!(top > 100.0, "cartographic labels sit under the marker, top={top}");
		assert!((top - (100.0 + SELECTION_RING_PX * 0.5 + 8.0)).abs() < 1e-4);
	}

	#[test]
	fn selected_respawn_labels_are_larger_than_idle_pois() {
		assert!(SELECTED_POI_LABEL_PX > map_label_size(MapLabelKind::Poi));
	}

	#[test]
	fn picker_title_keeps_menu_gutters() {
		assert_eq!(PICKER_TITLE_GUTTER, 48.0);
	}

	#[test]
	fn picker_title_only_shows_on_a_locked_map() {
		assert!(picker_prompt_visible(&WorldMapView {
			open: true,
			close_locked: true,
			..default()
		}));
		assert!(!picker_prompt_visible(&WorldMapView {
			open: true,
			close_locked: false,
			..default()
		}));
		assert!(!picker_prompt_visible(&WorldMapView::default()));
	}

	#[test]
	fn respawn_map_uses_the_bones_icon_for_death() {
		assert_eq!(BONES_ICON, "iconography/bones_icon.png");
		assert!(DEATH_BONES_PX > PLAYER_MARKER_PX);
	}

	#[test]
	fn map_uses_kenney_cartography_marks() {
		assert_eq!(MAP_TREE_ICON, "iconography/kenney/cartography/tree_pine.png");
		assert_eq!(MAP_GROVE_ICON, "iconography/kenney/cartography/tree_pines.png");
		assert_ne!(MAP_TREE_ICON, MAP_GROVE_ICON);
		assert_eq!(MAP_HOUSE_ICON, "iconography/kenney/cartography/house.png");
		assert_eq!(MAP_ARROW_ICON, "iconography/kenney/game-icons/arrow_up.png");
		assert_eq!(
			mark_for_poi(&test_poi(Vec2::ZERO), &LanguageOverlay::default(), None),
			Some(MapMarkKind::Tree)
		);
		assert_eq!(
			mark_for_name(&NamedOverlay {
				key: NameKey::ProvisionalPlace { qx: 0, qz: 0, label: 1 },
				surface: "house".into(),
				english: vec!["amber".into(), "house".into()],
				provisional: true,
				xz: Vec2::ZERO,
				extent: Rect::from_center_size(Vec2::ZERO, Vec2::splat(8.0)),
			}),
			Some(MapMarkKind::House)
		);
		assert_eq!(mark_from_english(&["blue".into(), "lake".into()]), Some(MapMarkKind::Water));
		assert_eq!(
			mark_from_english(&["rolling".into(), "hills".into()]),
			Some(MapMarkKind::Mountain)
		);
	}

	#[test]
	fn type_marks_sit_left_of_the_name() {
		let target = MapPinWanted {
			id: MapPinTarget::Poi(PoiId(1)),
			xz: Vec2::ZERO,
			extent: Rect::from_center_size(Vec2::ZERO, Vec2::splat(12.0)),
			label: "Grove".into(),
			size: SELECTED_POI_LABEL_PX,
			mark: Some(MapMarkKind::Grove),
		};
		let label = Vec2::new(200.0, 100.0);
		let mark = type_mark_screen(label, &target, Some(PoiId(1)));
		let width = pin_width(target.size, &target.label);
		assert!(mark.x < label.x - width * 0.5);
		assert!(mark.y > label.y);
	}

	#[test]
	fn edge_labels_stay_inside_the_screen_and_point_out() {
		let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(800.0, 600.0));
		let (screen, dir) = comfortable_label_screen(
			Vec2::new(790.0, 20.0),
			false,
			viewport,
			Vec2::new(40.0, 16.0),
			false,
			false,
		)
		.expect("near-edge names stay on the frame");
		assert!(screen.x < 800.0 - 40.0);
		assert!(screen.y > 40.0);
		let dir = dir.expect("off-screen labels keep an arrow");
		assert!(dir.x > 0.0);
		assert!(dir.y < 0.0);
	}

	#[test]
	fn far_picker_candidates_stay_off_the_name_list() {
		let view = Rect::from_center_size(Vec2::ZERO, Vec2::splat(200.0));
		assert!(near_view_rect(Vec2::new(80.0, 0.0), view));
		assert!(!near_view_rect(Vec2::new(400.0, 0.0), view));
	}

	#[test]
	fn far_off_screen_pois_do_not_crowd_the_rim() {
		let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(800.0, 600.0));
		assert!(comfortable_label_screen(
			Vec2::new(2_000.0, 300.0),
			false,
			viewport,
			Vec2::new(40.0, 16.0),
			true,
			false,
		)
		.is_none());
		let (screen, dir) = comfortable_label_screen(
			Vec2::new(2_000.0, 300.0),
			false,
			viewport,
			Vec2::new(40.0, 16.0),
			true,
			true,
		)
		.expect("the selected spawn stays on the rim");
		assert!(screen.x > 600.0);
		assert!(dir.is_some());
	}

	#[test]
	fn title_band_survives_thin_intersections() {
		for height in [0.0, 1.0, 4.0, 7.9, 8.0] {
			let hit = Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(12.0, height));
			let at = title_band(hit);
			assert!(at.x.is_finite() && at.y.is_finite(), "height {height}");
			if height > 0.0 {
				assert!(
					at.y >= hit.min.y - 1e-4 && at.y <= hit.max.y + 1e-4,
					"height {height} y={}",
					at.y
				);
			}
		}
	}

	#[test]
	fn player_marker_prefers_the_live_body() {
		let live = Transform::from_xyz(4.0, 1.0, -3.0);
		assert_eq!(
			player_map_xz(Some(&live), Some(Vec2::new(90.0, 80.0))),
			Some(Vec2::new(4.0, -3.0))
		);
		assert_eq!(player_map_xz(None, Some(Vec2::new(90.0, 80.0))), Some(Vec2::new(90.0, 80.0)));
		assert_eq!(player_map_xz(None, None), None);
	}
}
