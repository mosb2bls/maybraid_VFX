//! World-space spark streaks with an elongated tapered mask.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, ImageSampleMapping, LinearDragModifier, OrientMode, OrientModifier,
	ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier, ShapeDimension,
	SimulationSpace, SpawnerSettings,
};

use crate::composition::{EffectDefinition, EffectLayer, ParticlePart, ParticleShade};
use crate::names::SPARKS;
use crate::particles::{
	add_instance_properties, init_lifetime, init_palette_color, init_radial_velocity,
	update_palette_color, update_scaled_size3, value_lifetime_gradient,
};

pub const SPARKS_COUNT: f32 = 28.0;
pub const SPARKS_CAPACITY: u32 = 48;
pub const SPARKS_LIFE_MIN: f32 = 0.18;
pub const SPARKS_LIFE_MAX: f32 = 0.4;

pub fn compile(effects: &mut Assets<EffectAsset>, taper: Handle<Image>) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: (writer.lit(0.06) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel =
		init_radial_velocity(&writer, &props.scale, &props.playback, &props.seed, 3.5, 7.0);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, SPARKS_LIFE_MIN, SPARKS_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(1.2) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, -10.0, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.24, 0.036, 1.0),
		Vec3::new(0.10, 0.014, 1.0),
	);
	let texture_slot = writer.lit(0u32).expr();
	let color = value_lifetime_gradient(&[(0.0, 1.0), (0.4, 1.0), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("spark");

	let effect = effects.add(
		EffectAsset::new(SPARKS_CAPACITY, SpawnerSettings::once(SPARKS_COUNT.into()), module)
			.with_name("vfx-sparks")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
			.update(update_drag)
			.update(update_accel)
			.update(update_size)
			.update(update_color)
			.render(ParticleTextureModifier {
				texture_slot,
				sample_mapping: ImageSampleMapping::Modulate,
			})
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			})
			.render(OrientModifier::new(OrientMode::AlongVelocity)),
	);

	ParticlePart {
		name: SPARKS.into(),
		effect,
		images: vec![taper],
		count: SPARKS_COUNT,
		capacity: SPARKS_CAPACITY,
		max_lifetime: SPARKS_LIFE_MAX,
		shade: ParticleShade::Spark,
	}
}

pub fn definition(part: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(SPARKS, [EffectLayer::particle(part)])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compile_is_an_immediate_streak_burst() {
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile(&mut effects, Handle::default());
		assert_eq!(part.name, SPARKS);
		assert_eq!(part.images.len(), 1);
		assert!((definition(part).duration() - SPARKS_LIFE_MAX).abs() < 1e-4);
	}
}
