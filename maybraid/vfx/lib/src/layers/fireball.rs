//! Near fireball: overlapping mesh lobes plus optional flame cards.

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
use crate::names::FIREBALL;
use crate::particles::{
	add_instance_properties, init_lifetime, init_palette_color, init_radial_velocity,
	update_palette_color, update_scaled_size3, value_lifetime_gradient,
};

pub const FIREBALL_COUNT: f32 = 6.0;
pub const FIREBALL_CAPACITY: u32 = 16;
pub const FIREBALL_LIFE_MIN: f32 = 0.35;
pub const FIREBALL_LIFE_MAX: f32 = 0.55;

pub fn compile_mesh(mesh: Handle<Mesh>) -> MeshPart {
	MeshPart::duration_from_lobes(
		FIREBALL,
		mesh,
		LobeKind::Fire,
		vec![
			LobeSpec::new(Vec3::new(0.00, 0.05, 0.00), Vec3::new(0.72, 0.46, 0.58))
				.with_euler(Vec3::new(0.15, 0.40, -0.22))
				.with_expand(0.88)
				.with_rise(0.08)
				.with_roll(0.35)
				.with_duration(0.55),
			LobeSpec::new(Vec3::new(0.16, 0.10, 0.05), Vec3::new(0.34, 0.52, 0.28))
				.with_euler(Vec3::new(-0.55, 0.80, 0.18))
				.with_expand(0.72)
				.with_rise(0.12)
				.with_roll(0.55)
				.with_duration(0.50),
			LobeSpec::new(Vec3::new(-0.14, 0.08, 0.10), Vec3::new(0.48, 0.30, 0.40))
				.with_euler(Vec3::new(0.70, -0.35, 0.45))
				.with_expand(0.74)
				.with_rise(0.10)
				.with_roll(0.40)
				.with_duration(0.52),
			LobeSpec::new(Vec3::new(0.04, 0.14, -0.16), Vec3::new(0.28, 0.44, 0.50))
				.with_euler(Vec3::new(-0.20, 1.10, -0.60))
				.with_expand(0.70)
				.with_rise(0.14)
				.with_roll(0.70)
				.with_duration(0.48),
			LobeSpec::new(Vec3::new(-0.08, -0.02, -0.12), Vec3::new(0.42, 0.26, 0.36))
				.with_euler(Vec3::new(0.95, 0.25, 0.10))
				.with_expand(0.62)
				.with_rise(0.06)
				.with_roll(0.30)
				.with_duration(0.46),
			LobeSpec::new(Vec3::new(0.10, -0.04, 0.14), Vec3::new(0.24, 0.38, 0.46))
				.with_euler(Vec3::new(-0.80, -0.50, 0.75))
				.with_expand(0.64)
				.with_rise(0.07)
				.with_roll(0.50)
				.with_duration(0.47),
		],
	)
}

pub fn compile_wisps(effects: &mut Assets<EffectAsset>, fire: &FlipbookAsset) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: (writer.lit(0.08) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel =
		init_radial_velocity(&writer, &props.scale, &props.playback, &props.seed, 1.2, 2.4);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, FIREBALL_LIFE_MIN, FIREBALL_LIFE_MAX);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(1.4) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, 0.6, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size =
		update_scaled_size3(&writer, &props.scale, Vec3::splat(0.32), Vec3::splat(0.72));
	let update_sprite = update_sprite_index(&writer, fire);
	let texture_slot = writer.lit(0u32).expr();
	let color = value_lifetime_gradient(&[(0.0, 0.95), (0.4, 0.72), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("fire");

	let effect = effects.add(
		EffectAsset::new(FIREBALL_CAPACITY, SpawnerSettings::once(FIREBALL_COUNT.into()), module)
			.with_name("vfx-fireball-wisps")
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
			.render(FlipbookModifier { sprite_grid_size: fire.grid })
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			})
			.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
	);

	ParticlePart {
		name: format!("{FIREBALL}-wisps"),
		effect,
		images: vec![fire.image.clone()],
		count: FIREBALL_COUNT,
		capacity: FIREBALL_CAPACITY,
		max_lifetime: FIREBALL_LIFE_MAX,
		shade: ParticleShade::Fire,
	}
}

pub fn definition(mesh: MeshPart, wisps: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(FIREBALL, [EffectLayer::mesh(mesh), EffectLayer::particle(wisps)])
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::assets::fire_flipbook;
	use crate::composition::EffectPart;

	#[test]
	fn fireball_owns_mesh_core_and_wisps() {
		let mut images = Assets::<Image>::default();
		let mut effects = Assets::<EffectAsset>::default();
		let wisps = compile_wisps(&mut effects, &fire_flipbook(&mut images));
		let def = definition(compile_mesh(Handle::default()), wisps);
		assert_eq!(def.layers.len(), 2);
		assert!(matches!(def.layers[0].part, EffectPart::Mesh(_)));
		assert_eq!(
			effects
				.get(match &def.layers[1].part {
					EffectPart::Particle(part) => &part.effect,
					_ => panic!("expected wisps"),
				})
				.unwrap()
				.simulation_space,
			SimulationSpace::Global
		);
	}

	#[test]
	fn fireball_lobes_are_anisotropic() {
		let mesh = compile_mesh(Handle::default());
		assert!(mesh.lobes.iter().all(|lobe| (lobe.scale.x - lobe.scale.y).abs() > 0.04));
	}
}
