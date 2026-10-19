//! Offscreen 3D target + window blit with visual style treatments.
//! Despawned entirely for Native (window resolution, no treatment) unless
//! [`super::VisualStyleSetting::force_offscreen`] keeps the apparatus.
//! Sharp always keeps the apparatus at its 1080p (height-fit) target.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::ui::Node;
use bevy::window::PrimaryWindow;
use player_camera::FollowCamera;

use super::presentation::{
	create_presentation_material, ImpressionistMaterial, NearestNeighborMaterial,
	PrePostModernMaterial, PresentationMaterial,
};
use super::visual_style::{PresentationTreatment, VisualStyle};
use super::{PresentationAction, PresentationChurn, RenderScale, VisualStyleSetting};

/// Present camera order matches the loading backdrop (`1`) and sits above the 3D camera (`0`).
const PRESENT_ORDER: isize = 1;
/// Isolated from world layer 0 so CPU visibility rejects scene entities early.
pub const PRESENT_RENDER_LAYER: usize = 29;
/// Ignore macOS fullscreen height flicker (1920×1080 ↔ 1920×1052 is 28 px).
const WINDOW_STABLE_DELTA: u32 = 32;

#[derive(Resource, Default)]
pub(crate) struct AppliedRenderScale {
	image: Option<Handle<Image>>,
	blit_mesh: Option<Handle<Mesh>>,
	last_offscreen: bool,
	last_size: UVec2,
	last_window_physical: Option<UVec2>,
	last_window_logical: Option<Vec2>,
	last_scale: f32,
	last_style: Option<VisualStyle>,
	last_treatment: Option<PresentationTreatment>,
	source: Option<(Entity, SourceTarget)>,
}

impl AppliedRenderScale {
	/// Offscreen 3D target actually written this frame, when the blit path is up.
	pub(crate) fn capture_source(&self) -> Option<(Handle<Image>, UVec2)> {
		if !self.last_offscreen {
			return None;
		}
		let handle = self.image.clone()?;
		if self.last_size.x == 0 || self.last_size.y == 0 {
			return None;
		}
		Some((handle, self.last_size))
	}
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

/// UI stamped onto [`PRESENT_RENDER_LAYER`] while the present camera is up.
#[derive(Component)]
pub(super) struct PresentUiLayer;

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

pub(super) fn present_render_layers() -> RenderLayers {
	RenderLayers::layer(PRESENT_RENDER_LAYER)
}

fn stabilize_window(last: Option<UVec2>, physical: UVec2) -> UVec2 {
	let Some(last) = last else {
		return physical;
	};
	if physical.x.abs_diff(last.x) < WINDOW_STABLE_DELTA
		&& physical.y.abs_diff(last.y) < WINDOW_STABLE_DELTA
	{
		last
	} else {
		physical
	}
}

pub(super) fn sync_render_scale_target(
	mut commands: Commands,
	visual_style: Res<VisualStyleSetting>,
	scale: Res<RenderScale>,
	windows: Query<&Window, With<PrimaryWindow>>,
	mut images: ResMut<Assets<Image>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut impressionist: ResMut<Assets<ImpressionistMaterial>>,
	mut pre_post_modern: ResMut<Assets<PrePostModernMaterial>>,
	mut nearest_neighbor: ResMut<Assets<NearestNeighborMaterial>>,
	mut applied: ResMut<AppliedRenderScale>,
	mut churn: MessageWriter<PresentationChurn>,
	mut sources: Query<
		(Entity, &Camera, &mut RenderTarget, &mut Projection, Has<IsDefaultUiCamera>),
		(With<Camera3d>, With<FollowCamera>, Without<RenderScalePresentCamera>),
	>,
	presents: Query<(Entity, &Camera, Has<IsDefaultUiCamera>), With<RenderScalePresentCamera>>,
	sprites: Query<Entity, With<RenderScalePresentSprite>>,
	mut blit_transforms: Query<&mut Transform, With<RenderScalePresentSprite>>,
	mut blit_sprites: Query<&mut Sprite, With<RenderScalePresentSprite>>,
) {
	let Ok(window) = windows.single() else {
		return;
	};
	let raw_physical = UVec2::new(window.physical_width(), window.physical_height());
	let physical = stabilize_window(applied.last_window_physical, raw_physical);

	let style_plan = visual_style.plan(physical, window.scale_factor());
	let legacy_plan = scale.plan(physical, window.scale_factor());

	let (
		target_size,
		window_logical,
		image_scale_factor,
		treatment,
		effective_style,
		requires_offscreen,
	) = if visual_style.requested().is_some() || !scale.is_explicit() {
		(
			style_plan.target_size,
			style_plan.window_logical,
			style_plan.image_scale_factor,
			style_plan.treatment,
			Some(style_plan.style),
			style_plan.requires_offscreen(physical),
		)
	} else {
		(
			legacy_plan.target_size,
			legacy_plan.window_logical,
			legacy_plan.image_scale_factor,
			PresentationTreatment::None,
			None,
			legacy_plan.offscreen,
		)
	};

	let Ok((source, source_camera, mut render_target, mut projection, source_is_ui)) =
		sources.single_mut()
	else {
		teardown_present(&mut commands, &presents, &sprites);
		return;
	};

	if !requires_offscreen || !source_camera.is_active {
		if source_camera.is_active && !source_is_ui {
			commands.entity(source).insert(IsDefaultUiCamera);
		}
		if retarget(&mut applied, source, SourceTarget::Window) {
			*render_target = RenderTarget::default();
			projection.set_changed();
		}
		if applied.last_offscreen {
			teardown_present(&mut commands, &presents, &sprites);
		}
		if applied.last_offscreen || applied.last_size == UVec2::ZERO {
			if let Some(style) = effective_style {
				info!(
					"visual style {}: window {}x{} (direct render, no treatment)",
					style.label(),
					physical.x,
					physical.y
				);
			} else {
				info!(
					"render scale 1.0: window {}x{} (offscreen path off)",
					physical.x, physical.y
				);
			}
			applied.last_offscreen = false;
			applied.last_size = physical;
			applied.last_window_physical = Some(physical);
			applied.last_window_logical = Some(window_logical);
			applied.last_scale = 1.0;
			applied.last_style = effective_style;
			applied.last_treatment = None;
		}
		return;
	}

	let unchanged = applied.last_offscreen
		&& applied.last_size == target_size
		&& applied.last_style == effective_style
		&& applied.last_treatment == Some(treatment)
		&& applied.last_window_logical == Some(window_logical)
		&& !presents.is_empty()
		&& !sprites.is_empty();
	if unchanged {
		if let Ok((present, present_camera, present_is_ui)) = presents.single() {
			if !present_camera.is_active {
				commands.entity(present).insert((
					Camera {
						order: PRESENT_ORDER,
						is_active: true,
						clear_color: ClearColorConfig::Custom(Color::BLACK),
						..default()
					},
					IsDefaultUiCamera,
					present_render_layers(),
				));
			} else if !present_is_ui {
				commands.entity(present).insert(IsDefaultUiCamera);
			}
		}
		return;
	}

	let handle = ensure_target_image(&mut images, &mut applied, target_size, treatment);
	let wash_strength = effective_style.map(VisualStyle::wash_strength).unwrap_or(1.0);
	let size_changed = applied.last_size != target_size;
	let rebuild_blit = sprites.is_empty()
		|| applied.last_treatment != Some(treatment)
		|| applied.last_style != effective_style
		|| size_changed
		|| !applied.last_offscreen;
	if !applied.last_offscreen
		|| size_changed
		|| applied.last_style != effective_style
		|| applied.last_treatment != Some(treatment)
	{
		if let Some(style) = effective_style {
			info!(
				"visual style {}: 3D {}x{} -> window {}x{} ({})",
				style.label(),
				target_size.x,
				target_size.y,
				physical.x,
				physical.y,
				treatment_label(treatment)
			);
		} else {
			let scale = target_size.x as f32 / physical.x.max(1) as f32;
			info!(
				"render scale {:.3}: 3D {}x{} -> window {}x{}",
				scale, target_size.x, target_size.y, physical.x, physical.y
			);
		}
	}
	if size_changed && applied.last_offscreen && applied.last_size != UVec2::ZERO {
		churn.write(PresentationChurn {
			action: PresentationAction::Resize,
			window: physical,
			target: target_size,
			style: effective_style,
		});
	}
	applied.last_offscreen = true;
	applied.last_size = target_size;
	applied.last_window_physical = Some(physical);
	applied.last_window_logical = Some(window_logical);
	applied.last_scale = target_size.x as f32 / physical.x.max(1) as f32;
	applied.last_style = effective_style;
	applied.last_treatment = Some(treatment);

	let image_target = SourceTarget::Image { size: target_size, scale_factor: image_scale_factor };
	if retarget(&mut applied, source, image_target) {
		*render_target = RenderTarget::Image(ImageRenderTarget {
			handle: handle.clone(),
			scale_factor: image_scale_factor,
		});
		projection.set_changed();
	}
	if source_is_ui {
		commands.entity(source).remove::<IsDefaultUiCamera>();
	}

	match presents.single() {
		Ok((present, present_camera, present_is_ui)) => {
			if !present_camera.is_active {
				commands.entity(present).insert((
					Camera {
						order: PRESENT_ORDER,
						is_active: true,
						clear_color: ClearColorConfig::Custom(Color::BLACK),
						..default()
					},
					IsDefaultUiCamera,
					present_render_layers(),
				));
			} else if !present_is_ui {
				commands.entity(present).insert(IsDefaultUiCamera);
			}
			if rebuild_blit {
				for entity in &sprites {
					commands.entity(entity).despawn();
				}
				spawn_present_blit(
					&mut commands,
					&mut applied,
					&mut meshes,
					&mut impressionist,
					&mut pre_post_modern,
					&mut nearest_neighbor,
					handle,
					target_size,
					physical,
					window_logical,
					treatment,
					wash_strength,
				);
				churn.write(PresentationChurn {
					action: PresentationAction::Recreate,
					window: physical,
					target: target_size,
					style: effective_style,
				});
			} else {
				write_blit_if_changed(
					&mut blit_transforms,
					&mut blit_sprites,
					window_logical,
					treatment,
				);
			}
		}
		Err(_) => {
			teardown_present(&mut commands, &presents, &sprites);
			spawn_present(
				&mut commands,
				&mut applied,
				&mut meshes,
				&mut impressionist,
				&mut pre_post_modern,
				&mut nearest_neighbor,
				handle,
				target_size,
				physical,
				window_logical,
				treatment,
				wash_strength,
			);
			churn.write(PresentationChurn {
				action: PresentationAction::Recreate,
				window: physical,
				target: target_size,
				style: effective_style,
			});
		}
	}
}

fn write_blit_if_changed(
	blit_transforms: &mut Query<&mut Transform, With<RenderScalePresentSprite>>,
	blit_sprites: &mut Query<&mut Sprite, With<RenderScalePresentSprite>>,
	window_logical: Vec2,
	treatment: PresentationTreatment,
) {
	if uses_sprite(treatment) {
		if let Ok(mut sprite) = blit_sprites.single_mut() {
			if sprite.custom_size != Some(window_logical) {
				sprite.custom_size = Some(window_logical);
			}
		}
		return;
	}
	if let Ok(mut transform) = blit_transforms.single_mut() {
		let want = Vec3::new(window_logical.x, window_logical.y, 1.0);
		if transform.scale != want {
			transform.scale = want;
		}
	}
}

fn uses_sprite(treatment: PresentationTreatment) -> bool {
	matches!(treatment, PresentationTreatment::None | PresentationTreatment::NearestNeighbor)
}

pub(super) fn stamp_present_ui_layers(
	presents: Query<Entity, With<RenderScalePresentCamera>>,
	mut commands: Commands,
	unlabeled: Query<Entity, (With<Node>, Without<RenderLayers>, Without<PresentUiLayer>)>,
	stamped: Query<Entity, With<PresentUiLayer>>,
) {
	if presents.is_empty() {
		for entity in &stamped {
			commands.entity(entity).remove::<(PresentUiLayer, RenderLayers)>();
		}
		return;
	}
	let layers = present_render_layers();
	for entity in &unlabeled {
		commands.entity(entity).insert((layers.clone(), PresentUiLayer));
	}
}

fn treatment_label(treatment: PresentationTreatment) -> &'static str {
	match treatment {
		PresentationTreatment::None => "no treatment",
		PresentationTreatment::NearestNeighbor => "nearest-neighbor",
		PresentationTreatment::Impressionist => "painterly",
		PresentationTreatment::PrePostModern => "color-print",
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
	treatment: PresentationTreatment,
) -> Handle<Image> {
	let sampler = match treatment {
		PresentationTreatment::NearestNeighbor => ImageSampler::nearest(),
		_ => ImageSampler::linear(),
	};

	if let Some(handle) = applied.image.clone() {
		if let Some(mut image) = images.get_mut(&handle) {
			let current = UVec2::new(image.width(), image.height());
			if current != size {
				image.resize(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 });
			}
			if image.sampler != sampler {
				image.sampler = sampler;
			}
			return handle;
		}
	}
	let handle = images.add(create_target_image(size, sampler));
	applied.image = Some(handle.clone());
	handle
}

fn create_target_image(size: UVec2, sampler: ImageSampler) -> Image {
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
	image.sampler = sampler;
	image
}

fn spawn_present(
	commands: &mut Commands,
	applied: &mut AppliedRenderScale,
	meshes: &mut Assets<Mesh>,
	impressionist: &mut Assets<ImpressionistMaterial>,
	pre_post_modern: &mut Assets<PrePostModernMaterial>,
	nearest_neighbor: &mut Assets<NearestNeighborMaterial>,
	image: Handle<Image>,
	source_size: UVec2,
	window_physical: UVec2,
	window_logical: Vec2,
	treatment: PresentationTreatment,
	wash_strength: f32,
) {
	commands.spawn((
		Name::new("render-scale-present"),
		Camera2d,
		Camera {
			order: PRESENT_ORDER,
			clear_color: ClearColorConfig::Custom(Color::BLACK),
			..default()
		},
		IsDefaultUiCamera,
		present_render_layers(),
		RenderScalePresentCamera,
	));
	spawn_present_blit(
		commands,
		applied,
		meshes,
		impressionist,
		pre_post_modern,
		nearest_neighbor,
		image,
		source_size,
		window_physical,
		window_logical,
		treatment,
		wash_strength,
	);
}

fn spawn_present_blit(
	commands: &mut Commands,
	applied: &mut AppliedRenderScale,
	meshes: &mut Assets<Mesh>,
	impressionist: &mut Assets<ImpressionistMaterial>,
	pre_post_modern: &mut Assets<PrePostModernMaterial>,
	nearest_neighbor: &mut Assets<NearestNeighborMaterial>,
	image: Handle<Image>,
	source_size: UVec2,
	window_physical: UVec2,
	window_logical: Vec2,
	treatment: PresentationTreatment,
	wash_strength: f32,
) {
	let layers = present_render_layers();
	if uses_sprite(treatment) {
		commands.spawn((
			Name::new("render-scale-blit"),
			Sprite { image, custom_size: Some(window_logical), ..default() },
			Transform::IDENTITY,
			layers,
			RenderScalePresentSprite,
		));
		return;
	}
	let mesh = applied.blit_mesh.clone().unwrap_or_else(|| {
		let handle = meshes.add(Rectangle::new(1.0, 1.0));
		applied.blit_mesh = Some(handle.clone());
		handle
	});
	let transform = Transform {
		translation: Vec3::ZERO,
		scale: Vec3::new(window_logical.x, window_logical.y, 1.0),
		..default()
	};
	let mut entity = commands.spawn((
		Name::new("render-scale-blit"),
		Mesh2d(mesh),
		transform,
		layers,
		RenderScalePresentSprite,
	));
	if let Some(material) = create_presentation_material(
		treatment,
		image,
		source_size,
		window_physical,
		wash_strength,
		impressionist,
		pre_post_modern,
		nearest_neighbor,
	) {
		match material {
			PresentationMaterial::Impressionist(handle) => {
				entity.insert(MeshMaterial2d(handle));
			}
			PresentationMaterial::PrePostModern(handle) => {
				entity.insert(MeshMaterial2d(handle));
			}
		}
	}
}

fn teardown_present(
	commands: &mut Commands,
	presents: &Query<(Entity, &Camera, Has<IsDefaultUiCamera>), With<RenderScalePresentCamera>>,
	sprites: &Query<Entity, With<RenderScalePresentSprite>>,
) {
	for (entity, _, _) in presents {
		commands.entity(entity).despawn();
	}
	for entity in sprites {
		commands.entity(entity).despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::render_scale::VisualStyleSetting;

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

	fn boot_target_app() -> App {
		use bevy::render::camera::camera_system;
		use bevy::render::texture::ManualTextureViews;
		use bevy::window::{WindowCreated, WindowResized, WindowScaleFactorChanged};

		let mut app = App::new();
		app.init_resource::<Assets<Image>>()
			.init_resource::<Assets<Mesh>>()
			.init_resource::<Assets<ImpressionistMaterial>>()
			.init_resource::<Assets<PrePostModernMaterial>>()
			.init_resource::<Assets<NearestNeighborMaterial>>()
			.init_resource::<ManualTextureViews>()
			.init_resource::<AppliedRenderScale>()
			.insert_resource(VisualStyleSetting::default())
			.insert_resource(RenderScale::default())
			.add_message::<WindowCreated>()
			.add_message::<WindowResized>()
			.add_message::<WindowScaleFactorChanged>()
			.add_message::<AssetEvent<Image>>()
			.add_message::<PresentationChurn>()
			.add_systems(
				Update,
				(sync_render_scale_target, stamp_present_ui_layers, camera_system).chain(),
			);
		app.world_mut().spawn((Window::default(), PrimaryWindow));
		app.world_mut().spawn((Camera3d::default(), FollowCamera::default()));
		app
	}

	fn present_count(app: &mut App) -> usize {
		let mut presents =
			app.world_mut().query_filtered::<Entity, With<RenderScalePresentCamera>>();
		presents.iter(app.world()).count()
	}

	fn present_layers(app: &mut App) -> Option<RenderLayers> {
		let mut presents = app
			.world_mut()
			.query_filtered::<&RenderLayers, With<RenderScalePresentCamera>>();
		presents.single(app.world()).ok().cloned()
	}

	fn blit_is_sprite(app: &mut App) -> bool {
		let mut sprites =
			app.world_mut().query_filtered::<&Sprite, With<RenderScalePresentSprite>>();
		sprites.single(app.world()).is_ok()
	}

	fn take_churn(app: &mut App) -> Vec<PresentationChurn> {
		let mut messages = app.world_mut().resource_mut::<Messages<PresentationChurn>>();
		messages.drain().collect()
	}

	fn set_window(app: &mut App, physical: UVec2) {
		let mut windows = app.world_mut().query_filtered::<&mut Window, With<PrimaryWindow>>();
		let mut window = windows.single_mut(app.world_mut()).expect("window");
		window.resolution.set_physical_resolution(physical.x, physical.y);
	}

	#[test]
	fn every_step_resizes_the_3d_pass() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		app.world_mut().resource_mut::<RenderScale>().set(1.0);
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
	fn visual_styles_resize_and_native_is_direct() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		for (style, size, present) in [
			(VisualStyle::Impressionist, UVec2::new(700, 394), true),
			(VisualStyle::Watercolor, UVec2::new(1280, 720), true),
			(VisualStyle::Native, UVec2::new(1280, 720), false),
			(VisualStyle::Sharp, UVec2::new(1280, 720), true),
			(VisualStyle::Retro, UVec2::new(540, 304), true),
			(VisualStyle::Antediluvian, UVec2::new(400, 225), true),
			(VisualStyle::PrePostModern, UVec2::new(1280, 720), true),
		] {
			app.world_mut().resource_mut::<VisualStyleSetting>().set(style);
			app.update();
			assert_eq!(source_target_size(&mut app), Some(size), "{style:?}");
			assert_eq!(present_count(&mut app) > 0, present, "{style:?} present");
		}
		Ok(())
	}

	#[test]
	fn sharp_renders_1080p_offscreen_on_a_larger_window() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		set_window(&mut app, UVec2::new(2560, 1440));
		app.world_mut().resource_mut::<VisualStyleSetting>().set(VisualStyle::Sharp);
		app.update();
		assert_eq!(source_target_size(&mut app), Some(UVec2::new(1920, 1080)));
		assert_eq!(present_count(&mut app), 1);
		assert!(blit_is_sprite(&mut app));
		take_churn(&mut app);
		app.update();
		assert!(take_churn(&mut app).is_empty(), "unchanged Sharp presentation is a no-op");
		Ok(())
	}

	#[test]
	fn fullscreen_height_flicker_does_not_resize_the_3d_target() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		set_window(&mut app, UVec2::new(1920, 1080));
		app.world_mut()
			.resource_mut::<VisualStyleSetting>()
			.set(VisualStyle::Impressionist);
		app.update();
		assert_eq!(source_target_size(&mut app), Some(UVec2::new(700, 394)));
		take_churn(&mut app);
		set_window(&mut app, UVec2::new(1920, 1052));
		app.update();
		assert_eq!(source_target_size(&mut app), Some(UVec2::new(700, 394)));
		assert!(take_churn(&mut app).is_empty(), "28 px height flicker must be a no-op");
		Ok(())
	}

	#[test]
	fn unchanged_presentation_is_a_noop() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		app.world_mut()
			.resource_mut::<VisualStyleSetting>()
			.set(VisualStyle::Impressionist);
		app.update();
		take_churn(&mut app);
		app.update();
		assert!(take_churn(&mut app).is_empty());
		assert_eq!(present_count(&mut app), 1);
		Ok(())
	}

	#[test]
	fn present_camera_uses_an_isolated_layer() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		app.world_mut()
			.resource_mut::<VisualStyleSetting>()
			.set(VisualStyle::Impressionist);
		app.update();
		let layers = present_layers(&mut app).ok_or_else(|| anyhow::anyhow!("present layers"))?;
		assert!(layers.intersects(&present_render_layers()));
		assert!(!layers.intersects(&RenderLayers::layer(0)));
		Ok(())
	}

	#[test]
	fn retro_and_antediluvian_use_a_nearest_sprite() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		for style in [VisualStyle::Retro, VisualStyle::Antediluvian] {
			app.world_mut().resource_mut::<VisualStyleSetting>().set(style);
			app.update();
			assert!(blit_is_sprite(&mut app), "{style:?}");
		}
		app.world_mut()
			.resource_mut::<VisualStyleSetting>()
			.set(VisualStyle::Impressionist);
		app.update();
		assert!(!blit_is_sprite(&mut app), "Impressionist keeps the material blit");
		Ok(())
	}

	#[test]
	fn force_offscreen_native_keeps_the_present_camera() -> anyhow::Result<()> {
		let mut app = boot_target_app();
		{
			let mut style = app.world_mut().resource_mut::<VisualStyleSetting>();
			style.set(VisualStyle::Native);
			style.set_force_offscreen(true);
		}
		app.update();
		assert_eq!(source_target_size(&mut app), Some(UVec2::new(1280, 720)));
		assert_eq!(present_count(&mut app), 1);
		assert!(blit_is_sprite(&mut app));
		Ok(())
	}

	#[test]
	fn impressionist_window_mapping_scales_into_the_render_target() -> anyhow::Result<()> {
		let plan = VisualStyle::Impressionist.plan(UVec2::new(1920, 1080), 1.0);
		let window = plan.window_logical;
		let render = plan.target_size.as_vec2() / plan.image_scale_factor;
		let cursor = window * 0.5;
		let mapped = map_window_to_render(cursor, window, render);
		let back = map_render_to_window(mapped, window, render);
		assert!((back - cursor).length() < 1e-3);
		assert!(mapped.x > 0.0 && mapped.y > 0.0);
		Ok(())
	}

	#[test]
	fn image_target_matches_resolved_output_usage() -> anyhow::Result<()> {
		let image = create_target_image(UVec2::new(64, 32), ImageSampler::linear());
		assert_eq!(image.width(), 64);
		assert_eq!(image.height(), 32);
		assert_eq!(image.texture_descriptor.format, TextureFormat::Bgra8UnormSrgb);
		let usage = image.texture_descriptor.usage;
		assert!(usage.contains(TextureUsages::RENDER_ATTACHMENT));
		assert!(usage.contains(TextureUsages::TEXTURE_BINDING));
		assert!(usage.contains(TextureUsages::COPY_SRC));
		Ok(())
	}

	#[test]
	fn stabilize_window_ignores_sub_delta_flicker() {
		let locked = UVec2::new(1920, 1080);
		assert_eq!(stabilize_window(Some(locked), UVec2::new(1920, 1052)), locked);
		assert_eq!(stabilize_window(Some(locked), UVec2::new(1920, 800)), UVec2::new(1920, 800));
		assert_eq!(stabilize_window(None, UVec2::new(1920, 1052)), UVec2::new(1920, 1052));
	}
}
