//! Corner viewports, map cameras, and the debraid overlay.

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use bevy::ui::widget::ViewportNode;
use character_inventory_user::InventoryUser;
use character_items::{Inventory, InventoryItem};
use item_icons::{ItemIconCache, FIREARM_SILHOUETTE_RESOLUTION};
use menu_components::{
	spawn_hud_text_card, spawn_hud_text_card_label, HudFonts, HUD_TEXT_CARD_FACE_PX,
	PANEL_BLOCK_FONT_SIZE, TEXT_YELLOW,
};

use crate::cursor::SkillMapCursor;
use crate::map::{authored_map_from_spec, render_layer, AuthoredMap, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::tiles::spawn_map_tiles;
use crate::user::{SkillMapEquip, SkillMapHeld, SkillMapMember, SkillMapSession, SkillMapUser};
use crate::SkillMapEnabled;

pub(crate) const VIEWPORT_PX: f32 = 228.0;
/// `WindowSize` area is viewport pixels times this. 1.0 showed almost the whole 256 map.
pub(crate) const MAP_CAMERA_SCALE: f32 = 0.4;

pub(crate) fn map_view_extent() -> Vec2 {
	Vec2::splat(VIEWPORT_PX * MAP_CAMERA_SCALE)
}
const VIEWPORT_GAP: f32 = 12.0;
const VIEWPORT_INSET: f32 = 16.0;
const FRAME_BORDER: f32 = 2.0;
const FRAME_RADIUS: f32 = 14.0;
const FRAME_INNER_RADIUS: f32 = FRAME_RADIUS - FRAME_BORDER;
const LIVE_BORDER: Color = Color::srgb(1.0, 0.48, 0.08);
const IDLE_BORDER: Color = Color::srgba(1.0, 0.86, 0.22, 0.42);
const PROMPT_ICON_PX: f32 = 18.0;
const FIREARM_CARD_PAD_X: f32 = 8.0;
const FIREARM_CARD_PAD_Y: f32 = 6.0;
const FIREARM_CARD_BORDER: f32 = 1.5;
const FIREARM_CARD_RADIUS: f32 = 8.0;
const FIREARM_CARD_FILL: Color = Color::srgba(0.52, 0.52, 0.56, 0.82);
const FIREARM_CARD_STROKE: Color = Color::srgba(0.78, 0.78, 0.82, 0.9);
const GRENADE_SILHOUETTE_PX: f32 = 40.0;
const GRENADE_SILHOUETTE_STROKE: f32 = 2.0;

fn weapon_icon_slot() -> Node {
	Node {
		width: Val::Percent(100.0),
		aspect_ratio: Some(
			FIREARM_SILHOUETTE_RESOLUTION.width as f32
				/ FIREARM_SILHOUETTE_RESOLUTION.height as f32,
		),
		flex_shrink: 0.0,
		justify_content: JustifyContent::Center,
		align_items: AlignItems::Center,
		overflow: Overflow::clip(),
		..default()
	}
}

fn weapon_slot_display(show: bool) -> Display {
	if show {
		Display::Flex
	} else {
		Display::None
	}
}
const FIREARM_SILHOUETTE: Color =
	Color::srgb(0x84 as f32 / 255.0, 0x88 as f32 / 255.0, 0x84 as f32 / 255.0);

/// Kenney outline Xbox **RB** (hold to steer the map).
pub const SKILL_MAP_RB_ICON: &str = "iconography/kenney/input-prompts/xbox_rb_outline.png";
/// Kenney outline Xbox **RT** (fire the held gun).
pub const SKILL_MAP_RT_ICON: &str = "iconography/kenney/input-prompts/xbox_rt_outline.png";
/// Kenney outline Xbox **Y** (swap the held gun).
pub const SKILL_MAP_Y_ICON: &str = "iconography/kenney/input-prompts/xbox_button_y_outline.png";

#[derive(Component)]
pub struct SkillMapHud;

#[derive(Component)]
pub struct SkillMapViewport;

#[derive(Component)]
pub struct SkillMapViewportCamera;

#[derive(Component)]
pub struct SkillMapLabel;

#[derive(Component)]
pub struct SkillMapFirearmHud;

#[derive(Component)]
pub struct SkillMapFirearmName;

#[derive(Component)]
pub(crate) struct SkillMapFirearmSilhouette;

#[derive(Component)]
pub(crate) struct SkillMapFirearmSilhouettePlaceholder;

#[derive(Component)]
pub(crate) struct SkillMapFirearmSilhouetteImage;

#[derive(Component)]
pub(crate) struct SkillMapGrenadeSilhouette;

#[derive(Component)]
pub(crate) struct SkillMapGrenadePlaceholder;

#[derive(Component)]
pub(crate) struct SkillMapGrenadeSilhouetteImage;

#[derive(Component)]
pub(crate) struct SkillMapGrenadeChargeFill;

#[derive(Component)]
pub(crate) struct SkillMapGrenadeChargeImage;

#[derive(Component)]
pub(crate) struct SkillMapGrenadeChargeGlyph;

#[derive(Component)]
pub(crate) struct SkillMapWeaponAction;

#[derive(Component)]
pub struct Debraid {
	pub remaining: f32,
}

#[derive(Component)]
pub(crate) struct DebraidOverlay;

/// Present the equipped map. Rebuild when [`SkillMapEquip`] changes.
pub fn present_skill_maps(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	asset_server: Res<AssetServer>,
	tiles: Res<SkillMapTileAssets>,
	users: Query<(Entity, &SkillMapUser, &SkillMapEquip)>,
	mut sessions: Query<&mut SkillMapSession>,
	members: Query<(Entity, &SkillMapMember)>,
) {
	for (user, mapping, equip) in &users {
		let Ok(mut session) = sessions.get_mut(mapping.maps) else {
			continue;
		};
		if session.presented == equip.spec && (equip.spec.is_none() || !session.cameras.is_empty())
		{
			continue;
		}
		clear_presented(&mut commands, mapping.maps, &mut session, &members);
		session.presented = equip.spec;
		let Some(spec) = equip.spec else {
			continue;
		};
		spawn_one_map(
			&mut commands,
			&mut images,
			&HudFonts::load(asset_server.as_ref()),
			asset_server.as_ref(),
			&tiles,
			user,
			mapping.maps,
			&mut session,
			authored_map_from_spec(spec),
			0,
		);
	}
}

fn clear_presented(
	commands: &mut Commands,
	session: Entity,
	viewports: &mut SkillMapSession,
	members: &Query<(Entity, &SkillMapMember)>,
) {
	for (entity, member) in members {
		if member.session == session {
			commands.entity(entity).try_despawn();
		}
	}
	viewports.cameras.clear();
	viewports.nodes.clear();
	viewports.presented = None;
}

/// Camera + tiles into an image. [`ViewportNode`] resizes the target and composites it.
pub struct SpawnedSkillMapView {
	pub camera: Entity,
	pub image: Handle<Image>,
}

/// Same off-screen `Camera2d` + authored tiles used by the live corner map.
pub fn spawn_skill_map_view(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	tiles: &SkillMapTileAssets,
	spec: AuthoredMap,
	member: SkillMapMember,
	order: isize,
	projection: OrthographicProjection,
	extras: impl Bundle,
) -> SpawnedSkillMapView {
	let layer = render_layer(spec.id);
	let mut image = Image::new_uninit(
		default(),
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::all(),
	);
	image.texture_descriptor.usage =
		TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
	let image_handle = images.add(image);
	let camera = commands
		.spawn((
			Name::new(format!("skill-map-camera-{}", spec.label)),
			Camera2d,
			Camera {
				order,
				clear_color: ClearColorConfig::Custom(spec.kind.viewport_clear()),
				..default()
			},
			RenderTarget::Image(image_handle.clone().into()),
			Projection::Orthographic(projection),
			Transform::from_xyz(0.0, 0.0, 1.0),
			spec.id,
			member,
			layer,
			extras,
		))
		.id();
	spawn_map_tiles(commands, spec, member, tiles);
	SpawnedSkillMapView { camera, image: image_handle }
}

fn spawn_one_map(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	fonts: &HudFonts,
	asset_server: &AssetServer,
	tiles: &SkillMapTileAssets,
	user: Entity,
	session: Entity,
	viewports: &mut SkillMapSession,
	spec: AuthoredMap,
	stack_index: usize,
) {
	let member = SkillMapMember { user, session };
	let mut projection = OrthographicProjection::default_2d();
	projection.scale = MAP_CAMERA_SCALE;
	let view = spawn_skill_map_view(
		commands,
		images,
		tiles,
		spec,
		member,
		-2 - stack_index as isize,
		projection,
		SkillMapViewportCamera,
	);
	let camera = view.camera;
	let layer = render_layer(spec.id);

	let bottom = VIEWPORT_INSET + stack_index as f32 * (VIEWPORT_PX + VIEWPORT_GAP);
	let caption = format!("{} {:04X}", spec.label, spec.seed as u16);
	let mut node = Entity::PLACEHOLDER;
	commands
		.spawn((
			Name::new(format!("skill-map-hud-{}", spec.label)),
			SkillMapHud,
			spec.id,
			member,
			Node {
				position_type: PositionType::Absolute,
				bottom: Val::Px(bottom),
				right: Val::Px(VIEWPORT_INSET),
				width: Val::Px(VIEWPORT_PX),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Stretch,
				row_gap: Val::Px(8.0),
				..default()
			},
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|hud| {
			spawn_firearm_hud(hud, fonts, asset_server, member);
			node = spawn_map_frame(hud, fonts, asset_server, camera, spec, member, caption);
		});

	commands.spawn((
		Name::new(format!("skill-map-cursor-{}", spec.label)),
		SkillMapCursor,
		spec.id,
		member,
		Mesh2d(tiles.cursor_mesh.clone()),
		MeshMaterial2d(tiles.cursor.clone()),
		Transform::from_xyz(0.0, 0.0, 1.0),
		layer,
	));

	viewports.cameras.insert(spec.id, camera);
	viewports.nodes.insert(spec.id, node);
}

fn spawn_map_frame(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	asset_server: &AssetServer,
	camera: Entity,
	spec: AuthoredMap,
	member: SkillMapMember,
	caption: String,
) -> Entity {
	parent
		.spawn((
			Name::new(format!("skill-map-viewport-{}", spec.label)),
			SkillMapViewport,
			spec.id,
			member,
			Node {
				width: Val::Px(VIEWPORT_PX),
				height: Val::Px(VIEWPORT_PX),
				border: UiRect::all(Val::Px(FRAME_BORDER)),
				border_radius: BorderRadius::all(Val::Px(FRAME_RADIUS)),
				overflow: Overflow::clip(),
				..default()
			},
			BorderColor::all(IDLE_BORDER),
			BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.12)),
			Pickable::IGNORE,
		))
		.with_children(|frame| {
			frame.spawn((
				ViewportNode::new(camera),
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(FRAME_BORDER),
					right: Val::Px(FRAME_BORDER),
					top: Val::Px(FRAME_BORDER),
					bottom: Val::Px(FRAME_BORDER),
					border_radius: BorderRadius::all(Val::Px(FRAME_INNER_RADIUS)),
					..default()
				},
				Pickable::IGNORE,
			));
			frame
				.spawn((
					Node {
						position_type: PositionType::Absolute,
						top: Val::Px(8.0),
						left: Val::Px(8.0),
						right: Val::Px(8.0),
						justify_content: JustifyContent::Center,
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|slot| {
					spawn_hud_text_card_label(slot, fonts, caption, SkillMapLabel);
				});
			frame
				.spawn((
					Node {
						position_type: PositionType::Absolute,
						bottom: Val::Px(8.0),
						left: Val::Px(8.0),
						right: Val::Px(8.0),
						justify_content: JustifyContent::Center,
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|slot| {
					spawn_prompt_chip(slot, fonts, asset_server, SKILL_MAP_RB_ICON, "Hold", ());
				});
		})
		.id()
}

fn spawn_firearm_hud(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	asset_server: &AssetServer,
	member: SkillMapMember,
) {
	parent
		.spawn((
			Name::new("skill-map-firearm-hud"),
			SkillMapFirearmHud,
			member,
			Node {
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Center,
				row_gap: Val::Px(6.0),
				width: Val::Percent(100.0),
				padding: UiRect::axes(Val::Px(FIREARM_CARD_PAD_X), Val::Px(FIREARM_CARD_PAD_Y)),
				border: UiRect::all(Val::Px(FIREARM_CARD_BORDER)),
				border_radius: BorderRadius::all(Val::Px(FIREARM_CARD_RADIUS)),
				..default()
			},
			BackgroundColor(FIREARM_CARD_FILL),
			BorderColor::all(FIREARM_CARD_STROKE),
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|stack| {
			spawn_hud_text_card(stack, (), |card| {
				card.spawn((
					SkillMapFirearmName,
					member,
					Text::new(""),
					fonts.item(HUD_TEXT_CARD_FACE_PX),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			spawn_firearm_silhouette(stack);
			spawn_grenade_silhouette(stack);
			stack
				.spawn((
					Node {
						flex_direction: FlexDirection::Row,
						align_items: AlignItems::Center,
						column_gap: Val::Px(10.0),
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|row| {
					spawn_prompt_chip(
						row,
						fonts,
						asset_server,
						SKILL_MAP_RT_ICON,
						"Fire",
						SkillMapWeaponAction,
					);
					spawn_prompt_chip(row, fonts, asset_server, SKILL_MAP_Y_ICON, "Switch", ());
				});
		});
}

fn spawn_prompt_chip(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	asset_server: &AssetServer,
	icon: &'static str,
	label: &str,
	extra: impl Bundle,
) {
	spawn_hud_text_card(parent, (), |card| {
		card.spawn((
			ImageNode { image: asset_server.load(icon), color: TEXT_YELLOW, ..default() },
			Node {
				width: Val::Px(PROMPT_ICON_PX),
				height: Val::Px(PROMPT_ICON_PX),
				flex_shrink: 0.0,
				..default()
			},
			Pickable::IGNORE,
		));
		card.spawn((
			Text::new(label),
			fonts.item(HUD_TEXT_CARD_FACE_PX),
			TextColor(TEXT_YELLOW),
			Pickable::IGNORE,
			extra,
		));
	});
}

fn spawn_firearm_silhouette(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			SkillMapFirearmSilhouette,
			weapon_icon_slot(),
			Visibility::Inherited,
			Pickable::IGNORE,
		))
		.with_children(|gun| {
			spawn_firearm_silhouette_placeholder(gun);
			gun.spawn((
				SkillMapFirearmSilhouetteImage,
				ImageNode { color: FIREARM_SILHOUETTE, ..default() },
				Node {
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					flex_shrink: 0.0,
					..default()
				},
				Visibility::Hidden,
				Pickable::IGNORE,
			));
		});
}

fn spawn_firearm_silhouette_placeholder(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			SkillMapFirearmSilhouettePlaceholder,
			Node {
				width: Val::Px(96.0),
				height: Val::Px(28.0),
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::Center,
				..default()
			},
			Visibility::Inherited,
			Pickable::IGNORE,
		))
		.with_children(|gun| {
			gun.spawn((
				Node {
					width: Val::Px(14.0),
					height: Val::Px(12.0),
					margin: UiRect::right(Val::Px(2.0)),
					..default()
				},
				BackgroundColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
			gun.spawn((
				Node {
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::Center,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|mid| {
				mid.spawn((
					Node { width: Val::Px(34.0), height: Val::Px(14.0), ..default() },
					BackgroundColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
				mid.spawn((
					Node { width: Val::Px(8.0), height: Val::Px(11.0), ..default() },
					BackgroundColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			gun.spawn((
				Node {
					width: Val::Px(40.0),
					height: Val::Px(6.0),
					margin: UiRect::left(Val::Px(1.0)),
					..default()
				},
				BackgroundColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
		});
}

fn spawn_grenade_silhouette(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			SkillMapGrenadeSilhouette,
			{
				let mut slot = weapon_icon_slot();
				slot.display = Display::None;
				slot
			},
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|slot| {
			slot.spawn((
				Node {
					width: Val::Px(GRENADE_SILHOUETTE_PX),
					height: Val::Px(GRENADE_SILHOUETTE_PX),
					justify_content: JustifyContent::FlexEnd,
					align_items: AlignItems::Center,
					overflow: Overflow::clip(),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|orb| {
				orb.spawn((
					SkillMapGrenadePlaceholder,
					Node {
						width: Val::Percent(100.0),
						height: Val::Percent(100.0),
						justify_content: JustifyContent::FlexEnd,
						align_items: AlignItems::Center,
						overflow: Overflow::clip(),
						border: UiRect::all(Val::Px(GRENADE_SILHOUETTE_STROKE)),
						border_radius: BorderRadius::all(Val::Px(GRENADE_SILHOUETTE_PX)),
						..default()
					},
					BorderColor::all(TEXT_YELLOW),
					Pickable::IGNORE,
				))
				.with_children(|circle| {
					circle.spawn((
						SkillMapGrenadeChargeFill,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							border_radius: BorderRadius::all(Val::Px(GRENADE_SILHOUETTE_PX)),
							..default()
						},
						BackgroundColor(TEXT_YELLOW),
						Pickable::IGNORE,
					));
				});
				orb.spawn((
					SkillMapGrenadeSilhouetteImage,
					ImageNode { color: FIREARM_SILHOUETTE, ..default() },
					Node {
						position_type: PositionType::Absolute,
						width: Val::Percent(100.0),
						height: Val::Percent(100.0),
						..default()
					},
					Visibility::Hidden,
					Pickable::IGNORE,
				));
				orb.spawn((
					SkillMapGrenadeChargeImage,
					Node {
						position_type: PositionType::Absolute,
						left: Val::Px(0.0),
						right: Val::Px(0.0),
						bottom: Val::Px(0.0),
						width: Val::Percent(100.0),
						height: Val::Percent(100.0),
						overflow: Overflow::clip(),
						..default()
					},
					Visibility::Hidden,
					Pickable::IGNORE,
				))
				.with_children(|clip| {
					clip.spawn((
						SkillMapGrenadeChargeGlyph,
						ImageNode { color: TEXT_YELLOW, ..default() },
						Node {
							position_type: PositionType::Absolute,
							left: Val::Px(0.0),
							bottom: Val::Px(0.0),
							width: Val::Percent(100.0),
							height: Val::Px(GRENADE_SILHOUETTE_PX),
							..default()
						},
						Pickable::IGNORE,
					));
				});
			});
		});
}

pub fn sync_viewport_chrome(
	enabled: Res<SkillMapEnabled>,
	users: Query<(&SkillMapUser, &SkillMapHeld)>,
	mut huds: Query<
		(&SkillMapMember, &mut Visibility),
		(
			With<SkillMapHud>,
			Without<SkillMapFirearmHud>,
			Without<SkillMapFirearmSilhouette>,
			Without<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapGrenadeSilhouette>,
			Without<SkillMapGrenadeSilhouetteImage>,
			Without<SkillMapGrenadeChargeImage>,
		),
	>,
	mut nodes: Query<
		(&SkillMapMember, &mut BorderColor, &mut BackgroundColor),
		(With<SkillMapViewport>, Without<SkillMapHud>, Without<SkillMapGrenadeChargeFill>),
	>,
) {
	for (_, mut visibility) in &mut huds {
		*visibility = if enabled.0 { Visibility::Inherited } else { Visibility::Hidden };
	}
	if !enabled.0 {
		return;
	}
	for (member, mut border, mut background) in &mut nodes {
		let held = users.iter().any(|(user, held)| user.maps == member.session && held.0);
		if held {
			*border = BorderColor::all(LIVE_BORDER);
			background.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
		} else {
			*border = BorderColor::all(IDLE_BORDER);
			background.0 = Color::srgba(0.06, 0.05, 0.04, 0.45);
		}
	}
}

pub fn sync_firearm_chrome(
	users: Query<(Entity, Option<&InventoryUser>), With<SkillMapUser>>,
	bags: Query<&Inventory>,
	icons: Option<Res<ItemIconCache>>,
	members: Query<&SkillMapMember, With<SkillMapFirearmHud>>,
	mut blocks: Query<
		(&SkillMapMember, &mut Visibility),
		(
			With<SkillMapFirearmHud>,
			Without<SkillMapHud>,
			Without<SkillMapFirearmSilhouette>,
			Without<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapGrenadeSilhouette>,
			Without<SkillMapGrenadeSilhouetteImage>,
			Without<SkillMapGrenadeChargeImage>,
		),
	>,
	mut names: Query<(&SkillMapMember, &mut Text), With<SkillMapFirearmName>>,
	mut guns: Query<
		(&mut Node, &mut Visibility),
		(
			With<SkillMapFirearmSilhouette>,
			Without<SkillMapHud>,
			Without<SkillMapFirearmHud>,
			Without<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapGrenadeSilhouette>,
			Without<SkillMapGrenadeSilhouetteImage>,
			Without<SkillMapGrenadeChargeImage>,
			Without<SkillMapFirearmSilhouettePlaceholder>,
		),
	>,
	mut placeholders: Query<
		&mut Node,
		(
			With<SkillMapFirearmSilhouettePlaceholder>,
			Without<SkillMapFirearmSilhouette>,
			Without<SkillMapGrenadePlaceholder>,
			Without<SkillMapGrenadeChargeFill>,
			Without<SkillMapGrenadeChargeImage>,
		),
	>,
	mut silhouettes: Query<
		(&mut ImageNode, &mut Visibility),
		(
			With<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapHud>,
			Without<SkillMapFirearmHud>,
			Without<SkillMapFirearmSilhouette>,
			Without<SkillMapGrenadeSilhouette>,
			Without<SkillMapGrenadeSilhouetteImage>,
			Without<SkillMapGrenadeChargeImage>,
			Without<SkillMapGrenadeChargeGlyph>,
		),
	>,
	mut actions: Query<&mut Text, (With<SkillMapWeaponAction>, Without<SkillMapFirearmName>)>,
) {
	for (member, mut visibility) in &mut blocks {
		let name = primary_weapon_name(member.user, &users, &bags);
		*visibility = if name.is_some() { Visibility::Inherited } else { Visibility::Hidden };
	}
	for (member, mut text) in &mut names {
		text.0 = primary_weapon_name(member.user, &users, &bags).unwrap_or_default();
	}
	let Some(member) = members.iter().next() else {
		return;
	};
	let weapon = primary_weapon(member.user, &users, &bags);
	let grenade = weapon.and_then(grenade_charge);
	let show_orb = grenade.is_some();
	let silhouette =
		weapon.and_then(|item| icons.as_ref().and_then(|cache| cache.silhouette_for_item(item)));
	let gun_icon = if show_orb { None } else { silhouette };
	for (mut node, mut visibility) in &mut guns {
		node.display = weapon_slot_display(!show_orb);
		*visibility = if show_orb { Visibility::Hidden } else { Visibility::Inherited };
	}
	for mut node in &mut placeholders {
		// `Visibility::Hidden` still reserves layout and shoves the image right.
		node.display = if show_orb || gun_icon.is_some() { Display::None } else { Display::Flex };
	}
	for (mut image, mut visibility) in &mut silhouettes {
		if let Some(handle) = gun_icon.as_ref() {
			image.image = handle.clone();
			image.color = FIREARM_SILHOUETTE;
			*visibility = Visibility::Inherited;
		} else {
			*visibility = Visibility::Hidden;
		}
	}
	let verb = if show_orb { "Throw" } else { "Fire" };
	for mut text in &mut actions {
		if text.0 != verb {
			text.0 = verb.to_string();
		}
	}
}

pub fn sync_grenade_chrome(
	users: Query<(Entity, Option<&InventoryUser>), With<SkillMapUser>>,
	bags: Query<&Inventory>,
	icons: Option<Res<ItemIconCache>>,
	members: Query<&SkillMapMember, With<SkillMapFirearmHud>>,
	mut orbs: Query<
		(&mut Node, &mut Visibility),
		(
			With<SkillMapGrenadeSilhouette>,
			Without<SkillMapHud>,
			Without<SkillMapFirearmHud>,
			Without<SkillMapFirearmSilhouette>,
			Without<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapGrenadeSilhouetteImage>,
			Without<SkillMapGrenadeChargeImage>,
			Without<SkillMapGrenadePlaceholder>,
			Without<SkillMapGrenadeChargeFill>,
		),
	>,
	mut placeholders: Query<
		&mut Node,
		(
			With<SkillMapGrenadePlaceholder>,
			Without<SkillMapGrenadeSilhouette>,
			Without<SkillMapFirearmSilhouettePlaceholder>,
			Without<SkillMapGrenadeChargeFill>,
			Without<SkillMapGrenadeChargeImage>,
		),
	>,
	mut images: Query<
		(&mut ImageNode, &mut Visibility),
		(
			With<SkillMapGrenadeSilhouetteImage>,
			Without<SkillMapHud>,
			Without<SkillMapFirearmHud>,
			Without<SkillMapFirearmSilhouette>,
			Without<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapGrenadeChargeImage>,
			Without<SkillMapGrenadeChargeGlyph>,
			Without<SkillMapGrenadeSilhouette>,
		),
	>,
	mut charge_clips: Query<
		(&mut Node, &mut Visibility, &Children),
		(
			With<SkillMapGrenadeChargeImage>,
			Without<SkillMapHud>,
			Without<SkillMapFirearmHud>,
			Without<SkillMapFirearmSilhouette>,
			Without<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapGrenadeSilhouetteImage>,
			Without<SkillMapGrenadePlaceholder>,
			Without<SkillMapGrenadeChargeFill>,
			Without<SkillMapGrenadeSilhouette>,
			Without<SkillMapFirearmSilhouettePlaceholder>,
		),
	>,
	mut charge_glyphs: Query<
		&mut ImageNode,
		(
			With<SkillMapGrenadeChargeGlyph>,
			Without<SkillMapFirearmSilhouetteImage>,
			Without<SkillMapGrenadeSilhouetteImage>,
		),
	>,
	mut fills: Query<
		(&mut Node, &mut BackgroundColor),
		(
			With<SkillMapGrenadeChargeFill>,
			Without<SkillMapViewport>,
			Without<SkillMapGrenadeSilhouette>,
			Without<SkillMapGrenadePlaceholder>,
			Without<SkillMapGrenadeChargeImage>,
			Without<SkillMapFirearmSilhouettePlaceholder>,
		),
	>,
) {
	let Some(member) = members.iter().next() else {
		return;
	};
	let weapon = primary_weapon(member.user, &users, &bags);
	let grenade = weapon.and_then(grenade_charge);
	let show_orb = grenade.is_some();
	let charge_icon = show_orb
		.then_some(weapon)
		.flatten()
		.and_then(|item| icons.as_ref().and_then(|cache| cache.silhouette_for_item(item)));
	for (mut node, mut visibility) in &mut orbs {
		node.display = weapon_slot_display(show_orb);
		*visibility = if show_orb { Visibility::Inherited } else { Visibility::Hidden };
	}
	for mut node in &mut placeholders {
		node.display =
			if show_orb && charge_icon.is_none() { Display::Flex } else { Display::None };
	}
	for (mut image, mut visibility) in &mut images {
		if let Some(handle) = charge_icon.as_ref() {
			image.image = handle.clone();
			image.color = FIREARM_SILHOUETTE;
			*visibility = Visibility::Inherited;
		} else {
			*visibility = Visibility::Hidden;
		}
	}
	for (mut node, mut visibility, children) in &mut charge_clips {
		if let (Some(handle), Some(progress)) = (charge_icon.as_ref(), grenade) {
			for child in children.iter() {
				if let Ok(mut image) = charge_glyphs.get_mut(child) {
					image.image = handle.clone();
					image.color = if progress >= 1.0 - 1e-3 {
						TEXT_YELLOW
					} else {
						TEXT_YELLOW.with_alpha(0.55 + 0.45 * progress)
					};
				}
			}
			node.height = Val::Percent((progress * 100.0).clamp(0.0, 100.0));
			*visibility = Visibility::Inherited;
		} else {
			*visibility = Visibility::Hidden;
		}
	}
	if let Some(progress) = grenade {
		for (mut node, mut fill) in &mut fills {
			node.height = Val::Percent((progress * 100.0).clamp(0.0, 100.0));
			fill.0 = if progress >= 1.0 - 1e-3 {
				TEXT_YELLOW
			} else {
				TEXT_YELLOW.with_alpha(0.55 + 0.45 * progress)
			};
		}
	}
}

fn primary_weapon<'a>(
	user: Entity,
	users: &'a Query<(Entity, Option<&InventoryUser>), With<SkillMapUser>>,
	bags: &'a Query<&Inventory>,
) -> Option<&'a InventoryItem> {
	let (_, inventory) = users.iter().find(|(entity, _)| *entity == user)?;
	bags.get(inventory?.bag).ok()?.primary_weapon()
}

fn primary_weapon_name(
	user: Entity,
	users: &Query<(Entity, Option<&InventoryUser>), With<SkillMapUser>>,
	bags: &Query<&Inventory>,
) -> Option<String> {
	Some(primary_weapon(user, users, bags)?.name())
}

fn grenade_charge(item: &InventoryItem) -> Option<f32> {
	let InventoryItem::Grenade { stats, recharge, .. } = item else {
		return None;
	};
	if stats.recharge <= 1e-4 {
		return Some(1.0);
	}
	Some((1.0 - recharge.remaining / stats.recharge).clamp(0.0, 1.0))
}

type TrackedCameras<'w, 's> = Query<
	'w,
	's,
	(&'static SkillMapId, &'static SkillMapMember, &'static Transform),
	(With<SkillMapViewportCamera>, Without<SkillMapCursor>, Changed<Transform>),
>;

type TrackedCursors<'w, 's> = Query<
	'w,
	's,
	(&'static SkillMapId, &'static SkillMapMember, &'static mut Transform),
	(With<SkillMapCursor>, Without<SkillMapViewportCamera>),
>;

pub fn track_cursors(cameras: TrackedCameras, mut cursors: TrackedCursors) {
	for (map, camera_member, camera) in &cameras {
		for (cursor_map, cursor_member, mut cursor) in &mut cursors {
			if cursor_map.0 == map.0 && cursor_member.session == camera_member.session {
				cursor.translation.x = camera.translation.x;
				cursor.translation.y = camera.translation.y;
			}
		}
	}
}

pub fn spawn_debraid(
	commands: &mut Commands,
	fonts: Option<&HudFonts>,
	session: &SkillMapSession,
	id: SkillMapId,
	secs: f32,
) {
	let Some(node) = session.nodes.get(&id).copied() else {
		return;
	};
	commands.entity(node).with_children(|parent| {
		parent
			.spawn((
				DebraidOverlay,
				Debraid { remaining: secs },
				id,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(FRAME_BORDER),
					right: Val::Px(FRAME_BORDER),
					top: Val::Px(FRAME_BORDER),
					bottom: Val::Px(FRAME_BORDER),
					border_radius: BorderRadius::all(Val::Px(FRAME_INNER_RADIUS)),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					overflow: Overflow::clip(),
					..default()
				},
				BackgroundColor(Color::srgba(0.85, 0.08, 0.06, 0.55)),
				Pickable::IGNORE,
			))
			.with_children(|overlay| {
				let font =
					fonts.map(|fonts| fonts.header(PANEL_BLOCK_FONT_SIZE)).unwrap_or(TextFont {
						font_size: FontSize::Px(PANEL_BLOCK_FONT_SIZE),
						..default()
					});
				overlay.spawn((
					Text::new("DEBRAID"),
					font,
					TextColor(Color::WHITE),
					Pickable::IGNORE,
				));
			});
	});
}

pub fn tick_debraid(
	time: Res<Time>,
	mut commands: Commands,
	mut overlays: Query<(Entity, &mut Debraid), With<DebraidOverlay>>,
) {
	let dt = time.delta_secs();
	for (entity, mut debraid) in &mut overlays {
		debraid.remaining -= dt;
		if debraid.remaining <= 0.0 {
			commands.entity(entity).despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::SkillMapEnabled;

	#[test]
	fn unused_weapon_slot_leaves_the_layout() {
		assert_eq!(weapon_slot_display(false), Display::None);
		assert_eq!(weapon_slot_display(true), Display::Flex);
		assert_eq!(
			weapon_icon_slot().aspect_ratio,
			Some(
				FIREARM_SILHOUETTE_RESOLUTION.width as f32
					/ FIREARM_SILHOUETTE_RESOLUTION.height as f32
			)
		);
	}

	#[test]
	fn chrome_and_track_queries_are_disjoint() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_resource::<SkillMapEnabled>().add_systems(
			Update,
			(
				sync_viewport_chrome,
				sync_firearm_chrome,
				sync_grenade_chrome,
				track_cursors,
				tick_debraid,
			),
		);
		app.update();
	}

	#[test]
	fn kenney_steer_and_fire_icons_are_in_the_asset_tree() {
		let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
		assert!(root.join(SKILL_MAP_RB_ICON).is_file());
		assert!(root.join(SKILL_MAP_RT_ICON).is_file());
		assert!(root.join(SKILL_MAP_Y_ICON).is_file());
	}

	#[test]
	fn primary_weapon_label_is_the_hashed_name() {
		use character_items::{FirearmMesh, Inventory, InventoryItem};

		let mut bag = Inventory::default();
		bag.items.push(InventoryItem::firearm(FirearmMesh::Bullpup));
		bag.weapons.push(0);
		assert_eq!(bag.primary_weapon().map(InventoryItem::name), Some(bag.items[0].name()));
		assert!(bag.primary_weapon().is_some());
	}

	#[test]
	fn firearm_hud_keeps_placeholder_until_silhouette_is_ready() {
		use character_items::{FirearmMesh, InventoryItem};

		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let cache = ItemIconCache::default();
		assert!(cache.silhouette_for_item(&item).is_none());
		assert!(item.capture_id_label().is_some());
	}

	#[test]
	fn grenade_charge_fills_as_recharge_elapses() {
		use character_items::{GrenadeRecharge, GrenadeSpec, GrenadeStats, InventoryItem};

		let stats = GrenadeStats::standard();
		let ready = InventoryItem::Grenade {
			spec: GrenadeSpec::standard(),
			stats,
			recharge: GrenadeRecharge { remaining: 0.0 },
		};
		let half = InventoryItem::Grenade {
			spec: GrenadeSpec::standard(),
			stats,
			recharge: GrenadeRecharge { remaining: stats.recharge * 0.5 },
		};
		assert!((grenade_charge(&ready).unwrap() - 1.0).abs() < 1e-4);
		assert!((grenade_charge(&half).unwrap() - 0.5).abs() < 1e-4);
		assert!(grenade_charge(&InventoryItem::firearm(character_items::FirearmMesh::Bullpup))
			.is_none());
	}
}
