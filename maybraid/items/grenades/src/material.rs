//! Stylized sci-fi grenade look: chrome equator, gadget POM.

use bevy::{
	asset::embedded_asset,
	prelude::*,
	reflect::TypePath,
	render::render_resource::{AsBindGroup, ShaderType},
	shader::ShaderRef,
};

const SHELL: Color = Color::srgb(0.10, 0.14, 0.16);
const CHROME: Color = Color::srgb(0.82, 0.86, 0.92);
const LED: Color = Color::srgb(0.12, 0.92, 1.0);
const POM_DEPTH_M: f32 = 0.016;

/// Packed GPU look. `shell.w` / `chrome.w` are roughness; `extras.x` is POM depth.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct GrenadeMaterialUniform {
	pub shell: Vec4,
	pub chrome: Vec4,
	pub led: Vec4,
	pub extras: Vec4,
}

/// Opaque PBR grenade shell with a chrome belt and relief gadgetry.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GrenadeMaterial {
	#[uniform(0)]
	pub params: GrenadeMaterialUniform,
}

impl GrenadeMaterial {
	pub fn standard() -> Self {
		let shell = LinearRgba::from(SHELL);
		let chrome = LinearRgba::from(CHROME);
		let led = LinearRgba::from(LED);
		Self {
			params: GrenadeMaterialUniform {
				shell: Vec4::new(shell.red, shell.green, shell.blue, 0.40),
				chrome: Vec4::new(chrome.red, chrome.green, chrome.blue, 0.10),
				led: Vec4::new(led.red, led.green, led.blue, 1.6),
				extras: Vec4::new(POM_DEPTH_M, 0.0, 0.0, 0.0),
			},
		}
	}
}

impl Default for GrenadeMaterial {
	fn default() -> Self {
		Self::standard()
	}
}

impl Material for GrenadeMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "grenade_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "grenade_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}

pub fn grenade_material() -> GrenadeMaterial {
	GrenadeMaterial::standard()
}

/// Embedded shader plus [`MaterialPlugin`].
pub struct GrenadeMaterialPlugin;

impl Plugin for GrenadeMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "grenade_material.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<GrenadeMaterial>>() {
			app.add_plugins(MaterialPlugin::<GrenadeMaterial>::default());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn standard_is_opaque_chrome_shell() {
		let material = GrenadeMaterial::standard();
		assert_eq!(material.alpha_mode(), AlphaMode::Opaque);
		assert!(material.params.extras.x > 0.0);
		assert!(material.params.chrome.w < material.params.shell.w);
	}
}
