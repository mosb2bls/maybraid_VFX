use bevy::{
	prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CheckerboardMaterial {
	#[uniform(0)]
	pub checker_size_m: f32,
	#[uniform(1)]
	pub color1: LinearRgba,
	#[uniform(2)]
	pub color2: LinearRgba,
}

impl Material for CheckerboardMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/checkerboard_material.wgsl".into()
	}
}
