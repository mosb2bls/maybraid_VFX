//! Capture shading modes.

/// How the isolated scene is rendered into the icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureMode {
	/// Original materials and shaders under the request lighting.
	Shaded,
	/// Uniform grayscale foreground, transparent background, antialiased edges.
	Silhouette,
}

impl CaptureMode {
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Shaded => "shaded",
			Self::Silhouette => "silhouette",
		}
	}
}

impl std::fmt::Display for CaptureMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}
