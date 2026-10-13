//! Bake per-entity + per-part transforms and concatenate meshes.

use bevy::asset::AssetId;
use bevy::mesh::{Indices, Mesh, MeshVertexAttributeId, VertexAttributeValues};
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, ChildOf, Entity, Mesh3d, Transform, World};
use bevy::world_serialization::WorldAsset;

/// One `Mesh3d` inside a part [`WorldAsset`], with its local-to-root transform.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceMeshInstance {
	pub mesh: AssetId<Mesh>,
	pub transform: Transform,
}

/// Walk `source` once and record every `Mesh3d` + local-to-root transform.
///
/// Missing mesh bytes → [`None`] and do **not** treat the walk as complete
/// (caller must retry). An empty-of-meshes world is also [`None`].
pub(crate) fn collect_source_mesh_instances(
	source: &WorldAsset,
	meshes: &Assets<Mesh>,
) -> Option<Vec<SourceMeshInstance>> {
	let mut instances = Vec::new();
	for entity_ref in source.world.iter_entities() {
		let Some(mesh3d) = entity_ref.get::<Mesh3d>() else {
			continue;
		};
		// Same mesh asset may appear on multiple entities with different transforms.
		let _ = meshes.get(&mesh3d.0)?;
		instances.push(SourceMeshInstance {
			mesh: mesh3d.0.id(),
			transform: entity_transform_in_world(&source.world, entity_ref.id()),
		});
	}
	(!instances.is_empty()).then_some(instances)
}

/// Bake `part_transform * instance.transform` for each cached source instance.
pub(crate) fn bake_source_instances(
	instances: &[SourceMeshInstance],
	part_transform: Transform,
	meshes: &Assets<Mesh>,
	prepared: &mut Vec<Mesh>,
) -> Option<()> {
	prepared.reserve(instances.len());
	for instance in instances {
		let mesh = meshes.get(instance.mesh)?;
		let baked = part_transform.mul_transform(instance.transform);
		prepared.push(bake_mesh(mesh, baked));
	}
	Some(())
}

/// Collect every `Mesh3d` in `source`, bake `part_transform * entity_transform`,
/// then merge into one mesh.
///
/// Attributes not shared by every source mesh are dropped so [`Mesh::merge`] stays
/// vertex-count consistent. Materials are ignored.
#[cfg(test)]
pub(crate) fn merge_world_asset_meshes(
	source: &WorldAsset,
	part_transform: Transform,
	meshes: &Assets<Mesh>,
) -> Option<Mesh> {
	let instances = collect_source_mesh_instances(source, meshes)?;
	let mut prepared = Vec::new();
	bake_source_instances(&instances, part_transform, meshes, &mut prepared)?;
	merge_meshes(prepared)
}

/// Merge already-baked meshes (tests / callers that skip WorldAsset walk).
pub(crate) fn merge_meshes(mut prepared: Vec<Mesh>) -> Option<Mesh> {
	if prepared.is_empty() {
		return None;
	}

	retain_common_attributes(&mut prepared);

	let extra_verts = prepared.iter().skip(1).map(Mesh::count_vertices).sum::<usize>();
	let extra_indices = prepared.iter().skip(1).map(index_len).sum::<usize>();

	let mut iter = prepared.into_iter();
	let mut merged = iter.next()?;
	reserve_mesh(&mut merged, extra_verts, extra_indices);
	for other in iter {
		merged.merge(&other).ok()?;
	}
	Some(merged)
}

fn index_len(mesh: &Mesh) -> usize {
	mesh.indices().map(Indices::len).unwrap_or(0)
}

fn reserve_mesh(mesh: &mut Mesh, extra_verts: usize, extra_indices: usize) {
	if extra_verts > 0 {
		let ids: Vec<_> = mesh.attributes().map(|(attr, _)| attr.id).collect();
		for id in ids {
			if let Some(values) = mesh.attribute_mut(id) {
				reserve_attribute_values(values, extra_verts);
			}
		}
	}
	if extra_indices == 0 {
		return;
	}
	match mesh.indices_mut() {
		Some(Indices::U32(indices)) => indices.reserve(extra_indices),
		Some(Indices::U16(indices)) => indices.reserve(extra_indices),
		None => {}
	}
}

fn reserve_attribute_values(values: &mut VertexAttributeValues, extra: usize) {
	match values {
		VertexAttributeValues::Float32(v) => v.reserve(extra),
		VertexAttributeValues::Sint32(v) => v.reserve(extra),
		VertexAttributeValues::Uint32(v) => v.reserve(extra),
		VertexAttributeValues::Float32x2(v) => v.reserve(extra),
		VertexAttributeValues::Sint32x2(v) => v.reserve(extra),
		VertexAttributeValues::Uint32x2(v) => v.reserve(extra),
		VertexAttributeValues::Float32x3(v) => v.reserve(extra),
		VertexAttributeValues::Sint32x3(v) => v.reserve(extra),
		VertexAttributeValues::Uint32x3(v) => v.reserve(extra),
		VertexAttributeValues::Sint32x4(v) => v.reserve(extra),
		VertexAttributeValues::Uint32x4(v) => v.reserve(extra),
		VertexAttributeValues::Float32x4(v) => v.reserve(extra),
		VertexAttributeValues::Sint16x2(v) => v.reserve(extra),
		VertexAttributeValues::Snorm16x2(v) => v.reserve(extra),
		VertexAttributeValues::Uint16x2(v) => v.reserve(extra),
		VertexAttributeValues::Unorm16x2(v) => v.reserve(extra),
		VertexAttributeValues::Sint16x4(v) => v.reserve(extra),
		VertexAttributeValues::Snorm16x4(v) => v.reserve(extra),
		VertexAttributeValues::Uint16x4(v) => v.reserve(extra),
		VertexAttributeValues::Unorm16x4(v) => v.reserve(extra),
		VertexAttributeValues::Sint8x2(v) => v.reserve(extra),
		VertexAttributeValues::Snorm8x2(v) => v.reserve(extra),
		VertexAttributeValues::Uint8x2(v) => v.reserve(extra),
		VertexAttributeValues::Unorm8x2(v) => v.reserve(extra),
		VertexAttributeValues::Sint8x4(v) => v.reserve(extra),
		VertexAttributeValues::Snorm8x4(v) => v.reserve(extra),
		VertexAttributeValues::Uint8x4(v) => v.reserve(extra),
		VertexAttributeValues::Unorm8x4(v) => v.reserve(extra),
		_ => {}
	}
}

fn bake_mesh(mesh: &Mesh, transform: Transform) -> Mesh {
	let kit_local = kit_local_colors(mesh, part_scale(&transform));
	// Copy attributes except COLOR (replaced by kit-local). Avoids clone+overwrite
	// of a vertex-color channel the kit GLB may already carry.
	let mut out = Mesh::new(mesh.primitive_topology(), mesh.asset_usage);
	for (attr, values) in mesh.attributes() {
		if attr.id == Mesh::ATTRIBUTE_COLOR.id {
			continue;
		}
		out.insert_attribute(*attr, values.clone());
	}
	if let Some(indices) = mesh.indices() {
		out.insert_indices(indices.clone());
	}
	if let Some(colors) = kit_local {
		out.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
	}
	out.transform_by(transform);
	if transform.scale.x * transform.scale.y * transform.scale.z < 0.0 {
		let _ = out.invert_winding();
	}
	out
}

fn part_scale(transform: &Transform) -> f32 {
	transform.scale.abs().max_element().max(1e-4)
}

/// Pre-bake [`POSITION`](Mesh::ATTRIBUTE_POSITION) as [`COLOR`](Mesh::ATTRIBUTE_COLOR).
///
/// Object-space materials treat `local_pos` as a unit kit. Merge bakes part
/// transforms into `POSITION`, so those shaders read `COLOR.xyz` instead when
/// `VERTEX_COLORS` is set. `COLOR.w` is the part bake scale (max axis).
fn kit_local_colors(mesh: &Mesh, scale: f32) -> Option<Vec<[f32; 4]>> {
	let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
	else {
		return None;
	};
	Some(positions.iter().map(|p| [p[0], p[1], p[2], scale]).collect())
}

/// Local-to-scene-root transform by walking [`ChildOf`] + [`Transform`].
fn entity_transform_in_world(world: &World, entity: Entity) -> Transform {
	let mut transform = world.get::<Transform>(entity).copied().unwrap_or(Transform::IDENTITY);
	let mut current = entity;
	while let Some(child_of) = world.get::<ChildOf>(current) {
		current = child_of.parent();
		if let Some(parent_tf) = world.get::<Transform>(current) {
			transform = parent_tf.mul_transform(transform);
		}
	}
	transform
}

/// Drop attributes that are not present (same id + format) on every mesh.
///
/// Format identity uses [`std::mem::discriminant`] on [`VertexAttributeValues`] —
/// this is merge-time compatibility only, not part of the [`MultiSceneMerge`] cache key.
fn retain_common_attributes(meshes: &mut [Mesh]) {
	if meshes.is_empty() {
		return;
	}

	let mut common: Option<
		HashMap<MeshVertexAttributeId, std::mem::Discriminant<VertexAttributeValues>>,
	> = None;
	for mesh in meshes.iter() {
		let mut attrs = HashMap::default();
		for (attr, values) in mesh.attributes() {
			attrs.insert(attr.id, std::mem::discriminant(values));
		}
		common = Some(match common.take() {
			None => attrs,
			Some(prev) => prev.into_iter().filter(|(id, key)| attrs.get(id) == Some(key)).collect(),
		});
	}

	let Some(common) = common else {
		return;
	};

	for mesh in meshes.iter_mut() {
		let drop: Vec<_> = mesh
			.attributes()
			.filter(|(attr, values)| {
				common.get(&attr.id).is_none_or(|key| *key != std::mem::discriminant(*values))
			})
			.map(|(attr, _)| attr.id)
			.collect();
		for id in drop {
			mesh.remove_attribute(id);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};

	fn triangle(offset: [f32; 3]) -> Mesh {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![
				[offset[0], offset[1], offset[2]],
				[offset[0] + 1.0, offset[1], offset[2]],
				[offset[0], offset[1] + 1.0, offset[2]],
			],
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_NORMAL,
			vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
		mesh
	}

	#[test]
	fn merge_concatenates_vertices_and_rebases_indices() -> anyhow::Result<()> {
		let merged = merge_meshes(vec![triangle([0.0, 0.0, 0.0]), triangle([10.0, 0.0, 0.0])])
			.expect("merge");
		assert_eq!(merged.count_vertices(), 6);
		match merged.indices() {
			Some(Indices::U32(idx)) => assert_eq!(idx.as_slice(), &[0, 1, 2, 3, 4, 5]),
			other => anyhow::bail!("unexpected indices: {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn bake_applies_translation() -> anyhow::Result<()> {
		let baked = bake_mesh(&triangle([0.0, 0.0, 0.0]), Transform::from_xyz(2.0, 0.0, 0.0));
		let Some(VertexAttributeValues::Float32x3(positions)) =
			baked.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!((positions[0][0] - 2.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn bake_packs_kit_local_into_color() -> anyhow::Result<()> {
		let baked = bake_mesh(&triangle([0.3, 0.0, 0.0]), Transform::from_xyz(2.0, 0.0, 0.0));
		let Some(VertexAttributeValues::Float32x4(colors)) = baked.attribute(Mesh::ATTRIBUTE_COLOR)
		else {
			anyhow::bail!("expected colors");
		};
		assert!((colors[0][0] - 0.3).abs() < 1e-5);
		assert!((colors[1][0] - 1.3).abs() < 1e-5);
		assert!((colors[0][3] - 1.0).abs() < 1e-5);
		let Some(VertexAttributeValues::Float32x3(positions)) =
			baked.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!((positions[0][0] - 2.3).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn bake_packs_part_scale_into_color_w() -> anyhow::Result<()> {
		let transform = Transform {
			translation: bevy::prelude::Vec3::new(2.0, 0.0, 0.0),
			scale: bevy::prelude::Vec3::splat(0.25),
			..Transform::IDENTITY
		};
		let baked = bake_mesh(&triangle([0.3, 0.0, 0.0]), transform);
		let Some(VertexAttributeValues::Float32x4(colors)) = baked.attribute(Mesh::ATTRIBUTE_COLOR)
		else {
			anyhow::bail!("expected colors");
		};
		assert!((colors[0][0] - 0.3).abs() < 1e-5);
		assert!((colors[0][3] - 0.25).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn collect_instances_skips_worlds_without_mesh_bytes() -> anyhow::Result<()> {
		let mut world = World::new();
		let missing = bevy::prelude::Handle::<Mesh>::default();
		world.spawn((Mesh3d(missing), Transform::IDENTITY));
		let source = WorldAsset::new(world);
		let meshes = Assets::<Mesh>::default();
		assert!(collect_source_mesh_instances(&source, &meshes).is_none());
		Ok(())
	}

	#[test]
	fn many_parts_of_one_kit_match_per_part_merges() -> anyhow::Result<()> {
		let mut meshes = Assets::<Mesh>::default();
		let handle = meshes.add(triangle([0.0, 0.0, 0.0]));
		let mut world = World::new();
		world.spawn((Mesh3d(handle.clone()), Transform::IDENTITY));
		let source = WorldAsset::new(world);
		let instances = collect_source_mesh_instances(&source, &meshes).expect("layout");
		assert_eq!(instances.len(), 1);

		let mut flat = Vec::new();
		let mut stacked = Vec::new();
		for i in 0..8 {
			let part = Transform::from_xyz(i as f32, 0.0, 0.0);
			bake_source_instances(&instances, part, &meshes, &mut flat).expect("bake");
			stacked.push(merge_world_asset_meshes(&source, part, &meshes).expect("part"));
		}
		let a = merge_meshes(flat).expect("flat");
		let b = merge_meshes(stacked).expect("stacked");
		assert_eq!(a.count_vertices(), b.count_vertices());
		assert_eq!(a.count_vertices(), 8 * 3);
		Ok(())
	}

	#[test]
	fn retain_drops_unshared_attributes() -> anyhow::Result<()> {
		let mut a = triangle([0.0, 0.0, 0.0]);
		a.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
		let b = triangle([1.0, 0.0, 0.0]);
		let merged = merge_meshes(vec![a, b]).expect("merge");
		assert!(merged.attribute(Mesh::ATTRIBUTE_UV_0).is_none());
		assert!(merged.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
		Ok(())
	}
}
