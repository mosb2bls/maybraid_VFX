//! Real-time VFX: particles, flipbooks, mesh lobes, and short-lived composite hosts.
//!
//! Domain shaders and world materials stay in their crates. This lib is the
//! effect graph — concepts land here before they are wired into firearms or world.
//!
//! First delivery ([#945](https://github.com/ramate-io/maybraid/issues/945)):
//! reusable flash / fireball / smoke / sparks layers and the composed
//! `fiery_explosion`.

pub mod assets;
pub mod atlas;
pub mod composition;
pub mod effects;
pub mod flipbook;
pub mod layers;
pub mod library;
pub mod lifecycle;
pub mod lobe_material;
pub mod lobes;
pub mod membership;
pub mod names;
pub mod palette;
pub mod particles;
pub mod seed;
pub mod spawn;

pub use assets::{FlipbookAsset, VfxFlipbooks};
pub use composition::{
	EffectDefinition, EffectLayer, EffectPart, LightPulse, LobeKind, LobeSpec, MeshPart,
	ParticlePart, ParticleShade, ScaleBounds, DEFAULT_SCALE_MAX, DEFAULT_SCALE_MIN,
};
pub use library::{canonicalize_effect_name, VfxLibrary};
pub use lobe_material::{LobeMaterial, LobeMaterialPlugin};
pub use names::{FIERY_EXPLOSION, FIREBALL, FLASH, SMOKE, SPARKS};
pub use palette::ExplosionPalette;
pub use spawn::{spawn_vfx, SpawnVfxExt, VfxInstance, VfxSpawn, MAX_PLAYBACK, MIN_PLAYBACK};

use bevy::prelude::*;
use bevy_hanabi::HanabiPlugin;

use crate::library::setup_vfx_library;
use crate::lifecycle::vfx_lifecycle_plugin;

/// Registers Hanabi, the lobe material, compiled definitions, and instance cleanup.
pub struct VfxPlugin;

impl Plugin for VfxPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<HanabiPlugin>() {
			app.add_plugins(HanabiPlugin);
		}
		if !app.is_plugin_added::<LobeMaterialPlugin>() {
			app.add_plugins(LobeMaterialPlugin);
		}
		app.add_systems(Startup, setup_vfx_library);
		vfx_lifecycle_plugin(app);
	}
}
