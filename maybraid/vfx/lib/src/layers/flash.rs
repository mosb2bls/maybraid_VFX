//! Brief emissive burst shape plus an optional light pulse.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectLayer, LightPulse, LobeKind, LobeSpec, MeshPart};
use crate::names::FLASH;
use crate::palette::ExplosionPalette;

pub const FLASH_FADE: f32 = 0.12;
pub const FLASH_PEAK: f32 = 40_000.0;
pub const FLASH_RANGE: f32 = 6.0;

pub fn pulse() -> LightPulse {
	LightPulse {
		color: ExplosionPalette::color(ExplosionPalette::maybraid().flash),
		peak_intensity: FLASH_PEAK,
		range: FLASH_RANGE,
		fade: FLASH_FADE,
	}
}

pub fn compile_mesh(mesh: Handle<Mesh>) -> MeshPart {
	MeshPart::duration_from_lobes(
		FLASH,
		mesh,
		LobeKind::Flash,
		vec![
			LobeSpec::new(Vec3::ZERO, Vec3::new(0.28, 0.18, 0.22))
				.with_euler(Vec3::new(0.25, 0.6, -0.15))
				.with_expand(1.55)
				.with_roll(0.2)
				.with_duration(FLASH_FADE),
			LobeSpec::new(Vec3::new(0.05, 0.03, -0.03), Vec3::new(0.14, 0.08, 0.11))
				.with_euler(Vec3::new(-0.7, 0.3, 0.5))
				.with_expand(1.1)
				.with_roll(0.4)
				.with_duration(FLASH_FADE * 0.7),
		],
	)
}

pub fn definition(mesh: MeshPart) -> EffectDefinition {
	EffectDefinition::new(FLASH, [EffectLayer::mesh(mesh), EffectLayer::light(pulse())])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flash_owns_shape_and_light() {
		let def = definition(compile_mesh(Handle::default()));
		assert_eq!(def.layers.len(), 2);
		assert!((def.duration() - FLASH_FADE).abs() < 1e-4);
	}
}
