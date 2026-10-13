//! Shared preview color helpers (thumbnail / UI). Character live paint uses
//! [`characters::MaterialRefRoot`] on the part host.

#![allow(dead_code)]

use bevy::prelude::*;

/// Quantized sRGB tint from recipe [`characters::MaterialRef`] palette[0].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreviewColor(pub [u8; 4]);

impl PreviewColor {
	pub const PLACEHOLDER: Self = Self([180, 180, 180, 255]);

	pub fn bevy_color(self) -> Color {
		let [r, g, b, a] = self.0;
		Color::srgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
	}

	pub fn from_bevy(color: Color) -> Self {
		let c = color.to_srgba();
		Self([
			(c.red * 255.0).round() as u8,
			(c.green * 255.0).round() as u8,
			(c.blue * 255.0).round() as u8,
			(c.alpha * 255.0).round() as u8,
		])
	}

	pub fn from_material(material: &characters::MaterialRef) -> Self {
		material
			.palette
			.first()
			.copied()
			.map(Self::from_bevy)
			.unwrap_or(Self::PLACEHOLDER)
	}

	pub fn from_part(part: &characters::PartNode) -> Self {
		Self::from_material(&part.material)
	}
}
