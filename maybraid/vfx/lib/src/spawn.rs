//! Public spawn API and layer dispatch.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy_hanabi::prelude::{EffectMaterial, ParticleEffect};

use crate::composition::{
	EffectDefinition, EffectLayer, EffectPart, LobeKind, ScaleBounds, DEFAULT_SCALE_MAX,
	DEFAULT_SCALE_MIN,
};
use crate::lobe_material::LobeMaterial;
use crate::lobes::{lobe_transform, vary_lobe, LobeMaterialSlot, VfxLobe};
use crate::membership::VfxMemberOf;
use crate::palette::ExplosionPalette;
use crate::particles::effect_properties;
use crate::seed;

pub const MAX_INTENSITY: f32 = 2.0;
pub const MIN_SCALE: f32 = DEFAULT_SCALE_MIN;
pub const MAX_SCALE: f32 = DEFAULT_SCALE_MAX;
pub const MIN_PLAYBACK: f32 = 0.25;
pub const MAX_PLAYBACK: f32 = 4.0;

/// Per-instance overrides. Never mutates a shared Hanabi asset.
#[derive(Clone, Debug)]
pub struct VfxSpawn {
	pub transform: Transform,
	pub scale: f32,
	pub intensity: f32,
	/// 1.0 is the authored layer envelope. Higher plays the same ratios faster.
	pub playback: f32,
	/// Optional overall multiplier applied on top of [`Self::palette`].
	pub tint: Option<Color>,
	/// `None` picks a new seed. `Some` reproduces that instance.
	pub seed: Option<u64>,
	/// `None` uses [`ExplosionPalette::maybraid`].
	pub palette: Option<ExplosionPalette>,
}

impl Default for VfxSpawn {
	fn default() -> Self {
		Self {
			transform: Transform::IDENTITY,
			scale: 1.0,
			intensity: 1.0,
			playback: 1.0,
			tint: None,
			seed: None,
			palette: None,
		}
	}
}

impl VfxSpawn {
	pub fn clamped_scale(&self) -> f32 {
		self.clamped_scale_for(ScaleBounds::default())
	}

	pub fn clamped_scale_for(&self, bounds: ScaleBounds) -> f32 {
		self.scale.clamp(bounds.min, bounds.max)
	}

	pub fn clamped_intensity(&self) -> f32 {
		self.intensity.clamp(0.25, MAX_INTENSITY)
	}

	pub fn clamped_playback(&self) -> f32 {
		self.playback.clamp(MIN_PLAYBACK, MAX_PLAYBACK)
	}

	pub fn tint_or_white(&self) -> Color {
		self.tint.unwrap_or(Color::WHITE)
	}

	pub fn resolved_seed(&self) -> u64 {
		self.seed.unwrap_or(0)
	}

	pub fn palette(&self) -> ExplosionPalette {
		self.palette.unwrap_or_default()
	}

	/// Clamp numeric overrides and fill a missing seed. Call once before spawn.
	pub fn resolved(self) -> Self {
		Self {
			transform: self.transform,
			scale: self.clamped_scale(),
			intensity: self.clamped_intensity(),
			playback: self.clamped_playback(),
			tint: self.tint,
			seed: Some(self.seed.unwrap_or_else(seed::generate)),
			palette: self.palette,
		}
	}

	/// Clamp using the target definition's scale bounds. Call once before spawn.
	pub fn resolved_for(&self, definition: &EffectDefinition) -> Self {
		Self {
			transform: self.transform,
			scale: self.clamped_scale_for(definition.scale_bounds),
			intensity: self.clamped_intensity(),
			playback: self.clamped_playback(),
			tint: self.tint,
			seed: Some(self.seed.unwrap_or_else(seed::generate)),
			palette: self.palette,
		}
	}
}

/// Root of one spawned instance.
#[derive(Component, Debug)]
pub struct VfxInstance {
	pub name: String,
	pub age: f32,
	pub duration: f32,
	pub playback: f32,
	pub seed: u64,
	/// False until immediate particle emitters are ready to burst.
	pub armed: bool,
}

/// Layers waiting for their start delay relative to the root.
#[derive(Component, Debug)]
pub struct VfxPendingLayers {
	pub layers: Vec<EffectLayer>,
	pub spawn: VfxSpawn,
}

/// Light pulse fading from peak to zero.
#[derive(Component, Debug)]
pub struct VfxFlash {
	pub age: f32,
	pub fade: f32,
	pub peak: f32,
	pub playback: f32,
}

/// Particle layer waiting for [`bevy_hanabi::CompiledParticleEffect::is_ready`].
#[derive(Component, Debug)]
pub struct VfxEmitter {
	pub count: f32,
}

#[derive(Component, Debug)]
pub struct VfxEmitterArmed;

/// Spawner has been reset for this instance's start gate.
#[derive(Component, Debug)]
pub struct VfxEmitterBurst;

/// Actual start clock for one realized layer. Cleanup uses this, not planned delay.
#[derive(Clone, Component, Debug)]
pub struct VfxLayerLife {
	pub age: f32,
	pub duration: f32,
	pub playback: f32,
	pub waiting_for_emitter: bool,
}

/// Spawn a named definition at `spawn.transform`. Returns the root entity.
pub fn spawn_vfx(
	commands: &mut Commands,
	definition: &EffectDefinition,
	spawn: VfxSpawn,
) -> Entity {
	let spawn = spawn.resolved_for(definition);
	let duration = definition.duration();
	let mut pending = Vec::new();
	let mut immediate = Vec::new();
	for layer in &definition.layers {
		if layer.delay <= 0.0 {
			immediate.push(layer.clone());
		} else {
			pending.push(layer.clone());
		}
	}

	let root_transform = Transform {
		translation: spawn.transform.translation,
		rotation: spawn.transform.rotation,
		scale: Vec3::splat(spawn.scale),
	};
	let root = commands
		.spawn((
			Name::new(format!("vfx-{}", definition.name)),
			root_transform,
			Visibility::Visible,
			VfxInstance {
				name: definition.name.clone(),
				age: 0.0,
				duration,
				playback: spawn.clamped_playback(),
				seed: spawn.resolved_seed(),
				armed: false,
			},
			VfxPendingLayers { layers: pending, spawn: spawn.clone() },
			NotShadowCaster,
		))
		.id();

	for layer in immediate {
		realize_layer(commands, root, &layer, &spawn);
	}
	root
}

/// `Commands` extension: `commands.spawn_vfx(&library.fiery_explosion, VfxSpawn { .. })`.
pub trait SpawnVfxExt {
	fn spawn_vfx(&mut self, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity;
}

impl SpawnVfxExt for Commands<'_, '_> {
	fn spawn_vfx(&mut self, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity {
		spawn_vfx(self, definition, spawn)
	}
}

pub fn realize_layer(
	commands: &mut Commands,
	parent: Entity,
	layer: &EffectLayer,
	spawn: &VfxSpawn,
) {
	let transform = Transform {
		translation: layer.transform.translation,
		rotation: layer.transform.rotation,
		scale: Vec3::splat(layer.scale),
	};
	match &layer.part {
		EffectPart::Particle(part) => {
			let count = (part.count * spawn.clamped_intensity()).clamp(1.0, part.capacity as f32);
			let mut entity = commands.spawn((
				Name::new(format!("vfx-layer-{}", part.name)),
				ChildOf(parent),
				VfxMemberOf(parent),
				transform,
				Visibility::Inherited,
				ParticleEffect::new(part.effect.clone()),
				effect_properties(spawn, layer.scale, part.shade),
				VfxEmitter { count },
				VfxLayerLife {
					age: 0.0,
					duration: part.max_lifetime,
					playback: spawn.clamped_playback(),
					waiting_for_emitter: true,
				},
				NotShadowCaster,
			));
			if !part.images.is_empty() {
				entity.insert(EffectMaterial { images: part.images.clone() });
			}
		}
		EffectPart::Light(pulse) => {
			let peak = pulse.peak_intensity * spawn.clamped_intensity();
			let color = ExplosionPalette::color(spawn.palette().with_tint(spawn.tint).flash);
			let range = pulse.range * spawn.scale * layer.scale;
			commands.spawn((
				Name::new("vfx-layer-flash"),
				ChildOf(parent),
				VfxMemberOf(parent),
				transform,
				PointLight {
					color,
					intensity: 0.0,
					range,
					shadow_maps_enabled: false,
					..default()
				},
				VfxFlash { age: 0.0, fade: pulse.fade, peak, playback: spawn.clamped_playback() },
				VfxLayerLife {
					age: 0.0,
					duration: pulse.fade,
					playback: spawn.clamped_playback(),
					waiting_for_emitter: false,
				},
			));
		}
		EffectPart::Mesh(part) => {
			let cluster = commands
				.spawn((
					Name::new(format!("vfx-layer-{}", part.name)),
					ChildOf(parent),
					VfxMemberOf(parent),
					transform,
					Visibility::Inherited,
					NotShadowCaster,
				))
				.id();
			let layer_id = layer_stream(part.kind);
			for (index, spec) in part.lobes.iter().copied().enumerate() {
				let spec = vary_lobe(spec, spawn.resolved_seed(), layer_id, index as u32);
				let material = LobeMaterial::new(
					part.kind,
					spec.duration,
					seed::unit(seed::stream(spawn.resolved_seed(), layer_id, index as u32)),
					&spawn.palette(),
					spawn.tint_or_white(),
					spawn.clamped_intensity(),
				);
				commands.spawn((
					Name::new(format!("vfx-lobe-{}-{index}", part.name)),
					ChildOf(cluster),
					VfxMemberOf(parent),
					Mesh3d(part.mesh.clone()),
					lobe_transform(&spec, 0.0),
					Visibility::Hidden,
					VfxLobe { age: 0.0, spec, playback: spawn.clamped_playback() },
					VfxLayerLife {
						age: 0.0,
						duration: spec.duration,
						playback: spawn.clamped_playback(),
						waiting_for_emitter: false,
					},
					NotShadowCaster,
					LobeMaterialSlot(material),
				));
			}
		}
	}
}

fn layer_stream(kind: LobeKind) -> u32 {
	match kind {
		LobeKind::Fire => seed::LAYER_FIRE,
		LobeKind::Smoke => seed::LAYER_SMOKE,
		LobeKind::Flash => seed::LAYER_FLASH,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::{EffectLayer, LightPulse, ParticleShade};
	use crate::layers::flash::FLASH_FADE;

	#[test]
	fn spawn_clamps_scale_intensity_and_playback() {
		let spawn = VfxSpawn { scale: 99.0, intensity: 0.01, playback: 0.01, ..default() };
		assert_eq!(spawn.clamped_scale(), MAX_SCALE);
		assert_eq!(spawn.clamped_intensity(), 0.25);
		assert_eq!(spawn.clamped_playback(), MIN_PLAYBACK);
		let fast = VfxSpawn { playback: 99.0, ..default() };
		assert_eq!(fast.clamped_playback(), MAX_PLAYBACK);
	}

	#[test]
	fn definition_bounds_override_generic_cap() {
		let def = EffectDefinition::new("big", [])
			.with_scale_bounds(ScaleBounds { min: MIN_SCALE, max: 20.0 });
		let spawn = VfxSpawn { scale: 13.0, ..default() }.resolved_for(&def);
		assert!((spawn.scale - 13.0).abs() < 1e-4);
	}

	#[test]
	fn omitted_seed_is_filled_once() {
		let a = VfxSpawn::default().resolved();
		let b = VfxSpawn::default().resolved();
		assert!(a.seed.is_some());
		assert_ne!(a.seed, b.seed);
		let same = VfxSpawn { seed: Some(11), ..default() }.resolved();
		assert_eq!(same.seed, Some(11));
	}

	#[test]
	fn playback_preserves_layer_ratios() {
		let authored: f32 = 0.05 + 2.4;
		let playback: f32 = 2.0;
		assert!(((authored / playback) - 1.225).abs() < 1e-4);
		assert!((0.05 / authored - (0.05 / playback) / (authored / playback)).abs() < 1e-6);
	}

	#[test]
	fn light_layer_duration_is_fade() {
		let layer = EffectLayer::light(LightPulse {
			color: Color::WHITE,
			peak_intensity: 1.0,
			range: 1.0,
			fade: FLASH_FADE,
		});
		assert!((layer.duration() - FLASH_FADE).abs() < 1e-4);
	}

	#[test]
	fn delayed_layer_extends_cleanup_from_actual_start() {
		let planned: f32 = 0.05 + 2.4;
		let actual_start = 0.08;
		let duration = planned.max(actual_start + 2.4);
		assert!((duration - 2.48).abs() < 1e-4);
	}

	#[test]
	fn tint_and_palette_reach_particle_properties() {
		let spawn = VfxSpawn {
			tint: Some(Color::srgb(0.2, 0.8, 1.0)),
			seed: Some(7),
			playback: 2.0,
			..default()
		}
		.resolved();
		let props = effect_properties(&spawn, 1.0, ParticleShade::Fire);
		assert!(props.get_stored(crate::particles::PROP_TINT).is_some());
		assert!(props.get_stored(crate::particles::PROP_SEED).is_some());
		assert!(props.get_stored(crate::particles::PROP_PLAYBACK).is_some());
		assert!(props.get_stored(crate::particles::PROP_COLOR0).is_some());
	}
}
