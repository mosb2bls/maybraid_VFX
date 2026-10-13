//! Posed furniture GLBs with deferred [`MaterialRef`] paint.
//!
//! One [`scene_ref::SceneRef`] per part — do not [`scene_ref::MultiSceneMerge`].

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use building_components::{pose, scene_children, AssetPath};
use lod::LodLazyPending;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};

use crate::parts::{FurnitureKitPart, PlacedPart};

/// Authored kit under a transform, with propagating [`MaterialRefRoot`].
pub fn posed_kit(
	asset: AssetPath,
	material: MaterialRef,
	transform: Transform,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![Box::new((
		bsn! {
			template_value(MaterialRefRoot(material))
			PropagateToDescendants
			LodLazyPending
		},
		asset.scene_ref().scene(),
	))];
	bsn! {
		template_value(transform)
		Visibility::default()
		Children [ {children} ]
	}
}

/// [`posed_kit`] plus a [`FurnitureKitPart`] on the posed root (the transform world swings).
pub fn posed_kit_part(
	asset: AssetPath,
	material: MaterialRef,
	transform: Transform,
	part: FurnitureKitPart,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![Box::new((
		bsn! {
			template_value(MaterialRefRoot(material))
			PropagateToDescendants
			LodLazyPending
		},
		asset.scene_ref().scene(),
	))];
	bsn! {
		template_value(transform)
		template_value(part)
		Visibility::default()
		Children [ {children} ]
	}
}

/// Instance each part as its own posed GLB.
pub fn assembly_scene(parts: &[PlacedPart]) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = parts
		.iter()
		.map(|part| {
			Box::new(posed_kit(part.kind.asset_path(), part.material.clone(), pose(part.placement)))
				as Box<dyn Scene>
		})
		.collect();
	scene_children(children)
}
