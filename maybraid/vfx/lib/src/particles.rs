//! Shared Hanabi instance properties and smoke launch math.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	Attribute, EffectProperties, ExprWriter, SetAttributeModifier, WriterExpr,
};

use crate::composition::ParticleShade;
use crate::palette::ExplosionPalette;
use crate::seed;
use crate::spawn::VfxSpawn;

pub const PROP_SCALE: &str = "vfx_scale";
pub const PROP_TINT: &str = "vfx_tint";
pub const PROP_SEED: &str = "vfx_seed";
pub const PROP_PLAYBACK: &str = "vfx_playback";
pub const PROP_COLOR0: &str = "vfx_color0";
pub const PROP_COLOR1: &str = "vfx_color1";
pub const PROP_COLOR2: &str = "vfx_color2";

pub struct InstanceExprs {
	pub scale: WriterExpr,
	pub tint: WriterExpr,
	pub seed: WriterExpr,
	pub playback: WriterExpr,
	pub color0: WriterExpr,
	pub color1: WriterExpr,
	pub color2: WriterExpr,
}

pub fn add_instance_properties(writer: &ExprWriter) -> InstanceExprs {
	let defaults = ExplosionPalette::maybraid();
	let scale = writer.add_property(PROP_SCALE, 1.0.into());
	let tint = writer.add_property(PROP_TINT, Vec4::ONE.into());
	let seed = writer.add_property(PROP_SEED, 0.0.into());
	let playback = writer.add_property(PROP_PLAYBACK, 1.0.into());
	let color0 = writer.add_property(PROP_COLOR0, ExplosionPalette::vec4(defaults.fire_hot).into());
	let color1 = writer.add_property(PROP_COLOR1, ExplosionPalette::vec4(defaults.fire_mid).into());
	let color2 =
		writer.add_property(PROP_COLOR2, ExplosionPalette::vec4(defaults.fire_cool).into());
	InstanceExprs {
		scale: writer.prop(scale),
		tint: writer.prop(tint),
		seed: writer.prop(seed),
		playback: writer.prop(playback),
		color0: writer.prop(color0),
		color1: writer.prop(color1),
		color2: writer.prop(color2),
	}
}

pub fn effect_properties(
	spawn: &VfxSpawn,
	layer_scale: f32,
	shade: ParticleShade,
) -> EffectProperties {
	let mut properties = EffectProperties::default();
	properties.set(PROP_SCALE, (spawn.scale * layer_scale).into());
	let tint = spawn.tint.map(LinearRgba::from).unwrap_or(LinearRgba::WHITE);
	properties.set(PROP_TINT, Vec4::new(tint.red, tint.green, tint.blue, 1.0).into());
	properties.set(PROP_SEED, seed::unit(spawn.resolved_seed()).into());
	properties.set(PROP_PLAYBACK, spawn.clamped_playback().into());
	let [c0, c1, c2] = shade_colors(shade, &spawn.palette());
	properties.set(PROP_COLOR0, c0.into());
	properties.set(PROP_COLOR1, c1.into());
	properties.set(PROP_COLOR2, c2.into());
	properties
}

pub fn shade_colors(shade: ParticleShade, palette: &ExplosionPalette) -> [Vec4; 3] {
	match shade {
		ParticleShade::Fire => [
			ExplosionPalette::vec4(palette.fire_hot),
			ExplosionPalette::vec4(palette.fire_mid),
			ExplosionPalette::vec4(palette.fire_cool),
		],
		ParticleShade::Smoke => [
			ExplosionPalette::vec4(palette.smoke_lit),
			ExplosionPalette::vec4(mix_lin(palette.smoke_lit, palette.smoke_shadow, 0.45)),
			ExplosionPalette::vec4(palette.smoke_shadow),
		],
		ParticleShade::Spark => [
			ExplosionPalette::vec4(palette.spark),
			ExplosionPalette::vec4(palette.fire_mid),
			ExplosionPalette::vec4(palette.fire_cool),
		],
	}
}

fn mix_lin(a: LinearRgba, b: LinearRgba, t: f32) -> LinearRgba {
	LinearRgba {
		red: a.red + (b.red - a.red) * t,
		green: a.green + (b.green - a.green) * t,
		blue: a.blue + (b.blue - a.blue) * t,
		alpha: a.alpha + (b.alpha - a.alpha) * t,
	}
}

/// Hash `seed` with local position so instance seed controls speed and lifetime.
pub fn seeded_unit(writer: &ExprWriter, seed: &WriterExpr, salt: f32) -> WriterExpr {
	let pos = writer.attr(Attribute::POSITION);
	(seed.clone() * writer.lit(12.9898)
		+ pos.clone().x() * writer.lit(78.233)
		+ pos.clone().y() * writer.lit(37.719)
		+ pos.z() * writer.lit(salt))
	.sin()
	.fract()
}

pub fn seeded_range(
	writer: &ExprWriter,
	seed: &WriterExpr,
	salt: f32,
	min: f32,
	max: f32,
) -> WriterExpr {
	writer.lit(min).mix(writer.lit(max), seeded_unit(writer, seed, salt))
}

/// `radial_direction * outward_speed + up * rise_speed`, optional ground clamp.
pub fn smoke_launch_velocity(
	position: Vec3,
	outward_speed: f32,
	rise_speed: f32,
	ground: bool,
) -> Vec3 {
	let dir = if position.length_squared() > 1e-8 { position.normalize() } else { Vec3::Y };
	let mut velocity = dir * outward_speed + Vec3::Y * rise_speed;
	if ground {
		velocity.y = velocity.y.max(0.2);
	}
	velocity
}

/// GPU init: radial from the current (local) position plus an upward bias.
///
/// Hanabi prints swizzles without wrapping the product, so `(vel * scale).y`
/// becomes `vel * scale.y` and fails because `vfx_scale` is a float. Keep
/// ground lift in the rise term instead of clamping a swizzled Y.
pub fn init_smoke_velocity(
	writer: &ExprWriter,
	scale: &WriterExpr,
	playback: &WriterExpr,
	seed: &WriterExpr,
	outward_min: f32,
	outward_max: f32,
	rise: f32,
	ground: bool,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let dir = pos.clone().div(pos.length().max(writer.lit(1e-3)));
	let outward = seeded_range(writer, seed, 1.17, outward_min, outward_max);
	let lift = if ground { rise.max(outward_max + 0.2) } else { rise };
	let vel =
		(dir * outward + writer.lit(Vec3::Y) * writer.lit(lift)) * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn init_radial_velocity(
	writer: &ExprWriter,
	scale: &WriterExpr,
	playback: &WriterExpr,
	seed: &WriterExpr,
	speed_min: f32,
	speed_max: f32,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let dir = pos.clone().div(pos.length().max(writer.lit(1e-3)));
	let speed = seeded_range(writer, seed, 2.53, speed_min, speed_max);
	let vel = dir * speed * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn init_lifetime(
	writer: &ExprWriter,
	playback: &WriterExpr,
	seed: &WriterExpr,
	min: f32,
	max: f32,
) -> SetAttributeModifier {
	let life = seeded_range(writer, seed, 3.91, min, max) / playback.clone();
	SetAttributeModifier::new(Attribute::LIFETIME, life.expr())
}

pub fn init_palette_color(props: &InstanceExprs) -> SetAttributeModifier {
	SetAttributeModifier::new(
		Attribute::HDR_COLOR,
		(props.color0.clone() * props.tint.clone()).expr(),
	)
}

pub fn update_palette_color(writer: &ExprWriter, props: &InstanceExprs) -> SetAttributeModifier {
	let t = writer
		.attr(Attribute::AGE)
		.div(writer.attr(Attribute::LIFETIME).max(writer.lit(1e-3)))
		.saturate();
	let early = props
		.color0
		.clone()
		.mix(props.color1.clone(), (t.clone() * writer.lit(2.0)).saturate());
	let color =
		early.mix(props.color2.clone(), (t * writer.lit(2.0) + writer.lit(-1.0)).saturate());
	SetAttributeModifier::new(Attribute::HDR_COLOR, (color * props.tint.clone()).expr())
}

pub fn update_scaled_size3(
	writer: &ExprWriter,
	scale: &WriterExpr,
	start: Vec3,
	end: Vec3,
) -> SetAttributeModifier {
	let t = writer
		.attr(Attribute::AGE)
		.div(writer.attr(Attribute::LIFETIME).max(writer.lit(1e-3)))
		.saturate();
	let size = writer.lit(start).mix(writer.lit(end), t) * scale.clone();
	SetAttributeModifier::new(Attribute::SIZE3, size.expr())
}

/// White heat/alpha envelope. Palette RGB lives in instance properties.
pub fn value_lifetime_gradient(keys: &[(f32, f32)]) -> bevy_hanabi::Gradient<Vec4> {
	let mut color = bevy_hanabi::Gradient::new();
	for &(t, alpha) in keys {
		color.add_key(t, Vec4::new(1.0, 1.0, 1.0, alpha));
	}
	color
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn smoke_launch_is_outward_and_up() {
		for position in [
			Vec3::new(0.1, 0.05, 0.0),
			Vec3::new(-0.08, -0.12, 0.06),
			Vec3::new(0.0, -0.14, 0.0),
			Vec3::ZERO,
		] {
			let velocity = smoke_launch_velocity(position, 0.6, 1.1, true);
			assert!(velocity.y > 0.0, "downward launch {velocity:?} from {position:?}");
		}
	}

	#[test]
	fn air_blast_keeps_downward_component() {
		let velocity = smoke_launch_velocity(Vec3::new(0.0, -0.14, 0.0), 0.8, 0.1, false);
		assert!(velocity.y < 0.0);
	}

	#[test]
	fn hashed_seed_units_stay_distinct() {
		assert!((seed::unit(1u64 << 40) - seed::unit((1u64 << 40) + 1)).abs() > 1e-6);
	}
}
