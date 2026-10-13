//! Shared explosion colors. Tint is an optional overall multiplier.

use bevy::prelude::*;

/// Fire, smoke, spark, and flash colors for one instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplosionPalette {
	pub fire_hot: LinearRgba,
	pub fire_mid: LinearRgba,
	pub fire_cool: LinearRgba,
	pub smoke_lit: LinearRgba,
	pub smoke_shadow: LinearRgba,
	pub spark: LinearRgba,
	pub flash: LinearRgba,
}

impl Default for ExplosionPalette {
	fn default() -> Self {
		Self::maybraid()
	}
}

impl ExplosionPalette {
	pub fn maybraid() -> Self {
		Self {
			fire_hot: LinearRgba::rgb(1.12, 0.88, 0.38),
			fire_mid: LinearRgba::rgb(0.98, 0.34, 0.06),
			fire_cool: LinearRgba::rgb(0.32, 0.05, 0.02),
			smoke_lit: LinearRgba::rgb(0.46, 0.36, 0.28),
			smoke_shadow: LinearRgba::rgb(0.18, 0.22, 0.28),
			spark: LinearRgba::rgb(1.6, 1.05, 0.32),
			flash: LinearRgba::rgb(1.0, 0.82, 0.45),
		}
	}

	pub fn with_tint(self, tint: Option<Color>) -> Self {
		let Some(tint) = tint else {
			return self;
		};
		let tint = LinearRgba::from(tint);
		Self {
			fire_hot: mul(self.fire_hot, tint),
			fire_mid: mul(self.fire_mid, tint),
			fire_cool: mul(self.fire_cool, tint),
			smoke_lit: mul(self.smoke_lit, tint),
			smoke_shadow: mul(self.smoke_shadow, tint),
			spark: mul(self.spark, tint),
			flash: mul(self.flash, tint),
		}
	}

	pub fn vec4(color: LinearRgba) -> Vec4 {
		Vec4::new(color.red, color.green, color.blue, color.alpha)
	}

	pub fn color(color: LinearRgba) -> Color {
		Color::from(color)
	}
}

fn mul(a: LinearRgba, b: LinearRgba) -> LinearRgba {
	LinearRgba {
		red: a.red * b.red,
		green: a.green * b.green,
		blue: a.blue * b.blue,
		alpha: a.alpha * b.alpha,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tint_multiplies_instead_of_replacing() {
		let tinted = ExplosionPalette::maybraid().with_tint(Some(Color::srgb(0.0, 1.0, 1.0)));
		assert!(tinted.fire_hot.red < 0.05);
		assert!(tinted.fire_hot.green > 0.5);
		assert!(tinted.flash.red < 0.05);
	}
}
