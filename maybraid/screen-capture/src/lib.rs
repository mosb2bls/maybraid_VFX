//! Isolated Bevy scene capture with a persistent PNG cache.
//!
//! Parallel to [`scene_ref`] and [`material_ref`]: a deferred identity
//! ([`CaptureSubject`] / [`CaptureRequest`]), a plugin that fulfills work
//! ([`ScreenCapturePlugin`]), and a disk-backed handle cache.

mod cache;
mod crop;
mod error;
mod id;
mod key;
mod lighting;
mod mode;
mod plugin;
mod png;
mod pose;
mod queue;
mod request;
mod session;
mod subject;
mod view;

pub use cache::CaptureCache;
pub use error::{
	CaptureCompleted, CaptureError, CaptureFailed, CaptureSource, CaptureWarning,
	CaptureWarningKind, RequestCapture,
};
pub use id::{sanitize_file_stem, CaptureId};
pub use key::CaptureCacheKey;
pub use lighting::{CaptureDirectionalLight, CaptureLighting, CaptureLightingPreset};
pub use mode::CaptureMode;
pub use plugin::{CaptureCacheRoot, CaptureClient, ScreenCapturePlugin, ScreenCaptureSystems};
pub use pose::{
	pose_capture_label, representative_pose_view, RepresentativePose, HOST_REPRESENTATIVE_POSE,
};
pub use queue::{CaptureRequestNotification, CaptureRequestOutcome, CaptureTicket};
pub use request::{CaptureRequest, CaptureResolution, CaptureRevision};
pub use session::{
	CaptureExtents, CaptureHostReady, CaptureHostRoot, CaptureSilhouetteRoot, CAPTURE_RENDER_LAYER,
};
pub use subject::{CaptureMesh, CaptureMeshKind, CaptureScene, CaptureSubject, MeshCaptureSubject};
pub use view::{CaptureFraming, CaptureProjection, CaptureView};

pub use png::{bgra_to_rgba, encode_rgba_png, write_all_atomic};
pub use scene_ref::{SceneRef, SceneRefPlugin};

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::*;

	fn plugin_app(dir: &std::path::Path) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Image>()
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.add_plugins(ScreenCapturePlugin::new(dir));
		app
	}

	#[test]
	fn cache_hit_completes_without_a_scene() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let subject = MeshCaptureSubject::cuboid([0.2, 0.5, 0.9, 1.0]);
		let request = CaptureRequest::from_subject(
			&subject,
			CaptureMode::Shaded,
			CaptureResolution::new(2, 2),
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		let key = CaptureCacheKey::from_request(&request);
		let cache = CaptureCache::new(dir.path());
		let rgba = vec![10_u8, 20, 30, 255].repeat(4);
		cache
			.save_rgba_blocking(&key, 2, 2, &rgba)
			.map_err(|error| anyhow::anyhow!(error))?;

		let mut app = plugin_app(dir.path());
		app.world_mut().write_message(RequestCapture(request));
		app.update();

		let completed = app
			.world_mut()
			.resource_mut::<Messages<CaptureCompleted>>()
			.drain()
			.collect::<Vec<_>>();
		assert_eq!(completed.len(), 1);
		assert_eq!(completed[0].source, CaptureSource::Cache);
		assert_eq!(completed[0].key, key);
		assert_eq!(app.world().resource::<crate::queue::CaptureQueue>().pending_len(), 0);
		Ok(())
	}

	#[test]
	fn revision_change_is_a_new_cache_entry() -> anyhow::Result<()> {
		let subject = MeshCaptureSubject::cuboid([0.4, 0.4, 0.4, 1.0]);
		let a = CaptureRequest::from_subject(
			&subject,
			CaptureMode::Silhouette,
			CaptureResolution::ICON,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		let b = CaptureRequest::from_subject(
			&subject,
			CaptureMode::Silhouette,
			CaptureResolution::ICON,
			CaptureLighting::studio(),
			CaptureRevision::new(CaptureRevision::CURRENT.0.wrapping_add(1)),
		);
		assert_ne!(CaptureCacheKey::from_request(&a), CaptureCacheKey::from_request(&b));
		Ok(())
	}

	#[test]
	fn isolated_spawn_does_not_touch_live_materials() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let mut app = plugin_app(dir.path());
		let live_mesh = app
			.world_mut()
			.resource_mut::<Assets<Mesh>>()
			.add(Mesh::from(Cuboid::from_length(1.0)));
		let live_mat = app
			.world_mut()
			.resource_mut::<Assets<StandardMaterial>>()
			.add(StandardMaterial { base_color: Color::srgb(1.0, 0.0, 0.0), ..Default::default() });
		let live = app
			.world_mut()
			.spawn((Mesh3d(live_mesh), MeshMaterial3d(live_mat.clone()), Transform::IDENTITY))
			.id();

		let request = CaptureRequest::from_subject(
			&MeshCaptureSubject::cuboid([0.0, 1.0, 0.0, 1.0]),
			CaptureMode::Shaded,
			CaptureResolution::new(8, 8),
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		app.world_mut().write_message(RequestCapture(request));
		app.update();
		app.update();

		let live_handle = app
			.world()
			.get::<MeshMaterial3d<StandardMaterial>>(live)
			.ok_or_else(|| anyhow::anyhow!("live material"))?
			.0
			.clone();
		assert_eq!(live_handle, live_mat);
		let live_color = app
			.world()
			.resource::<Assets<StandardMaterial>>()
			.get(&live_handle)
			.ok_or_else(|| anyhow::anyhow!("material asset"))?
			.base_color;
		assert_eq!(live_color, Color::srgb(1.0, 0.0, 0.0));
		Ok(())
	}

	#[test]
	fn invalid_request_forwards_one_failure_with_the_real_key() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let mut app = plugin_app(dir.path());
		let mut request = CaptureRequest::from_subject(
			&MeshCaptureSubject::cuboid([0.1, 0.2, 0.3, 1.0]),
			CaptureMode::Shaded,
			CaptureResolution::ICON,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		request.id = CaptureId::new("");
		let key = CaptureCacheKey::from_request(&request);
		app.world_mut().write_message(RequestCapture(request));
		app.update();

		let completed = app
			.world_mut()
			.resource_mut::<Messages<CaptureCompleted>>()
			.drain()
			.collect::<Vec<_>>();
		let failed = app
			.world_mut()
			.resource_mut::<Messages<CaptureFailed>>()
			.drain()
			.collect::<Vec<_>>();
		assert!(completed.is_empty());
		assert_eq!(failed.len(), 1);
		assert_eq!(failed[0].key, key);
		assert!(matches!(failed[0].error, CaptureError::InvalidRequest { .. }));
		assert_eq!(app.world().resource::<crate::queue::CaptureQueue>().pending_len(), 0);
		Ok(())
	}

	#[test]
	fn pending_and_in_flight_requests_forward_no_notification() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let mut app = plugin_app(dir.path());
		for color in [[0.4, 0.5, 0.6, 1.0], [0.7, 0.1, 0.2, 1.0]] {
			app.world_mut().write_message(RequestCapture(CaptureRequest::from_subject(
				&MeshCaptureSubject::cuboid(color),
				CaptureMode::Shaded,
				CaptureResolution::new(8, 8),
				CaptureLighting::studio(),
				CaptureRevision::CURRENT,
			)));
		}
		app.update();

		let completed = app
			.world_mut()
			.resource_mut::<Messages<CaptureCompleted>>()
			.drain()
			.collect::<Vec<_>>();
		let failed = app
			.world_mut()
			.resource_mut::<Messages<CaptureFailed>>()
			.drain()
			.collect::<Vec<_>>();
		assert!(completed.is_empty());
		assert!(failed.is_empty());
		assert_eq!(app.world().resource::<crate::queue::CaptureQueue>().pending_len(), 1);
		Ok(())
	}
}
