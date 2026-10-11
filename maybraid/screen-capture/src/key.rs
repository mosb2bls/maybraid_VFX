//! Deterministic cache key for a capture request.

use crate::request::CaptureRequest;
use crate::{CaptureId, CaptureMode, CaptureResolution, CaptureRevision};

/// All inputs that must invalidate a cached PNG.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureCacheKey {
	pub id: CaptureId,
	pub mode: CaptureMode,
	pub resolution: CaptureResolution,
	pub view: String,
	pub lighting: String,
	pub scene: String,
	pub revision: CaptureRevision,
	pub file_stem: String,
}

impl CaptureCacheKey {
	pub fn from_request(request: &CaptureRequest) -> Self {
		Self {
			id: request.id.clone(),
			mode: request.mode,
			resolution: request.resolution,
			view: request.view.cache_fragment(),
			lighting: request.lighting.cache_fragment(),
			scene: request.scene.cache_fragment(),
			revision: request.revision,
			file_stem: request.file_stem.clone(),
		}
	}

	pub fn canonical(&self) -> String {
		format!(
			"id={}|mode={}|res={}|view={}|light={}|scene={}|crev={}",
			self.id.as_str(),
			self.mode.as_str(),
			self.resolution.cache_fragment(),
			self.view,
			self.lighting,
			self.scene,
			self.revision.0,
		)
	}

	pub fn hash_hex(&self) -> String {
		format!("{:016x}", fnv1a64(self.canonical().as_bytes()))
	}

	pub fn file_name(&self) -> String {
		let prefix = if self.file_stem.is_empty() {
			self.id.file_prefix()
		} else {
			format!("{}_{}", self.file_stem, self.id.file_prefix())
		};
		format!(
			"{}_{}_{}_{}.png",
			prefix,
			self.mode.as_str(),
			self.resolution.cache_fragment(),
			self.hash_hex(),
		)
	}
}

fn fnv1a64(bytes: &[u8]) -> u64 {
	let mut hash = 0xcbf29ce484222325;
	for byte in bytes {
		hash ^= u64::from(*byte);
		hash = hash.wrapping_mul(0x100000001b3);
	}
	hash
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::subject::MeshCaptureSubject;
	use crate::{CaptureLighting, CaptureMode, CaptureRequest, CaptureResolution, CaptureRevision};

	fn request(mode: CaptureMode, revision: CaptureRevision) -> CaptureRequest {
		CaptureRequest::from_subject(
			&MeshCaptureSubject::cuboid([0.8, 0.2, 0.2, 1.0]),
			mode,
			CaptureResolution::ICON,
			CaptureLighting::studio(),
			revision,
		)
	}

	#[test]
	fn same_request_shares_key() -> anyhow::Result<()> {
		let a =
			CaptureCacheKey::from_request(&request(CaptureMode::Shaded, CaptureRevision::CURRENT));
		let b =
			CaptureCacheKey::from_request(&request(CaptureMode::Shaded, CaptureRevision::CURRENT));
		assert_eq!(a, b);
		assert_eq!(a.hash_hex(), b.hash_hex());
		Ok(())
	}

	#[test]
	fn mode_and_revision_change_key() -> anyhow::Result<()> {
		let shaded =
			CaptureCacheKey::from_request(&request(CaptureMode::Shaded, CaptureRevision::CURRENT));
		let silhouette = CaptureCacheKey::from_request(&request(
			CaptureMode::Silhouette,
			CaptureRevision::CURRENT,
		));
		let bumped =
			CaptureCacheKey::from_request(&request(CaptureMode::Shaded, CaptureRevision::new(4)));
		assert_ne!(shaded, silhouette);
		assert_ne!(shaded, bumped);
		Ok(())
	}

	#[test]
	fn english_stem_is_in_the_file_name_not_the_hash() -> anyhow::Result<()> {
		let plain =
			CaptureCacheKey::from_request(&request(CaptureMode::Shaded, CaptureRevision::CURRENT));
		let named = CaptureCacheKey::from_request(
			&request(CaptureMode::Shaded, CaptureRevision::CURRENT)
				.with_file_stem("Cylint Wine Snailer"),
		);
		assert_eq!(plain.hash_hex(), named.hash_hex());
		assert!(named.file_name().starts_with("cylint-wine-snailer_"));
		assert!(!plain.file_name().contains("cylint-wine-snailer"));
		Ok(())
	}
}
