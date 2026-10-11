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
}
