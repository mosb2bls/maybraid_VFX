//! Warm [`LodSceneHost`] scaffolding for vegetation content.

use bevy::light::NotShadowCaster;
use bevy::prelude::{
	Added, ChildOf, Children, Commands, Component, Entity, Mesh3d, Query, Transform, Visibility,
	With, Without,
};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use lod::scene::LodSceneLevel;
use lod::LodLazyPending;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
use scene_ref::MultiSceneMerge;

use crate::assets::AssetPath;

/// Optional GLB under a transform (no deferred material).
pub fn posed_asset_tier(asset: Option<AssetPath>, transform: Transform) -> impl Scene + 'static {
	posed_material_asset_tier(asset, transform, None)
}

/// Optional GLB under a transform, with [`MaterialRefRoot`] + [`PropagateToDescendants`] on the
/// scene root when `material` is set.
pub fn posed_material_asset_tier(
	asset: Option<AssetPath>,
	transform: Transform,
	material: Option<MaterialRef>,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => vec![Box::new(material_asset_scene(a, material))],
		None => vec![],
	};
	bsn! {
		template_value(transform)
		Visibility::Inherited
		Children [ {children} ]
	}
}

fn material_asset_scene(asset: AssetPath, material: Option<MaterialRef>) -> Box<dyn Scene> {
	let scene = asset.scene_ref().scene();
	match material {
		Some(material) => Box::new((
			bsn! {
				template_value(MaterialRefRoot(material))
				PropagateToDescendants
				LodLazyPending
			},
			scene,
		)),
		None => Box::new(scene),
	}
}

/// Foliage GLB under a transform, with propagating [`MaterialRefRoot`].
pub fn posed_foliage_asset_tier(
	asset: Option<AssetPath>,
	transform: Transform,
	material: MaterialRef,
) -> impl Scene + 'static {
	posed_material_asset_tier(asset, transform, Some(material))
}

/// Frond GLB under a transform, with propagating [`MaterialRefRoot`].
pub fn posed_frond_asset_tier(
	asset: Option<AssetPath>,
	transform: Transform,
	material: MaterialRef,
) -> impl Scene + 'static {
	posed_material_asset_tier(asset, transform, Some(material))
}

/// Merged frond collection posed as one unit, with propagating [`MaterialRefRoot`].
///
/// `merge` parts must already be collection-/unit-local. `transform` is applied on the
/// same entity as [`scene_ref::MultiSceneMergeRoot`] so the whole merged mesh is placed
/// once (not as a parent of an identity child).
pub fn posed_frond_multi_scene_merge(
	merge: MultiSceneMerge,
	transform: Transform,
	material: MaterialRef,
) -> impl Scene + 'static {
	(
		bsn! {
			template_value(MaterialRefRoot(material))
			PropagateToDescendants
			LodLazyPending
		},
		merge.scene_at(transform),
	)
}

/// Cheap-ball collection merge: same as [`posed_frond_multi_scene_merge`], plus
/// [`NotShadowCaster`] on the root (mesh children inherit via
/// [`inherit_not_shadow_caster_on_meshes`]).
pub fn posed_foliage_multi_scene_merge(
	merge: MultiSceneMerge,
	transform: Transform,
	material: MaterialRef,
) -> impl Scene + 'static {
	(
		bsn! {
			NotShadowCaster
			template_value(MaterialRefRoot(material))
			PropagateToDescendants
			LodLazyPending
		},
		merge.scene_at(transform),
	)
}

/// Copy ancestor [`NotShadowCaster`] onto newly spawned `Mesh3d` children.
///
/// Merged [`scene_ref::WorldAsset`](bevy::world_serialization::WorldAsset) instances
/// put `Mesh3d` on a child; Bevy only honors the marker on the mesh entity.
pub fn inherit_not_shadow_caster_on_meshes(
	mut commands: Commands,
	added: Query<Entity, (Added<Mesh3d>, Without<NotShadowCaster>)>,
	parents: Query<&ChildOf>,
	marked: Query<(), With<NotShadowCaster>>,
) {
	for entity in &added {
		let mut cursor = entity;
		loop {
			let Ok(child_of) = parents.get(cursor) else {
				break;
			};
			cursor = child_of.parent();
			if marked.contains(cursor) {
				commands.entity(entity).insert(NotShadowCaster);
				break;
			}
		}
	}
}

/// Warm host whose level roots are arbitrary scene content.
pub fn warm_content_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	roots: impl IntoIterator<Item = (LodSceneLevel, Box<dyn Scene>)>,
) -> impl Scene + 'static {
	let root_scenes: Vec<Box<dyn Scene>> = roots
		.into_iter()
		.map(|(root_level, content)| {
			let visible = level == root_level;
			content_level_root(root_level, content, visible)
		})
		.collect();
	host_with_probe_only(level, probe, Transform::IDENTITY, root_scenes)
}

pub fn warm_content_host_hsl<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	high: impl Scene + 'static,
	mid: impl Scene + 'static,
	low: impl Scene + 'static,
) -> impl Scene + 'static {
	warm_content_host(
		level,
		probe,
		[
			(LodSceneLevel::High, Box::new(high) as Box<dyn Scene>),
			(LodSceneLevel::Medium, Box::new(mid) as Box<dyn Scene>),
			(LodSceneLevel::Low, Box::new(low) as Box<dyn Scene>),
		],
	)
}

/// Warm High / Medium / Low / UltraLow content roots (structural UltraLow tier).
pub fn warm_content_host_hslu<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	high: impl Scene + 'static,
	mid: impl Scene + 'static,
	low: impl Scene + 'static,
	ultra: impl Scene + 'static,
) -> impl Scene + 'static {
	warm_content_host(
		level,
		probe,
		[
			(LodSceneLevel::High, Box::new(high) as Box<dyn Scene>),
			(LodSceneLevel::Medium, Box::new(mid) as Box<dyn Scene>),
			(LodSceneLevel::Low, Box::new(low) as Box<dyn Scene>),
			(LodSceneLevel::UltraLow, Box::new(ultra) as Box<dyn Scene>),
		],
	)
}

/// Warm host with optional per-level mesh assets.
pub fn warm_mesh_level_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	roots: impl IntoIterator<Item = (LodSceneLevel, Option<AssetPath>)>,
) -> impl Scene + 'static {
	let root_list: Vec<(LodSceneLevel, Option<AssetPath>)> = roots.into_iter().collect();
	let root_scenes: Vec<Box<dyn Scene>> = root_list
		.iter()
		.map(|(root_level, asset)| mesh_level_root(*root_level, *asset, level == *root_level, None))
		.collect();
	host_with_probe_only(level, probe, transform, root_scenes)
}

/// Warm host for foliage GLB LOD triads (propagating leaf [`MaterialRef`]).
pub fn warm_foliage_mesh_level_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	material: MaterialRef,
	roots: impl IntoIterator<Item = (LodSceneLevel, Option<AssetPath>)>,
) -> impl Scene + 'static {
	let root_list: Vec<(LodSceneLevel, Option<AssetPath>)> = roots.into_iter().collect();
	let root_scenes: Vec<Box<dyn Scene>> = root_list
		.iter()
		.map(|(root_level, asset)| {
			mesh_level_root(*root_level, *asset, level == *root_level, Some(material.clone()))
		})
		.collect();
	host_with_probe_only(level, probe, transform, root_scenes)
}

/// Warm host for frond GLB LOD triads (propagating leaf [`MaterialRef`]).
pub fn warm_frond_mesh_level_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	material: MaterialRef,
	roots: impl IntoIterator<Item = (LodSceneLevel, Option<AssetPath>)>,
) -> impl Scene + 'static {
	warm_foliage_mesh_level_host(level, probe, transform, material, roots)
}

fn host_with_probe_only<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	roots: Vec<Box<dyn Scene>>,
) -> impl Scene + 'static {
	let level_roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
		Children [ {roots} ]
	});
	let host_children = vec![level_roots];
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(probe)
		template_value(transform)
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

fn mesh_level_root(
	level: LodSceneLevel,
	asset: Option<AssetPath>,
	visible: bool,
	material: Option<MaterialRef>,
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => vec![Box::new(material_asset_scene(a, material))],
		None => vec![],
	};
	let visibility = if visible { Visibility::Inherited } else { Visibility::Hidden };
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}

fn content_level_root(
	level: LodSceneLevel,
	content: Box<dyn Scene>,
	visible: bool,
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = vec![content];
	let visibility = if visible { Visibility::Inherited } else { Visibility::Hidden };
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}
