//! Flattened furniture kits under one 50 m [`FurnitureCell`] host.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use building_components::{pose, scene_children, FurnitureNode, FLATTENED_KIT_CHUNK_WEIGHT};
use furniture_components::{posed_kit_part, FurnitureKitPart};
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;

use furniture_assemblies::posed_assembly;

use crate::cell::FurnitureCellExtent;
use crate::colliders::{walk_collider, walk_collider_scene};

/// One 50 m furniture host: flattened painted / wireframe kits, no nested hosts.
#[derive(Component, Clone, Debug)]
pub struct FurnitureCell {
	pub extent: FurnitureCellExtent,
	pub slots: Vec<FurnitureNode>,
}

impl FurnitureCell {
	pub fn new(extent: FurnitureCellExtent, slots: Vec<FurnitureNode>) -> Self {
		Self { extent, slots }
	}

	pub fn bounds(&self) -> Aabb3d {
		let mut min = Vec3::new(self.extent.min.x, 0.0, self.extent.min.z);
		let mut max = Vec3::new(self.extent.max.x, 4.0, self.extent.max.z);
		for slot in &self.slots {
			let bounds = slot.scene_bounds();
			min = min.min(Vec3::from(bounds.min));
			max = max.max(Vec3::from(bounds.max));
		}
		Aabb3d::from_min_max(min, max)
	}
}

fn furniture_kit_scene(node: &FurnitureNode, slot: u32) -> Box<dyn Scene> {
	if let Some(parts) = posed_assembly(node) {
		Box::new(marked_assembly(&parts, node.finish_seed, slot))
	} else {
		furniture_wireframe_scene(node)
	}
}

fn marked_assembly(
	parts: &[furniture_components::PlacedPart],
	finish_seed: u64,
	slot: u32,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = parts
		.iter()
		.map(|part| Box::new(furniture_part_scene(part, finish_seed, slot)) as Box<dyn Scene>)
		.collect();
	building_components::scene_children(children)
}

fn furniture_wireframe_scene(node: &FurnitureNode) -> Box<dyn Scene> {
	let identity = Transform::IDENTITY;
	let bounds = node.scene_bounds();
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	Box::new(node.scene_with_level(&lod_ref, LodSceneLevel::High))
}

fn furniture_part_scene(
	part: &furniture_components::PlacedPart,
	finish_seed: u64,
	slot: u32,
) -> impl Scene + 'static {
	posed_kit_part(
		part.kind.asset_path(),
		part.material.clone(),
		pose(part.placement),
		FurnitureKitPart { kind: part.kind, finish_seed, slot },
	)
}

/// One fulfill quantum per painted GLB (or one wireframe kit).
///
/// A single lazy stream — nested lazy would subtract part counts from the slot
/// remaining-primitive budget and drop later slots.
fn furniture_cell_chunks(slots: Vec<FurnitureNode>) -> SceneChunk {
	if slots.is_empty() {
		return SceneChunk::primitive(scene_children(Vec::new()));
	}
	let kit_w = FLATTENED_KIT_CHUNK_WEIGHT;
	let estimate = slots.len() * 16;
	let mut slot_index = 0usize;
	let mut parts: Option<Vec<furniture_components::PlacedPart>> = None;
	let mut part_index = 0usize;
	SceneChunk::lazy(estimate as u32 * kit_w, estimate, move || loop {
		if let Some(current) = parts.as_ref() {
			if part_index < current.len() {
				let finish_seed = slots[slot_index].finish_seed;
				let scene =
					furniture_part_scene(&current[part_index], finish_seed, slot_index as u32);
				part_index += 1;
				return Some(SceneChunk::weighted(kit_w, scene));
			}
			parts = None;
			slot_index += 1;
		}
		if slot_index >= slots.len() {
			return None;
		}
		if let Some(assembled) = posed_assembly(&slots[slot_index]) {
			parts = Some(assembled);
			part_index = 0;
			continue;
		}
		let scene = furniture_wireframe_scene(&slots[slot_index]);
		slot_index += 1;
		return Some(SceneChunk::weighted(kit_w, scene));
	})
}

#[cfg(test)]
fn furniture_slot_chunks(node: FurnitureNode) -> SceneChunk {
	furniture_cell_chunks(vec![node])
}

impl LodScene for FurnitureCell {
	fn scene_lod_level(&self, _lod_ref: &LodRef) -> LodSceneLevel {
		LodSceneLevel::High
	}

	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		if !matches!(level, LodSceneLevel::High) {
			return Box::new(scene_children(Vec::new())) as Box<dyn Scene>;
		}
		let mut children: Vec<Box<dyn Scene>> = self
			.slots
			.iter()
			.enumerate()
			.map(|(slot, node)| furniture_kit_scene(node, slot as u32))
			.collect();
		if let Some(collider) = walk_collider(&self.slots) {
			children.push(Box::new(walk_collider_scene(collider)));
		}
		Box::new(scene_children(children)) as Box<dyn Scene>
	}

	/// The walk collider first, so the cell bears weight before its kits land.
	fn scene_chunks_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		if !matches!(level, LodSceneLevel::High) || self.slots.is_empty() {
			return SceneChunk::primitive(scene_children(Vec::new()));
		}
		let kits = furniture_cell_chunks(self.slots.clone());
		match walk_collider(&self.slots) {
			Some(collider) => {
				SceneChunk::chunks([SceneChunk::primitive(walk_collider_scene(collider)), kits])
			}
			None => kits,
		}
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.bounds()
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		lod_host_scene_pending(level, self.scene_bounds())
	}
}

/// Spawn a pending flattened furniture host. Slot placements are already world-space.
pub fn spawn_furniture_cell(commands: &mut Commands, cell: FurnitureCell) -> Entity {
	let bounds = cell.bounds();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let level = cell.scene_lod_level(&lod_ref);
	let entity = commands
		.spawn_scene((
			lod_host_scene_pending(level, bounds),
			bsn! {
				Transform::default()
				Visibility::default()
			},
		))
		.id();
	commands.entity(entity).insert(cell);
	entity
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::Placement;

	fn lod_ref<'a>(tf: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: tf,
			current_transform: tf,
			bounds,
		}
	}

	#[test]
	fn high_chunks_are_lazy_slots_not_one_blob() {
		let cell = FurnitureCell::new(
			FurnitureCellExtent::from_cell_index(0, 0),
			vec![
				FurnitureNode::chair(Placement::IDENTITY).with_finish_seed(1),
				FurnitureNode::chest(Placement::IDENTITY).with_finish_seed(2),
			],
		);
		let tf = Transform::IDENTITY;
		let bounds = cell.bounds();
		let chunks = cell.scene_chunks_with_level(&lod_ref(&tf, &bounds), LodSceneLevel::High);
		assert!(chunks.total_primitives() >= 2);
		assert!(chunks.total_weight() >= 2 * FLATTENED_KIT_CHUNK_WEIGHT);
	}

	#[test]
	fn painted_slot_expands_one_chunk_per_part() {
		let chair = FurnitureNode::chair(Placement::IDENTITY).with_finish_seed(3);
		let chunks = furniture_slot_chunks(chair);
		let mut queue = std::collections::VecDeque::from([chunks]);
		let mut pulled = 0u32;
		let mut weight = 0u32;
		while let Some((w, _)) = lod::scene::pull_primitive(&mut queue) {
			pulled += 1;
			weight += w;
		}
		assert_eq!(pulled, 6);
		assert_eq!(weight, 6 * FLATTENED_KIT_CHUNK_WEIGHT);
	}
}
