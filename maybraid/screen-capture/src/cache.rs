//! Persistent PNG cache in the application's writable cache directory.

use bevy::prelude::{Handle, Image};
use bevy::tasks::IoTaskPool;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CaptureWarning, CaptureWarningKind};
use crate::key::CaptureCacheKey;
use crate::png::{encode_rgba_png, image_from_png};

/// Filesystem root for captured PNGs.
#[derive(Clone, Debug)]
pub struct CaptureCache {
	pub root: PathBuf,
}

impl CaptureCache {
	pub fn new(root: impl Into<PathBuf>) -> Self {
		Self { root: root.into() }
	}

	pub fn path_for(&self, key: &CaptureCacheKey) -> PathBuf {
		self.root.join(key.file_name())
	}

	/// Load a PNG as a Bevy image. Missing or corrupt files are misses.
	pub fn load(&self, key: &CaptureCacheKey) -> Option<Image> {
		let path = self.path_for(key);
		let bytes = fs::read(&path).ok()?;
		if bytes.is_empty() {
			return None;
		}
		image_from_png(&bytes).ok()
	}

	/// Encode and write on a background IO worker. Failures do not drop the image.
	pub fn save_rgba_background(
		&self,
		key: CaptureCacheKey,
		width: u32,
		height: u32,
		rgba: Vec<u8>,
		warnings: &mut Vec<CaptureWarning>,
	) {
		let path = self.path_for(&key);
		let root = self.root.clone();
		let write = move || match encode_rgba_png(width, height, &rgba) {
			Ok(bytes) => match crate::png::write_all_atomic(&path, &bytes) {
				Ok(()) => None,
				Err(reason) => Some(CaptureWarning {
					key: key.clone(),
					warning: CaptureWarningKind::CacheWriteFailed {
						path: path.display().to_string(),
						reason,
					},
				}),
			},
			Err(reason) => Some(CaptureWarning {
				key,
				warning: CaptureWarningKind::CacheWriteFailed {
					path: root.display().to_string(),
					reason,
				},
			}),
		};

		if let Some(pool) = IoTaskPool::try_get() {
			pool.spawn(async move {
				if let Some(warning) = write() {
					bevy::log::warn!(
						"screen-capture cache write failed for {}: {:?}",
						warning.key.hash_hex(),
						warning.warning
					);
				}
			})
			.detach();
			let _ = warnings;
		} else if let Some(warning) = write() {
			warnings.push(warning);
		}
	}

	/// Synchronous save used by tests.
	pub fn save_rgba_blocking(
		&self,
		key: &CaptureCacheKey,
		width: u32,
		height: u32,
		rgba: &[u8],
	) -> Result<PathBuf, String> {
		let path = self.path_for(key);
		let bytes = encode_rgba_png(width, height, rgba)?;
		crate::png::write_all_atomic(&path, &bytes)?;
		Ok(path)
	}

	pub fn exists(&self, key: &CaptureCacheKey) -> bool {
		self.path_for(key).is_file()
	}

	pub fn root(&self) -> &Path {
		&self.root
	}
}

/// Insert decoded cache pixels into `images`, returning a strong handle.
pub fn add_cached_image(images: &mut bevy::prelude::Assets<Image>, image: Image) -> Handle<Image> {
	images.add(image)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::key::CaptureCacheKey;
	use crate::subject::MeshCaptureSubject;
	use crate::{CaptureLighting, CaptureMode, CaptureRequest, CaptureResolution, CaptureRevision};

	fn key() -> CaptureCacheKey {
		CaptureCacheKey::from_request(&CaptureRequest::from_subject(
			&MeshCaptureSubject::cuboid([0.1, 0.2, 0.3, 1.0]),
			CaptureMode::Shaded,
			CaptureResolution::ICON,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		))
	}

	#[test]
	fn miss_then_hit_round_trip() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let cache = CaptureCache::new(dir.path());
		let key = key();
		assert!(cache.load(&key).is_none());

		let rgba = vec![10_u8, 20, 30, 255, 40, 50, 60, 128];
		cache
			.save_rgba_blocking(&key, 2, 1, &rgba)
			.map_err(|error| anyhow::anyhow!(error))?;
		let loaded = cache.load(&key).ok_or_else(|| anyhow::anyhow!("expected hit"))?;
		assert_eq!(loaded.width(), 2);
		assert_eq!(loaded.height(), 1);
		Ok(())
	}

	#[test]
	fn missing_and_corrupt_files_are_misses() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let cache = CaptureCache::new(dir.path());
		let key = key();
		assert!(cache.load(&key).is_none());

		let path = cache.path_for(&key);
		fs::create_dir_all(path.parent().ok_or_else(|| anyhow::anyhow!("parent"))?)?;
		fs::write(&path, b"not a png")?;
		assert!(cache.load(&key).is_none());

		fs::write(&path, b"")?;
		assert!(cache.load(&key).is_none());
		Ok(())
	}

	#[test]
	fn failed_save_does_not_write_final_path() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let cache = CaptureCache::new(dir.path().join("missing-parent-as-file"));
		fs::write(cache.root(), b"blocked")?;
		let key = key();
		let result = cache.save_rgba_blocking(&key, 1, 1, &[1, 2, 3, 4]);
		assert!(result.is_err());
		assert!(!cache.exists(&key));
		Ok(())
	}
}
