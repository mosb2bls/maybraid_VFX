//! Slot wireframes plus posed furniture GLBs.

use bevy::scene::Scene;
use building_components::{pose, scene_children, wireframe_box_with_handles, FurnitureNode};
use furniture_components::assembly_scene;

use crate::fill::{abutment_wall, posed_assembly};
use crate::plugin::FurnitureKitMeshes;

/// Slot wireframe + painted GLB parts + optional abutment wall.
pub fn filled_slot_scene(node: &FurnitureNode, kits: &FurnitureKitMeshes) -> impl Scene + 'static {
	use building_components::furniture::FurnitureWireframeAssets;

	let mut children: Vec<Box<dyn Scene>> = vec![Box::new(wireframe_box_with_handles(
		FurnitureWireframeAssets::unit_cube(),
		FurnitureWireframeAssets::material_for(node.geometry),
		pose(node.placement),
	))];
	if let Some(parts) = posed_assembly(node) {
		children.push(Box::new(assembly_scene(&parts)));
	}
	if let Some(wall) = abutment_wall(node) {
		children.push(Box::new(wireframe_box_with_handles(
			kits.unit_cube.clone(),
			kits.wall.clone(),
			pose(wall),
		)));
	}
	scene_children(children)
}

/// Group several filled slots under one root.
pub fn filled_slots_scene<'a>(
	nodes: impl IntoIterator<Item = &'a FurnitureNode>,
	kits: &FurnitureKitMeshes,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = nodes
		.into_iter()
		.map(|node| Box::new(filled_slot_scene(node, kits)) as Box<dyn Scene>)
		.collect();
	scene_children(children)
}
