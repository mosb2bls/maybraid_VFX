//! Plugin, resources, and capture systems.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::gpu_readback::GpuReadbackPlugin;
use scene_ref::SceneRefPlugin;
use std::path::PathBuf;

use crate::cache::CaptureCache;
use crate::error::{
	CaptureCompleted, CaptureFailed, CaptureSource, CaptureWarning, RequestCapture,
};
use crate::queue::CaptureQueue;
use crate::session::{
	apply_silhouette, despawn_session, scene_ready, spawn_readback, spawn_session,
	stamp_layers_on_descendants, take_pixels, CaptureHostReady, CapturePhase, CapturePixels,
	CaptureReadback, CaptureSession, CAPTURE_RENDER_LAYER,
};

/// Isolated capture schedule. Hosts realize posed subjects before [`Self::Advance`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenCaptureSystems {
	Isolate,
	Advance,
}

/// Writable cache directory and isolated-scene settings.
#[derive(Resource, Clone, Debug)]
pub struct CaptureCacheRoot {
	pub cache: CaptureCache,
	pub render_layer: usize,
	pub max_wait_frames: u32,
	pub settle_frames: u32,
}

impl CaptureCacheRoot {
	pub fn new(path: impl Into<PathBuf>) -> Self {
		Self {
			cache: CaptureCache::new(path),
			render_layer: CAPTURE_RENDER_LAYER,
			max_wait_frames: 180,
			settle_frames: 3,
		}
	}

	pub fn with_render_layer(mut self, layer: usize) -> Self {
		self.render_layer = layer;
		self
	}
}

#[derive(Resource, Default)]
pub(crate) struct ActiveCapture {
	session: Option<CaptureSession>,
	next_slot: u32,
}

/// Isolated scene capture with a persistent PNG cache.
pub struct ScreenCapturePlugin {
	pub cache_dir: PathBuf,
	pub render_layer: usize,
}

impl ScreenCapturePlugin {
	pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
		Self { cache_dir: cache_dir.into(), render_layer: CAPTURE_RENDER_LAYER }
	}

	pub fn with_render_layer(mut self, layer: usize) -> Self {
		self.render_layer = layer;
		self
	}
}

impl Plugin for ScreenCapturePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.init_asset::<bevy::world_serialization::WorldAsset>();
			app.init_resource::<bevy::ecs::reflect::AppTypeRegistry>();
			app.add_plugins(SceneRefPlugin);
		}
		if app.get_sub_app(bevy::render::RenderApp).is_some()
			&& !app.is_plugin_added::<GpuReadbackPlugin>()
		{
			app.add_plugins(GpuReadbackPlugin::default());
		}
		app.insert_resource(
			CaptureCacheRoot::new(self.cache_dir.clone()).with_render_layer(self.render_layer),
		)
		.init_resource::<CaptureQueue>()
		.init_resource::<ActiveCapture>()
		.add_message::<RequestCapture>()
		.add_message::<CaptureCompleted>()
		.add_message::<CaptureFailed>()
		.add_message::<CaptureWarning>()
		.configure_sets(
			PostUpdate,
			ScreenCaptureSystems::Advance.after(ScreenCaptureSystems::Isolate),
		)
		.add_systems(Update, ingest_requests)
		.add_systems(PostUpdate, stamp_isolate_layers.in_set(ScreenCaptureSystems::Isolate))
		.add_systems(PostUpdate, advance_sessions.in_set(ScreenCaptureSystems::Advance));
	}
}

/// SystemParam used by applications to enqueue captures.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CaptureClient<'w> {
	pub cache: Res<'w, CaptureCacheRoot>,
	pub queue: ResMut<'w, CaptureQueue>,
	pub images: ResMut<'w, Assets<Image>>,
	pub completed: MessageWriter<'w, CaptureCompleted>,
	pub failed: MessageWriter<'w, CaptureFailed>,
}

impl CaptureClient<'_> {
	pub fn request(&mut self, request: crate::CaptureRequest) -> crate::CaptureTicket {
		match self.queue.request(&self.cache.cache, &mut self.images, request) {
			Ok((ticket, completed, failed)) => {
				if let Some(completed) = completed {
					self.completed.write(completed);
				}
				if let Some(failed) = failed {
					self.failed.write(failed);
				}
				ticket
			}
			Err(_) => crate::CaptureTicket {
				key: crate::CaptureCacheKey::from_request(&crate::CaptureRequest::from_subject(
					&crate::MeshCaptureSubject::cuboid([0.0, 0.0, 0.0, 0.0]),
					crate::CaptureMode::Shaded,
					crate::CaptureResolution::ICON,
					crate::CaptureLighting::studio(),
					crate::CaptureRevision::CURRENT,
				)),
				image: self
					.images
					.add(crate::png::placeholder_image(crate::CaptureResolution::ICON)),
				source: None,
			},
		}
	}

	pub fn cached(&self, key: &crate::CaptureCacheKey) -> Option<Handle<Image>> {
		self.queue.remembered(key)
	}
}

fn ingest_requests(
	mut messages: MessageReader<RequestCapture>,
	cache: Res<CaptureCacheRoot>,
	mut queue: ResMut<CaptureQueue>,
	mut images: ResMut<Assets<Image>>,
	mut completed: MessageWriter<CaptureCompleted>,
	mut failed: MessageWriter<CaptureFailed>,
) {
	for RequestCapture(request) in messages.read() {
		match queue.request(&cache.cache, &mut images, request.clone()) {
			Ok((.., done, fail)) => {
				if let Some(done) = done {
					completed.write(done);
				}
				if let Some(fail) = fail {
					failed.write(fail);
				}
			}
			Err(error) => {
				failed.write(CaptureFailed {
					key: crate::CaptureCacheKey::from_request(request),
					error,
				});
			}
		}
	}
}

fn stamp_isolate_layers(
	mut commands: Commands,
	active: Res<ActiveCapture>,
	cache: Res<CaptureCacheRoot>,
	children: Query<&Children>,
	layered: Query<(), With<RenderLayers>>,
) {
	let Some(session) = active.session.as_ref() else {
		return;
	};
	stamp_layers_on_descendants(
		&mut commands,
		session.root,
		cache.render_layer,
		&children,
		&layered,
	);
}

fn advance_sessions(
	mut commands: Commands,
	cache: Res<CaptureCacheRoot>,
	mut queue: ResMut<CaptureQueue>,
	mut active: ResMut<ActiveCapture>,
	mut images: ResMut<Assets<Image>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	world_roots: Query<&WorldAssetRoot>,
	mesh_entities: Query<Entity, With<Mesh3d>>,
	children: Query<&Children>,
	hosts: Query<(), With<CaptureHostReady>>,
	standard: Query<&MeshMaterial3d<StandardMaterial>>,
	pixels: Query<(Entity, &CaptureReadback, &CapturePixels)>,
	mut completed: MessageWriter<CaptureCompleted>,
	mut failed: MessageWriter<CaptureFailed>,
	mut warnings: MessageWriter<CaptureWarning>,
) {
	if active.session.is_none() {
		if let Some(queued) = queue.pop_pending() {
			let slot = active.next_slot;
			active.next_slot = active.next_slot.saturating_add(1);
			active.session = Some(spawn_session(
				&mut commands,
				&mut images,
				&mut meshes,
				&mut materials,
				queued,
				slot,
				cache.render_layer,
			));
		}
	}

	let Some(session) = active.session.as_mut() else {
		return;
	};

	if let Some((entity, readback, captured)) = pixels.iter().next() {
		match take_pixels(
			&mut images,
			&readback.ticket,
			readback.width,
			readback.height,
			&captured.0,
		) {
			Ok(rgba) => {
				let mut write_warnings = Vec::new();
				cache.cache.save_rgba_background(
					readback.key.clone(),
					readback.width,
					readback.height,
					rgba,
					&mut write_warnings,
				);
				for warning in write_warnings {
					warnings.write(warning);
				}
				completed.write(CaptureCompleted {
					key: readback.key.clone(),
					image: readback.ticket.clone(),
					source: CaptureSource::Rendered,
				});
			}
			Err(error) => {
				failed.write(CaptureFailed { key: readback.key.clone(), error });
			}
		}
		commands.entity(entity).try_despawn();
		despawn_session(&mut commands, session);
		active.session = None;
		return;
	}

	match &mut session.phase {
		CapturePhase::Load { frames } => {
			*frames += 1;
			if *frames > cache.max_wait_frames {
				fail_session(
					&mut commands,
					session,
					&mut failed,
					crate::error::CaptureError::SceneNotReady,
				);
				active.session = None;
				return;
			}
			if scene_ready(session, &world_roots, &mesh_entities, &children, &hosts) {
				match apply_silhouette(
					&mut commands,
					session,
					&mut materials,
					&children,
					&standard,
					&mesh_entities,
				) {
					Ok(warning) => {
						if let Some(warning) = warning {
							warnings.write(warning);
						}
						session.phase = CapturePhase::Settle { remaining: cache.settle_frames };
					}
					Err(error) => {
						fail_session(&mut commands, session, &mut failed, error);
						active.session = None;
					}
				}
			}
		}
		CapturePhase::Settle { remaining } => {
			*remaining = remaining.saturating_sub(1);
			if *remaining == 0 {
				spawn_readback(&mut commands, session);
				session.phase = CapturePhase::Readback { frames: 0 };
			}
		}
		CapturePhase::Readback { frames } => {
			*frames += 1;
			if *frames > cache.max_wait_frames {
				fail_session(
					&mut commands,
					session,
					&mut failed,
					crate::error::CaptureError::Readback,
				);
				active.session = None;
			}
		}
	}
}

fn fail_session(
	commands: &mut Commands,
	session: &CaptureSession,
	failed: &mut MessageWriter<CaptureFailed>,
	error: crate::error::CaptureError,
) {
	failed.write(CaptureFailed { key: session.key.clone(), error });
	despawn_session(commands, session);
}
