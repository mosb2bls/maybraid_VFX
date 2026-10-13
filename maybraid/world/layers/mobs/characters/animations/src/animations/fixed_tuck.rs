use crate::animations::TuckProfile;

/// Held gymnastics tuck; pose is constant across progress.
#[derive(Debug, Clone)]
pub struct FixedTuck {
	/// `1.0` matches the tuned default tuck shape.
	pub tightness: f32,
}

impl Default for FixedTuck {
	fn default() -> Self {
		Self { tightness: TuckProfile::DEFAULT_TIGHTNESS }
	}
}

impl FixedTuck {
	pub fn new(tightness: f32) -> Self {
		Self { tightness }
	}

	pub fn tightness(&self) -> f32 {
		self.tightness
	}

	pub fn profile(&self) -> TuckProfile {
		TuckProfile::new(self.tightness)
	}
}
