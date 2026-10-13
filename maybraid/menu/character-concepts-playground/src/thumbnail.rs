use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use character_ui_menu::{ThumbnailCamera, ThumbnailRequest};
use std::collections::HashMap;

const THUMBNAIL_SPACING: f32 = 8.0;

#[derive(Resource, Default)]
pub struct ThumbnailCache {
	entries: HashMap<ThumbnailKey, ThumbnailEntry>,
	next_slot: u32,
	revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ThumbnailKey {
	path: &'static str,
	color: [u8; 4],
}

#[derive(Clone)]
struct ThumbnailEntry {
	camera: Entity,
	image: Handle<Image>,
	last_seen_revision: u64,
}

#[derive(Component)]
pub struct ThumbnailPreview {
	pub color: Color,
}

impl ThumbnailCache {
	pub fn begin_ui_rebuild(&mut self) {
		self.revision += 1;
	}

	pub fn cached_image(&self, path: &'static str, color: Color) -> Option<Handle<Image>> {
		self.entries
			.get(&ThumbnailKey { path, color: color_key(color) })
			.map(|entry| entry.image.clone())
	}
}

pub fn image_for_asset(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	asset_server: &AssetServer,
	cache: &mut ThumbnailCache,
	label: &'static str,
	path: &'static str,
	color: Color,
	thumbnail_camera: ThumbnailCamera,
) -> Handle<Image> {
	let key = ThumbnailKey { path, color: color_key(color) };
	if let Some(entry) = cache.entries.get_mut(&key) {
		entry.last_seen_revision = cache.revision;
		return entry.image.clone();
	}

	let slot = cache.next_slot;
	cache.next_slot += 1;
	let base = Vec3::new(4000.0 + slot as f32 * THUMBNAIL_SPACING, 0.0, 0.0);
	let image = images.add(thumbnail_image());
	let image_handle = image.clone();
	let camera_transform = Transform::from_translation(base + thumbnail_camera.position)
		.looking_at(base + thumbnail_camera.look_at, Vec3::Y);
	let camera = commands
		.spawn((
			Camera3d::default(),
			Camera { order: -1, ..default() },
			RenderTarget::Image(image.into()),
			Projection::Perspective(PerspectiveProjection {
				fov: 0.75,
				near: 0.05,
				far: 10.0,
				..default()
			}),
			camera_transform,
			Name::new(format!("thumbnail_camera_{label}")),
		))
		.id();

	commands.spawn((
		WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
		Transform::from_translation(base),
		ThumbnailPreview { color },
		Name::new(format!("thumbnail_asset_{label}")),
	));

	commands.spawn((
		PointLight { intensity: 450.0, range: 6.0, shadow_maps_enabled: false, ..default() },
		Transform::from_translation(base + Vec3::new(0.3, 1.8, 1.6)),
		Name::new(format!("thumbnail_light_{label}")),
	));

	cache.entries.insert(
		key,
		ThumbnailEntry { camera, image: image_handle.clone(), last_seen_revision: cache.revision },
	);
	image_handle
}

pub fn sync_thumbnail_camera_activity(cache: Res<ThumbnailCache>, mut cameras: Query<&mut Camera>) {
	for entry in cache.entries.values() {
		let Ok(mut camera) = cameras.get_mut(entry.camera) else {
			continue;
		};
		camera.is_active = entry.last_seen_revision == cache.revision;
	}
}

fn color_key(color: Color) -> [u8; 4] {
	let color = color.to_srgba();
	[
		(color.red * 255.0).round() as u8,
		(color.green * 255.0).round() as u8,
		(color.blue * 255.0).round() as u8,
		(color.alpha * 255.0).round() as u8,
	]
}

pub fn prewarm_thumbnail_requests(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	asset_server: &AssetServer,
	cache: &mut ThumbnailCache,
	requests: &[ThumbnailRequest],
) {
	for request in requests {
		if request.path.is_empty() {
			continue;
		}
		let color = Color::srgba(
			request.color[0] as f32 / 255.0,
			request.color[1] as f32 / 255.0,
			request.color[2] as f32 / 255.0,
			request.color[3] as f32 / 255.0,
		);
		let _ = image_for_asset(
			commands,
			images,
			asset_server,
			cache,
			"prewarm",
			request.path,
			color,
			request.camera,
		);
	}
}

pub fn thumbnail_image() -> Image {
	let mut image = Image::new_uninit(
		default(),
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::all(),
	);
	image.texture_descriptor.usage =
		TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
	image
}
