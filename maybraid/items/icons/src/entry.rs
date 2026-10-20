//! Full-key icon entries. Identity is the capture cache key (subject, mode,
//! view/framing, revision). The shared capture queue still owns render and
//! dedup; this store only mirrors tickets and completion notifications.

use bevy::prelude::*;
use character_items::icon_catalog_key_for_label;
use screen_capture::{
	CaptureCacheKey, CaptureCompleted, CaptureFailed, CaptureId, CaptureMode, CaptureRevision,
	CaptureTicket,
};
use std::collections::HashMap;

use crate::ITEM_ICON_RESOLUTION;

/// Lifecycle of one full-key still.
///
/// Transitions:
/// - missing + ticket → [`Pending`](Self::Pending)
/// - [`Pending`](Self::Pending) + [`CaptureCompleted`] → [`Ready`](Self::Ready)
/// - [`Pending`](Self::Pending) + [`CaptureFailed`] → [`Failed`](Self::Failed) (terminal)
/// - [`Failed`](Self::Failed) + ticket → [`Pending`](Self::Pending) (retry)
/// - [`Ready`](Self::Ready) is sticky; a later failure or ticket does not regress it
/// - [`Failed`](Self::Failed) ignores a later completion for the same key (stale)
///
/// A completion whose key differs (old revision, other framing) writes that
/// other slot and leaves the current-revision entry untouched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconEntryStatus {
	Pending,
	Ready,
	Failed,
}

impl IconEntryStatus {
	pub fn is_terminal(self) -> bool {
		matches!(self, Self::Ready | Self::Failed)
	}

	fn rank(self) -> u8 {
		match self {
			Self::Ready => 2,
			Self::Failed => 1,
			Self::Pending => 0,
		}
	}
}

/// Capture handle plus status for one view/revision/framing still.
#[derive(Clone, Debug)]
pub struct IconEntry {
	key: CaptureCacheKey,
	image: Handle<Image>,
	status: IconEntryStatus,
}

impl IconEntry {
	fn pending(ticket: &CaptureTicket) -> Self {
		Self {
			key: ticket.key.clone(),
			image: ticket.image.clone(),
			status: IconEntryStatus::Pending,
		}
	}

	fn ready(key: CaptureCacheKey, image: Handle<Image>) -> Self {
		Self { key, image, status: IconEntryStatus::Ready }
	}

	fn failed(key: CaptureCacheKey, image: Handle<Image>) -> Self {
		Self { key, image, status: IconEntryStatus::Failed }
	}

	pub fn key(&self) -> &CaptureCacheKey {
		&self.key
	}

	/// Ticket or completed handle. Pending stills keep the queue placeholder.
	pub fn image(&self) -> &Handle<Image> {
		&self.image
	}

	pub fn status(&self) -> IconEntryStatus {
		self.status
	}

	pub fn is_terminal(&self) -> bool {
		self.status.is_terminal()
	}

	pub fn ready_image(&self) -> Option<Handle<Image>> {
		matches!(self.status, IconEntryStatus::Ready).then(|| self.image.clone())
	}
}

/// Counts for one all-icons barrier. Menu tiles rebuild when this changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IconCatalogProgress {
	pub ready: u32,
	pub failed: u32,
	pub pending: u32,
	pub missing: u32,
}

impl IconCatalogProgress {
	pub fn all_terminal(self) -> bool {
		self.pending == 0 && self.missing == 0
	}
}

/// Full-key icon store. Catalog and HUD helpers resolve the current revision
/// only, so a stale completion cannot satisfy a newer still.
#[derive(Resource, Default, Debug)]
pub struct ItemIconCache {
	entries: HashMap<CaptureCacheKey, IconEntry>,
}

impl ItemIconCache {
	pub fn entry(&self, key: &CaptureCacheKey) -> Option<&IconEntry> {
		self.entries.get(key)
	}

	/// Current-revision still for a catalog tile. Shaded and silhouette stay
	/// distinct; view/framing live on the entry key.
	pub fn catalog_entry(&self, catalog_key: u64, silhouette: bool) -> Option<&IconEntry> {
		self.current_entry(None, Some(catalog_key), mode_for(silhouette))
	}

	pub fn catalog_need_is_terminal(&self, catalog_key: u64, silhouette: bool) -> bool {
		self.catalog_entry(catalog_key, silhouette).is_some_and(IconEntry::is_terminal)
	}

	pub fn catalog_progress(
		&self,
		needs: impl IntoIterator<Item = (u64, bool)>,
	) -> IconCatalogProgress {
		let mut progress = IconCatalogProgress::default();
		for (catalog_key, silhouette) in needs {
			match self.catalog_entry(catalog_key, silhouette).map(IconEntry::status) {
				Some(IconEntryStatus::Ready) => progress.ready += 1,
				Some(IconEntryStatus::Failed) => progress.failed += 1,
				Some(IconEntryStatus::Pending) => progress.pending += 1,
				None => progress.missing += 1,
			}
		}
		progress
	}

	/// Record the immediate queue ticket. Failed entries become Pending again.
	pub fn remember_ticket(&mut self, ticket: &CaptureTicket) {
		match self.entries.get_mut(&ticket.key) {
			Some(entry) if entry.status == IconEntryStatus::Ready => {}
			Some(entry) => {
				entry.image = ticket.image.clone();
				entry.status = IconEntryStatus::Pending;
			}
			None => {
				self.entries.insert(ticket.key.clone(), IconEntry::pending(ticket));
			}
		}
	}

	pub fn apply_completed(&mut self, event: &CaptureCompleted) {
		match self.entries.get_mut(&event.key) {
			Some(entry) if entry.status == IconEntryStatus::Failed => {}
			Some(entry) => {
				entry.image = event.image.clone();
				entry.status = IconEntryStatus::Ready;
			}
			None => {
				self.entries.insert(
					event.key.clone(),
					IconEntry::ready(event.key.clone(), event.image.clone()),
				);
			}
		}
	}

	pub fn apply_failed(&mut self, event: &CaptureFailed) {
		match self.entries.get_mut(&event.key) {
			Some(entry) if entry.status == IconEntryStatus::Ready => {}
			Some(entry) => {
				entry.status = IconEntryStatus::Failed;
			}
			None => {
				self.entries.insert(
					event.key.clone(),
					IconEntry::failed(event.key.clone(), Handle::default()),
				);
			}
		}
	}

	pub fn shaded(&self, id: &CaptureId) -> Option<Handle<Image>> {
		self.current_entry(Some(id), None, CaptureMode::Shaded)
			.and_then(IconEntry::ready_image)
	}

	pub fn silhouette(&self, id: &CaptureId) -> Option<Handle<Image>> {
		self.current_entry(Some(id), None, CaptureMode::Silhouette)
			.and_then(IconEntry::ready_image)
	}

	pub fn shaded_for_key(&self, key: u64) -> Option<Handle<Image>> {
		self.catalog_entry(key, false).and_then(IconEntry::ready_image)
	}

	pub fn silhouette_for_key(&self, key: u64) -> Option<Handle<Image>> {
		self.catalog_entry(key, true).and_then(IconEntry::ready_image)
	}

	/// Catalog tiles prefer a shaded still and fall back to a silhouette
	/// (Self Lubikan and Tanz pose icons).
	pub fn catalog_for_key(&self, key: u64) -> Option<Handle<Image>> {
		self.shaded_for_key(key).or_else(|| self.silhouette_for_key(key))
	}

	pub fn silhouette_for_item(
		&self,
		item: &character_items::InventoryItem,
	) -> Option<Handle<Image>> {
		self.silhouette(&CaptureId::new(item.capture_id_label()?))
	}

	pub fn shaded_for_item(&self, item: &character_items::InventoryItem) -> Option<Handle<Image>> {
		self.shaded(&CaptureId::new(item.capture_id_label()?))
	}

	pub fn failed(&self, id: &CaptureId, mode: CaptureMode) -> bool {
		self.current_entry(Some(id), None, mode)
			.is_some_and(|entry| entry.status == IconEntryStatus::Failed)
	}

	pub fn failed_for_key(&self, key: u64, mode: CaptureMode) -> bool {
		self.catalog_entry(key, mode == CaptureMode::Silhouette)
			.is_some_and(|entry| entry.status == IconEntryStatus::Failed)
	}

	/// Filled handle or a reported failure — hosts can stop waiting.
	pub fn filled_or_failed(&self, key: u64, silhouette: bool) -> bool {
		self.catalog_need_is_terminal(key, silhouette)
	}

	pub fn insert_shaded(&mut self, id: CaptureId, image: Handle<Image>) {
		self.insert_ready(compat_key(id, CaptureMode::Shaded), image);
	}

	pub fn insert_silhouette(&mut self, id: CaptureId, image: Handle<Image>) {
		self.insert_ready(compat_key(id, CaptureMode::Silhouette), image);
	}

	fn insert_ready(&mut self, key: CaptureCacheKey, image: Handle<Image>) {
		self.entries.insert(key.clone(), IconEntry::ready(key, image));
	}

	fn current_entry(
		&self,
		id: Option<&CaptureId>,
		catalog_key: Option<u64>,
		mode: CaptureMode,
	) -> Option<&IconEntry> {
		self.entries
			.values()
			.filter(|entry| {
				entry.key.revision == CaptureRevision::CURRENT
					&& entry.key.mode == mode
					&& id.is_none_or(|id| entry.key.id == *id)
					&& catalog_key
						.is_none_or(|key| icon_catalog_key_for_label(entry.key.id.as_str()) == key)
			})
			.max_by_key(|entry| entry.status.rank())
	}
}

fn mode_for(silhouette: bool) -> CaptureMode {
	if silhouette {
		CaptureMode::Silhouette
	} else {
		CaptureMode::Shaded
	}
}

/// Test / HUD helper key. Real captures use [`CaptureCacheKey::from_request`].
fn compat_key(id: CaptureId, mode: CaptureMode) -> CaptureCacheKey {
	CaptureCacheKey {
		id,
		mode,
		resolution: ITEM_ICON_RESOLUTION,
		view: String::from("compat"),
		lighting: String::from("compat"),
		scene: String::from("compat"),
		revision: CaptureRevision::CURRENT,
		file_stem: String::new(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{FirearmMesh, InventoryItem};
	use screen_capture::{
		CaptureError, CaptureFraming, CaptureLighting, CaptureRequest, CaptureSource,
	};

	use crate::{InventoryItemSubject, ITEM_ICON_RESOLUTION};

	fn firearm() -> anyhow::Result<InventoryItem> {
		Ok(InventoryItem::firearm(FirearmMesh::Bullpup))
	}

	fn request_for(item: &InventoryItem, mode: CaptureMode) -> anyhow::Result<CaptureRequest> {
		let subject =
			InventoryItemSubject::from_item(item).ok_or_else(|| anyhow::anyhow!("subject"))?;
		Ok(CaptureRequest::from_subject(
			&subject,
			mode,
			ITEM_ICON_RESOLUTION,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		)
		.with_file_stem(item.name())
		.with_framing(CaptureFraming::ICON))
	}

	fn ticket(request: &CaptureRequest, image: Handle<Image>) -> CaptureTicket {
		CaptureTicket { key: CaptureCacheKey::from_request(request), image, source: None }
	}

	fn completed(request: &CaptureRequest, image: Handle<Image>) -> CaptureCompleted {
		CaptureCompleted {
			key: CaptureCacheKey::from_request(request),
			image,
			source: CaptureSource::Rendered,
		}
	}

	fn failed(request: &CaptureRequest) -> CaptureFailed {
		CaptureFailed { key: CaptureCacheKey::from_request(request), error: CaptureError::Readback }
	}

	fn stale_request(item: &InventoryItem, mode: CaptureMode) -> anyhow::Result<CaptureRequest> {
		let mut request = request_for(item, mode)?;
		request.revision = CaptureRevision::new(CaptureRevision::CURRENT.0.wrapping_sub(1));
		Ok(request)
	}

	#[test]
	fn mixed_success_and_failure_are_both_terminal() -> anyhow::Result<()> {
		let bullpup = firearm()?;
		let reltor = InventoryItem::firearm(FirearmMesh::Reltor);
		let shaded = request_for(&bullpup, CaptureMode::Shaded)?;
		let other = request_for(&reltor, CaptureMode::Shaded)?;
		let mut cache = ItemIconCache::default();
		cache.remember_ticket(&ticket(&shaded, Handle::default()));
		cache.remember_ticket(&ticket(&other, Handle::default()));
		cache.apply_completed(&completed(&shaded, Handle::default()));
		cache.apply_failed(&failed(&other));

		let bullpup_key =
			bullpup.icon_catalog_key().ok_or_else(|| anyhow::anyhow!("bullpup key"))?;
		let reltor_key = reltor.icon_catalog_key().ok_or_else(|| anyhow::anyhow!("reltor key"))?;
		assert_eq!(
			cache.catalog_entry(bullpup_key, false).map(IconEntry::status),
			Some(IconEntryStatus::Ready)
		);
		assert_eq!(
			cache.catalog_entry(reltor_key, false).map(IconEntry::status),
			Some(IconEntryStatus::Failed)
		);
		assert!(cache.shaded_for_key(bullpup_key).is_some());
		assert!(cache.shaded_for_key(reltor_key).is_none());
		assert!(cache.catalog_need_is_terminal(bullpup_key, false));
		assert!(cache.catalog_need_is_terminal(reltor_key, false));
		assert!(cache
			.catalog_progress([(bullpup_key, false), (reltor_key, false)])
			.all_terminal());
		Ok(())
	}

	#[test]
	fn stale_completion_does_not_fill_the_current_entry() -> anyhow::Result<()> {
		let item = firearm()?;
		let current = request_for(&item, CaptureMode::Shaded)?;
		let stale = stale_request(&item, CaptureMode::Shaded)?;
		let mut cache = ItemIconCache::default();
		cache.remember_ticket(&ticket(&current, Handle::default()));
		cache.apply_completed(&completed(&stale, Handle::default()));

		let catalog = item.icon_catalog_key().ok_or_else(|| anyhow::anyhow!("key"))?;
		assert_eq!(
			cache.entry(&CaptureCacheKey::from_request(&current)).map(IconEntry::status),
			Some(IconEntryStatus::Pending)
		);
		assert_eq!(
			cache.entry(&CaptureCacheKey::from_request(&stale)).map(IconEntry::status),
			Some(IconEntryStatus::Ready)
		);
		assert!(cache.catalog_entry(catalog, false).is_some_and(|entry| {
			entry.status() == IconEntryStatus::Pending
				&& entry.key().revision == CaptureRevision::CURRENT
		}));
		assert!(cache.shaded_for_key(catalog).is_none());
		assert!(!cache.catalog_need_is_terminal(catalog, false));
		Ok(())
	}

	#[test]
	fn failed_entry_retries_then_can_become_ready() -> anyhow::Result<()> {
		let item = firearm()?;
		let request = request_for(&item, CaptureMode::Silhouette)?;
		let mut cache = ItemIconCache::default();
		let first = ticket(&request, Handle::default());
		cache.remember_ticket(&first);
		cache.apply_failed(&failed(&request));
		assert_eq!(cache.entry(&first.key).map(IconEntry::status), Some(IconEntryStatus::Failed));

		cache.apply_completed(&completed(&request, Handle::default()));
		assert_eq!(
			cache.entry(&first.key).map(IconEntry::status),
			Some(IconEntryStatus::Failed),
			"failure is terminal until a new ticket retries the still"
		);

		let retry = ticket(&request, Handle::default());
		cache.remember_ticket(&retry);
		assert_eq!(cache.entry(&retry.key).map(IconEntry::status), Some(IconEntryStatus::Pending));
		cache.apply_completed(&completed(&request, Handle::default()));
		assert_eq!(cache.entry(&retry.key).map(IconEntry::status), Some(IconEntryStatus::Ready));
		assert!(cache.silhouette_for_item(&item).is_some());
		Ok(())
	}

	#[test]
	fn pending_keeps_the_placeholder_ticket() -> anyhow::Result<()> {
		let item = firearm()?;
		let request = request_for(&item, CaptureMode::Shaded)?;
		let placeholder = Handle::default();
		let mut cache = ItemIconCache::default();
		cache.remember_ticket(&ticket(&request, placeholder.clone()));
		let entry = cache
			.entry(&CaptureCacheKey::from_request(&request))
			.ok_or_else(|| anyhow::anyhow!("pending entry"))?;
		assert_eq!(entry.status(), IconEntryStatus::Pending);
		assert_eq!(entry.image(), &placeholder);
		assert!(entry.ready_image().is_none());
		Ok(())
	}
}
