//! Form-specific material intent (placeholder until tattoo lane #1231).

use bevy::prelude::Color;
use characters::species::epiphant::{EpiphantColor, EpiphantColors};

/// Presentation knobs that do not change assembly topology.
#[derive(Debug, Clone, PartialEq)]
pub struct BikanwardaPresentation {
	/// Body/head palette (placeholder — no tattoo UVs yet).
	pub colors: EpiphantColors,
	/// Eye emissive multiplier for encounter readability.
	pub eye_glow: f32,
	pub label: &'static str,
}

impl BikanwardaPresentation {
	pub fn encounter() -> Self {
		Self {
			colors: EpiphantColors::default(),
			eye_glow: 1.0,
			label: "encounter-placeholder",
		}
	}

	pub fn ward() -> Self {
		Self {
			colors: EpiphantColors::default(),
			eye_glow: 0.85,
			label: "ward-placeholder",
		}
	}

	pub fn enraged() -> Self {
		let mut colors = EpiphantColors::default();
		colors.eyes = EpiphantColor::Red;
		Self { colors, eye_glow: 1.35, label: "enraged-placeholder" }
	}

	pub fn eye_tint(&self) -> Color {
		let base = self.colors.eyes.color();
		let scale = self.eye_glow.clamp(0.5, 2.0);
		Color::srgb(
			(base.to_srgba().red * scale).min(1.0),
			(base.to_srgba().green * scale).min(1.0),
			(base.to_srgba().blue * scale).min(1.0),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn enraged_eyes_are_hotter_than_encounter() {
		let calm = BikanwardaPresentation::encounter().eye_tint();
		let hot = BikanwardaPresentation::enraged().eye_tint();
		assert!(hot.to_srgba().red >= calm.to_srgba().red);
	}
}
