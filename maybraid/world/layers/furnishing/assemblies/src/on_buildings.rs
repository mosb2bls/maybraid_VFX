//! Isolated paint helper. World present uses 50 m [`crate::FurnitureCell`] hosts.

use bevy::prelude::{ChildOf, Commands, CommandsSceneExt, Component, Entity};
use building_components::FurnitureNode;
use furniture_components::assembly_scene;

use crate::fill::posed_assembly;

/// Marker on a painted furniture overlay parented to a development host.
#[derive(Component, Clone, Copy, Debug)]
pub struct PaintedFurniture;

/// Instance kit GLBs for paintable slots as children of `host`.
///
/// Slot placements stay host-local. Unpainted kinds (dresser, wardrobe, …)
/// stay on the building High wireframe pass.
pub fn paint_host_furniture(commands: &mut Commands, host: Entity, nodes: &[FurnitureNode]) {
	for node in nodes {
		let Some(parts) = posed_assembly(node) else {
			continue;
		};
		let child = commands.spawn_scene(assembly_scene(&parts)).id();
		commands.entity(child).insert((PaintedFurniture, ChildOf(host)));
	}
}
