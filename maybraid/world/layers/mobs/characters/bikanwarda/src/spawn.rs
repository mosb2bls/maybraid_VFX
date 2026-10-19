//! Encounter / Ward / Enraged spawn entry points (shared assembly).

use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use characters::{components::CharacterComponents, spawn_fixed_character_assembly, CharacterRecipe};
use lod_avian::PhysicsInteractionLayer;
use crate::anchors::{BikanwardaAnchors, BikanwardaChainAnchor, CHAIN_ANCHOR_NAMES};
use crate::colliders::motor_collider;
use crate::recipe::{BikanwardaActorRecipe};
use crate::spec::BikanwardaSpec;

/// Domain marker on the boss assembly root.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaRoot;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaEncounter;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaWard;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaEnraged;

pub fn spawn_bikanwarda_encounter(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
) -> Entity {
	spawn_bikanwarda(commands, spec, transform, BikanwardaEncounter)
}

pub fn spawn_bikanwarda_ward(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
) -> Entity {
	spawn_bikanwarda(commands, spec, transform, BikanwardaWard)
}

pub fn spawn_bikanwarda_enraged(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
) -> Entity {
	spawn_bikanwarda(commands, spec, transform, BikanwardaEnraged)
}

fn spawn_bikanwarda<M: Component>(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
	marker: M,
) -> Entity {
	let recipe = BikanwardaActorRecipe::from_spec(spec.clone());
	let actor = recipe.components();
	let scale = spec.uniform_scale();
	let visual_transform = transform.with_scale(Vec3::splat(scale));
	let visual = spawn_fixed_character_assembly(commands, &actor, visual_transform);
	commands.entity(visual).insert((
		Name::new("bikanwarda"),
		BikanwardaRoot,
		marker,
		spec.clone(),
	));
	let capsule = actor.locomotion_capsule();
	spawn_physics(commands, visual, capsule);
	spawn_chain_anchors(commands, visual, actor.scaled_locomotion_capsule());
	visual
}

fn spawn_physics(commands: &mut Commands, root: Entity, capsule: characters::LocomotionCapsule) {
	commands
		.entity(root)
		.insert((RigidBody::Static, motor_collider(capsule), PhysicsInteractionLayer::fixed_layers()));
}

fn spawn_chain_anchors(commands: &mut Commands, root: Entity, capsule: characters::LocomotionCapsule) {
	let anchors = BikanwardaAnchors::for_capsule(capsule);
	for name in CHAIN_ANCHOR_NAMES {
		let local = anchors.get(name).copied().unwrap_or(Transform::IDENTITY);
		commands.spawn((
			Name::new(name),
			ChildOf(root),
			local,
			BikanwardaChainAnchor { name },
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encounter_recipe_matches_spec_form() {
		let spec = BikanwardaSpec::encounter();
		let recipe = BikanwardaActorRecipe::from_spec(spec.clone());
		assert_eq!(recipe.spec.form, spec.form);
	}
}
