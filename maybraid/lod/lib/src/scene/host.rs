//! Runtime ECS hosts that switch LOD level roots without despawning the host.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::scene::cull::closest_available_lod_level;
use crate::scene::level::LodSceneLevel;

/// Marker: this entity owns LOD level roots and a current [`LodSceneLevel`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodSceneHost;

/// Marker: this entity roots an independently hidden LOD tree.
///
/// Revealing an ancestor prunes its walk at nested active hide roots.
#[derive(Debug, Clone, Copy, Default, Component)]
struct LodTreeHideActive;

/// Whether a visibility value counts as on-screen (warm-hold / cold-fill).
#[inline]
pub fn lod_root_is_shown(visibility: Visibility) -> bool {
	!matches!(visibility, Visibility::Hidden)
}

/// Shown to default queries: not [`Visibility::Hidden`] and not [`Disabled`].
#[inline]
pub fn lod_world_entity_is_shown(world: &World, entity: Entity) -> bool {
	world.get::<Disabled>(entity).is_none()
		&& world
			.get::<Visibility>(entity)
			.is_some_and(|visibility| lod_root_is_shown(*visibility))
}

/// Hide a warm LOD tree from default queries (visibility / extract / host index).
///
/// Stamps [`Visibility::Hidden`], [`Disabled`], and the hide marker on `entity`
/// immediately. Descendant [`Disabled`] is queued onto [`LodTreeVisQueue`] and
/// drained by [`drain_lod_tree_vis`] (Bevy does not cascade [`Disabled`]).
/// Without the queue (unit tests), descendants are stamped in this call.
/// Pending fulfill roots stay Hidden-only so new children still receive
/// visibility propagate.
pub fn hide_lod_tree(commands: &mut Commands, entity: Entity) {
	commands.queue(move |world: &mut World| hide_lod_tree_now(world, entity));
}

/// Reveal a warm LOD tree, preserving independently hidden nested trees.
pub fn show_lod_tree(commands: &mut Commands, entity: Entity) {
	commands.queue(move |world: &mut World| show_lod_tree_now(world, entity));
}

/// Exclusive-world hide (cull enqueue from `World`).
pub fn hide_lod_tree_world(entity: &mut EntityWorldMut) {
	let root = entity.id();
	entity.world_scope(|world| hide_lod_tree_now(world, root));
}

fn hide_lod_tree_now(world: &mut World, root: Entity) {
	let children = {
		let Ok(mut entity) = world.get_entity_mut(root) else {
			return;
		};
		if entity.contains::<LodTreeHideActive>() {
			entity.insert(Visibility::Hidden);
			return;
		}
		let children = entity
			.get::<Children>()
			.map(|c| c.iter().collect::<Vec<_>>())
			.unwrap_or_default();
		entity.insert((Visibility::Hidden, Disabled, LodTreeHideActive));
		children
	};
	if children.is_empty() {
		return;
	}
	if world.get_resource::<LodTreeVisQueue>().is_some() {
		world.resource_mut::<LodTreeVisQueue>().enqueue_hide(root, children);
		return;
	}
	hide_lod_tree_children_now(world, &children);
}

fn hide_lod_tree_children_now(world: &mut World, children: &[Entity]) {
	for &child in children {
		if let Ok(mut entity) = world.get_entity_mut(child) {
			entity.insert_recursive::<Children>(Disabled);
		}
	}
}

fn has_active_hide_ancestor(world: &World, entity: Entity) -> bool {
	let mut current = world.get::<ChildOf>(entity).map(|child| child.parent());
	while let Some(entity) = current {
		if world.get::<LodTreeHideActive>(entity).is_some() {
			return true;
		}
		current = world.get::<ChildOf>(entity).map(|child| child.parent());
	}
	false
}

fn show_lod_tree_now(world: &mut World, root: Entity) {
	let Ok(mut root_entity) = world.get_entity_mut(root) else {
		return;
	};
	let was_hidden_here = root_entity.contains::<LodTreeHideActive>();
	let was_disabled = root_entity.contains::<Disabled>();
	root_entity.insert(Visibility::Inherited);
	if !was_hidden_here && !was_disabled {
		return;
	}
	if has_active_hide_ancestor(world, root) {
		world.entity_mut(root).remove::<LodTreeHideActive>();
		return;
	}

	world.entity_mut(root).remove::<(LodTreeHideActive, Disabled)>();
	let children = world
		.get::<Children>(root)
		.map(|c| c.iter().collect::<Vec<_>>())
		.unwrap_or_default();
	if children.is_empty() {
		return;
	}
	if world.get_resource::<LodTreeVisQueue>().is_some() {
		world.resource_mut::<LodTreeVisQueue>().enqueue_show(root, children);
		return;
	}
	show_lod_tree_descendants_now(world, root, children);
}

fn show_lod_tree_descendants_now(world: &mut World, root: Entity, mut stack: Vec<Entity>) {
	while let Some(entity) = stack.pop() {
		let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
			continue;
		};
		if entity != root && entity_mut.contains::<LodTreeHideActive>() {
			continue;
		}
		let children = entity_mut
			.get::<Children>()
			.map(|c| c.iter().collect::<Vec<_>>())
			.unwrap_or_default();
		entity_mut.remove::<Disabled>();
		stack.extend(children);
	}
}

/// Per-frame target for recursive [`Disabled`] hide/show walks.
///
/// Root hide/show still stamps immediately (Hidden + root [`Disabled`], or
/// Inherited + root enable). Descendant [`Disabled`] apply is drained by
/// [`drain_lod_tree_vis`] so one grove tile cannot take 500 ms of ApplyDeferred.
///
/// [`Self::time_per_frame`] is checked **between** visits, not a hard cap: one
/// node that copies a large [`Children`] list can still overrun (Tracy drain
/// max ~6 ms against the 2 ms target). The initial root-child collect in
/// hide/show is also outside the drain loop.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodTreeVisBudget {
	/// Wall-clock **target** for [`drain_lod_tree_vis`]. Default 2 ms.
	///
	/// Checked after each visit; a single visit can exceed it.
	pub time_per_frame: Duration,
	/// Max entities visited this frame. Default 512.
	pub entities_per_frame: u32,
}

impl Default for LodTreeVisBudget {
	fn default() -> Self {
		Self { time_per_frame: Duration::from_millis(2), entities_per_frame: 512 }
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LodTreeVisKind {
	Hide,
	Show,
}

struct LodTreeVisJob {
	kind: LodTreeVisKind,
	root: Entity,
	stack: Vec<Entity>,
}

/// Remaining descendant [`Disabled`] work from hide/show.
#[derive(Resource, Default)]
pub struct LodTreeVisQueue {
	shows: VecDeque<LodTreeVisJob>,
	hides: VecDeque<LodTreeVisJob>,
}

impl LodTreeVisQueue {
	fn enqueue_hide(&mut self, root: Entity, children: Vec<Entity>) {
		self.cancel(root);
		if !children.is_empty() {
			self.hides.push_back(LodTreeVisJob {
				kind: LodTreeVisKind::Hide,
				root,
				stack: children,
			});
		}
	}

	fn enqueue_show(&mut self, root: Entity, children: Vec<Entity>) {
		self.cancel(root);
		if !children.is_empty() {
			self.shows.push_back(LodTreeVisJob {
				kind: LodTreeVisKind::Show,
				root,
				stack: children,
			});
		}
	}

	fn cancel(&mut self, root: Entity) {
		self.shows.retain(|job| job.root != root);
		self.hides.retain(|job| job.root != root);
	}

	fn pop_visit(&mut self) -> Option<(LodTreeVisKind, Entity, Entity)> {
		Self::pop_from(&mut self.shows).or_else(|| Self::pop_from(&mut self.hides))
	}

	fn pop_from(jobs: &mut VecDeque<LodTreeVisJob>) -> Option<(LodTreeVisKind, Entity, Entity)> {
		while let Some(job) = jobs.front_mut() {
			if let Some(entity) = job.stack.pop() {
				let kind = job.kind;
				let root = job.root;
				if job.stack.is_empty() {
					jobs.pop_front();
				}
				return Some((kind, root, entity));
			}
			jobs.pop_front();
		}
		None
	}

	fn push_children(&mut self, kind: LodTreeVisKind, root: Entity, children: Vec<Entity>) {
		if children.is_empty() {
			return;
		}
		let jobs = match kind {
			LodTreeVisKind::Show => &mut self.shows,
			LodTreeVisKind::Hide => &mut self.hides,
		};
		if let Some(front) = jobs.front_mut() {
			if front.kind == kind && front.root == root {
				front.stack.extend(children);
				return;
			}
		}
		jobs.push_front(LodTreeVisJob { kind, root, stack: children });
	}
}

#[cfg(test)]
impl LodTreeVisQueue {
	fn is_empty(&self) -> bool {
		self.shows.is_empty() && self.hides.is_empty()
	}
}

/// Apply queued descendant [`Disabled`] stamps under [`LodTreeVisBudget`].
///
/// Shows drain first so a warm-swap band appears before leftover hides.
pub fn drain_lod_tree_vis(world: &mut World) {
	if world.get_resource::<LodTreeVisQueue>().is_none() {
		return;
	}
	let budget = world.get_resource::<LodTreeVisBudget>().copied().unwrap_or_default();
	let start = Instant::now();
	let mut visited = 0u32;
	loop {
		if visited > 0
			&& (visited >= budget.entities_per_frame.max(1)
				|| start.elapsed() >= budget.time_per_frame)
		{
			break;
		}
		let Some((kind, root, entity)) = world.resource_mut::<LodTreeVisQueue>().pop_visit() else {
			break;
		};
		let children = match kind {
			LodTreeVisKind::Hide => hide_lod_tree_visit(world, root, entity),
			LodTreeVisKind::Show => show_lod_tree_visit(world, root, entity),
		};
		if !children.is_empty() {
			world.resource_mut::<LodTreeVisQueue>().push_children(kind, root, children);
		}
		visited += 1;
	}
}

fn hide_lod_tree_visit(world: &mut World, root: Entity, entity: Entity) -> Vec<Entity> {
	let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
		return Vec::new();
	};
	if entity != root && entity_mut.contains::<LodTreeHideActive>() {
		return Vec::new();
	}
	let children = entity_mut
		.get::<Children>()
		.map(|c| c.iter().collect::<Vec<_>>())
		.unwrap_or_default();
	entity_mut.insert(Disabled);
	children
}

fn show_lod_tree_visit(world: &mut World, root: Entity, entity: Entity) -> Vec<Entity> {
	let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
		return Vec::new();
	};
	if entity != root && entity_mut.contains::<LodTreeHideActive>() {
		return Vec::new();
	}
	let children = entity_mut
		.get::<Children>()
		.map(|c| c.iter().collect::<Vec<_>>())
		.unwrap_or_default();
	entity_mut.remove::<Disabled>();
	children
}

/// True when this entity or an ancestor [`LodSceneHost`] is Hidden or [`Disabled`].
///
/// Present-layer hide stamps Hidden+Disabled on leaving grove parents so scene
/// cull does not nibble High roots on a host about to despawn.
pub fn lod_scene_host_or_ancestor_hidden(
	entity: Entity,
	child_of: &Query<&ChildOf, Allow<Disabled>>,
	hosts: &Query<(), (With<LodSceneHost>, Allow<Disabled>)>,
	visibilities: &Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) -> bool {
	let mut current = Some(entity);
	while let Some(entity) = current {
		if hosts.contains(entity)
			&& visibilities
				.get(entity)
				.is_ok_and(|(vis, disabled)| disabled || matches!(*vis, Visibility::Hidden))
		{
			return true;
		}
		current = child_of.get(entity).ok().map(|child| child.parent());
	}
	false
}

/// World walk used by exclusive enqueue (bypasses query filters).
pub fn lod_scene_host_or_ancestor_hidden_world(world: &World, entity: Entity) -> bool {
	let mut current = Some(entity);
	while let Some(entity) = current {
		if world.get::<LodSceneHost>(entity).is_some()
			&& (world.get::<Disabled>(entity).is_some()
				|| world
					.get::<Visibility>(entity)
					.is_some_and(|visibility| matches!(*visibility, Visibility::Hidden)))
		{
			return true;
		}
		current = world.get::<ChildOf>(entity).map(|child| child.parent());
	}
	false
}

/// Whether `host` currently shows a [`LodLevelRoot`] at `level` (not Hidden/Disabled).
pub fn host_shows_level_root(
	host: Entity,
	level: LodSceneLevel,
	children_q: &Query<&Children, Allow<Disabled>>,
	level_roots_bags: &Query<(), (With<LodLevelRoots>, Allow<Disabled>)>,
	root_keys: &Query<&LodLevelRoot, Allow<Disabled>>,
	visibilities: &Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) -> bool {
	let Ok(host_kids) = children_q.get(host) else {
		return false;
	};
	let Some(bag) = lod_level_roots_entity(host_kids, level_roots_bags) else {
		return false;
	};
	let Ok(root_kids) = children_q.get(bag) else {
		return false;
	};
	root_kids.iter().any(|root_e| {
		root_keys.get(root_e).is_ok_and(|root| root.0 == level)
			&& visibilities
				.get(root_e)
				.is_ok_and(|(vis, disabled)| !disabled && lod_root_is_shown(*vis))
	})
}

/// World equivalent of [`host_shows_level_root`].
pub fn host_shows_level_root_world(world: &World, host: Entity, level: LodSceneLevel) -> bool {
	let Some(host_children) = world.get::<Children>(host) else {
		return false;
	};
	let Some(bag) = host_children.iter().find(|&child| world.get::<LodLevelRoots>(child).is_some())
	else {
		return false;
	};
	world.get::<Children>(bag).is_some_and(|children| {
		children.iter().any(|root| {
			world.get::<LodLevelRoot>(root).is_some_and(|key| key.0 == level)
				&& lod_world_entity_is_shown(world, root)
		})
	})
}

/// Whether a nested host may run fine-phase **refresh** (level produce / cull / probe update).
///
/// Walks ancestors (skipping `entity` itself) and records the nearest enclosing
/// [`LodLevelRoot`]. When a parent [`LodSceneHost`] is found, refresh is allowed when
/// that root matches the parent's **desired** level **or** is currently **shown**
/// (Inherited/Visible warm-hold). No parent host → top-level → allowed. Under a parent
/// host but not under any level root → allowed (cold scaffolding).
///
/// Initial chunk fulfill (empty level-roots bag) is **not** gated by this — begin still
/// admits empty nested hosts; upgrades require this gate once any root exists.
pub fn nested_host_parent_allows_refresh(
	entity: Entity,
	child_of: &Query<&ChildOf, Allow<Disabled>>,
	host_levels: &Query<&LodSceneLevel, (With<LodSceneHost>, Allow<Disabled>)>,
	level_roots: &Query<&LodLevelRoot, Allow<Disabled>>,
	children_q: &Query<&Children, Allow<Disabled>>,
	level_roots_bags: &Query<(), (With<LodLevelRoots>, Allow<Disabled>)>,
	visibilities: &Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) -> bool {
	if visibilities.get(entity).is_ok_and(|(_, disabled)| disabled) {
		return false;
	}
	let Ok(parent) = child_of.get(entity) else {
		return true;
	};
	let mut current = parent.parent();
	let mut enclosing_root: Option<LodSceneLevel> = None;
	loop {
		if enclosing_root.is_none() {
			if let Ok(root) = level_roots.get(current) {
				enclosing_root = Some(root.0);
			}
		}
		if let Ok(desired) = host_levels.get(current) {
			return match enclosing_root {
				Some(root_level) => {
					root_level == *desired
						|| host_shows_level_root(
							current,
							root_level,
							children_q,
							level_roots_bags,
							level_roots,
							visibilities,
						)
				}
				None => true,
			};
		}
		let Ok(next) = child_of.get(current) else {
			return true;
		};
		current = next.parent();
	}
}

/// World walk used by exclusive produce / begin (bypasses query filters).
pub fn nested_host_parent_allows_refresh_world(world: &World, entity: Entity) -> bool {
	if world.get::<Disabled>(entity).is_some() {
		return false;
	}
	let Some(parent) = world.get::<ChildOf>(entity) else {
		return true;
	};
	let mut current = parent.parent();
	let mut enclosing_root: Option<LodSceneLevel> = None;
	loop {
		if enclosing_root.is_none() {
			enclosing_root = world.get::<LodLevelRoot>(current).map(|root| root.0);
		}
		if world.get::<LodSceneHost>(current).is_some() {
			return world.get::<LodSceneLevel>(current).is_none_or(|desired| {
				enclosing_root.is_none_or(|root_level| {
					root_level == *desired
						|| host_shows_level_root_world(world, current, root_level)
				})
			});
		}
		let Some(parent) = world.get::<ChildOf>(current) else {
			return true;
		};
		current = parent.parent();
	}
}

/// Desired [`LodSceneLevel`] of the nearest ancestor [`LodSceneHost`], or
/// [`LodSceneLevel::High`] when there is none (top-level ranking).
///
/// `host` is skipped — walk starts at its parent. Used for fulfill drain priority
/// `(parent_level, self_level)`.
pub fn parent_host_desired_or_high(
	host: Entity,
	child_of: &Query<&ChildOf, Allow<Disabled>>,
	host_levels: &Query<&LodSceneLevel, (With<LodSceneHost>, Allow<Disabled>)>,
) -> LodSceneLevel {
	let Ok(parent) = child_of.get(host) else {
		return LodSceneLevel::High;
	};
	let mut current = parent.parent();
	loop {
		if let Ok(level) = host_levels.get(current) {
			return *level;
		}
		let Ok(next) = child_of.get(current) else {
			return LodSceneLevel::High;
		};
		current = next.parent();
	}
}

/// Parent of level-root children (keeps level variants out of the structural child bag).
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRoots;

/// Direct child that owns [`LodLevelRoot`]s, if this host has one.
pub fn lod_level_roots_entity<D: QueryData, F: QueryFilter>(
	host_children: &Children,
	bags: &Query<D, F>,
) -> Option<Entity> {
	host_children.iter().find(|&child| bags.contains(child))
}

/// One spawned LOD variant under [`LodLevelRoots`] (keyed by [`LodSceneLevel`]).
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct LodLevelRoot(pub LodSceneLevel);

/// Request that a missing level root be spawned for this host.
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct LodLevelSpawnRequest {
	pub level: LodSceneLevel,
}

/// Build an initial host scene with a single active level root (lazy further levels).
pub fn lod_host_scene(
	level: LodSceneLevel,
	bounds: Aabb3d,
	content: impl Scene + 'static,
) -> impl Scene + 'static {
	let content_children: Vec<Box<dyn Scene>> = vec![Box::new(content)];
	let level_root: Box<dyn Scene> = Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		Visibility::Inherited
		Children [ {content_children} ]
	});
	let level_roots_children: Vec<Box<dyn Scene>> = vec![level_root];
	let roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
		Children [ {level_roots_children} ]
	});
	let host_children: Vec<Box<dyn Scene>> = vec![roots];
	let host_bounds = crate::scene::refresh::LodHostBounds(bounds);
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(host_bounds)
		Transform::default()
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

/// Host with an empty [`LodLevelRoots`] bag and a spawn request for `level`.
///
/// Chunk fulfill streams [`crate::SemanticLodScene::scene_chunks_with_level`] into the
/// pending root — used when skipping warm multi-root prewarm.
pub fn lod_host_scene_pending(level: LodSceneLevel, bounds: Aabb3d) -> impl Scene + 'static {
	let roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
	});
	let host_children: Vec<Box<dyn Scene>> = vec![roots];
	let host_bounds = crate::scene::refresh::LodHostBounds(bounds);
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(host_bounds)
		template_value(LodLevelSpawnRequest { level })
		Transform::default()
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

/// Host currently shows more than one ready level root.
///
/// Inserted when a newly revealed ready desired overlaps last frame's shown
/// band so extract still has meshes. [`settle_lod_level_root_visibility`] runs
/// at the start of the next frame and hides extras.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootOverlap;

/// Whether this level root should be Inherited given desired + overlap.
pub(crate) fn lod_root_should_show(
	root_level: LodSceneLevel,
	desired: LodSceneLevel,
	is_pending: bool,
	has_ready_desired: bool,
	has_ready_any: bool,
	warm_hold_level: Option<LodSceneLevel>,
	desired_was_shown: bool,
	this_was_shown_ready: bool,
) -> bool {
	if root_level == desired && !is_pending {
		return true;
	}
	if root_level == desired && is_pending && !has_ready_any {
		return true;
	}
	if !is_pending && warm_hold_level == Some(root_level) {
		return true;
	}
	// One-frame overlap: first reveal of a ready desired keeps the previously
	// shown ready band so extract still has last frame's meshes.
	if has_ready_desired && !desired_was_shown && !is_pending && this_was_shown_ready {
		return true;
	}
	false
}

fn apply_lod_level_root_visibility(
	commands: &mut Commands,
	desired: LodSceneLevel,
	child_ids: &[Entity],
	root_keys: &Query<&LodLevelRoot, Allow<Disabled>>,
	pending: &Query<(), (With<crate::LodLevelRootPending>, Allow<Disabled>)>,
	wants_cull: &Query<(), (With<crate::LodCullInFlight>, Allow<Disabled>)>,
	visibilities: &Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) -> bool {
	let mut has_ready_desired = false;
	let mut ready_levels: Vec<LodSceneLevel> = Vec::new();
	let mut desired_was_shown = false;
	let mut shown_ready: Vec<Entity> = Vec::new();
	for &child in child_ids {
		let Ok(root) = root_keys.get(child) else {
			continue;
		};
		if wants_cull.contains(child) {
			continue;
		}
		let is_pending = pending.contains(child);
		if !is_pending {
			ready_levels.push(root.0);
			if root.0 == desired {
				has_ready_desired = true;
			}
			if visibilities
				.get(child)
				.is_ok_and(|(v, disabled)| !disabled && lod_root_is_shown(*v))
			{
				shown_ready.push(child);
				if root.0 == desired {
					desired_was_shown = true;
				}
			}
		}
	}

	let warm_hold_level = if !has_ready_desired {
		closest_available_lod_level(desired, ready_levels.iter().copied())
	} else {
		None
	};
	let has_ready_any = !ready_levels.is_empty();

	let mut shown_ready_after = 0usize;
	for &child in child_ids {
		let Ok(root) = root_keys.get(child) else {
			continue;
		};
		if wants_cull.contains(child) {
			continue;
		}
		let is_pending = pending.contains(child);
		let show = lod_root_should_show(
			root.0,
			desired,
			is_pending,
			has_ready_desired,
			has_ready_any,
			warm_hold_level,
			desired_was_shown,
			shown_ready.contains(&child),
		);
		let currently_disabled = visibilities.get(child).is_ok_and(|(_, disabled)| disabled);
		let currently_shown = visibilities
			.get(child)
			.is_ok_and(|(v, disabled)| !disabled && lod_root_is_shown(*v));
		if show {
			if currently_disabled || !currently_shown {
				show_lod_tree(commands, child);
			}
			if !is_pending {
				shown_ready_after += 1;
			}
		} else if is_pending {
			if currently_shown {
				commands.entity(child).insert(Visibility::Hidden);
			}
		} else {
			let already_warm_hidden = visibilities
				.get(child)
				.is_ok_and(|(v, disabled)| disabled && matches!(*v, Visibility::Hidden));
			if !already_warm_hidden {
				hide_lod_tree(commands, child);
			}
		}
	}
	shown_ready_after > 1
}

/// After [`LodSceneLevel`] changes on a host: request missing roots and flip visibility.
///
/// Flow per host:
/// 1. Find the [`LodLevelRoots`] child (or request a spawn if the host/roots bag is missing).
/// 2. Visibility:
///    - ready desired → show it
///    - pending desired + **cold** (no ready root yet) → show pending (stream in)
///    - desired not ready yet + **warm** (some other ready root exists) → show the
///      single ready root [`closest_available_lod_level`] to desired (not every warm
///      root), including the frame *before* fulfill creates the pending desired root
///    - first frame a ready desired is revealed → also keep the previously shown
///      ready band ([`LodLevelRootOverlap`]); next frame
///      [`settle_lod_level_root_visibility`] hides extras
/// 3. If no matching root exists yet, insert [`LodLevelSpawnRequest`].
///
/// Pending roots ([`crate::LodLevelRootPending`]) count as present for spawn
/// requests. Roots with [`crate::LodCullInFlight`] count as absent (dying).
/// Warm-swap reveal is owned by chunk fulfill completion (which also leaves the
/// prior ready band shown for one frame).
pub fn sync_lod_level_roots(
	mut commands: Commands,
	hosts: Query<
		(Entity, &LodSceneLevel, Option<&Children>),
		(With<LodSceneHost>, Changed<LodSceneLevel>),
	>,
	level_roots_heads: Query<&Children, (With<LodLevelRoots>, Allow<Disabled>)>,
	root_keys: Query<&LodLevelRoot, Allow<Disabled>>,
	pending: Query<(), (With<crate::LodLevelRootPending>, Allow<Disabled>)>,
	wants_cull: Query<(), (With<crate::LodCullInFlight>, Allow<Disabled>)>,
	visibilities: Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) {
	for (host, level, host_children) in &hosts {
		let desired = *level;

		// No children yet → nothing to show/hide; ask for the first level root to be spawned.
		let Some(host_children) = host_children else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			continue;
		};

		let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_heads) else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			continue;
		};

		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};

		let mut found_desired = false;
		for child in root_children.iter() {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			if wants_cull.contains(child) {
				continue;
			}
			if root.0 == desired {
				found_desired = true;
			}
		}

		if apply_lod_level_root_visibility(
			&mut commands,
			desired,
			root_children,
			&root_keys,
			&pending,
			&wants_cull,
			&visibilities,
		) {
			commands.entity(host).insert(LodLevelRootOverlap);
		} else {
			commands.entity(host).remove::<LodLevelRootOverlap>();
		}

		// Desired root present (ready or pending) → drop stale spawn request.
		// Missing → request fulfill so a culled band can come back.
		if found_desired {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
		} else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
		}
	}
}

/// Hide extra ready roots after a one-frame overlap.
///
/// Runs at the start of [`crate::LodRefreshSystems::SyncRoots`], before
/// [`sync_lod_level_roots`], so a same-frame settle cannot undo the overlap
/// that sync just created. Chunk fulfill inserts [`LodLevelRootOverlap`] when
/// it first shows a completed root without hiding siblings.
pub fn settle_lod_level_root_visibility(
	mut commands: Commands,
	hosts: Query<
		(Entity, &LodSceneLevel, Option<&Children>),
		(With<LodSceneHost>, With<LodLevelRootOverlap>),
	>,
	level_roots_heads: Query<&Children, (With<LodLevelRoots>, Allow<Disabled>)>,
	root_keys: Query<&LodLevelRoot, Allow<Disabled>>,
	pending: Query<(), (With<crate::LodLevelRootPending>, Allow<Disabled>)>,
	wants_cull: Query<(), (With<crate::LodCullInFlight>, Allow<Disabled>)>,
	visibilities: Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) {
	for (host, level, host_children) in &hosts {
		let Some(host_children) = host_children else {
			commands.entity(host).remove::<LodLevelRootOverlap>();
			continue;
		};
		let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_heads) else {
			commands.entity(host).remove::<LodLevelRootOverlap>();
			continue;
		};
		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};
		if !apply_lod_level_root_visibility(
			&mut commands,
			*level,
			root_children,
			&root_keys,
			&pending,
			&wants_cull,
			&visibilities,
		) {
			commands.entity(host).remove::<LodLevelRootOverlap>();
		}
	}
}

/// Plugin: marker types only. Prefer [`crate::LodRefreshCorePlugin`] for runtime systems
/// (track / sync / fulfill ordering).
pub struct LodSceneHostPlugin;

impl Plugin for LodSceneHostPlugin {
	fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ready_desired_always_shown() {
		assert!(lod_root_should_show(
			LodSceneLevel::High,
			LodSceneLevel::High,
			false,
			true,
			true,
			None,
			true,
			true,
		));
	}

	#[test]
	fn overlap_keeps_previous_until_desired_was_shown() {
		assert!(lod_root_should_show(
			LodSceneLevel::Medium,
			LodSceneLevel::High,
			false,
			true,
			true,
			None,
			false,
			true,
		));
	}

	#[test]
	fn after_desired_was_shown_previous_hides() {
		assert!(!lod_root_should_show(
			LodSceneLevel::Medium,
			LodSceneLevel::High,
			false,
			true,
			true,
			None,
			true,
			true,
		));
	}

	#[test]
	fn warm_hold_shows_closest_ready() {
		assert!(lod_root_should_show(
			LodSceneLevel::Medium,
			LodSceneLevel::High,
			false,
			false,
			true,
			Some(LodSceneLevel::Medium),
			false,
			true,
		));
	}

	#[derive(Component)]
	struct MeshStandIn;

	#[test]
	fn hide_lod_tree_stamps_recursive_disabled() {
		let mut world = World::new();
		let leaf = world.spawn(MeshStandIn).id();
		let root = world.spawn(Visibility::Inherited).id();
		world.entity_mut(root).add_child(leaf);
		hide_lod_tree_world(&mut world.entity_mut(root));

		assert!(world.get::<Visibility>(root).is_some_and(|v| matches!(*v, Visibility::Hidden)));
		assert!(world.get::<Disabled>(root).is_some());
		assert!(world.get::<Disabled>(leaf).is_some());

		let mut default_meshes = world.query_filtered::<Entity, With<MeshStandIn>>();
		assert_eq!(default_meshes.iter(&world).count(), 0);
		let mut allowed_meshes =
			world.query_filtered::<Entity, (With<MeshStandIn>, Allow<Disabled>)>();
		assert_eq!(allowed_meshes.iter(&world).count(), 1);
	}

	#[test]
	fn showing_parent_prunes_nested_hidden_tree() {
		let mut world = World::new();
		let nested_leaf = world.spawn(MeshStandIn).id();
		let nested_root = world.spawn(Visibility::Inherited).id();
		world.entity_mut(nested_root).add_child(nested_leaf);
		let shown_leaf = world.spawn(MeshStandIn).id();
		let shown_root = world.spawn(Visibility::Inherited).id();
		world.entity_mut(shown_root).add_child(shown_leaf);
		let parent = world.spawn(Visibility::Inherited).id();
		world.entity_mut(parent).add_children(&[nested_root, shown_root]);

		hide_lod_tree_now(&mut world, nested_root);
		hide_lod_tree_now(&mut world, parent);
		show_lod_tree_now(&mut world, parent);

		assert!(world.get::<Disabled>(parent).is_none());
		assert!(world.get::<Disabled>(shown_root).is_none());
		assert!(world.get::<Disabled>(shown_leaf).is_none());
		assert!(world.get::<Disabled>(nested_root).is_some());
		assert!(world.get::<Disabled>(nested_leaf).is_some());
		assert!(world.get::<LodTreeHideActive>(nested_root).is_some());

		show_lod_tree_now(&mut world, nested_root);
		assert!(world.get::<Disabled>(nested_root).is_none());
		assert!(world.get::<Disabled>(nested_leaf).is_none());
	}

	#[test]
	fn showing_child_beneath_hidden_parent_defers_enable() {
		let mut world = World::new();
		let leaf = world.spawn(MeshStandIn).id();
		let child = world.spawn(Visibility::Inherited).id();
		world.entity_mut(child).add_child(leaf);
		let parent = world.spawn(Visibility::Inherited).id();
		world.entity_mut(parent).add_child(child);

		hide_lod_tree_now(&mut world, child);
		hide_lod_tree_now(&mut world, parent);
		show_lod_tree_now(&mut world, child);

		assert!(world.get::<LodTreeHideActive>(child).is_none());
		assert!(world.get::<Disabled>(child).is_some());
		assert!(world.get::<Disabled>(leaf).is_some());

		show_lod_tree_now(&mut world, parent);
		assert!(world.get::<Disabled>(child).is_none());
		assert!(world.get::<Disabled>(leaf).is_none());
	}

	#[test]
	fn showing_parent_enables_hidden_pending_root() {
		let mut world = World::new();
		let pending_leaf = world.spawn(MeshStandIn).id();
		let pending_root = world.spawn((Visibility::Hidden, crate::LodLevelRootPending)).id();
		world.entity_mut(pending_root).add_child(pending_leaf);
		let parent = world.spawn(Visibility::Inherited).id();
		world.entity_mut(parent).add_child(pending_root);

		hide_lod_tree_now(&mut world, parent);
		show_lod_tree_now(&mut world, parent);

		assert!(world.get::<Disabled>(pending_root).is_none());
		assert!(world.get::<Disabled>(pending_leaf).is_none());
		assert!(world
			.get::<Visibility>(pending_root)
			.is_some_and(|v| matches!(*v, Visibility::Hidden)));
	}

	fn tree_with_leaves(world: &mut World, leaves: usize) -> (Entity, Vec<Entity>) {
		let leaf_ids: Vec<_> = (0..leaves).map(|_| world.spawn(MeshStandIn).id()).collect();
		let root = world.spawn(Visibility::Inherited).id();
		world.entity_mut(root).add_children(&leaf_ids);
		(root, leaf_ids)
	}

	fn drain_until_empty(world: &mut World) {
		for _ in 0..64 {
			if world.resource::<LodTreeVisQueue>().is_empty() {
				return;
			}
			drain_lod_tree_vis(world);
		}
		panic!("vis drain did not empty");
	}

	#[test]
	fn hide_budget_spreads_descendant_disabled() {
		let mut world = World::new();
		world.init_resource::<LodTreeVisQueue>();
		world.insert_resource(LodTreeVisBudget {
			time_per_frame: Duration::from_secs(1),
			entities_per_frame: 2,
		});
		let (root, leaves) = tree_with_leaves(&mut world, 6);
		hide_lod_tree_now(&mut world, root);

		assert!(world.get::<Disabled>(root).is_some());
		assert!(world.get::<LodTreeHideActive>(root).is_some());
		let disabled_after_hide =
			leaves.iter().filter(|leaf| world.get::<Disabled>(**leaf).is_some()).count();
		assert_eq!(disabled_after_hide, 0, "descendants wait for drain");

		drain_lod_tree_vis(&mut world);
		let disabled_after_one =
			leaves.iter().filter(|leaf| world.get::<Disabled>(**leaf).is_some()).count();
		assert_eq!(disabled_after_one, 2);

		drain_until_empty(&mut world);
		assert!(leaves.iter().all(|leaf| world.get::<Disabled>(*leaf).is_some()));
	}

	#[test]
	fn show_budget_spreads_descendant_enable() {
		let mut world = World::new();
		world.init_resource::<LodTreeVisQueue>();
		world.insert_resource(LodTreeVisBudget {
			time_per_frame: Duration::from_secs(1),
			entities_per_frame: 2,
		});
		let (root, leaves) = tree_with_leaves(&mut world, 6);
		hide_lod_tree_now(&mut world, root);
		drain_until_empty(&mut world);
		assert!(leaves.iter().all(|leaf| world.get::<Disabled>(*leaf).is_some()));

		show_lod_tree_now(&mut world, root);
		assert!(world.get::<Disabled>(root).is_none());
		let still_disabled =
			leaves.iter().filter(|leaf| world.get::<Disabled>(**leaf).is_some()).count();
		assert_eq!(still_disabled, 6, "descendants wait for drain");

		drain_lod_tree_vis(&mut world);
		let still_disabled =
			leaves.iter().filter(|leaf| world.get::<Disabled>(**leaf).is_some()).count();
		assert_eq!(still_disabled, 4);

		drain_until_empty(&mut world);
		assert!(leaves.iter().all(|leaf| world.get::<Disabled>(*leaf).is_none()));
	}

	#[test]
	fn queued_show_cancels_pending_hide() {
		let mut world = World::new();
		world.init_resource::<LodTreeVisQueue>();
		world.insert_resource(LodTreeVisBudget {
			time_per_frame: Duration::from_secs(1),
			entities_per_frame: 1,
		});
		let (root, leaves) = tree_with_leaves(&mut world, 4);
		hide_lod_tree_now(&mut world, root);
		show_lod_tree_now(&mut world, root);
		drain_until_empty(&mut world);
		assert!(world.get::<Disabled>(root).is_none());
		assert!(leaves.iter().all(|leaf| world.get::<Disabled>(*leaf).is_none()));
	}

	#[test]
	fn queued_hide_show_hide_converges_disabled() {
		let mut world = World::new();
		world.init_resource::<LodTreeVisQueue>();
		world.insert_resource(LodTreeVisBudget {
			time_per_frame: Duration::from_secs(1),
			entities_per_frame: 2,
		});
		let (root, leaves) = tree_with_leaves(&mut world, 6);

		hide_lod_tree_now(&mut world, root);
		drain_lod_tree_vis(&mut world);
		let disabled_after_hide =
			leaves.iter().filter(|leaf| world.get::<Disabled>(**leaf).is_some()).count();
		assert_eq!(disabled_after_hide, 2);

		show_lod_tree_now(&mut world, root);
		assert!(world.get::<Disabled>(root).is_none());
		drain_lod_tree_vis(&mut world);
		assert!(
			!world.resource::<LodTreeVisQueue>().is_empty(),
			"show drain is partial so leftover enable work remains"
		);

		hide_lod_tree_now(&mut world, root);
		assert!(world.get::<Disabled>(root).is_some());
		assert!(world.get::<LodTreeHideActive>(root).is_some());
		drain_until_empty(&mut world);
		assert!(leaves.iter().all(|leaf| world.get::<Disabled>(*leaf).is_some()));
	}

	#[test]
	fn queued_show_prunes_nested_hidden_tree() {
		let mut world = World::new();
		world.init_resource::<LodTreeVisQueue>();
		world.insert_resource(LodTreeVisBudget {
			time_per_frame: Duration::from_secs(1),
			entities_per_frame: 2,
		});

		let nested_leaf = world.spawn(MeshStandIn).id();
		let nested_root = world.spawn(Visibility::Inherited).id();
		world.entity_mut(nested_root).add_child(nested_leaf);
		let shown_leaf = world.spawn(MeshStandIn).id();
		let shown_root = world.spawn(Visibility::Inherited).id();
		world.entity_mut(shown_root).add_child(shown_leaf);
		let parent = world.spawn(Visibility::Inherited).id();
		world.entity_mut(parent).add_children(&[nested_root, shown_root]);

		hide_lod_tree_now(&mut world, nested_root);
		drain_until_empty(&mut world);
		hide_lod_tree_now(&mut world, parent);
		drain_until_empty(&mut world);
		show_lod_tree_now(&mut world, parent);
		drain_until_empty(&mut world);

		assert!(world.get::<Disabled>(parent).is_none());
		assert!(world.get::<Disabled>(shown_root).is_none());
		assert!(world.get::<Disabled>(shown_leaf).is_none());
		assert!(world.get::<Disabled>(nested_root).is_some());
		assert!(world.get::<Disabled>(nested_leaf).is_some());
		assert!(world.get::<LodTreeHideActive>(nested_root).is_some());

		show_lod_tree_now(&mut world, nested_root);
		drain_until_empty(&mut world);
		assert!(world.get::<Disabled>(nested_root).is_none());
		assert!(world.get::<Disabled>(nested_leaf).is_none());
	}
}
