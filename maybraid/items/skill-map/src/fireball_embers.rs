//! Continuous Hanabi embers parented to a live fireball.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, LinearDragModifier, OrientMode, OrientModifier, SetAttributeModifier,
	SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension, SimulationSpace,
	SizeOverLifetimeModifier, SpawnerSettings,
};
use bevy_hanabi::Gradient;

use crate::fireball_material::{fireball_visual_mesh, FireballMaterial};

/// Shared mesh + material for the head and every leave-behind bead.
#[derive(Resource, Clone)]
pub struct FireballEffects {
	pub mesh: Handle<Mesh>,
	pub material: Handle<FireballMaterial>,
	#[allow(dead_code)]
	pub embers: Handle<EffectAsset>,
}

pub fn setup_fireball_effects(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<FireballMaterial>>,
	mut effects: ResMut<Assets<EffectAsset>>,
) {
	let mesh = meshes.add(fireball_visual_mesh());
	let material = materials.add(FireballMaterial::new(1.0));
	let embers = effects.add(ember_effect());
	commands.insert_resource(FireballEffects { mesh, material, embers });
}

fn ember_effect() -> EffectAsset {
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: writer.lit(0.35).expr(),
		dimension: ShapeDimension::Volume,
	};
	// Velocity is away from this point. +Y is flight, so (0, 1, 0) sprays aft.
	let speed = writer.lit(2.2).uniform(writer.lit(6.5));
	let init_vel = SetVelocitySphereModifier {
		center: writer.lit(Vec3::new(0.0, 1.0, 0.0)).expr(),
		speed: speed.expr(),
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(0.18).uniform(writer.lit(0.4)).expr(),
	);
	let update_accel = AccelModifier::new(writer.lit(Vec3::new(0.0, -3.5, 0.0)).expr());
	let update_drag = LinearDragModifier::new(writer.lit(1.6).expr());

	let mut color = Gradient::new();
	color.add_key(0.0, Vec4::new(2.4, 1.6, 0.35, 1.0));
	color.add_key(0.4, Vec4::new(1.6, 0.35, 0.04, 0.85));
	color.add_key(1.0, Vec4::new(0.25, 0.03, 0.01, 0.0));
	let mut size = Gradient::new();
	size.add_key(0.0, Vec3::splat(0.11));
	size.add_key(1.0, Vec3::splat(0.02));

	EffectAsset::new(192, SpawnerSettings::rate(56.0.into()), writer.finish())
		.with_name("fireball-embers")
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ember_effect_is_named() {
		assert_eq!(ember_effect().name, "fireball-embers");
	}
}
