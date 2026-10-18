//! Per-instance stylized material for fire, smoke, and flash mesh lobes.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::composition::LobeKind;
use crate::palette::ExplosionPalette;

/// Age, seed, tint, kind, and palette packed for one lobe instance.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LobeMaterial {
	/// `x` age, `y` duration, `z` seed, `w` [`LobeKind`] as f32.
	#[uniform(0)]
	pub params: Vec4,
	#[uniform(1)]
	pub tint: Vec4,
	/// `x` emission/gain, `y` deform roll rate, `z` displace, `w` value-band count.
	#[uniform(2)]
	pub extras: Vec4,
	#[uniform(3)]
	pub color_hot: Vec4,
	#[uniform(4)]
	pub color_mid: Vec4,
	#[uniform(5)]
	pub color_cool: Vec4,
}

impl LobeMaterial {
	pub fn new(
		kind: LobeKind,
		duration: f32,
		seed: f32,
		palette: &ExplosionPalette,
		tint: Color,
		intensity: f32,
	) -> Self {
		let tint = LinearRgba::from(tint);
		let (gain, roll, displace, bands) = match kind {
			LobeKind::Fire => (1.15 * intensity, 0.45, 0.24, 4.0),
			LobeKind::Smoke => (0.95 * intensity, 1.85, 0.30, 3.0),
			LobeKind::Flash => (2.4 * intensity, 0.0, 0.10, 2.0),
		};
		let (hot, mid, cool) = match kind {
			LobeKind::Fire => (palette.fire_hot, palette.fire_mid, palette.fire_cool),
			LobeKind::Smoke => (palette.smoke_lit, palette.smoke_lit, palette.smoke_shadow),
			LobeKind::Flash => (palette.flash, palette.fire_hot, palette.fire_mid),
		};
		Self {
			params: Vec4::new(0.0, duration.max(1e-3), seed, kind.as_f32()),
			tint: Vec4::new(tint.red, tint.green, tint.blue, 1.0),
			extras: Vec4::new(gain, roll, displace, bands),
			color_hot: ExplosionPalette::vec4(hot),
			color_mid: ExplosionPalette::vec4(mid),
			color_cool: ExplosionPalette::vec4(cool),
		}
	}

	pub fn set_age(&mut self, age: f32) {
		self.params.x = age.max(0.0);
	}

	pub fn alpha_mode_for_kind(&self) -> AlphaMode {
		if self.params.w > 1.5 {
			AlphaMode::Add
		} else {
			AlphaMode::Blend
		}
	}
}

impl Default for LobeMaterial {
	fn default() -> Self {
		Self::new(LobeKind::Fire, 0.55, 0.0, &ExplosionPalette::maybraid(), Color::WHITE, 1.0)
	}
}

impl Material for LobeMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "lobe_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "lobe_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		self.alpha_mode_for_kind()
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = Some(Face::Back);
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

/// Embedded lobe shader plus [`MaterialPlugin`].
pub struct LobeMaterialPlugin;

impl Plugin for LobeMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "lobe_material.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<LobeMaterial>>() {
			app.add_plugins(MaterialPlugin::<LobeMaterial>::default());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flash_is_additive_fire_and_smoke_blend() {
		let palette = ExplosionPalette::maybraid();
		let fire = LobeMaterial::new(LobeKind::Fire, 0.5, 0.0, &palette, Color::WHITE, 1.0);
		let smoke = LobeMaterial::new(LobeKind::Smoke, 2.0, 1.0, &palette, Color::WHITE, 1.0);
		let flash = LobeMaterial::new(LobeKind::Flash, 0.1, 2.0, &palette, Color::WHITE, 1.0);
		assert_eq!(fire.alpha_mode(), AlphaMode::Blend);
		assert_eq!(smoke.alpha_mode(), AlphaMode::Blend);
		assert_eq!(flash.alpha_mode(), AlphaMode::Add);
		assert!(smoke.extras.z > fire.extras.z);
		assert!(smoke.extras.y > fire.extras.y);
	}
}
