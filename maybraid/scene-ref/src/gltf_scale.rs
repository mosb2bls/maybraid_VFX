//! Bake odd negative glTF node scale into mesh winding.
//!
//! Bevy flips cull only for **indexed** materials. A default-material primitive
//! (no `materials` entry) always keeps Back cull, so an odd node scale
//! (`scale.x * scale.y * scale.z < 0`) drops every triangle — the game clear
//! color shows through. Baking the sign into a mesh clone and abs-normalizing
//! the node scale keeps later material stamps on Back cull.

use bevy::camera::primitives::Aabb;
use bevy::mesh::{Mesh, VertexAttributeValues};
use bevy::prelude::{Assets, Children, Entity, Mesh3d, Transform, Vec3, World};

/// Local glTF node scale; detects odd-negative determinants and bakes into meshes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GltfNodeScale(Vec3);

impl GltfNodeScale {
	pub fn new(scale: Vec3) -> Self {
		Self(scale)
	}

	pub fn scale(self) -> Vec3 {
		self.0
	}

	/// True when an odd number of scale axes are negative (determinant < 0).
	pub fn is_odd_negative(self) -> bool {
		self.0.x * self.0.y * self.0.z < 0.0
	}

	/// Per-axis sign used to bake an odd scale into mesh space (`±1`, never 0).
	pub fn sign(self) -> Vec3 {
		Vec3::new(
			if self.0.x < 0.0 { -1.0 } else { 1.0 },
			if self.0.y < 0.0 { -1.0 } else { 1.0 },
			if self.0.z < 0.0 { -1.0 } else { 1.0 },
		)
	}

	/// Flip `mesh` by this scale's sign and invert winding so Back cull matches a positive node scale.
	pub fn bake_mesh(self, mesh: &Mesh) -> Mesh {
		let sign = self.sign();
		let mut out = mesh.clone();
		out.transform_by(Transform::from_scale(sign));
		let _ = out.invert_winding();
		out
	}
}

fn mesh_aabb(mesh: &Mesh) -> Option<Aabb> {
	let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
	else {
		return None;
	};
	Aabb::enclosing(positions.iter().map(|p| Vec3::from(*p)))
}

/// Bake every odd local scale in `world` into a private mesh clone.
///
/// Returns `false` when a `Mesh3d` is still missing from `meshes` so the caller
/// can retry. Nodes that cannot be baked are left unchanged.
pub fn bake_odd_scales_in_world(world: &mut World, meshes: &mut Assets<Mesh>) -> bool {
	let nodes: Vec<(Entity, Vec3)> = world
		.iter_entities()
		.filter_map(|entity| {
			let scale = entity.get::<Transform>()?.scale;
			GltfNodeScale::new(scale).is_odd_negative().then_some((entity.id(), scale))
		})
		.collect();
	let mut ready = true;
	for (node, scale) in nodes {
		ready &= bake_node(world, node, scale, meshes);
	}
	ready
}

fn bake_node(world: &mut World, node: Entity, scale: Vec3, meshes: &mut Assets<Mesh>) -> bool {
	let node_scale = GltfNodeScale::new(scale);
	let mut targets = Vec::new();
	if world.get::<Mesh3d>(node).is_some() {
		targets.push(node);
	}
	if let Some(children) = world.get::<Children>(node) {
		targets.extend(children.iter());
	}

	let mut baked_any = false;
	let mut missing = false;
	for target in targets {
		let Some(old) = world.get::<Mesh3d>(target).map(|m| m.0.clone()) else {
			continue;
		};
		let Some(source) = meshes.get(&old) else {
			missing = true;
			continue;
		};
		let baked = node_scale.bake_mesh(source);
		let aabb = mesh_aabb(&baked);
		let handle = meshes.add(baked);
		if let Ok(mut child) = world.get_entity_mut(target) {
			child.insert(Mesh3d(handle));
			if let Some(aabb) = aabb {
				child.insert(aabb);
			}
		}
		baked_any = true;
	}

	if baked_any {
		if let Some(mut tf) = world.get_mut::<Transform>(node) {
			tf.scale = tf.scale.abs();
		}
	}
	!missing
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::{Indices, PrimitiveTopology};

	fn triangle() -> Mesh {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
		);
		mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
		mesh
	}

	#[test]
	fn bakes_child_mesh_and_abs_node_scale() {
		let mut meshes = Assets::<Mesh>::default();
		let handle = meshes.add(triangle());
		let mut world = World::new();
		let node = world.spawn(Transform::from_scale(Vec3::new(-1.0, -0.2, -1.0))).id();
		world.spawn((Mesh3d(handle), Transform::IDENTITY, bevy::prelude::ChildOf(node)));

		assert!(bake_odd_scales_in_world(&mut world, &mut meshes));
		let scale = world.get::<Transform>(node).unwrap().scale;
		assert!((scale - Vec3::new(1.0, 0.2, 1.0)).length() < 1e-5);

		let child = world.get::<Children>(node).unwrap()[0];
		let baked_handle = &world.get::<Mesh3d>(child).unwrap().0;
		let baked = meshes.get(baked_handle).unwrap();
		match baked.indices() {
			Some(Indices::U32(idx)) => assert_eq!(idx.as_slice(), &[0, 2, 1]),
			other => panic!("unexpected indices: {other:?}"),
		}
	}

	#[test]
	fn odd_negative_scale_detection() {
		assert!(GltfNodeScale::new(Vec3::new(-1.0, -0.2, -1.0)).is_odd_negative());
		assert!(!GltfNodeScale::new(Vec3::new(-1.0, -0.2, 1.0)).is_odd_negative());
		assert!(!GltfNodeScale::new(Vec3::ONE).is_odd_negative());
	}

	#[test]
	fn bake_mesh_restores_front_winding() {
		let baked = GltfNodeScale::new(Vec3::NEG_ONE).bake_mesh(&triangle());
		match baked.indices() {
			Some(Indices::U32(idx)) => assert_eq!(idx.as_slice(), &[0, 2, 1]),
			other => panic!("unexpected indices: {other:?}"),
		}
	}
}
