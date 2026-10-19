//! In-flight request queue with deduplication.

use bevy::ecs::message::MessageWriter;
use bevy::prelude::{Assets, Handle, Image};
use std::collections::{HashMap, VecDeque};

use crate::cache::{add_cached_image, CaptureCache};
use crate::error::{CaptureCompleted, CaptureError, CaptureFailed, CaptureSource};
use crate::key::CaptureCacheKey;
use crate::request::CaptureRequest;

/// Handle returned immediately; filled from cache or a later capture.
#[derive(Clone, Debug)]
pub struct CaptureTicket {
	pub key: CaptureCacheKey,
	pub image: Handle<Image>,
	pub source: Option<CaptureSource>,
}

/// Immediate enqueue result. Pending and in-flight work carry no notification.
#[derive(Clone, Debug)]
pub struct CaptureRequestOutcome {
	pub ticket: CaptureTicket,
	pub notification: Option<CaptureRequestNotification>,
}

/// At most one completion or failure is produced per [`CaptureQueue::request`].
#[derive(Clone, Debug)]
pub enum CaptureRequestNotification {
	Completed(CaptureCompleted),
	Failed(CaptureFailed),
}

impl CaptureRequestOutcome {
	/// Forward the optional notification and return the placeholder ticket.
	pub fn forward(
		self,
		completed: &mut MessageWriter<CaptureCompleted>,
		failed: &mut MessageWriter<CaptureFailed>,
	) -> CaptureTicket {
		match self.notification {
			Some(CaptureRequestNotification::Completed(message)) => {
				completed.write(message);
			}
			Some(CaptureRequestNotification::Failed(message)) => {
				failed.write(message);
			}
			None => {}
		}
		self.ticket
	}

	fn completed(ticket: CaptureTicket, completed: CaptureCompleted) -> Self {
		Self { ticket, notification: Some(CaptureRequestNotification::Completed(completed)) }
	}

	fn failed(ticket: CaptureTicket, failed: CaptureFailed) -> Self {
		Self { ticket, notification: Some(CaptureRequestNotification::Failed(failed)) }
	}

	fn pending(ticket: CaptureTicket) -> Self {
		Self { ticket, notification: None }
	}
}

#[derive(Clone, Debug)]
pub(crate) struct QueuedCapture {
	pub request: CaptureRequest,
	pub image: Handle<Image>,
}

/// Deduped capture work. Cache hits never enter [`Self::pending`].
#[derive(bevy::prelude::Resource, Default)]
pub struct CaptureQueue {
	pub(crate) pending: VecDeque<QueuedCapture>,
	tickets: HashMap<CaptureCacheKey, Handle<Image>>,
}

impl CaptureQueue {
	pub fn request(
		&mut self,
		cache: &CaptureCache,
		images: &mut Assets<Image>,
		request: CaptureRequest,
	) -> CaptureRequestOutcome {
		if let Err(reason) = request.validate() {
			let key = CaptureCacheKey::from_request(&request);
			let ticket = CaptureTicket {
				key: key.clone(),
				image: images.add(crate::png::placeholder_image(request.resolution)),
				source: None,
			};
			return CaptureRequestOutcome::failed(
				ticket,
				CaptureFailed { key, error: CaptureError::InvalidRequest { reason } },
			);
		}

		let key = CaptureCacheKey::from_request(&request);
		if let Some(image) = self.tickets.get(&key).cloned() {
			return CaptureRequestOutcome::pending(CaptureTicket {
				key,
				image,
				source: Some(CaptureSource::Cache),
			});
		}

		if let Some(cached) = cache.load(&key) {
			let image = add_cached_image(images, cached);
			self.tickets.insert(key.clone(), image.clone());
			let completed = CaptureCompleted {
				key: key.clone(),
				image: image.clone(),
				source: CaptureSource::Cache,
			};
			return CaptureRequestOutcome::completed(
				CaptureTicket { key, image, source: Some(CaptureSource::Cache) },
				completed,
			);
		}

		let image = images.add(crate::png::placeholder_image(request.resolution));
		self.tickets.insert(key.clone(), image.clone());
		self.pending.push_back(QueuedCapture { request, image: image.clone() });
		CaptureRequestOutcome::pending(CaptureTicket { key, image, source: None })
	}

	pub fn remembered(&self, key: &CaptureCacheKey) -> Option<Handle<Image>> {
		self.tickets.get(key).cloned()
	}

	pub fn forget(&mut self, key: &CaptureCacheKey) {
		self.tickets.remove(key);
	}

	pub fn pending_len(&self) -> usize {
		self.pending.len()
	}

	pub(crate) fn pop_pending(&mut self) -> Option<QueuedCapture> {
		self.pending.pop_front()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::subject::MeshCaptureSubject;
	use crate::{CaptureLighting, CaptureMode, CaptureResolution, CaptureRevision};
	use bevy::prelude::Assets;

	fn request(mode: CaptureMode) -> CaptureRequest {
		CaptureRequest::from_subject(
			&MeshCaptureSubject::cuboid([0.9, 0.1, 0.1, 1.0]),
			mode,
			CaptureResolution::ICON,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		)
	}

	fn assert_no_notification(outcome: &CaptureRequestOutcome) {
		assert!(outcome.notification.is_none());
	}

	#[test]
	fn matching_requests_dedupe() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let cache = CaptureCache::new(dir.path());
		let mut images = Assets::<Image>::default();
		let mut queue = CaptureQueue::default();
		let first = queue.request(&cache, &mut images, request(CaptureMode::Shaded));
		let second = queue.request(&cache, &mut images, request(CaptureMode::Shaded));
		assert_eq!(first.ticket.image, second.ticket.image);
		assert_no_notification(&first);
		assert_no_notification(&second);
		assert_eq!(queue.pending_len(), 1);
		Ok(())
	}

	#[test]
	fn cache_hit_skips_pending() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let cache = CaptureCache::new(dir.path());
		let req = request(CaptureMode::Silhouette);
		let key = CaptureCacheKey::from_request(&req);
		let rgba = vec![200_u8, 200, 200, 255];
		cache
			.save_rgba_blocking(&key, 1, 1, &rgba)
			.map_err(|error| anyhow::anyhow!(error))?;

		let mut images = Assets::<Image>::default();
		let mut queue = CaptureQueue::default();
		let outcome = queue.request(&cache, &mut images, req);
		assert_eq!(outcome.ticket.source, Some(CaptureSource::Cache));
		assert_eq!(queue.pending_len(), 0);
		match outcome.notification {
			Some(CaptureRequestNotification::Completed(completed)) => {
				assert_eq!(completed.source, CaptureSource::Cache);
				assert_eq!(completed.key, key);
			}
			other => anyhow::bail!("expected one cache completion, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn invalid_request_keeps_the_real_key_and_one_failure() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let cache = CaptureCache::new(dir.path());
		let mut images = Assets::<Image>::default();
		let mut queue = CaptureQueue::default();
		let mut req = request(CaptureMode::Shaded);
		req.id = crate::CaptureId::new("");
		let key = CaptureCacheKey::from_request(&req);
		let outcome = queue.request(&cache, &mut images, req);
		assert_eq!(outcome.ticket.key, key);
		assert!(outcome.ticket.source.is_none());
		assert_eq!(queue.pending_len(), 0);
		match outcome.notification {
			Some(CaptureRequestNotification::Failed(failed)) => {
				assert_eq!(failed.key, key);
				assert!(matches!(failed.error, CaptureError::InvalidRequest { .. }));
			}
			other => anyhow::bail!("expected one invalid-request failure, got {other:?}"),
		}
		Ok(())
	}
}
