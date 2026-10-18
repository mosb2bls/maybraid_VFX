//! Near smoke: shaded mesh puffs plus thin rising wisps.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, FlipbookModifier, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier,
	ShapeDimension, SimulationSpace, SpawnerSettings,
};

use crate::assets::FlipbookAsset;
use crate::composition::{
	EffectDefinition, EffectLayer, LobeKind, LobeSpec, MeshPart, ParticlePart, ParticleShade,
};
use crate::flipbook::update_sprite_index;
use crate::names::SMOKE;
use crate::particles::{
	add_instance_properties, init_lifetime, init_palette_color, init_smoke_velocity,
	update_palette_color, update_scaled_size3, value_lifetime_gradient,
};

pub const SMOKE_COUNT: f32 = 10.0;
pub const SMOKE_CAPACITY: u32 = 32;
pub const SMOKE_LIFE_MIN: f32 = 1.2;
pub const SMOKE_LIFE_MAX: f32 = 2.4;
pub const SMOKE_DELAY: f32 = 0.05;
pub const SMOKE_OUTWARD_MIN: f32 = 0.35;
pub const SMOKE_OUTWARD_MAX: f32 = 0.9;
pub const SMOKE_RISE: f32 = 1.15;

pub fn compile_mesh(mesh: Handle<Mesh>) -> MeshPart {
	MeshPart::duration_from_lobes(
		SMOKE,
		mesh,
		LobeKind::Smoke,
		vec![
			LobeSpec::new(Vec3::new(0.00, 0.10, 0.00), Vec3::new(0.82, 0.48, 0.64))
				.with_euler(Vec3::new(0.20, 0.55, -0.15))
				.with_expand(1.05)
				.with_rise(0.55)
				.with_roll(1.4)
				.with_duration(2.20),
			LobeSpec::new(Vec3::new(0.20, 0.16, 0.08), Vec3::new(0.38, 0.62, 0.44))
				.with_euler(Vec3::new(-0.70, 0.90, 0.30))
				.with_expand(0.95)
				.with_rise(0.62)
				.with_roll(1.8)
				.with_duration(2.05),
			LobeSpec::new(Vec3::new(-0.18, 0.14, 0.12), Vec3::new(0.58, 0.36, 0.50))
				.with_euler(Vec3::new(0.85, -0.40, 0.55))
				.with_expand(0.98)
				.with_rise(0.58)
				.with_roll(1.2)
				.with_duration(2.10),
			LobeSpec::new(Vec3::new(0.06, 0.22, -0.18), Vec3::new(0.46, 0.70, 0.34))
				.with_euler(Vec3::new(-0.25, 1.20, -0.65))
				.with_expand(0.90)
				.with_rise(0.70)
				.with_roll(2.0)
				.with_duration(2.30),
			LobeSpec::new(Vec3::new(-0.12, 0.06, -0.16), Vec3::new(0.54, 0.32, 0.48))
				.with_euler(Vec3::new(1.05, 0.20, 0.15))
				.with_expand(0.88)
				.with_rise(0.48)
				.with_roll(1.1)
				.with_duration(1.90),
			LobeSpec::new(Vec3::new(0.14, 0.04, 0.18), Vec3::new(0.30, 0.50, 0.66))
				.with_euler(Vec3::new(-0.90, -0.55, 0.80))
				.with_expand(0.86)
				.with_rise(0.50)
				.with_roll(1.6)
				.with_duration(1.95),
			LobeSpec::new(Vec3::new(-0.04, 0.28, 0.04), Vec3::new(0.64, 0.40, 0.28))
				.with_euler(Vec3::new(0.35, 0.75, -0.90))
				.with_expand(0.80)
				.with_rise(0.78)
				.with_roll(1.5)
				.with_duration(2.40),
		],
	)
}

pub fn compile_wisps(effects: &mut Assets<EffectAsset>, smoke: &FlipbookAsset) -> ParticlePart {
	compile_wisps_kind(effects, smoke, true)
}

pub fn compile_wisps_kind(
	effects: &mut Assets<EffectAsset>,
	smoke: &FlipbookAsset,
	ground: bool,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: (writer.lit(0.14) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_smoke_velocity(
		&writer,
		&props.scale,
		&props.playback,
		&props.seed,
		SMOKE_OUTWARD_MIN,
		SMOKE_OUTWARD_MAX,
		SMOKE_RISE,
		ground,
	);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, SMOKE_LIFE_MIN, SMOKE_LIFE_MAX);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(1.1) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, 0.55, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size =
		update_scaled_size3(&writer, &props.scale, Vec3::splat(0.42), Vec3::splat(0.95));
	let update_sprite = update_sprite_index(&writer, smoke);
	let texture_slot = writer.lit(0u32).expr();
	let color = value_lifetime_gradient(&[(0.0, 0.72), (0.45, 0.42), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("smoke");

	let effect = effects.add(
		EffectAsset::new(SMOKE_CAPACITY, SpawnerSettings::once(SMOKE_COUNT.into()), module)
			.with_name(if ground { "vfx-smoke-wisps" } else { "vfx-smoke-wisps-air" })
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_sprite)
			.init(init_color)
			.update(update_drag)
			.update(update_accel)
			.update(update_size)
			.update(update_sprite)
			.update(update_color)
			.render(ParticleTextureModifier {
				texture_slot,
				sample_mapping: ImageSampleMapping::Modulate,
			})
			.render(FlipbookModifier { sprite_grid_size: smoke.grid })
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			})
			.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
	);

	ParticlePart {
		name: format!("{SMOKE}-wisps"),
		effect,
		images: vec![smoke.image.clone()],
		count: SMOKE_COUNT,
		capacity: SMOKE_CAPACITY,
		max_lifetime: SMOKE_LIFE_MAX,
		shade: ParticleShade::Smoke,
	}
}

pub fn definition(mesh: MeshPart, wisps: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(
		SMOKE,
		[
			EffectLayer::mesh(mesh).with_delay(SMOKE_DELAY),
			EffectLayer::particle(wisps).with_delay(SMOKE_DELAY),
		],
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::assets::smoke_flipbook;
	use crate::particles::smoke_launch_velocity;

	#[test]
	fn standalone_smoke_keeps_its_delay() {
		let mut images = Assets::<Image>::default();
		let mut effects = Assets::<EffectAsset>::default();
		let wisps = compile_wisps(&mut effects, &smoke_flipbook(&mut images));
		let def = definition(compile_mesh(Handle::default()), wisps);
		assert_eq!(def.layers.len(), 2);
		assert!((def.layers[0].delay - SMOKE_DELAY).abs() < 1e-4);
		assert!((def.duration() - (SMOKE_DELAY + SMOKE_LIFE_MAX)).abs() < 1e-4);
	}

	#[test]
	fn spawn_in_old_radius_does_not_launch_down() {
		let velocity = smoke_launch_velocity(
			Vec3::new(0.0, 0.0, 0.0) - Vec3::new(0.0, 0.35, 0.0),
			0.6,
			SMOKE_RISE,
			true,
		);
		assert!(velocity.y > 0.0, "{velocity:?}");
	}

	#[test]
	fn smoke_lobes_are_anisotropic() {
		let mesh = compile_mesh(Handle::default());
		assert!(mesh.lobes.iter().all(|lobe| (lobe.scale.x - lobe.scale.y).abs() > 0.04));
	}
}
