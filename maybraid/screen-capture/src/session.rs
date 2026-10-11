//! Isolated capture scene, readiness, silhouette, and GPU readback.

use bevy::asset::{Assets, RenderAssetUsages};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::TextureFormat;
use bevy::world_serialization::WorldAssetRoot;
use scene_ref::SceneRefRoot;

use crate::error::{CaptureError, CaptureWarning, CaptureWarningKind};
use crate::key::CaptureCacheKey;
use crate::mode::CaptureMode;
use crate::png::{bgra_to_rgba, render_target_image};
use crate::queue::QueuedCapture;
use crate::request::CaptureRequest;
use crate::subject::{CaptureMeshKind, CaptureScene};
use crate::view::CaptureProjection;

/// Render layer reserved for isolated capture cameras and subjects.
pub const CAPTURE_RENDER_LAYER: usize = 31;

const SLOT_SPACING: f32 = 24.0;
const SLOT_ORIGIN_X: f32 = 8_000.0;
const SILHOUETTE_GRAY: f32 = 0.72;

#[derive(Component)]
pub(crate) struct CaptureRoot;

#[derive(Component)]
pub(crate) struct CaptureCamera;

#[derive(Component)]
pub(crate) struct CaptureReadback {
	pub key: CaptureCacheKey,
	pub ticket: Handle<Image>,
	pub width: u32,
	pub height: u32,
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
	let key = CaptureCacheKey::from_request(&request);
	let origin = Vec3::new(SLOT_ORIGIN_X + slot as f32 * SLOT_SPACING, 0.0, 0.0);
	let layers = RenderLayers::layer(layer);
	let target = images.add(render_target_image(request.resolution));
	let root = spawn_subject(commands, meshes, materials, &request.scene, origin, layers.clone());
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
	}
}

fn spawn_subject(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	scene: &CaptureScene,
	origin: Vec3,
	layers: RenderLayers,
) -> Entity {
	let transform = Transform::from_translation(origin);
	match scene {
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
	}
}

fn spawn_camera(
	commands: &mut Commands,
	request: &CaptureRequest,
	origin: Vec3,
	target: Handle<Image>,
	layers: RenderLayers,
) -> Entity {
	let view = request.view;
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
) -> bool {
	match &session.request.scene {
		CaptureScene::Mesh(_) => mesh_entities.get(session.root).is_ok(),
		CaptureScene::SceneRef(_) => {
			if world_roots.get(session.root).is_err() {
				return false;
			}
			let mut found = false;
			visit_descendants(session.root, children, &mut |entity| {
				if mesh_entities.get(entity).is_ok() {
					found = true;
				}
			});
			found || matches!(session.phase, CapturePhase::Load { frames } if frames > 8)
		}
	}
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
		let Ok(handle) = standard.get(entity) else {
			custom += 1;
			return;
		};
		let Some(original) = materials.get(&handle.0).cloned() else {
			custom += 1;
			return;
		};
		let silhouette = silhouette_from_standard(&original);
		commands.entity(entity).insert(MeshMaterial3d(materials.add(silhouette)));
		converted += 1;
	});
	if converted == 0 && custom > 0 {
		return Err(CaptureError::UnsupportedCustomMaterial {
			reason: format!("{custom} mesh(es) have no StandardMaterial outline"),
		});
	}
	if custom > 0 {
		warning = Some(CaptureWarning {
			key: session.key.clone(),
			warning: CaptureWarningKind::UnsupportedCustomMaterial {
				reason: format!("{custom} custom mesh(es) skipped for silhouette"),
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
	let entity = commands
		.spawn((
			Readback::texture(session.target.clone()),
			CaptureReadback {
				key: session.key.clone(),
				ticket: session.ticket.clone(),
				width: session.request.resolution.width,
				height: session.request.resolution.height,
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
	commands.entity(entity).insert(CapturePixels(completed.data.clone()));
}

pub(crate) fn take_pixels(
	images: &mut Assets<Image>,
	ticket: &Handle<Image>,
	width: u32,
	height: u32,
	bgra: &[u8],
) -> Result<Vec<u8>, CaptureError> {
	let rgba = bgra_to_rgba(bgra, width, height).map_err(|_| CaptureError::Readback)?;
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
}
