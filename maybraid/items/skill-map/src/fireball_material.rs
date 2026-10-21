//! Shared fireball [`Material`]. One handle for the head and every bead;
//! pose and scale stay on [`Transform`].

use bevy::{
	asset::embedded_asset, light::NotShadowCaster, prelude::*, reflect::TypePath,
	render::render_resource::AsBindGroup, shader::ShaderRef,
};

/// Visual capsule is a bit larger than the hit sphere so the short tail can pull.
pub const FIREBALL_VISUAL_RADIUS: f32 = 1.18;
/// Short aft shaft. The bead train, not this mesh, traces the flight.
pub const FIREBALL_VISUAL_LENGTH: f32 = 0.9;

const FIRE_FILL: Vec4 = Vec4::new(1.0, 0.28, 0.05, 1.0);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FireballMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

impl FireballMaterial {
	pub fn new(intensity: f32) -> Self {
		Self { base_color: FIRE_FILL * intensity.max(0.0) }
	}
}

impl Default for FireballMaterial {
	fn default() -> Self {
		Self::new(1.0)
	}
}

impl Material for FireballMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "fireball_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "fireball_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
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
}

pub struct FireballMaterialPlugin;

impl Plugin for FireballMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "fireball_material.wgsl");
		app.add_plugins(MaterialPlugin::<FireballMaterial>::default());
		app.add_systems(PostUpdate, disable_fireball_shadow_casters);
	}
}

fn disable_fireball_shadow_casters(
	mut commands: Commands,
	query: Query<Entity, (With<MeshMaterial3d<FireballMaterial>>, Without<NotShadowCaster>)>,
) {
	for entity in &query {
		commands.entity(entity).insert(NotShadowCaster);
	}
}

/// Short capsule with the ball at the origin and a small shaft along `-Y`.
pub fn fireball_visual_mesh() -> Mesh {
	Capsule3d::new(FIREBALL_VISUAL_RADIUS, FIREBALL_VISUAL_LENGTH)
		.mesh()
		.latitudes(12)
		.longitudes(20)
		.rings(6)
		.build()
		.translated_by(Vec3::new(0.0, -FIREBALL_VISUAL_LENGTH * 0.5, 0.0))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shared_fill_is_flame() {
		let material = FireballMaterial::new(1.0);
		assert_eq!(material.base_color, FIRE_FILL);
	}

	#[test]
	fn visual_mesh_is_a_short_aft_shaft() {
		let mesh = fireball_visual_mesh();
		let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
			mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			panic!("fireball mesh needs positions");
		};
		let min_y = positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
		let max_y = positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
		assert!(mesh.count_vertices() > 32);
		assert!(max_y < FIREBALL_VISUAL_RADIUS + 0.05);
		assert!(min_y > -(FIREBALL_VISUAL_LENGTH + FIREBALL_VISUAL_RADIUS) - 0.05);
		assert!(min_y < -FIREBALL_VISUAL_LENGTH * 0.5);
	}
}
