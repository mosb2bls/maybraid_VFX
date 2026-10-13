//! Terrain-mesh bump-out [`Material`] — neighborhood displacement and opaque cheese.

use bevy::{
	asset::embedded_asset,
	light::NotShadowCaster,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};
use material_ref::{
	MaterialRasters, MaterialRef, MATERIAL_PALETTE_SLOTS, MATERIAL_RASTER_CHANNELS,
	MATERIAL_RASTER_WIDTH, MATERIAL_SCALAR_FLOATS,
};

/// Named recipe resolved to [`BumpOutMaterial`] by domain / composed material libs.
pub const BUMP_OUT_MATERIAL: &str = "chico_bump_out";

/// Shader channel: neighborhood density.
pub const RASTER_DENSITY: usize = 0;
/// Shader channel: world-space bite diameter.
pub const RASTER_BITE_SIZE: usize = 1;
/// Shader channel: bite-size deviation in binary octaves.
pub const RASTER_BITE_SIZE_DEVIATION: usize = 2;
/// Shader channel: typical canopy / cover height.
pub const RASTER_AVERAGE_HEIGHT: usize = 3;
/// Shader channel: height deviation around the typical height.
pub const RASTER_HEIGHT_DEVIATION: usize = 4;

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;

/// Matches `BumpOutStyle::default()` when a ref carries no style scalars.
const DEFAULT_STYLE: [f32; 7] = [0.04, 0.92, 0.25, 0.75, 1.0, 3.5, 0.18];

/// Packed, fixed-layout GPU representation of one material reference.
///
/// Channel meaning is a shader contract. Bump-out uses rasters 0–4 and scalars 0–6.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct BumpOutUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` broad noise frequency, `y` amplitude, `z` seed.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
}

impl BumpOutUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let fallback = [
			Vec4::new(0.16, 0.36, 0.14, 1.0),
			Vec4::new(0.24, 0.52, 0.20, 1.0),
			Vec4::new(0.38, 0.64, 0.24, 1.0),
		];
		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		if material_ref.palette.is_empty() {
			for (slot, color) in colors.iter_mut().zip(fallback) {
				*slot = color;
			}
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				let linear = LinearRgba::from(*color);
				*slot = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
			}
			let first = colors[0];
			for color in colors.iter_mut().take(3).skip(material_ref.palette.len()) {
				*color = first;
			}
		}

		let mut scalars = [Vec4::ZERO; SCALAR_VEC4S];
		let values = material_ref.scalar_values();
		for (i, slot) in scalars.iter_mut().enumerate() {
			let base = i * 4;
			*slot = Vec4::new(
				values.get(base).copied().unwrap_or(0.0),
				values.get(base + 1).copied().unwrap_or(0.0),
				values.get(base + 2).copied().unwrap_or(0.0),
				values.get(base + 3).copied().unwrap_or(0.0),
			);
		}
		if values.is_empty() {
			scalars[0] =
				Vec4::new(DEFAULT_STYLE[0], DEFAULT_STYLE[1], DEFAULT_STYLE[2], DEFAULT_STYLE[3]);
			scalars[1] = Vec4::new(DEFAULT_STYLE[4], DEFAULT_STYLE[5], DEFAULT_STYLE[6], 0.0);
		}

		let mut rasters = [[Vec4::ZERO; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS];
		for channel in 0..MATERIAL_RASTER_CHANNELS {
			let default = match channel {
				RASTER_DENSITY => 1.0,
				RASTER_BITE_SIZE => 12.0,
				_ => 0.0,
			};
			let samples = material_ref.rasters.get_or(channel, default);
			let rows = MaterialRasters::packed_rows(samples);
			rasters[channel] = rows.map(|row| Vec4::from_array(row));
		}

		Self {
			colors,
			noise: Vec4::new(
				material_ref.noise.frequency.max(1e-6),
				material_ref.noise.amplitude,
				material_ref.noise.seed as f32,
				0.0,
			),
			scalars,
			rasters,
		}
	}
}

impl Default for BumpOutUniform {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(BUMP_OUT_MATERIAL))
	}
}

/// Vertex-displaced, noise-masked material used by ground-cover and canopy bump outs.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct BumpOutMaterial {
	#[uniform(0)]
	pub uniform: BumpOutUniform,
}

impl BumpOutMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { uniform: BumpOutUniform::from_material_ref(material_ref) }
	}
}

impl Material for BumpOutMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "bump_out_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "bump_out_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn enable_prepass() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		Ok(())
	}
}

/// Registers the embedded shader and Bevy material asset.
pub struct BumpOutMaterialPlugin;

impl Plugin for BumpOutMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "bump_out_material.wgsl");
		app.add_plugins(MaterialPlugin::<BumpOutMaterial>::default())
			.add_systems(PostUpdate, disable_bump_out_shadow_casters);
	}
}

fn disable_bump_out_shadow_casters(
	mut commands: Commands,
	query: Query<Entity, (With<MeshMaterial3d<BumpOutMaterial>>, Without<NotShadowCaster>)>,
) {
	for entity in &query {
		commands.entity(entity).insert(NotShadowCaster);
	}
}
