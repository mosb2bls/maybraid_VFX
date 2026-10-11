//! In-flight request queue with deduplication.

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
	) -> Result<(CaptureTicket, Option<CaptureCompleted>, Option<CaptureFailed>), CaptureError> {
		if let Err(reason) = request.validate() {
			let key = CaptureCacheKey::from_request(&request);
			return Ok((
				CaptureTicket {
					key: key.clone(),
					image: images.add(crate::png::placeholder_image(request.resolution)),
					source: None,
				},
				None,
				Some(CaptureFailed { key, error: CaptureError::InvalidRequest { reason } }),
			));
		}

		let key = CaptureCacheKey::from_request(&request);
		if let Some(image) = self.tickets.get(&key).cloned() {
			return Ok((
				CaptureTicket { key: key.clone(), image, source: Some(CaptureSource::Cache) },
				None,
				None,
			));
		}

		if let Some(cached) = cache.load(&key) {
			let image = add_cached_image(images, cached);
			self.tickets.insert(key.clone(), image.clone());
			let completed = CaptureCompleted {
				key: key.clone(),
				image: image.clone(),
				source: CaptureSource::Cache,
			};
			return Ok((
				CaptureTicket { key, image, source: Some(CaptureSource::Cache) },
				Some(completed),
				None,
			));
		}

		let image = images.add(crate::png::placeholder_image(request.resolution));
		self.tickets.insert(key.clone(), image.clone());
		self.pending.push_back(QueuedCapture { request, image: image.clone() });
		Ok((CaptureTicket { key, image, source: None }, None, None))
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

	#[test]
	fn matching_requests_dedupe() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let cache = CaptureCache::new(dir.path());
		let mut images = Assets::<Image>::default();
		let mut queue = CaptureQueue::default();
		let first = queue.request(&cache, &mut images, request(CaptureMode::Shaded))?;
		let second = queue.request(&cache, &mut images, request(CaptureMode::Shaded))?;
		assert_eq!(first.0.image, second.0.image);
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
		let (ticket, completed, failed) = queue.request(&cache, &mut images, req)?;
		assert!(failed.is_none());
		let completed = completed.ok_or_else(|| anyhow::anyhow!("cache complete"))?;
		assert_eq!(completed.source, CaptureSource::Cache);
		assert_eq!(ticket.source, Some(CaptureSource::Cache));
		assert_eq!(queue.pending_len(), 0);
		Ok(())
	}
}
