//! Compiled handles and named definitions, loaded once.

use bevy::prelude::*;
use bevy_hanabi::prelude::{EffectMaterial, ParticleEffect};
use bevy_hanabi::EffectAsset;

use crate::assets::{fire_flipbook, smoke_flipbook, spark_taper, VfxFlipbooks};
use crate::composition::EffectDefinition;
use crate::effects::fiery_explosion;
use crate::layers::{fireball, flash, smoke, sparks};
use crate::lobes::rounded_lobe_mesh;
use crate::names::{FIERY_EXPLOSION, FIREBALL, FLASH, SMOKE, SPARKS};

/// Shared definitions. Clone a handle out; never mutate the compiled assets.
#[derive(Resource, Clone, Debug)]
pub struct VfxLibrary {
	pub flipbooks: VfxFlipbooks,
	pub flash: EffectDefinition,
	pub fireball: EffectDefinition,
	pub smoke: EffectDefinition,
	pub sparks: EffectDefinition,
	pub fiery_explosion: EffectDefinition,
}

impl VfxLibrary {
	pub fn get(&self, name: &str) -> Option<&EffectDefinition> {
		match canonicalize_effect_name(name)? {
			FLASH => Some(&self.flash),
			FIREBALL => Some(&self.fireball),
			SMOKE => Some(&self.smoke),
			SPARKS => Some(&self.sparks),
			FIERY_EXPLOSION => Some(&self.fiery_explosion),
			_ => None,
		}
	}
}

/// Accept kebab, snake, and a few aliases used by the playground.
pub fn canonicalize_effect_name(name: &str) -> Option<&'static str> {
	match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
		"flash" => Some(FLASH),
		"fireball" | "fire" => Some(FIREBALL),
		"smoke" => Some(SMOKE),
		"sparks" | "spark" => Some(SPARKS),
		"fiery-explosion" | "explosion" => Some(FIERY_EXPLOSION),
		_ => None,
	}
}

pub fn setup_vfx_library(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut effects: ResMut<Assets<EffectAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
) {
	let lobe_mesh = meshes.add(rounded_lobe_mesh());
	let flipbooks = VfxFlipbooks {
		fire: fire_flipbook(&mut images),
		smoke: smoke_flipbook(&mut images),
		spark: spark_taper(&mut images),
	};
	let flash_mesh = flash::compile_mesh(lobe_mesh.clone());
	let fireball_mesh = fireball::compile_mesh(lobe_mesh.clone());
	let fireball_wisps = fireball::compile_wisps(&mut effects, &flipbooks.fire);
	let smoke_mesh = smoke::compile_mesh(lobe_mesh);
	let smoke_wisps = smoke::compile_wisps(&mut effects, &flipbooks.smoke);
	let sparks_part = sparks::compile(&mut effects, flipbooks.spark.clone());
	let library = VfxLibrary {
		flash: flash::definition(flash_mesh.clone()),
		fireball: fireball::definition(fireball_mesh.clone(), fireball_wisps.clone()),
		smoke: smoke::definition(smoke_mesh.clone(), smoke_wisps.clone()),
		sparks: sparks::definition(sparks_part.clone()),
		fiery_explosion: fiery_explosion::definition(
			flash_mesh,
			fireball_mesh,
			fireball_wisps.clone(),
			smoke_mesh,
			smoke_wisps.clone(),
			sparks_part.clone(),
		),
		flipbooks,
	};
	for part in [&fireball_wisps, &smoke_wisps, &sparks_part] {
		let mut warmup = commands.spawn((
			Name::new(format!("vfx-warmup-{}", part.name)),
			ParticleEffect::new(part.effect.clone()),
			Transform::from_xyz(0.0, -80.0, 0.0),
			Visibility::Hidden,
		));
		if !part.images.is_empty() {
			warmup.insert(EffectMaterial { images: part.images.clone() });
		}
	}
	commands.insert_resource(library);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn aliases_resolve_to_canonical_names() {
		assert_eq!(canonicalize_effect_name("fiery_explosion"), Some(FIERY_EXPLOSION));
		assert_eq!(canonicalize_effect_name("fiery-explosion"), Some(FIERY_EXPLOSION));
		assert_eq!(canonicalize_effect_name("Fire"), Some(FIREBALL));
		assert_eq!(canonicalize_effect_name("nope"), None);
	}
}
