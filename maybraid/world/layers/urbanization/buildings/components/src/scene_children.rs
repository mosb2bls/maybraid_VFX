//! Shared BSN helpers for composing child scenes and poses.

use bevy::prelude::{
	Children, Handle, Mesh, Mesh3d, MeshMaterial3d, StandardMaterial, Transform, Visibility,
};
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::assets::AssetPath;
use crate::placed::Placement;

/// Group child scenes under a hierarchy-safe root.
///
/// Bevy B0004: ancestors of entities with [`Transform`] / [`Visibility`] must
/// also carry those components, or `GlobalTransform` / visibility inheritance
/// breaks (meshes pile at the origin / vanish).
pub fn scene_children(children: Vec<Box<dyn Scene>>) -> impl Scene + 'static {
	bsn! {
		Transform::default()
		Visibility::default()
		Children [ {children} ]
	}
}

/// Build a [`Transform`] from a cell-space [`Placement`].
pub fn pose(placement: Placement) -> Transform {
	Transform::from_translation(placement.translation)
		.with_rotation(placement.rotation())
		.with_scale(placement.scale)
}

/// GLB asset with an applied pose transform.
pub fn posed_glb(asset: AssetPath, transform: Transform) -> impl Scene + 'static {
	(
		asset.scene_ref().scene(),
		bsn! {
			template_value(transform)
		},
	)
}

/// Wrap a child scene with an applied pose transform.
///
/// Merges [`Transform`] onto the child's root entity. Prefer [`posed_scene`] when
/// the child already inserts its own [`Transform`] (e.g. [`scene_children`]), so
/// the pose becomes a parent rather than fighting the child's identity transform.
pub fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

/// Parent `child` under a new root carrying `transform` (and [`Visibility`]).
///
/// Use this to apply a world/cell pose to a group whose root already has
/// [`Transform`] (see [`scene_children`]).
pub fn posed_scene(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(child)];
	bsn! {
		template_value(transform)
		Visibility::default()
		Children [ {children} ]
	}
}

/// Posed line-list cube using pre-registered mesh/material handles.
///
/// Unit mesh spans \([-0.5, 0.5]^3\); use transform scale as full edge lengths.
pub fn wireframe_box_with_handles(
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
	transform: Transform,
) -> impl Scene + 'static {
	bsn! {
		Mesh3d({mesh})
		MeshMaterial3d::<StandardMaterial>({material})
		template_value(transform)
		Visibility::default()
	}
}
