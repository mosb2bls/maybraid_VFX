//! Scene loading via references to avoid loading the same scene multiple times.
//!
//! [`SceneRef`] names a GLB (optionally with a `#SceneN` label) and an optional
//! [`MirrorAxis`]. [`MultiSceneMerge`] combines several refs (each with a local
//! transform) into one mesh [`WorldAsset`].
//!
//! [`SceneRefPlugin`] installs [`SceneRefHandles`] and [`MultiSceneMergeHandles`],
//! which memoize [`Handle<WorldAsset>`]s. [`SceneRef::mirrored`] rebuilds meshes
//! (axis flip + winding reverse); [`SceneRef::reflected`] also conjugates instance
//! transforms. Both are distinct cache keys. Use [`SceneRef::scene`] /
//! [`SceneRefRoot`] or [`MultiSceneMerge::scene`] / [`MultiSceneMergeRoot`];
//! fulfill systems insert [`WorldAssetRoot`] when ready.

mod fulfill;
mod gltf_scale;
mod handles;
mod mirror;
mod multi_merge;
mod scene_ref;
mod world_asset;

use bevy::prelude::{App, Plugin, Resource, Update};

pub use gltf_scale::{bake_odd_scales_in_world, GltfNodeScale};
pub use handles::SceneRefHandles;
pub use multi_merge::{
	MultiSceneMerge, MultiSceneMergeHandles, MultiSceneMergeRoot, MultiScenePart, TransformKey,
};
pub use scene_ref::{MirrorAxis, SceneRef, SceneRefRoot};

use fulfill::{fulfill_multi_scene_merge_roots, fulfill_scene_ref_roots};

/// Per-frame cap on new [`bevy::world_serialization::WorldAssetRoot`] inserts.
///
/// Drain can emit many [`SceneRefRoot`]s in one apply; instance spawn is uncapped
/// unless this binds. Default is unlimited so other hosts stay unchanged.
///
/// [`Self::new_merge_meshes_per_frame`] is a separate cap on **first-time**
/// [`MultiSceneMerge`] bakes (`meshes.add`). Cache hits still fulfill under
/// [`Self::per_frame`] only.
///
/// [`Self::inspect_per_frame`] caps how many pending roots are **probed**
/// (cache lookup / asset-ready check) each tick. Drain can leave thousands of
/// unready [`MultiSceneMergeRoot`]s in the query; without this cap, fulfill
/// hashes every key every frame after the bake budget is spent. `u32::MAX` =
/// walk the whole query (tests / playgrounds).
#[derive(Resource, Debug, Clone, Copy)]
pub struct SceneRefAdmitBudget {
	/// Max SceneRef / MultiSceneMerge roots fulfilled this frame (each path).
	pub per_frame: u32,
	/// Max new merged mesh assets created this frame. `u32::MAX` = unlimited.
	pub new_merge_meshes_per_frame: u32,
	/// Max pending-root probes this frame (cache hits and misses). Round-robin
	/// so later roots are not starved when the front of the query is unready.
	pub inspect_per_frame: u32,
}

impl Default for SceneRefAdmitBudget {
	fn default() -> Self {
		Self {
			per_frame: u32::MAX,
			new_merge_meshes_per_frame: u32::MAX,
			inspect_per_frame: u32::MAX,
		}
	}
}

/// Installs handle caches and fulfills scene / merge roots → [`WorldAssetRoot`].
pub struct SceneRefPlugin;

impl Plugin for SceneRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<SceneRefHandles>()
			.init_resource::<MultiSceneMergeHandles>()
			.init_resource::<SceneRefAdmitBudget>()
			.add_systems(Update, (fulfill_scene_ref_roots, fulfill_multi_scene_merge_roots));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
	use bevy::prelude::Transform;

	#[test]
	fn unlabeled_glb_gets_scene0() -> anyhow::Result<()> {
		let m = SceneRef::glb("urban/foo.glb");
		assert_eq!(m.labeled_path(), "urban/foo.glb#Scene0");
		Ok(())
	}

	#[test]
	fn labeled_path_preserved() -> anyhow::Result<()> {
		let m = SceneRef::glb("urban/foo.glb#Scene1");
		assert_eq!(m.labeled_path(), "urban/foo.glb#Scene1");
		Ok(())
	}

	#[test]
	fn mirror_changes_cache_key() -> anyhow::Result<()> {
		let base = SceneRef::glb("urban/foo.glb");
		let mirrored = base.clone().mirrored(MirrorAxis::X);
		let reflected = base.clone().reflected(MirrorAxis::X);
		assert_ne!(base, mirrored);
		assert_ne!(mirrored, reflected);
		assert_eq!(base.labeled_path(), mirrored.labeled_path());
		assert_eq!(mirrored.mirror, Some(MirrorAxis::X));
		assert!(!mirrored.reflect_instance);
		assert_eq!(reflected.mirror, Some(MirrorAxis::X));
		assert!(reflected.reflect_instance);
		Ok(())
	}

	#[test]
	fn mirror_mesh_flips_axis_and_reverses_winding() -> anyhow::Result<()> {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_NORMAL,
			vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(Indices::U32(vec![0, 1, 2]));

		let mirrored = MirrorAxis::X.mirror_mesh(&mesh);

		let Some(VertexAttributeValues::Float32x3(positions)) =
			mirrored.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!((positions[0][0] - 0.0).abs() < 1e-5);
		assert!((positions[1][0] - (-1.0)).abs() < 1e-5);
		assert!((positions[2][0] - 0.0).abs() < 1e-5);

		let Some(VertexAttributeValues::Float32x3(normals)) =
			mirrored.attribute(Mesh::ATTRIBUTE_NORMAL)
		else {
			anyhow::bail!("expected normals");
		};
		// Uniform +Z normals are unchanged by X-scale (scale_recip.z = 1).
		assert!((normals[0][2] - 1.0).abs() < 1e-5);

		match mirrored.indices() {
			Some(Indices::U32(idx)) => assert_eq!(idx.as_slice(), &[0, 2, 1]),
			other => anyhow::bail!("unexpected indices: {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn admit_budget_default_is_unlimited() {
		let budget = SceneRefAdmitBudget::default();
		assert_eq!(budget.per_frame, u32::MAX);
		assert_eq!(budget.new_merge_meshes_per_frame, u32::MAX);
		assert_eq!(budget.inspect_per_frame, u32::MAX);
	}

	#[test]
	fn odd_scale_is_three_negatives() {
		assert!(GltfNodeScale::new(bevy::prelude::Vec3::new(-1.0, -0.2, -1.0)).is_odd_negative());
		assert!(!GltfNodeScale::new(bevy::prelude::Vec3::new(-1.0, -0.2, 1.0)).is_odd_negative());
		assert!(!GltfNodeScale::new(bevy::prelude::Vec3::ONE).is_odd_negative());
	}

	#[test]
	fn bake_mesh_restores_front_winding() -> anyhow::Result<()> {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
		);
		mesh.insert_indices(Indices::U32(vec![0, 1, 2]));

		let baked = GltfNodeScale::new(bevy::prelude::Vec3::NEG_ONE).bake_mesh(&mesh);
		match baked.indices() {
			Some(Indices::U32(idx)) => assert_eq!(idx.as_slice(), &[0, 2, 1]),
			other => anyhow::bail!("unexpected indices: {other:?}"),
		}
		let Some(VertexAttributeValues::Float32x3(positions)) =
			baked.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!((positions[1][0] + 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn multi_scene_merge_cache_key_uses_transform_bits() -> anyhow::Result<()> {
		let a = MultiSceneMerge::new([MultiScenePart::new(
			SceneRef::glb("a.glb"),
			Transform::from_xyz(1.0, 0.0, 0.0),
		)]);
		let b = MultiSceneMerge::new([MultiScenePart::new(
			SceneRef::glb("a.glb"),
			Transform::from_xyz(1.0, 0.0, 0.0),
		)]);
		let c = MultiSceneMerge::new([MultiScenePart::new(
			SceneRef::glb("a.glb"),
			Transform::from_xyz(1.0, 0.0, 1.0),
		)]);
		assert_eq!(a, b);
		assert_ne!(a, c);
		Ok(())
	}
}
