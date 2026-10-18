//! Named-bone index for one armature, scoped to that [`RigRoot`].

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::member::AssemblyHost;

/// Opaque identity for one armature inside an assembly.
///
/// Domain crates choose the strings (`"body"`, `"receiver"`, …). This crate
/// only stores and compares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RigKey(pub &'static str);

impl RigKey {
	pub const fn named(name: &'static str) -> Self {
		Self(name)
	}

	pub const fn as_str(self) -> &'static str {
		self.0
	}
}

impl From<&'static str> for RigKey {
	fn from(value: &'static str) -> Self {
		Self(value)
	}
}

/// Marker: this entity owns a [`BoneMap`] over its named descendants.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RigRoot {
	pub key: RigKey,
	/// Bones that must exist before the map is considered ready. Empty = always ready.
	pub landmarks: &'static [&'static str],
}

impl RigRoot {
	pub const fn new(key: RigKey) -> Self {
		Self { key, landmarks: &[] }
	}

	pub const fn with_landmarks(self, landmarks: &'static [&'static str]) -> Self {
		Self { key: self.key, landmarks }
	}
}

/// Named-bone index for one [`RigRoot`] (scoped to that armature).
#[derive(Component, Default, Clone)]
pub struct BoneMap {
	pub by_name: HashMap<String, Entity>,
}

impl BoneMap {
	/// True when every named landmark bone exists in the map.
	pub fn landmarks_ready(&self, landmarks: &[&str]) -> bool {
		landmarks.iter().all(|bone| self.by_name.contains_key(*bone))
	}

	/// Landmark names still missing from the map.
	pub fn missing_landmarks<'a>(&self, landmarks: &'a [&'a str]) -> Vec<&'a str> {
		landmarks
			.iter()
			.copied()
			.filter(|bone| !self.by_name.contains_key(*bone))
			.collect()
	}
}

/// Rebuild a rig's [`BoneMap`] from named descendants, stopping at nested
/// [`AssemblyHost`]s (nested rigs and parts).
///
/// Walks only when [`Children`] / [`Name`] changed under that rig. Incomplete
/// landmarks are not a perpetual dirty — High fulfill adds named bones as
/// children, which already marks the owning root.
pub fn build_bone_maps(
	mut maps: Query<&mut BoneMap, With<RigRoot>>,
	roots: Query<(Entity, &Children, &RigRoot), With<BoneMap>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	hosts: Query<(), With<AssemblyHost>>,
	child_of: Query<&ChildOf>,
	changed_children: Query<Entity, Changed<Children>>,
	changed_names: Query<Entity, Changed<Name>>,
) {
	let mut dirty = HashSet::new();
	for entity in changed_children.iter().chain(changed_names.iter()) {
		if let Some(rig) = owning_rig_root(entity, &child_of, &roots, &hosts) {
			dirty.insert(rig);
		}
	}

	for rig in dirty {
		let Ok((_, children, _)) = roots.get(rig) else {
			continue;
		};
		let children: Vec<Entity> = children.iter().collect();
		let Ok(mut map) = maps.get_mut(rig) else {
			continue;
		};
		rebuild_bone_map(&mut map, &children, &children_q, &names_q, &hosts);
	}
}

fn owning_rig_root(
	mut entity: Entity,
	child_of: &Query<&ChildOf>,
	rig_roots: &Query<(Entity, &Children, &RigRoot), With<BoneMap>>,
	hosts: &Query<(), With<AssemblyHost>>,
) -> Option<Entity> {
	loop {
		if rig_roots.contains(entity) {
			return Some(entity);
		}
		let parent = child_of.get(entity).ok()?.parent();
		if hosts.contains(entity) {
			return None;
		}
		entity = parent;
	}
}

fn rebuild_bone_map(
	map: &mut BoneMap,
	children: &[Entity],
	children_q: &Query<&Children>,
	names_q: &Query<&Name>,
	hosts: &Query<(), With<AssemblyHost>>,
) {
	map.by_name.clear();
	let mut stack: Vec<Entity> = children.to_vec();
	while let Some(entity) = stack.pop() {
		if hosts.contains(entity) {
			continue;
		}
		if let Ok(name) = names_q.get(entity) {
			map.by_name.insert(name.to_string(), entity);
		}
		if let Ok(children) = children_q.get(entity) {
			stack.extend(children.iter());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const LANDMARKS: &[&str] = &["root", "pelvis"];

	fn app() -> App {
		let mut app = App::new();
		app.add_systems(Update, build_bone_maps);
		app
	}

	fn spawn_rig(world: &mut World, landmarks: &'static [&'static str]) -> Entity {
		world
			.spawn((
				RigRoot::new(RigKey::named("body")).with_landmarks(landmarks),
				BoneMap::default(),
			))
			.id()
	}

	fn spawn_named_child(world: &mut World, parent: Entity, name: &str) -> Entity {
		world.spawn((Name::new(name.to_string()), ChildOf(parent))).id()
	}

	fn map_of(world: &World, rig: Entity) -> &BoneMap {
		world.get::<BoneMap>(rig).expect("BoneMap")
	}

	fn insert_sentinel(world: &mut World, rig: Entity) {
		world
			.get_mut::<BoneMap>(rig)
			.expect("BoneMap")
			.by_name
			.insert("__sentinel__".into(), Entity::PLACEHOLDER);
	}

	fn has_sentinel(world: &World, rig: Entity) -> bool {
		map_of(world, rig).by_name.contains_key("__sentinel__")
	}

	#[test]
	fn empty_landmarks_are_ready() {
		assert!(BoneMap::default().landmarks_ready(&[]));
	}

	#[test]
	fn missing_landmarks_are_listed() {
		let map = BoneMap::default();
		assert_eq!(map.missing_landmarks(&["root", "grip"]), vec!["root", "grip"]);
	}

	#[test]
	fn high_fulfill_makes_the_map_ready_on_the_next_build() {
		let mut app = app();
		let rig = spawn_rig(app.world_mut(), LANDMARKS);
		let scene = spawn_named_child(app.world_mut(), rig, "Scene");
		app.update();
		assert!(!map_of(app.world(), rig).landmarks_ready(LANDMARKS));
		assert!(map_of(app.world(), rig).by_name.contains_key("Scene"));

		spawn_named_child(app.world_mut(), scene, "root");
		spawn_named_child(app.world_mut(), scene, "pelvis");
		app.update();
		assert!(map_of(app.world(), rig).landmarks_ready(LANDMARKS));
	}

	#[test]
	fn ready_map_is_not_rebuilt_while_the_tree_is_quiet() {
		let mut app = app();
		let rig = spawn_rig(app.world_mut(), LANDMARKS);
		spawn_named_child(app.world_mut(), rig, "root");
		spawn_named_child(app.world_mut(), rig, "pelvis");
		app.update();
		assert!(map_of(app.world(), rig).landmarks_ready(LANDMARKS));

		insert_sentinel(app.world_mut(), rig);
		app.update();
		assert!(has_sentinel(app.world(), rig));
	}

	#[test]
	fn unready_map_is_not_rebuilt_while_the_tree_is_quiet() {
		let mut app = app();
		let rig = spawn_rig(app.world_mut(), LANDMARKS);
		spawn_named_child(app.world_mut(), rig, "Scene");
		app.update();
		assert!(!map_of(app.world(), rig).landmarks_ready(LANDMARKS));

		insert_sentinel(app.world_mut(), rig);
		app.update();
		assert!(has_sentinel(app.world(), rig));
	}

	#[test]
	fn renamed_bone_rebuilds_a_ready_map() {
		let mut app = app();
		let rig = spawn_rig(app.world_mut(), &[]);
		let bone = spawn_named_child(app.world_mut(), rig, "hip");
		app.update();
		assert!(map_of(app.world(), rig).by_name.contains_key("hip"));

		app.world_mut().entity_mut(bone).insert(Name::new("waist"));
		app.update();
		let map = map_of(app.world(), rig);
		assert!(!map.by_name.contains_key("hip"));
		assert_eq!(map.by_name.get("waist"), Some(&bone));
	}

	#[test]
	fn nested_host_changes_do_not_rebuild_the_parent_map() {
		let mut app = app();
		let rig = spawn_rig(app.world_mut(), LANDMARKS);
		spawn_named_child(app.world_mut(), rig, "root");
		spawn_named_child(app.world_mut(), rig, "pelvis");
		let part = app.world_mut().spawn((AssemblyHost, Name::new("part"), ChildOf(rig))).id();
		app.update();
		assert!(map_of(app.world(), rig).landmarks_ready(LANDMARKS));
		assert!(!map_of(app.world(), rig).by_name.contains_key("part"));

		insert_sentinel(app.world_mut(), rig);
		spawn_named_child(app.world_mut(), part, "mesh");
		app.update();
		assert!(has_sentinel(app.world(), rig));
		assert!(!map_of(app.world(), rig).by_name.contains_key("mesh"));
	}
}
