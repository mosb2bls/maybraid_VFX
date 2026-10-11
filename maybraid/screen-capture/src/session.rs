//! Isolated capture scene, readiness, silhouette, and GPU readback.

use bevy::asset::{Assets, RenderAssetUsages};
use bevy::camera::primitives::MeshAabb;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::TextureFormat;
use bevy::world_serialization::WorldAssetRoot;
use scene_ref::SceneRefRoot;

use crate::crop::crop_to_alpha;
use crate::error::{CaptureError, CaptureWarning, CaptureWarningKind};
use crate::key::CaptureCacheKey;
use crate::mode::CaptureMode;
use crate::png::{bgra_to_rgba, render_target_image};
use crate::queue::QueuedCapture;
use crate::request::{CaptureRequest, CaptureResolution};
use crate::subject::{CaptureMeshKind, CaptureScene};
use crate::view::{CaptureFraming, CaptureProjection, CaptureView};

/// Render layer reserved for isolated capture cameras and subjects.
pub const CAPTURE_RENDER_LAYER: usize = 31;

const SLOT_SPACING: f32 = 24.0;
const SLOT_ORIGIN_X: f32 = 8_000.0;
const SILHOUETTE_GRAY: f32 = 0.72;
/// Fitted captures render this many times larger, then crop to alpha.
const FIT_SUPERSAMPLE: u32 = 3;
/// 3D fit margin. Bind-pose AABBs can undershoot a posed kit; the alpha
/// crop removes whatever margin is left over.
const FIT_SCENE_PADDING: f32 = 0.2;

/// Isolated capture root. Application hosts parent posed subjects here.
#[derive(Component)]
pub struct CaptureHostRoot {
	pub kind: String,
	pub id: crate::CaptureId,
}

/// Application finished realizing a [`CaptureScene::Host`] (meshes + materials).
#[derive(Component)]
pub struct CaptureHostReady;

/// Silhouette session root. Hosts may strip custom materials under this marker.
#[derive(Component)]
pub struct CaptureSilhouetteRoot;

#[derive(Component)]
pub(crate) struct CaptureRoot;

#[derive(Component)]
pub(crate) struct CaptureCamera;

#[derive(Component)]
pub(crate) struct CaptureReadback {
	pub key: CaptureCacheKey,
	pub ticket: Handle<Image>,
	/// Output (cached PNG) size.
	pub width: u32,
	pub height: u32,
	/// GPU target size; larger than output when [`Self::crop`] is set.
	pub render_width: u32,
	pub render_height: u32,
	/// Image-space alpha crop padding for [`CaptureFraming::FitExtents`].
	pub crop: Option<f32>,
}

#[derive(Component)]
pub(crate) struct CapturePixels(pub Vec<u8>);

pub(crate) enum CapturePhase {
	Load { frames: u32 },
	Settle { remaining: u32 },
	Readback { frames: u32 },
}

pub(crate) struct CaptureSession {
	pub key: CaptureCacheKey,
	pub request: CaptureRequest,
	pub ticket: Handle<Image>,
	pub target: Handle<Image>,
	pub root: Entity,
	pub camera: Entity,
	pub extras: Vec<Entity>,
	pub readback: Option<Entity>,
	pub phase: CapturePhase,
	pub framed: bool,
}

impl CaptureSession {
	pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
		std::iter::once(self.root)
			.chain(std::iter::once(self.camera))
			.chain(self.extras.iter().copied())
			.chain(self.readback)
	}
}

pub(crate) fn spawn_session(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	queued: QueuedCapture,
	slot: u32,
	layer: usize,
) -> CaptureSession {
	let request = queued.request;
	let framed = matches!(request.framing, CaptureFraming::Authored);
	let key = CaptureCacheKey::from_request(&request);
	let origin = Vec3::new(SLOT_ORIGIN_X + slot as f32 * SLOT_SPACING, 0.0, 0.0);
	let layers = RenderLayers::layer(layer);
	let target = images.add(render_target_image(render_resolution(&request)));
	let root = spawn_subject(commands, meshes, materials, &request, origin, layers.clone());
	let camera = spawn_camera(commands, &request, origin, target.clone(), layers.clone());
	let mut extras = Vec::new();
	if request.mode == CaptureMode::Shaded {
		for (light, transform) in request.lighting.spawn_specs(origin) {
			extras.push(
				commands
					.spawn((
						light,
						transform,
						layers.clone(),
						CaptureRoot,
						Name::new("capture_light"),
					))
					.id(),
			);
		}
	}
	CaptureSession {
		key,
		request,
		ticket: queued.image,
		target,
		root,
		camera,
		extras,
		readback: None,
		phase: CapturePhase::Load { frames: 0 },
		framed,
	}
}

fn render_resolution(request: &CaptureRequest) -> CaptureResolution {
	match request.framing {
		CaptureFraming::Authored => request.resolution,
		CaptureFraming::FitExtents { .. } => CaptureResolution::new(
			(request.resolution.width * FIT_SUPERSAMPLE).min(4096),
			(request.resolution.height * FIT_SUPERSAMPLE).min(4096),
		),
	}
}

fn crop_padding(framing: CaptureFraming) -> Option<f32> {
	match framing {
		CaptureFraming::Authored => None,
		CaptureFraming::FitExtents { padding } => Some(padding),
	}
}

fn spawn_subject(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	request: &CaptureRequest,
	origin: Vec3,
	layers: RenderLayers,
) -> Entity {
	let transform = Transform::from_translation(origin);
	let silhouette = request.mode == CaptureMode::Silhouette;
	let root = match &request.scene {
		CaptureScene::SceneRef(scene_ref) => commands
			.spawn((
				SceneRefRoot(scene_ref.clone()),
				transform,
				Visibility::default(),
				layers,
				CaptureRoot,
				Name::new("capture_scene"),
			))
			.id(),
		CaptureScene::Mesh(mesh) => {
			let bevy_mesh = match mesh.kind {
				CaptureMeshKind::Cuboid => Mesh::from(Cuboid::from_length(1.0)),
			};
			let material = StandardMaterial {
				base_color: mesh.color(),
				perceptual_roughness: 0.6,
				..Default::default()
			};
			commands
				.spawn((
					Mesh3d(meshes.add(bevy_mesh)),
					MeshMaterial3d(materials.add(material)),
					transform,
					Visibility::default(),
					layers,
					CaptureRoot,
					Name::new("capture_mesh"),
				))
				.id()
		}
		CaptureScene::Host { kind, .. } => commands
			.spawn((
				CaptureHostRoot { kind: kind.clone(), id: request.id.clone() },
				transform,
				Visibility::default(),
				layers,
				CaptureRoot,
				Name::new("capture_host"),
			))
			.id(),
	};
	if silhouette {
		commands.entity(root).try_insert(CaptureSilhouetteRoot);
	}
	root
}

fn spawn_camera(
	commands: &mut Commands,
	request: &CaptureRequest,
	origin: Vec3,
	target: Handle<Image>,
	layers: RenderLayers,
) -> Entity {
	let (transform, projection) = camera_pose(origin, request.view);
	commands
		.spawn((
			Camera3d::default(),
			Camera {
				order: -20,
				is_active: true,
				clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
				..Default::default()
			},
			RenderTarget::Image(target.into()),
			projection,
			transform,
			Msaa::Sample4,
			layers,
			CaptureCamera,
			CaptureRoot,
			Name::new("capture_camera"),
		))
		.id()
}

fn camera_pose(origin: Vec3, view: CaptureView) -> (Transform, Projection) {
	let transform =
		Transform::from_translation(origin + view.eye).looking_at(origin + view.look_at, view.up);
	let projection = match view.projection {
		CaptureProjection::Perspective { fov, near, far } => {
			Projection::Perspective(PerspectiveProjection { fov, near, far, ..Default::default() })
		}
		CaptureProjection::Orthographic { scale, near, far } => {
			Projection::Orthographic(OrthographicProjection {
				scale,
				near,
				far,
				..OrthographicProjection::default_3d()
			})
		}
	};
	(transform, projection)
}

pub(crate) fn apply_fit_framing(
	commands: &mut Commands,
	session: &mut CaptureSession,
	meshes: &Assets<Mesh>,
	mesh_3d: &Query<&Mesh3d>,
	children: &Query<&Children>,
	globals: &Query<&GlobalTransform>,
) -> bool {
	let CaptureFraming::FitExtents { padding } = session.request.framing else {
		return true;
	};
	let Some(aabb) = subject_local_aabb(session.root, children, mesh_3d, meshes, globals) else {
		return false;
	};
	let aspect =
		session.request.resolution.width as f32 / session.request.resolution.height.max(1) as f32;
	let fitted = session.request.view.fit_extents(aabb, aspect, padding.max(FIT_SCENE_PADDING));
	let origin = globals
		.get(session.root)
		.map(|transform| transform.translation())
		.unwrap_or(Vec3::ZERO);
	let (transform, projection) = camera_pose(origin, fitted);
	commands.entity(session.camera).try_insert((transform, projection));
	session.request.view = fitted;
	true
}

fn subject_local_aabb(
	root: Entity,
	children: &Query<&Children>,
	mesh_3d: &Query<&Mesh3d>,
	meshes: &Assets<Mesh>,
	globals: &Query<&GlobalTransform>,
) -> Option<Aabb3d> {
	let root_tf = globals.get(root).ok()?;
	let inverse = root_tf.affine().inverse();
	let mut union: Option<Aabb3d> = None;
	visit_descendants(root, children, &mut |entity| {
		let Ok(handle) = mesh_3d.get(entity) else {
			return;
		};
		let Some(mesh) = meshes.get(&handle.0) else {
			return;
		};
		let Some(local) = mesh.compute_aabb() else {
			return;
		};
		let Ok(world) = globals.get(entity) else {
			return;
		};
		if world.to_scale_rotation_translation().0.max_element() < 1e-3 {
			return;
		}
		let min = Vec3::from(local.min());
		let max = Vec3::from(local.max());
		let world_aabb = transform_aabb(Aabb3d { min: min.into(), max: max.into() }, world);
		let local_aabb = transform_aabb_affine(world_aabb, inverse);
		union = Some(match union {
			Some(existing) => union_aabb(existing, local_aabb),
			None => local_aabb,
		});
	});
	union.filter(|aabb| (Vec3::from(aabb.max) - Vec3::from(aabb.min)).max_element() > 1e-4)
}

fn transform_aabb(aabb: Aabb3d, transform: &GlobalTransform) -> Aabb3d {
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for x in [aabb.min.x, aabb.max.x] {
		for y in [aabb.min.y, aabb.max.y] {
			for z in [aabb.min.z, aabb.max.z] {
				let point = transform.transform_point(Vec3::new(x, y, z));
				min = min.min(point);
				max = max.max(point);
			}
		}
	}
	Aabb3d { min: min.into(), max: max.into() }
}

fn transform_aabb_affine(aabb: Aabb3d, inverse: bevy::math::Affine3A) -> Aabb3d {
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for x in [aabb.min.x, aabb.max.x] {
		for y in [aabb.min.y, aabb.max.y] {
			for z in [aabb.min.z, aabb.max.z] {
				let point = inverse.transform_point3(Vec3::new(x, y, z));
				min = min.min(point);
				max = max.max(point);
			}
		}
	}
	Aabb3d { min: min.into(), max: max.into() }
}

fn union_aabb(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d {
		min: Vec3::from(a.min).min(Vec3::from(b.min)).into(),
		max: Vec3::from(a.max).max(Vec3::from(b.max)).into(),
	}
}

pub(crate) fn stamp_layers_on_descendants(
	commands: &mut Commands,
	root: Entity,
	layer: usize,
	children: &Query<&Children>,
	existing: &Query<(), With<RenderLayers>>,
) {
	let wanted = RenderLayers::layer(layer);
	visit_descendants(root, children, &mut |entity| {
		if existing.get(entity).is_err() {
			commands.entity(entity).try_insert(wanted.clone());
		}
	});
}

pub(crate) fn scene_ready(
	session: &CaptureSession,
	world_roots: &Query<&WorldAssetRoot>,
	mesh_entities: &Query<Entity, With<Mesh3d>>,
	children: &Query<&Children>,
	hosts: &Query<(), With<CaptureHostReady>>,
) -> bool {
	match &session.request.scene {
		CaptureScene::Mesh(_) => mesh_entities.get(session.root).is_ok(),
		CaptureScene::SceneRef(_) => {
			if world_roots.get(session.root).is_err() {
				return false;
			}
			descendant_has_mesh(session.root, children, mesh_entities)
		}
		CaptureScene::Host { .. } => {
			hosts.get(session.root).is_ok()
				&& descendant_has_mesh(session.root, children, mesh_entities)
		}
	}
}

fn descendant_has_mesh(
	root: Entity,
	children: &Query<&Children>,
	mesh_entities: &Query<Entity, With<Mesh3d>>,
) -> bool {
	let mut found = false;
	visit_descendants(root, children, &mut |entity| {
		if mesh_entities.get(entity).is_ok() {
			found = true;
		}
	});
	found
}

pub(crate) fn apply_silhouette(
	commands: &mut Commands,
	session: &CaptureSession,
	materials: &mut Assets<StandardMaterial>,
	children: &Query<&Children>,
	standard: &Query<&MeshMaterial3d<StandardMaterial>>,
	meshes: &Query<Entity, With<Mesh3d>>,
) -> Result<Option<CaptureWarning>, CaptureError> {
	if session.request.mode != CaptureMode::Silhouette {
		return Ok(None);
	}
	let mut warning = None;
	let mut converted = 0_u32;
	let mut custom = 0_u32;
	visit_descendants(session.root, children, &mut |entity| {
		if meshes.get(entity).is_err() {
			return;
		}
		let original = standard.get(entity).ok().and_then(|handle| materials.get(&handle.0));
		if original.is_some_and(is_silhouette_material) {
			converted += 1;
			return;
		}
		let silhouette = match original {
			Some(original) => silhouette_from_standard(original),
			None => {
				custom += 1;
				opaque_silhouette()
			}
		};
		commands.entity(entity).try_insert(MeshMaterial3d(materials.add(silhouette)));
		converted += 1;
	});
	if converted == 0 {
		return Err(CaptureError::UnsupportedCustomMaterial {
			reason: String::from("silhouette scene has no Mesh3d outline"),
		});
	}
	if custom > 0 {
		warning = Some(CaptureWarning {
			key: session.key.clone(),
			warning: CaptureWarningKind::UnsupportedCustomMaterial {
				reason: format!("{custom} mesh(es) had no StandardMaterial; used unlit gray"),
			},
		});
	}
	Ok(warning)
}

fn silhouette_from_standard(original: &StandardMaterial) -> StandardMaterial {
	StandardMaterial {
		base_color: Color::srgba(SILHOUETTE_GRAY, SILHOUETTE_GRAY, SILHOUETTE_GRAY, 1.0),
		unlit: true,
		alpha_mode: original.alpha_mode,
		base_color_texture: original.base_color_texture.clone(),
		perceptual_roughness: 1.0,
		metallic: 0.0,
		reflectance: 0.0,
		..Default::default()
	}
}

fn opaque_silhouette() -> StandardMaterial {
	StandardMaterial {
		base_color: Color::srgba(SILHOUETTE_GRAY, SILHOUETTE_GRAY, SILHOUETTE_GRAY, 1.0),
		unlit: true,
		alpha_mode: AlphaMode::Opaque,
		perceptual_roughness: 1.0,
		metallic: 0.0,
		reflectance: 0.0,
		..Default::default()
	}
}

fn is_silhouette_material(material: &StandardMaterial) -> bool {
	material.unlit
		&& material.base_color
			== Color::srgba(SILHOUETTE_GRAY, SILHOUETTE_GRAY, SILHOUETTE_GRAY, 1.0)
}

fn visit_descendants(root: Entity, children: &Query<&Children>, visit: &mut impl FnMut(Entity)) {
	visit(root);
	let Ok(kids) = children.get(root) else {
		return;
	};
	for child in kids.iter() {
		visit_descendants(child, children, visit);
	}
}

pub(crate) fn spawn_readback(commands: &mut Commands, session: &mut CaptureSession) {
	let render = render_resolution(&session.request);
	let entity = commands
		.spawn((
			Readback::texture(session.target.clone()),
			CaptureReadback {
				key: session.key.clone(),
				ticket: session.ticket.clone(),
				width: session.request.resolution.width,
				height: session.request.resolution.height,
				render_width: render.width,
				render_height: render.height,
				crop: crop_padding(session.request.framing),
			},
			CaptureRoot,
			Name::new("capture_readback"),
		))
		.observe(on_readback_complete)
		.id();
	session.readback = Some(entity);
}

fn on_readback_complete(
	event: On<ReadbackComplete>,
	mut commands: Commands,
	readbacks: Query<&CaptureReadback>,
) {
	let completed = event.event();
	let entity = completed.entity;
	let Ok(_) = readbacks.get(entity) else {
		return;
	};
	commands.entity(entity).try_insert(CapturePixels(completed.data.clone()));
}

pub(crate) fn take_pixels(
	images: &mut Assets<Image>,
	readback: &CaptureReadback,
	bgra: &[u8],
) -> Result<Vec<u8>, CaptureError> {
	let (width, height) = (readback.width, readback.height);
	let rendered = bgra_to_rgba(bgra, readback.render_width, readback.render_height)
		.map_err(|_| CaptureError::Readback)?;
	let rgba = match readback.crop {
		Some(padding) => crop_to_alpha(
			&rendered,
			readback.render_width,
			readback.render_height,
			width,
			height,
			padding,
		),
		None => rendered,
	};
	let ticket = &readback.ticket;
	let image = Image::new(
		bevy::render::render_resource::Extent3d { width, height, depth_or_array_layers: 1 },
		bevy::render::render_resource::TextureDimension::D2,
		rgba.clone(),
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::default(),
	);
	let _ = images.insert(ticket.id(), image);
	Ok(rgba)
}

pub(crate) fn despawn_session(commands: &mut Commands, session: &CaptureSession) {
	for entity in session.entities() {
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn silhouette_keeps_mask_alpha() -> anyhow::Result<()> {
		let original = StandardMaterial {
			alpha_mode: AlphaMode::Mask(0.5),
			base_color: Color::srgb(1.0, 0.0, 0.0),
			..Default::default()
		};
		let silhouette = silhouette_from_standard(&original);
		assert!(silhouette.unlit);
		assert_eq!(silhouette.alpha_mode, AlphaMode::Mask(0.5));
		Ok(())
	}

	#[test]
	fn unlit_gray_covers_meshes_without_a_standard_material() {
		let silhouette = opaque_silhouette();
		assert!(silhouette.unlit);
		assert_eq!(silhouette.alpha_mode, AlphaMode::Opaque);
		assert_eq!(
			silhouette.base_color,
			Color::srgba(SILHOUETTE_GRAY, SILHOUETTE_GRAY, SILHOUETTE_GRAY, 1.0)
		);
		assert!(is_silhouette_material(&silhouette));
	}
}
