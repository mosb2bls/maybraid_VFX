//! One-shot Hanabi bursts when a bolt or bullet first hits Fixed geometry.
//!
//! Spark and smoke particles, and the volume they spawn in, are three times
//! the first version of this burst.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, EffectMaterial, ExprWriter, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleEffect, ParticleTextureModifier, SetAttributeModifier,
	SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension, SimulationSpace,
	SizeOverLifetimeModifier, SpawnerSettings,
};
use bevy_hanabi::Gradient;
use procedural_common::Smoothstep01;

const BURST_LIFE: f32 = 1.1;
const SURFACE_LIFT: f32 = 0.03;
const PUFF_MASK_SIZE: u32 = 128;

/// Compiled spark / smoke assets for projectile impacts.
#[derive(Resource, Clone)]
pub struct ImpactEffects {
	pub sparks: Handle<EffectAsset>,
	pub smoke: Handle<EffectAsset>,
	pub puff: Handle<Image>,
}

#[derive(Component)]
pub(crate) struct ImpactBurst {
	age: f32,
}

pub(crate) fn setup_impact_effects(
	mut commands: Commands,
	mut effects: ResMut<Assets<EffectAsset>>,
	mut images: ResMut<Assets<Image>>,
) {
	let puff = images.add(puff_mask());
	let sparks = effects.add(spark_effect());
	let smoke = effects.add(smoke_effect());
	commands.insert_resource(ImpactEffects { sparks, smoke, puff });
}

pub(crate) fn spawn_impact(
	commands: &mut Commands,
	effects: &ImpactEffects,
	point: Vec3,
	normal: Vec3,
) {
	let transform = impact_transform(point, normal);
	commands.spawn((
		Name::new("bolt-sparks"),
		transform,
		Visibility::Visible,
		ParticleEffect::new(effects.sparks.clone()),
		ImpactBurst { age: 0.0 },
	));
	commands.spawn((
		Name::new("bolt-smoke"),
		transform,
		Visibility::Visible,
		ParticleEffect::new(effects.smoke.clone()),
		EffectMaterial { images: vec![effects.puff.clone()] },
		ImpactBurst { age: 0.0 },
	));
}

pub(crate) fn tick_impact_bursts(
	time: Res<Time>,
	mut commands: Commands,
	mut bursts: Query<(Entity, &mut ImpactBurst)>,
) {
	let dt = time.delta_secs();
	for (entity, mut burst) in &mut bursts {
		burst.age += dt;
		if burst.age > BURST_LIFE {
			commands.entity(entity).try_despawn();
		}
	}
}

fn impact_transform(point: Vec3, normal: Vec3) -> Transform {
	let normal = normal.normalize_or(Vec3::Y);
	let up = if normal.y.abs() > 0.9 { Vec3::Z } else { Vec3::Y };
	Transform::from_translation(point + normal * SURFACE_LIFT).looking_to(normal, up)
}

fn spark_effect() -> EffectAsset {
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: writer.lit(0.09).expr(),
		dimension: ShapeDimension::Volume,
	};
	let speed = writer.lit(3.).uniform(writer.lit(9.));
	let init_vel = SetVelocitySphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.0, 1.0)).expr(),
		speed: speed.expr(),
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(0.12).uniform(writer.lit(0.28)).expr(),
	);
	let update_accel = AccelModifier::new(writer.lit(Vec3::new(0.0, -14.0, 0.0)).expr());
	let update_drag = LinearDragModifier::new(writer.lit(2.5).expr());

	let mut color = Gradient::new();
	color.add_key(0.0, Vec4::new(0.6, 3.0, 3.2, 1.0));
	color.add_key(0.35, Vec4::new(0.3, 1.4, 1.6, 1.0));
	color.add_key(1.0, Vec4::new(0.1, 0.2, 0.25, 0.0));
	let mut size = Gradient::new();
	size.add_key(0.0, Vec3::splat(0.135));
	size.add_key(1.0, Vec3::splat(0.024));

	EffectAsset::new(64, SpawnerSettings::once(28.0.into()), writer.finish())
		.with_name("bolt-sparks")
		.with_simulation_space(SimulationSpace::Local)
		.init(init_pos)
		.init(init_vel)
		.init(init_age)
		.init(init_lifetime)
		.update(update_drag)
		.update(update_accel)
		.render(ColorOverLifetimeModifier {
			gradient: color,
			blend: ColorBlendMode::Overwrite,
			mask: ColorBlendMask::RGBA,
		})
		.render(SizeOverLifetimeModifier { gradient: size, screen_space_size: false })
		.render(OrientModifier::new(OrientMode::AlongVelocity))
}

pub(crate) fn puff_mask() -> Image {
	let n = PUFF_MASK_SIZE;
	let mut data = vec![0u8; (n * n) as usize];
	let c = (n as f32 - 1.0) * 0.5;
	for y in 0..n {
		for x in 0..n {
			let dx = (x as f32 - c) / c;
			let dy = (y as f32 - c) / c;
			let r = (dx * dx + dy * dy).sqrt();
			let t = Smoothstep01::eval((1.0 - r).max(0.0));
			data[(y * n + x) as usize] = (t * t * 255.0) as u8;
		}
	}
	let mut image = Image::new(
		Extent3d { width: n, height: n, depth_or_array_layers: 1 },
		TextureDimension::D2,
		data,
		TextureFormat::R8Unorm,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.sampler = ImageSampler::linear();
	image
}

fn smoke_effect() -> EffectAsset {
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: writer.lit(0.12).expr(),
		dimension: ShapeDimension::Volume,
	};
	let speed = writer.lit(0.12).uniform(writer.lit(0.45));
	let init_vel = SetVelocitySphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.0, 1.0)).expr(),
		speed: speed.expr(),
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(0.5).uniform(writer.lit(0.9)).expr(),
	);
	let update_accel = AccelModifier::new(writer.lit(Vec3::new(0.0, 0.8, 0.0)).expr());
	let update_drag = LinearDragModifier::new(writer.lit(1.8).expr());
	let texture_slot = writer.lit(0u32).expr();

	let mut color = Gradient::new();
	color.add_key(0.0, Vec4::new(0.5, 0.52, 0.54, 0.35));
	color.add_key(0.35, Vec4::new(0.32, 0.34, 0.36, 0.18));
	color.add_key(1.0, Vec4::new(0.18, 0.19, 0.2, 0.0));
	let mut size = Gradient::new();
	size.add_key(0.0, Vec3::splat(0.15));
	size.add_key(1.0, Vec3::splat(0.39));

	let mut module = writer.finish();
	module.add_texture_slot("puff");

	EffectAsset::new(48, SpawnerSettings::once(24.0.into()), module)
		.with_name("bolt-smoke")
		.with_simulation_space(SimulationSpace::Local)
		.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
		.init(init_pos)
		.init(init_vel)
		.init(init_age)
		.init(init_lifetime)
		.update(update_drag)
		.update(update_accel)
		.render(ParticleTextureModifier {
			texture_slot,
			sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
		})
		.render(ColorOverLifetimeModifier {
			gradient: color,
			blend: ColorBlendMode::Overwrite,
			mask: ColorBlendMask::RGBA,
		})
		.render(SizeOverLifetimeModifier { gradient: size, screen_space_size: false })
		.render(OrientModifier::new(OrientMode::FaceCameraPosition))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn impact_sits_along_hit_normal() {
		let t = impact_transform(Vec3::ZERO, Vec3::X);
		assert!(t.translation.x > 0.02, "{}", t.translation);
		assert!((t.forward().dot(Vec3::X) - 1.0).abs() < 1e-4, "{:?}", t.forward());
	}

	#[test]
	fn puff_mask_is_a_soft_disk() {
		let image = puff_mask();
		let data = image.data.as_ref().expect("puff pixels");
		let n = PUFF_MASK_SIZE;
		let at = |x: u32, y: u32| data[(y * n + x) as usize];
		assert!(at(n / 2, n / 2) > 200, "center {}", at(n / 2, n / 2));
		assert_eq!(at(0, 0), 0);
		assert_eq!(at(n - 1, 0), 0);
		assert_eq!(at(0, n - 1), 0);
		assert_eq!(at(n - 1, n - 1), 0);
	}
}
