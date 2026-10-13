//! Runtime character visuals as fixed assemblies (no nested [`lod::LodSceneHost`]).
//!
//! Mob High still gates member lifetime. Each materialized character is an
//! ordinary [`crate::CharacterRoot`] whose rigs and parts are budgeted
//! [`SceneChunk`] kits — not query-only LOD hosts.
//!
//! Playground / authoring [`crate::ComponentsOnly`] [`lod::LodScene::host`] is
//! unchanged.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::scene::prelude::WorldSceneExt;
use bevy::world_serialization::WorldAssetRoot;
use lod::scene::chunk::pull_primitive;
use lod::scene::LodSceneLevel;
use lod::SceneChunk;
use rigs::AssemblyRoot;
use scene_ref::SceneRefRoot;

use crate::components::CharacterComponents;
use crate::member::CharacterRoot;
use crate::scene_children::scene_children;
use character_motion::{motion_policy, CharacterHeading};

/// Drain cost of one rig or part GLB, matching flattened vegetation kits.
pub const FIXED_ASSEMBLY_CHUNK_WEIGHT: u32 = 4;

/// Unsettled assembly kits parented to a fixed character root.
#[derive(Component)]
pub struct PendingCharacterAssembly {
	queue: VecDeque<SceneChunk>,
}

/// Kit spawned [`Visibility::Hidden`] until its [`WorldAssetRoot`] is ready.
#[derive(Component)]
pub(crate) struct HiddenUntilSceneReady;

/// Per-frame admission for [`drain_character_assembly`].
#[derive(Resource, Debug, Clone, Copy)]
pub struct CharacterAssemblyBudget {
	pub spawn_weights_per_frame: u32,
	pub spawn_time_per_frame: Duration,
}

impl Default for CharacterAssemblyBudget {
	fn default() -> Self {
		Self { spawn_weights_per_frame: 64, spawn_time_per_frame: Duration::from_millis(2) }
	}
}

/// Last-frame assembly drain (queue depth is remaining primitives across jobs).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct CharacterAssemblyDiagnostics {
	pub queue_depth: usize,
	pub admitted_weight: u32,
	pub elapsed: Duration,
}

/// Lazy weighted kits for one character (rigs then parts). No nested hosts.
pub fn fixed_character_assembly_chunks(character: &impl CharacterComponents) -> SceneChunk {
	let level = LodSceneLevel::High;
	let rigs: Vec<_> = character.rig_nodes_for_level(level).flatten();
	let parts: Vec<_> = character.part_nodes_for_level(level).flatten();
	let n = rigs.len() + parts.len();
	if n == 0 {
		return SceneChunk::primitive(scene_children(Vec::new()));
	}

	let n_rigs = rigs.len();
	let mut index = 0usize;
	let kit_w = FIXED_ASSEMBLY_CHUNK_WEIGHT;
	SceneChunk::lazy(n as u32 * kit_w, n, move || {
		if index < n_rigs {
			let node = &rigs[index];
			index += 1;
			return Some(SceneChunk::weighted(kit_w, node.assembly_scene()));
		}
		let fi = index - n_rigs;
		if fi < parts.len() {
			let node = &parts[fi];
			index += 1;
			return Some(SceneChunk::weighted(kit_w, node.assembly_scene()));
		}
		None
	})
}

/// Spawn a fixed assembly root. Kits drain onto this entity over subsequent frames.
pub fn spawn_fixed_character_assembly(
	commands: &mut Commands,
	recipe: &impl CharacterComponents,
	transform: Transform,
) -> Entity {
	let policy = motion_policy(LodSceneLevel::High);
	let heading = CharacterHeading::from_rotation(transform.rotation);
	let visual = commands
		.spawn((
			Name::new("character-visual"),
			transform,
			AssemblyRoot,
			CharacterRoot,
			heading,
			Visibility::Inherited,
		))
		.id();
	if let Some(pitch) = policy.apply_terrain_pitch() {
		commands.entity(visual).insert(pitch);
	}
	commands.entity(visual).insert(PendingCharacterAssembly {
		queue: fixed_character_assembly_chunks(recipe).into_fulfill_queue(),
	});
	visual
}

/// Parent a fixed assembly under `body` (world / mob / player runtime path).
pub fn spawn_fixed_character_visual(
	commands: &mut Commands,
	body: Entity,
	recipe: impl CharacterComponents,
	facing: Quat,
	name: &'static str,
) -> Entity {
	let visual =
		spawn_fixed_character_assembly(commands, &recipe, Transform::from_rotation(facing));
	commands.entity(visual).insert((ChildOf(body), Name::new(name)));
	visual
}

/// Exclusive drain of pending kits. Stops on weight or wall-clock, like LOD fulfill.
pub fn drain_character_assembly(world: &mut World) {
	let budget = world.get_resource::<CharacterAssemblyBudget>().copied().unwrap_or_default();
	let start = Instant::now();
	let mut remaining = budget.spawn_weights_per_frame;
	let mut admitted = 0u32;

	let roots: Vec<Entity> = world
		.query_filtered::<Entity, With<PendingCharacterAssembly>>()
		.iter(world)
		.collect();

	for root in roots {
		if remaining == 0 || start.elapsed() >= budget.spawn_time_per_frame {
			break;
		}
		loop {
			if remaining == 0 || start.elapsed() >= budget.spawn_time_per_frame {
				break;
			}
			let Some((weight, scene)) = world
				.get_mut::<PendingCharacterAssembly>(root)
				.and_then(|mut pending| pull_primitive(&mut pending.queue))
			else {
				world.entity_mut(root).remove::<PendingCharacterAssembly>();
				break;
			};
			remaining = remaining.saturating_sub(weight);
			admitted = admitted.saturating_add(weight);
			match world.spawn_scene(scene) {
				Ok(spawned) => {
					let child = spawned.id();
					world.entity_mut(child).insert((ChildOf(root), HiddenUntilSceneReady));
				}
				Err(err) => {
					warn!(?err, root = ?root, "fixed character assembly spawn_scene failed");
				}
			}
		}
	}

	let queue_depth = world
		.query::<&PendingCharacterAssembly>()
		.iter(world)
		.map(|pending| pending.queue.iter().map(SceneChunk::total_primitives).sum::<usize>())
		.sum();
	if let Some(mut diagnostics) = world.get_resource_mut::<CharacterAssemblyDiagnostics>() {
		diagnostics.queue_depth = queue_depth;
		diagnostics.admitted_weight = admitted;
		diagnostics.elapsed = start.elapsed();
	}
}

pub(crate) fn reveal_ready_fixed_members(
	mut commands: Commands,
	mut visibilities: Query<
		(Entity, &mut Visibility),
		(With<HiddenUntilSceneReady>, With<SceneRefRoot>, Added<WorldAssetRoot>),
	>,
) {
	for (entity, mut visibility) in &mut visibilities {
		*visibility = Visibility::Inherited;
		commands.entity(entity).remove::<HiddenUntilSceneReady>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::species::braidman::BraidmanConfig;
	use crate::CharacterRecipe;
	use lod::lod_ref::LodRef;

	use crate::components::character_scene_chunks;

	#[test]
	fn fixed_chunks_weight_kits_instead_of_nested_hosts() {
		let clothed = BraidmanConfig::default_preview().clothed();
		let n = clothed.rig_nodes_for_level(LodSceneLevel::High).flatten().len()
			+ clothed.part_nodes_for_level(LodSceneLevel::High).flatten().len();
		assert!(n > 4, "preview recipe should emit several rigs and parts");
		let fixed = fixed_character_assembly_chunks(&clothed);
		assert_eq!(fixed.total_primitives(), n);
		assert_eq!(fixed.total_weight(), n as u32 * FIXED_ASSEMBLY_CHUNK_WEIGHT);

		let identity = Transform::IDENTITY;
		let bounds = crate::character_bounds(&clothed);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		let lod = character_scene_chunks(&clothed, &lod_ref, LodSceneLevel::High);
		assert_eq!(lod.total_primitives(), n);
		assert_eq!(lod.total_weight(), n as u32);
	}

	#[test]
	fn spawn_fixed_root_has_no_lod_host() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		let clothed = BraidmanConfig::default_preview().clothed();
		let visual = world
			.run_system_once(move |mut commands: Commands| {
				spawn_fixed_character_assembly(&mut commands, &clothed, Transform::IDENTITY)
			})
			.expect("spawn assembly");
		assert!(world.get::<CharacterRoot>(visual).is_some());
		assert!(world.get::<AssemblyRoot>(visual).is_some());
		assert!(world.get::<lod::LodSceneHost>(visual).is_none());
		assert!(world.get::<PendingCharacterAssembly>(visual).is_some());
	}

	#[test]
	fn pull_primitive_charges_kit_weight() {
		let clothed = BraidmanConfig::default_preview().clothed();
		let mut queue = fixed_character_assembly_chunks(&clothed).into_fulfill_queue();
		let (weight, _) = pull_primitive(&mut queue).expect("first kit");
		assert_eq!(weight, FIXED_ASSEMBLY_CHUNK_WEIGHT);
	}
}
