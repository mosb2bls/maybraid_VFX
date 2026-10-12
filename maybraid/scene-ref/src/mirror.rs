//! Axis-mirror rebuild for [`SceneRef`] meshes / worlds.

use bevy::asset::{AssetId, Handle};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, GlobalTransform, Mesh3d, Quat, Transform};
use bevy::world_serialization::WorldAsset;

use crate::scene_ref::MirrorAxis;

impl MirrorAxis {
	/// Clone `mesh`, negate this axis on positions/normals/tangents, and reverse winding.
	pub fn mirror_mesh(self, mesh: &Mesh) -> Mesh {
		let mut out = mesh.clone();
		out.transform_by(Transform::from_scale(self.scale()));
		// Odd negative scale reverses winding; restore front-face orientation.
		let _ = out.invert_winding();
		out
	}

	/// Reflect `transform` through this axis so it matches a parent scale-flip.
	///
	/// Vertex-only mirroring leaves GLB node translations on the source side of the
	/// axis. Conjugating each local TRS (`S M S`) plus [`Self::mirror_mesh`] is equivalent
	/// to a parent `scale(axis)` with positive scale at the caller.
	pub fn mirror_transform(self, transform: Transform) -> Transform {
		let s = self.scale();
		Transform {
			translation: transform.translation * s,
			rotation: self.mirror_rotation(transform.rotation),
			scale: transform.scale,
		}
	}

	fn mirror_rotation(self, rotation: Quat) -> Quat {
		match self {
			Self::X => Quat::from_xyzw(rotation.x, -rotation.y, -rotation.z, rotation.w),
			Self::Y => Quat::from_xyzw(-rotation.x, rotation.y, -rotation.z, rotation.w),
			Self::Z => Quat::from_xyzw(-rotation.x, -rotation.y, rotation.z, rotation.w),
		}
	}
}

/// Clone `source` and rewrite every `Mesh3d` to a newly registered mirrored mesh.
///
/// When `reflect_instance` is set, also conjugate every [`Transform`] (and
/// [`GlobalTransform`] if present) so hierarchy offsets match a parent axis-flip.
/// Caller must ensure the source handle is
/// [`AssetServer::is_loaded_with_dependencies`] so mesh bytes are in `Assets<Mesh>`.
pub(crate) fn mirror_world_asset(
	source: &WorldAsset,
	axis: MirrorAxis,
	reflect_instance: bool,
	meshes: &mut Assets<Mesh>,
	type_registry: &AppTypeRegistry,
) -> Option<WorldAsset> {
	let mut cloned = source.clone_with(type_registry).ok()?;

	let mut mesh_entities = Vec::new();
	let mut transform_entities = Vec::new();
	for entity in cloned.world.iter_entities() {
		if let Some(mesh3d) = entity.get::<Mesh3d>() {
			mesh_entities.push((entity.id(), mesh3d.0.clone()));
		}
		if reflect_instance && entity.get::<Transform>().is_some() {
			transform_entities.push(entity.id());
		}
	}

	let mut remap: HashMap<AssetId<Mesh>, Handle<Mesh>> = HashMap::default();
	for (entity, old_handle) in mesh_entities {
		let new_handle = if let Some(h) = remap.get(&old_handle.id()) {
			h.clone()
		} else {
			let mirrored = axis.mirror_mesh(meshes.get(&old_handle)?);
			let h = meshes.add(mirrored);
			remap.insert(old_handle.id(), h.clone());
			h
		};
		if let Some(mut mesh3d) = cloned.world.get_mut::<Mesh3d>(entity) {
			*mesh3d = Mesh3d(new_handle);
		}
	}

	for entity in transform_entities {
		let Some(transform) = cloned.world.get::<Transform>(entity).copied() else {
			continue;
		};
		let mirrored = axis.mirror_transform(transform);
		if let Some(mut transform) = cloned.world.get_mut::<Transform>(entity) {
			*transform = mirrored;
		}
		if cloned.world.get::<GlobalTransform>(entity).is_some() {
			if let Some(mut global) = cloned.world.get_mut::<GlobalTransform>(entity) {
				*global = GlobalTransform::from(mirrored);
			}
		}
	}

	Some(cloned)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use std::f32::consts::FRAC_PI_4;

	#[test]
	fn mirror_transform_x_negates_translation_and_conjugates_yaw() {
		let src = Transform {
			translation: Vec3::new(0.2, -0.1, 0.05),
			rotation: Quat::from_rotation_y(FRAC_PI_4),
			scale: Vec3::splat(0.15),
		};
		let mirrored = MirrorAxis::X.mirror_transform(src);
		assert!((mirrored.translation.x + 0.2).abs() < 1e-5);
		assert!((mirrored.translation.y + 0.1).abs() < 1e-5);
		assert!((mirrored.translation.z - 0.05).abs() < 1e-5);
		assert!((mirrored.scale - src.scale).length() < 1e-5);

		let expected_yaw = Quat::from_rotation_y(-FRAC_PI_4);
		assert!(mirrored.rotation.angle_between(expected_yaw) < 1e-4);
	}

	#[test]
	fn mirrored_local_matches_parent_scale_flip() {
		let m = Transform {
			translation: Vec3::new(0.2, -0.1, 0.05),
			rotation: Quat::from_rotation_y(FRAC_PI_4),
			scale: Vec3::splat(0.15),
		};
		let p = Vec3::new(0.3, 0.1, -0.2);
		let parent_flip = Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0));
		let via_parent = parent_flip.transform_point(m.transform_point(p));

		let m_prime = MirrorAxis::X.mirror_transform(m);
		let p_prime = Vec3::new(-p.x, p.y, p.z);
		let via_mirror = m_prime.transform_point(p_prime);

		assert!(
			via_parent.distance(via_mirror) < 1e-5,
			"parent={via_parent:?} mirrored={via_mirror:?}"
		);
	}
}
