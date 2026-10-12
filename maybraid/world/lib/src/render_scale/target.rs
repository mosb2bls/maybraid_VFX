//! Offscreen 3D target + window blit. Despawned entirely at scale 1.0.

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::PrimaryWindow;
use player_camera::FollowCamera;

use crate::render_scale::RenderScale;

/// Present camera order matches the loading backdrop (`1`) and sits above the 3D camera (`0`).
const PRESENT_ORDER: isize = 1;

#[derive(Resource, Default)]
pub(super) struct AppliedRenderScale {
	image: Option<Handle<Image>>,
	last_offscreen: bool,
	last_size: UVec2,
	last_scale: f32,
	source: Option<(Entity, SourceTarget)>,
}

/// What the follow camera last rendered into.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SourceTarget {
	Window,
	Image { size: UVec2, scale_factor: f32 },
}

#[derive(Component)]
pub(super) struct RenderScalePresentCamera;

#[derive(Component)]
pub(super) struct RenderScalePresentSprite;

/// Window logical → render-target logical.
pub fn map_window_to_render(window_pos: Vec2, window_logical: Vec2, render_logical: Vec2) -> Vec2 {
	scale_point(window_pos, window_logical, render_logical)
}

/// Render-target logical → window logical (HUD pins, `world_to_viewport` → UI).
pub fn map_render_to_window(render_pos: Vec2, window_logical: Vec2, render_logical: Vec2) -> Vec2 {
	scale_point(render_pos, render_logical, window_logical)
}

fn scale_point(pos: Vec2, from: Vec2, to: Vec2) -> Vec2 {
	if from.x.abs() < 1e-6 || from.y.abs() < 1e-6 {
		return pos;
	}
	Vec2::new(pos.x * to.x / from.x, pos.y * to.y / from.y)
}

pub(super) fn sync_render_scale_target(
	mut commands: Commands,
	scale: Res<RenderScale>,
	windows: Query<&Window, With<PrimaryWindow>>,
	mut images: ResMut<Assets<Image>>,
	mut applied: ResMut<AppliedRenderScale>,
	mut sources: Query<
		(Entity, &Camera, &mut RenderTarget, &mut Projection, Has<IsDefaultUiCamera>),
		(With<Camera3d>, With<FollowCamera>, Without<RenderScalePresentCamera>),
	>,
	presents: Query<(Entity, &Camera), With<RenderScalePresentCamera>>,
	sprites: Query<Entity, With<RenderScalePresentSprite>>,
	mut sprite_images: Query<&mut Sprite, With<RenderScalePresentSprite>>,
) {
	let Ok(window) = windows.single() else {
		return;
	};
	let physical = UVec2::new(window.physical_width(), window.physical_height());
	let plan = scale.plan(physical, window.scale_factor());
	let Ok((source, source_camera, mut render_target, mut projection, source_is_ui)) =
		sources.single_mut()
	else {
		teardown_present(&mut commands, &presents, &sprites);
		return;
	};

	if !plan.offscreen || !source_camera.is_active {
		if source_camera.is_active && !source_is_ui {
			commands.entity(source).insert(IsDefaultUiCamera);
		}
		if retarget(&mut applied, source, SourceTarget::Window) {
			*render_target = RenderTarget::default();
			projection.set_changed();
		}
		teardown_present(&mut commands, &presents, &sprites);
		if applied.last_offscreen || applied.last_size == UVec2::ZERO {
			info!(
				"render scale {:.3}: window {}x{} (offscreen path off)",
				plan.scale, physical.x, physical.y
			);
			applied.last_offscreen = false;
			applied.last_size = physical;
			applied.last_scale = plan.scale;
		}
		return;
	}

	let handle = ensure_target_image(&mut images, &mut applied, plan.target_size);
	if !applied.last_offscreen
		|| applied.last_size != plan.target_size
		|| (applied.last_scale - plan.scale).abs() > 1e-5
	{
		info!(
			"render scale {:.3}: 3D {}x{} -> window {}x{}",
			plan.scale, plan.target_size.x, plan.target_size.y, physical.x, physical.y
		);
		applied.last_offscreen = true;
		applied.last_size = plan.target_size;
		applied.last_scale = plan.scale;
	}
	let image_target =
		SourceTarget::Image { size: plan.target_size, scale_factor: plan.image_scale_factor };
	if retarget(&mut applied, source, image_target) {
		*render_target = RenderTarget::Image(ImageRenderTarget {
			handle: handle.clone(),
			scale_factor: plan.image_scale_factor,
		});
		projection.set_changed();
	}
	if source_is_ui {
		commands.entity(source).remove::<IsDefaultUiCamera>();
	}

	match presents.single() {
		Ok((present, present_camera)) => {
			if !present_camera.is_active {
				commands.entity(present).insert((
					Camera {
						order: PRESENT_ORDER,
						is_active: true,
						clear_color: ClearColorConfig::Custom(Color::BLACK),
						..default()
					},
					IsDefaultUiCamera,
				));
			} else {
				commands.entity(present).insert(IsDefaultUiCamera);
			}
			if let Ok(mut sprite) = sprite_images.single_mut() {
				sprite.image = handle;
				sprite.custom_size = Some(plan.window_logical);
			} else if sprites.is_empty() {
				spawn_present_sprite(&mut commands, handle, plan.window_logical);
			}
		}
		Err(_) => {
			teardown_present(&mut commands, &presents, &sprites);
			spawn_present(&mut commands, handle, plan.window_logical);
		}
	}
}

/// Record `want` as the source camera's target; `true` when the camera must be retargeted.
///
/// Bevy's `camera_system` only refreshes a camera's cached target size on window or
/// image asset events (read a frame late) or a projection change. Swapping
/// [`RenderTarget`] alone leaves the 3D pass at the previous target's resolution, so
/// callers mark [`Projection`] changed whenever this returns `true`.
fn retarget(applied: &mut AppliedRenderScale, source: Entity, want: SourceTarget) -> bool {
	if applied.source == Some((source, want)) {
		return false;
	}
	applied.source = Some((source, want));
	true
}

fn ensure_target_image(
	images: &mut Assets<Image>,
	applied: &mut AppliedRenderScale,
	size: UVec2,
) -> Handle<Image> {
	if let Some(handle) = applied.image.clone() {
		if let Some(mut image) = images.get_mut(&handle) {
			let current = UVec2::new(image.width(), image.height());
			if current != size {
				image.resize(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 });
			}
			return handle;
		}
	}
	let handle = images.add(create_target_image(size));
	applied.image = Some(handle.clone());
	handle
}

fn create_target_image(size: UVec2) -> Image {
	let mut image = Image::new_uninit(
		Extent3d { width: size.x.max(1), height: size.y.max(1), depth_or_array_layers: 1 },
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::all(),
	);
	image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
		| TextureUsages::RENDER_ATTACHMENT
		| TextureUsages::COPY_SRC
		| TextureUsages::COPY_DST;
	image.sampler = ImageSampler::linear();
	image
}

fn spawn_present(commands: &mut Commands, image: Handle<Image>, window_logical: Vec2) {
	commands.spawn((
		Name::new("render-scale-present"),
		Camera2d,
		Camera {
			order: PRESENT_ORDER,
			clear_color: ClearColorConfig::Custom(Color::BLACK),
			..default()
		},
		IsDefaultUiCamera,
		RenderScalePresentCamera,
	));
	spawn_present_sprite(commands, image, window_logical);
}

fn spawn_present_sprite(commands: &mut Commands, image: Handle<Image>, window_logical: Vec2) {
	commands.spawn((
		Name::new("render-scale-blit"),
		Sprite { image, custom_size: Some(window_logical), ..default() },
		Transform::from_xyz(0.0, 0.0, 0.0),
		RenderScalePresentSprite,
	));
}

fn teardown_present(
	commands: &mut Commands,
	presents: &Query<(Entity, &Camera), With<RenderScalePresentCamera>>,
	sprites: &Query<Entity, With<RenderScalePresentSprite>>,
) {
	for (entity, _) in presents {
		commands.entity(entity).despawn();
	}
	for entity in sprites {
		commands.entity(entity).despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mapping_is_identity_when_spaces_match() -> anyhow::Result<()> {
		let size = Vec2::new(1280.0, 720.0);
		let pos = Vec2::new(200.0, 40.0);
		assert_eq!(map_window_to_render(pos, size, size), pos);
		assert_eq!(map_render_to_window(pos, size, size), pos);
		Ok(())
	}

	#[test]
	fn mapping_scales_half_res_target_into_the_window() -> anyhow::Result<()> {
		let window = Vec2::new(1280.0, 720.0);
		let render = Vec2::new(640.0, 360.0);
		let window_pos = Vec2::new(640.0, 360.0);
		let render_pos = map_window_to_render(window_pos, window, render);
		assert!((render_pos - Vec2::new(320.0, 180.0)).length() < 1e-3);
		let back = map_render_to_window(render_pos, window, render);
		assert!((back - window_pos).length() < 1e-3);
		Ok(())
	}

	#[test]
	fn mapping_round_trips_retina_logical_identity() -> anyhow::Result<()> {
		let plan = RenderScale::default().plan(UVec2::new(2560, 1440), 2.0);
		let render_logical = plan.target_size.as_vec2() / plan.image_scale_factor;
		let cursor = Vec2::new(100.0, 200.0);
		let mapped = map_window_to_render(cursor, plan.window_logical, render_logical);
		assert!((mapped - cursor).length() < 1e-3);
		Ok(())
	}

	fn source_target_size(app: &mut App) -> Option<UVec2> {
		let mut cameras = app.world_mut().query_filtered::<&Camera, With<FollowCamera>>();
		cameras.single(app.world()).ok()?.physical_target_size()
	}

	#[test]
	fn every_step_resizes_the_3d_pass() -> anyhow::Result<()> {
		use bevy::render::camera::camera_system;
		use bevy::render::texture::ManualTextureViews;
		use bevy::window::{WindowCreated, WindowResized, WindowScaleFactorChanged};

		let mut app = App::new();
		app.init_resource::<Assets<Image>>()
			.init_resource::<ManualTextureViews>()
			.init_resource::<AppliedRenderScale>()
			.insert_resource(RenderScale::default())
			.add_message::<WindowCreated>()
			.add_message::<WindowResized>()
			.add_message::<WindowScaleFactorChanged>()
			.add_message::<AssetEvent<Image>>()
			.add_systems(Update, (sync_render_scale_target, camera_system).chain());
		app.world_mut().spawn((Window::default(), PrimaryWindow));
		app.world_mut().spawn((Camera3d::default(), FollowCamera::default()));
		app.update();
		assert_eq!(source_target_size(&mut app), Some(UVec2::new(1280, 720)));

		for (scale, size) in [
			(0.5, UVec2::new(640, 360)),
			(1.0, UVec2::new(1280, 720)),
			(0.75, UVec2::new(960, 540)),
			(1.0, UVec2::new(1280, 720)),
			(0.5, UVec2::new(640, 360)),
			(0.75, UVec2::new(960, 540)),
		] {
			app.world_mut().resource_mut::<RenderScale>().set(scale);
			app.update();
			assert_eq!(source_target_size(&mut app), Some(size), "scale {scale}");
		}
		Ok(())
	}

	#[test]
	fn image_target_matches_resolved_output_usage() -> anyhow::Result<()> {
		let image = create_target_image(UVec2::new(64, 32));
		assert_eq!(image.width(), 64);
		assert_eq!(image.height(), 32);
		assert_eq!(image.texture_descriptor.format, TextureFormat::Bgra8UnormSrgb);
		let usage = image.texture_descriptor.usage;
		assert!(usage.contains(TextureUsages::RENDER_ATTACHMENT));
		assert!(usage.contains(TextureUsages::TEXTURE_BINDING));
		assert!(usage.contains(TextureUsages::COPY_SRC));
		Ok(())
	}
}
