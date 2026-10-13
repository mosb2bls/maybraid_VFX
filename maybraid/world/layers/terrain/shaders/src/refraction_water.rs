//! Refraction water material (ported from naturescapes).

use bevy::{
	asset::embedded_asset, prelude::*, reflect::TypePath, render::render_resource::AsBindGroup,
	shader::ShaderRef,
};

/// Registers the embedded WGSL and [`MaterialPlugin`] for [`RefractionWater`].
pub struct RefractionWaterPlugin;

impl Plugin for RefractionWaterPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "refraction_water.wgsl");
		app.add_plugins(MaterialPlugin::<RefractionWater>::default());
	}
}

/// Screen-space refraction water: depth tint, cheap UV warp, and caustic flecks.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RefractionWater {
	/// Overall water tint + how much water color overrides the refracted scene.
	/// Alpha is used as opacity in the shader.
	#[uniform(0)]
	pub water_color: LinearRgba,
	/// Shallow water tint (beach / clear water).
	#[uniform(1)]
	pub shallow_color: LinearRgba,
	/// Deep water tint (ocean / lake depths).
	#[uniform(2)]
	pub deep_color: LinearRgba,
	/// `x` = distortion strength, `y` = depth scale, `z` = close-fade strength.
	#[uniform(3)]
	pub params: Vec4,
}

impl Default for RefractionWater {
	fn default() -> Self {
		Self {
			water_color: LinearRgba::new(0.05, 0.35, 0.45, 0.99),
			shallow_color: LinearRgba::new(0.15, 0.55, 0.60, 1.0),
			deep_color: LinearRgba::new(0.2, 0.4, 0.5, 1.0),
			params: Vec4::new(0.006, 30.0, 1.0, 0.0),
		}
	}
}

impl RefractionWater {
	#[inline]
	pub fn distortion_strength(&self) -> f32 {
		self.params.x
	}

	#[inline]
	pub fn depth_scale(&self) -> f32 {
		self.params.y
	}

	#[inline]
	pub fn close_fade_strength(&self) -> f32 {
		self.params.z
	}
}

impl Material for RefractionWater {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "refraction_water.wgsl").into()
	}

	fn reads_view_transmission_texture(&self) -> bool {
		true
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::AssetPlugin;
	use bevy::pbr::Material;
	use bevy::prelude::App;
	use bevy::shader::ShaderRef;
	use bevy::MinimalPlugins;

	#[test]
	fn default_material_uniforms() {
		let m = RefractionWater::default();
		assert!((m.water_color.alpha - 0.99).abs() < 1e-5);
		assert!((m.distortion_strength() - 0.006).abs() < 1e-6);
		assert!((m.depth_scale() - 30.0).abs() < 1e-5);
		assert!((m.close_fade_strength() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn material_alpha_mode_is_opaque() {
		assert_eq!(RefractionWater::default().alpha_mode(), AlphaMode::Opaque);
	}

	#[test]
	fn fragment_shader_ref_matches_embedded_asset_path() {
		let expected =
			concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "refraction_water.wgsl");
		match <RefractionWater as Material>::fragment_shader() {
			ShaderRef::Path(p) => assert_eq!(p.to_string(), expected),
			ShaderRef::Default => panic!("unexpected Default shader ref"),
			ShaderRef::Handle(_) => panic!("unexpected Handle shader ref"),
		}
	}

	#[test]
	fn embedded_wgsl_registers_with_asset_plugin() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		embedded_asset!(app, "refraction_water.wgsl");
		app.update();
	}
}
