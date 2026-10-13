//! Menu bind of the live skill-map view: same camera, tiles, and [`ViewportNode`].

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::ui::widget::ViewportNode;
use character_items::SkillMapSpec;

use crate::map::{authored_map, authored_map_from_spec, MapExtents, SkillKind, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::tiles::spawn_map_tiles_at;
use crate::user::SkillMapMember;
use crate::viewport::spawn_skill_map_view;

/// Square catalog cell. [`ViewportNode`] resizes the target to this.
pub const CATALOG_PREVIEW_PX: u32 = 160;
/// First [`SkillMapId`] reserved for menu previews (live play uses `0`).
pub const MENU_PREVIEW_ID_BASE: u32 = 16;

#[derive(Component, Clone, Copy, Debug)]
pub struct SkillMapMenuPreview {
	pub spec: SkillMapSpec,
}

/// Root HUD used while a starter skill map is revealed.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SkillMapSpinRevealHud;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SkillMapSpinRevealHudView;

/// Camera + image a catalog or reveal [`ViewportNode`] can bind.
#[derive(Clone, Debug)]
pub struct SkillMapCatalogPreview {
	pub host: Entity,
	pub camera: Entity,
	pub image: Handle<Image>,
	pub spec: SkillMapSpec,
}

/// First [`SkillMapId`] reserved for icon-cache paint booths (live `0`, menu `16`).
pub const ICON_PAINT_ID_BASE: u32 = 8;

pub(crate) fn catalog_world_span() -> f32 {
	let extents = MapExtents::default();
	extents.tile_size().x * extents.steps as f32
}

fn catalog_projection() -> OrthographicProjection {
	let mut projection = OrthographicProjection::default_2d();
	let span = catalog_world_span();
	projection.scaling_mode = ScalingMode::Fixed { width: span, height: span };
	projection.scale = 1.0;
	projection
}

/// Off-screen `Camera2d` that paints authored tiles into a sized image.
pub fn spawn_skill_map_paint_view(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	tiles: &SkillMapTileAssets,
	spec: SkillMapSpec,
	id: SkillMapId,
	resolution: u32,
	parent: Entity,
) -> Handle<Image> {
	let mut authored = authored_map_from_spec(spec);
	authored.id = id;
	let mut image = Image::new_uninit(
		Extent3d { width: resolution.max(1), height: resolution.max(1), depth_or_array_layers: 1 },
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
		| TextureUsages::COPY_SRC
		| TextureUsages::COPY_DST
		| TextureUsages::RENDER_ATTACHMENT;
	let image_handle = images.add(image);
	commands.entity(parent).with_children(|booth| {
		booth.spawn((
			Name::new(format!("skill-map-paint-{}", spec.kind.label())),
			Camera2d,
			Camera {
				order: -60 - id.0 as isize,
				clear_color: ClearColorConfig::Custom(authored.kind.viewport_clear()),
				..default()
			},
			RenderTarget::Image(image_handle.clone().into()),
			Projection::Orthographic(catalog_projection()),
			Transform::from_xyz(0.0, 0.0, 1.0),
			id,
			crate::map::render_layer(id),
		));
	});
	spawn_map_tiles_at(commands, authored, None, tiles, Vec2::ZERO, Some(parent));
	image_handle
}

/// Same [`spawn_skill_map_view`] path as play. Caller attaches [`ViewportNode`].
pub fn spawn_skill_map_catalog_preview(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	tiles: &SkillMapTileAssets,
	spec: SkillMapSpec,
	slot: u32,
) -> SkillMapCatalogPreview {
	let id = SkillMapId(MENU_PREVIEW_ID_BASE + slot);
	let mut authored = authored_map(SkillKind::from_item(spec.kind), spec.seed);
	authored.id = id;
	let host = commands
		.spawn((
			Name::new(format!("skill-map-catalog-{}", spec.kind.label())),
			SkillMapMenuPreview { spec },
		))
		.id();
	let member = SkillMapMember { user: host, session: host };
	let view = spawn_skill_map_view(
		commands,
		images,
		tiles,
		authored,
		member,
		-40 - slot as isize,
		catalog_projection(),
		(),
	);
	SkillMapCatalogPreview { host, camera: view.camera, image: view.image, spec }
}

/// Centered catalog PNG. Inventory no longer keeps a live map camera.
pub fn spawn_skill_map_spin_reveal_image(
	commands: &mut Commands,
	image: Handle<Image>,
	size: f32,
) -> Entity {
	commands
		.spawn((
			Name::new("skill-map-spin-reveal-hud"),
			SkillMapSpinRevealHud,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				right: Val::Px(0.0),
				top: Val::Px(0.0),
				bottom: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|center| {
			center.spawn((
				SkillMapSpinRevealHudView,
				ImageNode::new(image),
				Node { width: Val::Px(size), height: Val::Px(size), ..default() },
				Pickable::IGNORE,
			));
		})
		.id()
}

/// Centered root [`ViewportNode`], same attach as the live corner map.
pub fn spawn_skill_map_spin_reveal_hud(
	commands: &mut Commands,
	camera: Entity,
	size: f32,
) -> Entity {
	commands
		.spawn((
			Name::new("skill-map-spin-reveal-hud"),
			SkillMapSpinRevealHud,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				right: Val::Px(0.0),
				top: Val::Px(0.0),
				bottom: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|center| {
			center.spawn((
				SkillMapSpinRevealHudView,
				ViewportNode::new(camera),
				Node { width: Val::Px(size), height: Val::Px(size), ..default() },
				Pickable::IGNORE,
			));
		})
		.id()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn catalog_camera_fits_the_authored_extents() {
		let extents = MapExtents::default();
		let world = extents.tile_size().x * extents.steps as f32;
		assert!((catalog_world_span() - world).abs() < f32::EPSILON);
	}
}
