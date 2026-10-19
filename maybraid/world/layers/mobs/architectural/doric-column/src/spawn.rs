//! Encounter and runtime summon entry points (same assembly, different markers).

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use lod::scene::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;
use material_ref::{MaterialRefRoot, PropagateToDescendants};
use rigs::AssemblyRoot;

use crate::anchors::{names, DoricColumnAnchors, DoricColumnChainAnchor, DoricColumnGroundContact};
use crate::colliders::cover_collider_shapes;
use crate::parts::{DoricColumnComponents, DoricColumnPartNode};
use crate::placeholder::{DoricColumnPartMesh, PlaceholderVisual};
use crate::spec::DoricColumnSpec;
use crate::terrain::DoricColumnTerrainPlacement;
use building_physics::BUILDING_FRICTION;
use scene_ref::{SceneRef, SceneRefRoot};

/// Domain marker on the column assembly root.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DoricColumnRoot;

/// Placed by a Bikanwarda encounter layout (independent of boss AI).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DoricColumnEncounter;

/// Placed by a runtime summon (lifetime handled by summon lane).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DoricColumnSummon;

/// Queued cover shapes in assembly-local space; consumed by [`DoricColumnPlugin`].
#[derive(Component, Clone, Debug)]
pub struct DoricColumnCoverShapes {
	pub shapes: Vec<(Vec3, Quat, Collider)>,
}

/// Spawn the shared column assembly for an encounter.
pub fn spawn_doric_column_encounter(
	commands: &mut Commands,
	spec: &DoricColumnSpec,
	transform: Transform,
	use_glb: bool,
) -> Entity {
	spawn_doric_column(commands, spec, transform, use_glb, DoricColumnEncounter)
}

/// Spawn the shared column assembly for a runtime summon.
pub fn spawn_doric_column_summon(
	commands: &mut Commands,
	spec: &DoricColumnSpec,
	transform: Transform,
	use_glb: bool,
) -> Entity {
	spawn_doric_column(commands, spec, transform, use_glb, DoricColumnSummon)
}

fn spawn_doric_column<M: Component>(
	commands: &mut Commands,
	spec: &DoricColumnSpec,
	transform: Transform,
	use_glb: bool,
	marker: M,
) -> Entity {
	let layout = spec.layout();
	let shapes = cover_collider_shapes(&layout);
	let root = commands
		.spawn((
			Name::new("doric-column"),
			DoricColumnRoot,
			AssemblyRoot,
			marker,
			transform,
			Visibility::Inherited,
		))
		.id();
	commands.entity(root).insert(spec.clone());
	commands
		.entity(root)
		.insert((DoricColumnTerrainPlacement, DoricColumnCoverShapes { shapes }));

	spawn_parts(commands, root, spec, use_glb);
	spawn_anchors(commands, root, spec);
	root
}

fn spawn_parts(commands: &mut Commands, root: Entity, spec: &DoricColumnSpec, use_glb: bool) {
	let nodes = part_nodes(spec, use_glb);
	for node in nodes {
		let transform = node.transform();
		let mut entity = commands.spawn((
			ChildOf(root),
			Name::new(node.slot.label()),
			node.clone(),
			transform,
			MaterialRefRoot(node.material.clone()),
			PropagateToDescendants,
			Visibility::Inherited,
		));
		if node.use_authored_glb {
			entity.insert(SceneRefRoot(SceneRef::glb(node.slot.asset_path())));
		} else {
			entity.insert((DoricColumnPartMesh(node.slot), PlaceholderVisual));
		}
	}
}

fn part_nodes(spec: &DoricColumnSpec, use_glb: bool) -> Vec<DoricColumnPartNode> {
	spec.part_nodes_for_level(LodSceneLevel::High)
		.into_iter()
		.map(|mut node| {
			node.use_authored_glb = use_glb;
			node
		})
		.collect()
}

fn spawn_anchors(commands: &mut Commands, root: Entity, spec: &DoricColumnSpec) {
	let anchors = DoricColumnAnchors::for_spec(spec);
	for (name, local) in anchors.transforms {
		let mut entity =
			commands.spawn((ChildOf(root), Name::new(name), local, Visibility::Hidden));
		if name == names::GROUND_CONTACT {
			entity.insert(DoricColumnGroundContact);
		}
		if crate::anchors::CHAIN_ANCHOR_NAMES.contains(&name) {
			entity.insert(DoricColumnChainAnchor { name });
		}
	}
}

/// Stamp the Fixed compound cover collider once per root.
pub(crate) fn attach_cover_colliders(
	mut commands: Commands,
	pending: Query<(Entity, &DoricColumnCoverShapes), Without<DoricColumnCoverColliderAttached>>,
) {
	for (entity, spec) in &pending {
		if spec.shapes.is_empty() {
			continue;
		}
		let shapes = spec.shapes.clone();
		commands.entity(entity).insert(DoricColumnCoverColliderAttached);
		commands.spawn((
			Name::new("doric-column-cover"),
			ChildOf(entity),
			Transform::IDENTITY,
			Visibility::Hidden,
			DoricColumnCoverCollider,
			RigidBody::Static,
			Collider::compound(shapes),
			PhysicsInteractionLayer::fixed_layers(),
			BUILDING_FRICTION,
		));
	}
}

/// Marks the compound spawned from [`DoricColumnCoverShapes`].
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DoricColumnCoverCollider;

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct DoricColumnCoverColliderAttached;

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::app::App;

	#[test]
	fn repeated_spawns_get_distinct_roots() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let spec = DoricColumnSpec::default();
		let a = {
			let mut commands = app.world_mut().commands();
			spawn_doric_column_encounter(&mut commands, &spec, Transform::IDENTITY, false)
		};
		let b = {
			let mut commands = app.world_mut().commands();
			spawn_doric_column_summon(
				&mut commands,
				&spec,
				Transform::from_xyz(5.0, 0.0, 0.0),
				false,
			)
		};
		app.update();
		assert_ne!(a, b);
		let encounters = app.world_mut().query::<&DoricColumnEncounter>().iter(app.world()).count();
		let summons = app.world_mut().query::<&DoricColumnSummon>().iter(app.world()).count();
		assert_eq!(encounters, 1);
		assert_eq!(summons, 1);
	}
}
