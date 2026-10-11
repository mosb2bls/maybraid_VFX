//! Stable, serializable capture identity.

use std::fmt;

/// Identifies a capture subject's visual appearance.
///
/// The same appearance must reuse the same id across launches. Procedural or
/// customized variants must use distinct ids.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureId(pub String);

impl CaptureId {
	pub fn new(id: impl Into<String>) -> Self {
		Self(id.into())
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	/// Filesystem-safe prefix (not unique; pair with the cache hash).
	pub fn file_prefix(&self) -> String {
		self.0
			.chars()
			.map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
			.take(32)
			.collect()
	}
}

/// Lowercase alphanumeric slug for cache file names (`Celestial Red Tide` → `celestial-red-tide`).
pub fn sanitize_file_stem(name: &str) -> String {
	let mut stem = String::new();
	let mut dash = false;
	for ch in name.chars() {
		if ch.is_ascii_alphanumeric() {
			stem.extend(ch.to_lowercase());
			dash = false;
		} else if !dash && !stem.is_empty() {
			stem.push('-');
			dash = true;
		}
		if stem.len() >= 48 {
			break;
		}
	}
	while stem.ends_with('-') {
		stem.pop();
	}
	stem
}

impl fmt::Display for CaptureId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0)
	}
}

impl From<&str> for CaptureId {
	fn from(value: &str) -> Self {
		Self::new(value)
	}
}

impl From<String> for CaptureId {
	fn from(value: String) -> Self {
		Self::new(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn file_prefix_strips_punctuation() -> anyhow::Result<()> {
		let id = CaptureId::new("firearm:body=bullpup/looks=brushed");
		assert_eq!(id.file_prefix(), "firearm-body-bullpup-looks-brush");
		Ok(())
	}

	#[test]
	fn file_stem_keeps_hashed_english_words() -> anyhow::Result<()> {
		assert_eq!(sanitize_file_stem("Furnaced Umbral Silopup"), "furnaced-umbral-silopup");
		assert_eq!(sanitize_file_stem("O'Red Yoeman Snailer"), "o-red-yoeman-snailer");
		Ok(())
	}
}
