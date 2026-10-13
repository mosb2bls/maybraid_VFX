//! Placeholder label wireframe mesh / materials (no GLBs, no Text3d).

use std::sync::{Mutex, OnceLock};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

use crate::labels::style::LabelStyle;

/// Strong handle for the shared unit wireframe cube.
#[derive(Resource, Debug, Clone)]
pub struct LabelWireframeAssets {
	pub unit_cube: Handle<Mesh>,
}

static UNIT_CUBE: OnceLock<Handle<Mesh>> = OnceLock::new();
static MATERIALS: OnceLock<Mutex<std::collections::HashMap<u32, Handle<StandardMaterial>>>> =
	OnceLock::new();

impl LabelWireframeAssets {
	/// Unit cube line mesh (must be initialized by [`LabelWireframePlugin`]).
	pub fn unit_cube() -> Handle<Mesh> {
		UNIT_CUBE
			.get()
			.expect("LabelWireframePlugin must run before label scenes")
			.clone()
	}

	/// Unlit material for a label style's wireframe.
	pub fn material_for(style: LabelStyle) -> Handle<StandardMaterial> {
		let key = pack_color(style.color());
		let map = MATERIALS.get().expect("LabelWireframePlugin must run before label scenes");
		let guard = map.lock().expect("label wireframe material map");
		guard
			.get(&key)
			.cloned()
			.expect("label wireframe material registered at plugin startup")
	}

	fn init(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
		let cube = meshes.add(unit_cube_line_mesh());
		let _ = UNIT_CUBE.set(cube.clone());
		let mut map = std::collections::HashMap::new();
		for style in LabelStyle::ALL {
			let color = style.color();
			map.insert(pack_color(color), materials.add(wire_material(color)));
		}
		let _ = MATERIALS.set(Mutex::new(map));
	}
}

fn wire_material(color: Color) -> StandardMaterial {
	StandardMaterial {
		base_color: color,
		unlit: true,
		alpha_mode: AlphaMode::Blend,
		cull_mode: None,
		..default()
	}
}

fn pack_color(color: Color) -> u32 {
	let c = color.to_srgba();
	let r = (c.red.clamp(0.0, 1.0) * 255.0) as u32;
	let g = (c.green.clamp(0.0, 1.0) * 255.0) as u32;
	let b = (c.blue.clamp(0.0, 1.0) * 255.0) as u32;
	let a = (c.alpha.clamp(0.0, 1.0) * 255.0) as u32;
	(r << 24) | (g << 16) | (b << 8) | a
}

/// Unit cube wireframe: edges of the AABB \([-0.5, 0.5]^3\).
fn unit_cube_line_mesh() -> Mesh {
	let h = 0.5_f32;
	let corners = [
		[-h, -h, -h],
		[h, -h, -h],
		[h, h, -h],
		[-h, h, -h],
		[-h, -h, h],
		[h, -h, h],
		[h, h, h],
		[-h, h, h],
	];
	let edges: [[u32; 2]; 12] = [
		[0, 1],
		[1, 2],
		[2, 3],
		[3, 0],
		[4, 5],
		[5, 6],
		[6, 7],
		[7, 4],
		[0, 4],
		[1, 5],
		[2, 6],
		[3, 7],
	];
	let mut positions = Vec::with_capacity(24);
	let mut indices = Vec::with_capacity(24);
	for (i, [a, b]) in edges.iter().enumerate() {
		let i0 = (i * 2) as u32;
		positions.push(corners[*a as usize]);
		positions.push(corners[*b as usize]);
		indices.push(i0);
		indices.push(i0 + 1);
	}
	let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float32x3(positions));
	mesh.insert_indices(Indices::U32(indices));
	mesh
}

/// Registers shared wireframe cube + palette materials for label placeholders.
pub struct LabelWireframePlugin;

impl Plugin for LabelWireframePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_label_wireframes);
	}
}

fn init_label_wireframes(
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut commands: Commands,
) {
	LabelWireframeAssets::init(&mut meshes, &mut materials);
	commands.insert_resource(LabelWireframeAssets { unit_cube: LabelWireframeAssets::unit_cube() });
}
