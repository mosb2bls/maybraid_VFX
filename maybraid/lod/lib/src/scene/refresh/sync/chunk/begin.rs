//! Type-erased job begin with shared Presence / Desired admission.
//!
//! A **capped** host scan builds candidate lists; admission walks those lists in
//! class order after sorting each list by viewer XZ distance (near first). The
//! scan is still round-robin and limited to a multiple of remaining begin slots
//! ([`LodChunkFulfillBudget::begin_scan_per_frame`]) so a saturated type does not
//! starve later `T`s. Distance only ranks **classified** candidates. The scan is
//! skipped when the shared clock is empty. Active begin quota is rolled into
//! Desired (begins only start desired-level jobs). Each admitted job charges
//! shared begin weight for **prefilled** primitives only
//! ([`LodChunkFulfillBudget::begin_prefill_weights_per_job`]); lazy tails drain later.

use bevy::ecs::entity_disabling::Disabled;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::lod_ref::{point_bounds, LodNode, LodNodeBounds, LodNodePose, LodRef};
use crate::scene::host::{
	lod_level_roots_entity, nested_host_parent_allows_refresh,
	nested_host_parent_allows_refresh_world, parent_host_desired_or_high, LodLevelRoot,
	LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::refresh::LodHostBounds;
use crate::scene::SemanticLodScene;

use super::super::super::viewer::LodViewer;
use super::schedule::{admit_begin, begin_scan_limit, charge_begin_weight, LevelBand};
use super::types::{
	FulfillClass, LodChunkBeginClock, LodChunkBeginScanCursor, LodChunkFulfillBudget,
	LodChunkFulfillment, LodCullInFlight, LodLevelRootPending, LodLevelRootStreamed,
};
use super::util::has_present_root;
use crate::scene::chunk::materialize_front;
use crate::scene::LodLevelProducer;

/// Reused candidate buckets for the shared type-erased begin scan (cleared each frame).
#[derive(Resource, Default)]
pub(crate) struct LodChunkBeginCandidateScratch {
	presence_near: Vec<BeginCandidate>,
	presence_far: Vec<BeginCandidate>,
	desired_near: Vec<BeginCandidate>,
	desired_far: Vec<BeginCandidate>,
	/// Stale spawn requests removed after the scan (avoids `entity_mut` during scratch borrows).
	removals: Vec<Entity>,
	/// Hosts examined this frame (at most [`LodChunkFulfillBudget::begin_scan_per_frame`]).
	scan_batch: Vec<(Entity, LodLevelSpawnRequest)>,
}

#[derive(Clone, Copy)]
struct BeginCandidate {
	host: Entity,
	roots_entity: Entity,
	level: LodSceneLevel,
	cold: bool,
	parent_desired: LodSceneLevel,
	/// Viewer-to-host XZ distance squared (bounds center, then translation).
	dist_xz: f32,
}

fn roll_active_into_desired(clock: &mut LodChunkBeginClock) {
	clock.desired_remaining = clock.desired_remaining.saturating_add(clock.active_remaining);
	clock.active_remaining = 0;
}

fn roll_presence_into_desired(clock: &mut LodChunkBeginClock) {
	clock.desired_remaining = clock.desired_remaining.saturating_add(clock.presence_remaining);
	clock.presence_remaining = 0;
}

fn roll_desired_into_presence(clock: &mut LodChunkBeginClock) {
	clock.presence_remaining = clock.presence_remaining.saturating_add(clock.desired_remaining);
	clock.desired_remaining = 0;
}

/// Start a pending root + queue from [`LodLevelSpawnRequest`].
pub fn begin_chunk_lod_fulfill<T: Component + SemanticLodScene>(
	mut commands: Commands,
	viewer: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, With<LodViewer>)>,
	mut begin_clock: ResMut<LodChunkBeginClock>,
	budget: Res<LodChunkFulfillBudget>,
	mut host_sets: ParamSet<(
		Query<(Entity, &LodLevelSpawnRequest), (With<LodSceneHost>, With<T>)>,
		Query<&'static T, With<LodSceneHost>>,
	)>,
	root_keys: Query<&LodLevelRoot, Allow<Disabled>>,
	pending: Query<(), (With<LodLevelRootPending>, Allow<Disabled>)>,
	wants_cull: Query<(), (With<LodCullInFlight>, Allow<Disabled>)>,
	child_of: Query<&ChildOf, Allow<Disabled>>,
	host_levels: Query<&LodSceneLevel, (With<LodSceneHost>, Allow<Disabled>)>,
	children_q: Query<&Children, Allow<Disabled>>,
	level_roots_bags: Query<(), (With<LodLevelRoots>, Allow<Disabled>)>,
	visibilities: Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
	host_pose: Query<(&Transform, Option<&LodHostBounds>), With<LodSceneHost>>,
	mut scan_cursor: Local<u32>,
) {
	let Ok((viewer_entity, pose, viewer_bounds)) = viewer.single() else {
		return;
	};
	let driver_bounds = viewer_bounds
		.map(|b| b.0)
		.unwrap_or_else(|| point_bounds(pose.current.translation));
	let lod_ref = pose.as_lod_ref(viewer_entity, &driver_bounds);
	let viewer_xz = pose.current.translation;

	roll_active_into_desired(&mut begin_clock);

	let scan_limit = begin_scan_limit(&budget, &begin_clock);
	if scan_limit == 0 {
		return;
	}

	let mut presence_near = Vec::new();
	let mut presence_far = Vec::new();
	let mut desired_near = Vec::new();
	let mut desired_far = Vec::new();

	let start = *scan_cursor;
	let mut classified = 0u32;
	let mut index = 0u32;
	let mut next_cursor = 0u32;

	for (host, request) in host_sets.p0().iter() {
		let i = index;
		index += 1;
		if i < start {
			continue;
		}
		if let Some(candidate) = classify_begin_candidate(
			host,
			request,
			&mut commands,
			&root_keys,
			&pending,
			&wants_cull,
			&child_of,
			&host_levels,
			&children_q,
			&level_roots_bags,
			&visibilities,
		) {
			push_begin_candidate(
				with_host_xz_distance(candidate, viewer_xz, host, &host_pose),
				&mut presence_near,
				&mut presence_far,
				&mut desired_near,
				&mut desired_far,
			);
		}
		classified += 1;
		if classified >= scan_limit {
			next_cursor = i.saturating_add(1);
			break;
		}
	}

	if classified < scan_limit && start > 0 {
		index = 0;
		for (host, request) in host_sets.p0().iter() {
			let i = index;
			index += 1;
			if i >= start {
				break;
			}
			if let Some(candidate) = classify_begin_candidate(
				host,
				request,
				&mut commands,
				&root_keys,
				&pending,
				&wants_cull,
				&child_of,
				&host_levels,
				&children_q,
				&level_roots_bags,
				&visibilities,
			) {
				push_begin_candidate(
					with_host_xz_distance(candidate, viewer_xz, host, &host_pose),
					&mut presence_near,
					&mut presence_far,
					&mut desired_near,
					&mut desired_far,
				);
			}
			classified += 1;
			if classified >= scan_limit {
				next_cursor = i.saturating_add(1);
				break;
			}
		}
		if classified < scan_limit {
			next_cursor = 0;
		}
	} else if classified < scan_limit {
		next_cursor = 0;
	}

	*scan_cursor = next_cursor;

	sort_begin_near_first(&mut presence_near);
	sort_begin_near_first(&mut presence_far);
	sort_begin_near_first(&mut desired_near);
	sort_begin_near_first(&mut desired_far);

	let presence_first = matches!(begin_clock.first_class, FulfillClass::Presence);
	if presence_first {
		admit_candidates(
			&presence_near,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&presence_far,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		roll_presence_into_desired(&mut begin_clock);
		admit_candidates(
			&desired_near,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&desired_far,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
	} else {
		admit_candidates(
			&desired_near,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&desired_far,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		roll_desired_into_presence(&mut begin_clock);
		admit_candidates(
			&presence_near,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&presence_far,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
	}
}

/// Start pending roots once for all semantic host types.
///
/// Scene-specific work is dispatched through [`LodLevelProducer`], while
/// candidate scanning, distance ordering, and admission consume one shared
/// cursor and budget.
pub fn begin_chunk_lod_fulfill_erased(world: &mut World) {
	let Some((viewer_entity, pose, viewer_bounds)) = single_viewer(world) else {
		return;
	};
	let driver_bounds = viewer_bounds.unwrap_or_else(|| point_bounds(pose.current.translation));
	let lod_ref = pose.as_lod_ref(viewer_entity, &driver_bounds);
	let viewer_xz = pose.current.translation;

	{
		let mut clock = world.resource_mut::<LodChunkBeginClock>();
		roll_active_into_desired(&mut clock);
	}
	let budget = *world.resource::<LodChunkFulfillBudget>();
	let scan_limit = begin_scan_limit(&budget, world.resource::<LodChunkBeginClock>());
	if scan_limit == 0 {
		return;
	}

	let (scanned, next_cursor) = {
		let mut scratch =
			world.remove_resource::<LodChunkBeginCandidateScratch>().unwrap_or_default();
		scratch.presence_near.clear();
		scratch.presence_far.clear();
		scratch.desired_near.clear();
		scratch.desired_far.clear();
		scratch.removals.clear();
		scratch.scan_batch.clear();
		let (scanned, next_cursor) =
			scan_erased_begin_hosts(world, viewer_xz, scan_limit, &mut scratch);
		if scanned {
			sort_begin_near_first(&mut scratch.presence_near);
			sort_begin_near_first(&mut scratch.presence_far);
			sort_begin_near_first(&mut scratch.desired_near);
			sort_begin_near_first(&mut scratch.desired_far);
		}
		world.insert_resource(scratch);
		(scanned, next_cursor)
	};
	if !scanned {
		world.resource_mut::<LodChunkBeginScanCursor>().0 = 0;
		return;
	}
	world.resource_mut::<LodChunkBeginScanCursor>().0 = next_cursor;

	let (removals, presence_near, presence_far, desired_near, desired_far) = {
		let mut scratch = world
			.remove_resource::<LodChunkBeginCandidateScratch>()
			.expect("LodChunkBeginCandidateScratch inserted by LodChunkBudgetPlugin");
		let buckets = (
			std::mem::take(&mut scratch.removals),
			std::mem::take(&mut scratch.presence_near),
			std::mem::take(&mut scratch.presence_far),
			std::mem::take(&mut scratch.desired_near),
			std::mem::take(&mut scratch.desired_far),
		);
		world.insert_resource(scratch);
		buckets
	};
	for host in removals {
		world.entity_mut(host).remove::<LodLevelSpawnRequest>();
	}

	let presence_first =
		matches!(world.resource::<LodChunkBeginClock>().first_class, FulfillClass::Presence);
	if presence_first {
		admit_candidates_world(world, &presence_near, FulfillClass::Presence, &lod_ref, &budget);
		admit_candidates_world(world, &presence_far, FulfillClass::Presence, &lod_ref, &budget);
		{
			let mut clock = world.resource_mut::<LodChunkBeginClock>();
			roll_presence_into_desired(&mut clock);
		}
		admit_candidates_world(world, &desired_near, FulfillClass::Desired, &lod_ref, &budget);
		admit_candidates_world(world, &desired_far, FulfillClass::Desired, &lod_ref, &budget);
	} else {
		admit_candidates_world(world, &desired_near, FulfillClass::Desired, &lod_ref, &budget);
		admit_candidates_world(world, &desired_far, FulfillClass::Desired, &lod_ref, &budget);
		{
			let mut clock = world.resource_mut::<LodChunkBeginClock>();
			roll_desired_into_presence(&mut clock);
		}
		admit_candidates_world(world, &presence_near, FulfillClass::Presence, &lod_ref, &budget);
		admit_candidates_world(world, &presence_far, FulfillClass::Presence, &lod_ref, &budget);
	}
}

/// Round-robin host scan for type-erased begin (mirrors [`begin_chunk_lod_fulfill`]).
///
/// Returns `(false, 0)` when no hosts hold a spawn request.
fn scan_erased_begin_hosts(
	world: &mut World,
	viewer_xz: Vec3,
	scan_limit: u32,
	scratch: &mut LodChunkBeginCandidateScratch,
) -> (bool, u32) {
	let start = world.resource::<LodChunkBeginScanCursor>().0;
	let mut classified = 0u32;
	let mut index = 0u32;
	let mut next_cursor = 0u32;
	let mut saw_host = false;

	{
		let mut query = world
			.query_filtered::<(Entity, &LodLevelSpawnRequest), (With<LodSceneHost>, With<LodLevelProducer>)>(
			);
		for (host, request) in query.iter(world) {
			saw_host = true;
			let i = index;
			index += 1;
			if i < start {
				continue;
			}
			scratch.scan_batch.push((host, *request));
			classified += 1;
			if classified >= scan_limit {
				next_cursor = i.saturating_add(1);
				break;
			}
		}
	}

	if !saw_host {
		return (false, 0);
	}

	if classified < scan_limit && start > 0 {
		index = 0;
		let mut query = world
			.query_filtered::<(Entity, &LodLevelSpawnRequest), (With<LodSceneHost>, With<LodLevelProducer>)>(
			);
		for (host, request) in query.iter(world) {
			let i = index;
			index += 1;
			if i >= start {
				break;
			}
			scratch.scan_batch.push((host, *request));
			classified += 1;
			if classified >= scan_limit {
				next_cursor = i.saturating_add(1);
				break;
			}
		}
		if classified < scan_limit {
			next_cursor = 0;
		}
	} else if classified < scan_limit {
		next_cursor = 0;
	}

	for index in 0..scratch.scan_batch.len() {
		let (host, request) = scratch.scan_batch[index];
		classify_and_push_erased_begin(world, host, request, viewer_xz, scratch);
	}

	(true, next_cursor)
}

fn classify_and_push_erased_begin(
	world: &mut World,
	host: Entity,
	request: LodLevelSpawnRequest,
	viewer_xz: Vec3,
	scratch: &mut LodChunkBeginCandidateScratch,
) {
	match classify_begin_candidate_world(world, host, request) {
		BeginClassification::Candidate(candidate) => push_begin_candidate(
			with_host_xz_distance_world(candidate, viewer_xz, world),
			&mut scratch.presence_near,
			&mut scratch.presence_far,
			&mut scratch.desired_near,
			&mut scratch.desired_far,
		),
		BeginClassification::RemoveRequest => scratch.removals.push(host),
		BeginClassification::Skip => {}
	}
}

fn single_viewer(world: &mut World) -> Option<(Entity, LodNodePose, Option<Aabb3d>)> {
	let mut query = world.query_filtered::<
		(Entity, &LodNodePose, Option<&LodNodeBounds>),
		(With<LodNode>, With<LodViewer>),
	>();
	let mut iter = query.iter(world);
	let (entity, pose, bounds) = iter.next()?;
	let result = (entity, *pose, bounds.map(|bounds| bounds.0));
	if iter.next().is_some() {
		return None;
	}
	Some(result)
}

enum BeginClassification {
	Candidate(BeginCandidate),
	RemoveRequest,
	Skip,
}

fn classify_begin_candidate_world(
	world: &World,
	host: Entity,
	request: LodLevelSpawnRequest,
) -> BeginClassification {
	let Some(desired) = world.get::<LodSceneLevel>(host).copied() else {
		return BeginClassification::Skip;
	};
	if request.level != desired {
		return BeginClassification::RemoveRequest;
	}
	let Some(roots_entity) = level_roots_entity_world(world, host) else {
		return BeginClassification::RemoveRequest;
	};
	let root_children: Vec<_> = world
		.get::<Children>(roots_entity)
		.map(|children| children.iter().collect())
		.unwrap_or_default();
	let has_any_level_root = root_children.iter().any(|root| {
		world.get::<LodLevelRoot>(*root).is_some() && world.get::<LodCullInFlight>(*root).is_none()
	});
	if !nested_host_parent_allows_refresh_world(world, host) && has_any_level_root {
		return BeginClassification::RemoveRequest;
	}
	if root_children.iter().any(|root| {
		world.get::<LodLevelRoot>(*root).is_some_and(|key| key.0 == request.level)
			&& world.get::<LodCullInFlight>(*root).is_none()
	}) {
		return BeginClassification::RemoveRequest;
	}
	let cold = !has_any_level_root;
	BeginClassification::Candidate(BeginCandidate {
		host,
		roots_entity,
		level: request.level,
		cold,
		parent_desired: parent_host_desired_or_high_world(world, host),
		dist_xz: f32::MAX,
	})
}

fn level_roots_entity_world(world: &World, host: Entity) -> Option<Entity> {
	world
		.get::<Children>(host)?
		.iter()
		.find(|child| world.get::<LodLevelRoots>(*child).is_some())
}

fn parent_host_desired_or_high_world(world: &World, host: Entity) -> LodSceneLevel {
	let Some(parent) = world.get::<ChildOf>(host) else {
		return LodSceneLevel::High;
	};
	let mut current = parent.parent();
	loop {
		if world.get::<LodSceneHost>(current).is_some() {
			return world.get::<LodSceneLevel>(current).copied().unwrap_or(LodSceneLevel::High);
		}
		let Some(parent) = world.get::<ChildOf>(current) else {
			return LodSceneLevel::High;
		};
		current = parent.parent();
	}
}

fn with_host_xz_distance_world(
	mut candidate: BeginCandidate,
	origin: Vec3,
	world: &World,
) -> BeginCandidate {
	if let Some(transform) = world.get::<Transform>(candidate.host) {
		candidate.dist_xz =
			host_xz_distance2(origin, transform, world.get::<LodHostBounds>(candidate.host));
	}
	candidate
}

fn admit_candidates_world(
	world: &mut World,
	candidates: &[BeginCandidate],
	class: FulfillClass,
	lod_ref: &LodRef,
	budget: &LodChunkFulfillBudget,
) {
	for candidate in candidates {
		let Some(producer) = world.get::<LodLevelProducer>(candidate.host).copied() else {
			continue;
		};
		let Some(actual) = producer.level_for(world, candidate.host, lod_ref) else {
			continue;
		};
		if actual != candidate.level {
			world
				.entity_mut(candidate.host)
				.insert((actual, LodLevelSpawnRequest { level: actual }));
			continue;
		}
		let admitted = {
			let mut clock = world.resource_mut::<LodChunkBeginClock>();
			admit_begin(&mut clock, class)
		};
		if !admitted {
			break;
		}
		let Some(chunk) = producer.chunk_for(world, candidate.host, lod_ref, candidate.level)
		else {
			continue;
		};
		let expected = chunk.total_primitives();
		let mut queue = chunk.into_fulfill_queue();
		let prefill = world
			.resource::<LodChunkBeginClock>()
			.weight_remaining
			.min(budget.begin_prefill_weights_per_job);
		let begin_weight = materialize_front(&mut queue, prefill);
		{
			let mut clock = world.resource_mut::<LodChunkBeginClock>();
			charge_begin_weight(&mut clock, begin_weight.max(1));
		}

		let initial_visibility =
			if candidate.cold { Visibility::Inherited } else { Visibility::Hidden };
		let root_entity = world
			.spawn((
				LodLevelRoot(candidate.level),
				LodLevelRootPending,
				Transform::default(),
				initial_visibility,
			))
			.id();
		let fulfillment = LodChunkFulfillment {
			queue,
			expected,
			spawned: 0,
			cold: candidate.cold,
			host: candidate.host,
			parent_desired: candidate.parent_desired,
			nested_streamed: 0,
			nested_required: None,
		};
		if fulfillment.is_content_complete() {
			world.entity_mut(root_entity).insert(LodLevelRootStreamed);
		}
		world.entity_mut(root_entity).insert(fulfillment);
		world.entity_mut(candidate.roots_entity).add_child(root_entity);
		world.entity_mut(candidate.host).remove::<LodLevelSpawnRequest>();
	}
}

fn push_begin_candidate(
	candidate: BeginCandidate,
	presence_near: &mut Vec<BeginCandidate>,
	presence_far: &mut Vec<BeginCandidate>,
	desired_near: &mut Vec<BeginCandidate>,
	desired_far: &mut Vec<BeginCandidate>,
) {
	let near = LevelBand::from_level(candidate.level).is_near();
	match (candidate.cold, near) {
		(true, true) => presence_near.push(candidate),
		(true, false) => presence_far.push(candidate),
		(false, true) => desired_near.push(candidate),
		(false, false) => desired_far.push(candidate),
	}
}

/// World-space XZ distance² from the viewer to the host footprint center.
///
/// Forest grove hosts sit at [`Transform::IDENTITY`] with a world-space
/// [`LodHostBounds`]; translation alone would collapse them all to the origin.
fn host_xz_distance2(origin: Vec3, transform: &Transform, bounds: Option<&LodHostBounds>) -> f32 {
	let center = match bounds {
		Some(b) => transform.transform_point((Vec3::from(b.0.min) + Vec3::from(b.0.max)) * 0.5),
		None => transform.translation,
	};
	let dx = center.x - origin.x;
	let dz = center.z - origin.z;
	dx * dx + dz * dz
}

fn with_host_xz_distance(
	mut candidate: BeginCandidate,
	origin: Vec3,
	host: Entity,
	host_pose: &Query<(&Transform, Option<&LodHostBounds>), With<LodSceneHost>>,
) -> BeginCandidate {
	candidate.dist_xz = host_pose
		.get(host)
		.map(|(transform, bounds)| host_xz_distance2(origin, transform, bounds))
		.unwrap_or(f32::MAX);
	candidate
}

fn sort_begin_near_first(list: &mut [BeginCandidate]) {
	list.sort_by(|a, b| a.dist_xz.total_cmp(&b.dist_xz));
}

fn classify_begin_candidate(
	host: Entity,
	request: &LodLevelSpawnRequest,
	commands: &mut Commands,
	root_keys: &Query<&LodLevelRoot, Allow<Disabled>>,
	pending: &Query<(), (With<LodLevelRootPending>, Allow<Disabled>)>,
	wants_cull: &Query<(), (With<LodCullInFlight>, Allow<Disabled>)>,
	child_of: &Query<&ChildOf, Allow<Disabled>>,
	host_levels: &Query<&LodSceneLevel, (With<LodSceneHost>, Allow<Disabled>)>,
	children_q: &Query<&Children, Allow<Disabled>>,
	level_roots_bags: &Query<(), (With<LodLevelRoots>, Allow<Disabled>)>,
	visibilities: &Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) -> Option<BeginCandidate> {
	let Ok(desired) = host_levels.get(host) else {
		return None;
	};
	if request.level != *desired {
		commands.entity(host).remove::<LodLevelSpawnRequest>();
		return None;
	}

	let Ok(host_children) = children_q.get(host) else {
		commands.entity(host).remove::<LodLevelSpawnRequest>();
		return None;
	};
	let Some(roots_entity) = lod_level_roots_entity(host_children, level_roots_bags) else {
		commands.entity(host).remove::<LodLevelSpawnRequest>();
		return None;
	};

	let root_children = children_q.get(roots_entity).ok();
	let has_any_level_root = root_children.is_some_and(|children| {
		children
			.iter()
			.any(|child| root_keys.contains(child) && !wants_cull.contains(child))
	});

	if !nested_host_parent_allows_refresh(
		host,
		child_of,
		host_levels,
		root_keys,
		children_q,
		level_roots_bags,
		visibilities,
	) && has_any_level_root
	{
		commands.entity(host).remove::<LodLevelSpawnRequest>();
		return None;
	}

	let mut cold = true;
	if let Some(root_children) = root_children {
		let mut already_ready = false;
		let mut already_pending = false;
		for child in root_children.iter() {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			if wants_cull.contains(child) {
				continue;
			}
			if root.0 != request.level {
				continue;
			}
			if pending.contains(child) {
				already_pending = true;
			} else {
				already_ready = true;
			}
		}
		if already_ready || already_pending {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			return None;
		}
		cold = !has_present_root(root_children, root_keys, wants_cull);
	}

	let parent_desired = parent_host_desired_or_high(host, child_of, host_levels);
	Some(BeginCandidate {
		host,
		roots_entity,
		level: request.level,
		cold,
		parent_desired,
		dist_xz: f32::MAX,
	})
}

fn admit_candidates<T: Component + SemanticLodScene>(
	candidates: &[BeginCandidate],
	class: FulfillClass,
	commands: &mut Commands,
	lod_ref: &LodRef,
	begin_clock: &mut LodChunkBeginClock,
	budget: &LodChunkFulfillBudget,
	host_sets: &mut ParamSet<(
		Query<(Entity, &LodLevelSpawnRequest), (With<LodSceneHost>, With<T>)>,
		Query<&'static T, With<LodSceneHost>>,
	)>,
) {
	for candidate in candidates {
		{
			let scenes = host_sets.p1();
			let Ok(scene) = scenes.get(candidate.host) else {
				continue;
			};
			// Stale desired (identity spawn, produce not yet in range) must follow
			// the viewer. Do not drop the request: [`LodSceneCulls::should_cull`]
			// is for other bands, not the camera band.
			let actual = scene.scene_lod_level(lod_ref);
			if actual != candidate.level {
				commands.entity(candidate.host).insert(actual);
				commands.entity(candidate.host).insert(LodLevelSpawnRequest { level: actual });
				continue;
			}
		}

		if !admit_begin(begin_clock, class) {
			break;
		}

		let chunk = {
			let scenes = host_sets.p1();
			let Ok(scene) = scenes.get(candidate.host) else {
				continue;
			};
			scene.scene_chunks_with_level(lod_ref, candidate.level)
		};
		let expected = chunk.total_primitives();
		let mut queue = chunk.into_fulfill_queue();
		let prefill = begin_clock.weight_remaining.min(budget.begin_prefill_weights_per_job);
		let begin_weight = materialize_front(&mut queue, prefill);
		charge_begin_weight(begin_clock, begin_weight.max(1));

		let cold = candidate.cold;
		let initial_vis = if cold { Visibility::Inherited } else { Visibility::Hidden };
		let level_root = bsn! {
			template_value(LodLevelRoot(candidate.level))
			LodLevelRootPending
			Transform::default()
			template_value(initial_vis)
		};
		let root_entity = commands.spawn_scene(level_root).id();
		let fulfillment = LodChunkFulfillment {
			queue,
			expected,
			spawned: 0,
			cold,
			host: candidate.host,
			parent_desired: candidate.parent_desired,
			nested_streamed: 0,
			nested_required: None,
		};
		if fulfillment.is_content_complete() {
			commands.entity(root_entity).insert(LodLevelRootStreamed);
		}
		commands.entity(root_entity).insert(fulfillment);
		commands.entity(candidate.roots_entity).add_child(root_entity);
		commands.entity(candidate.host).remove::<LodLevelSpawnRequest>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;

	#[test]
	fn identity_transform_uses_bounds_center() {
		let transform = Transform::IDENTITY;
		let bounds = LodHostBounds(Aabb3d::from_min_max(
			Vec3::new(100.0, -1.0, -1.0),
			Vec3::new(120.0, 1.0, 1.0),
		));
		let dist = host_xz_distance2(Vec3::ZERO, &transform, Some(&bounds));
		assert!((dist - 110.0 * 110.0).abs() < 0.01);
	}

	#[test]
	fn missing_bounds_uses_translation() {
		let transform = Transform::from_xyz(30.0, 8.0, 40.0);
		let dist = host_xz_distance2(Vec3::ZERO, &transform, None);
		assert!((dist - (30.0 * 30.0 + 40.0 * 40.0)).abs() < 0.01);
	}

	/// `cargo test -p lod erased_begin_host_collect_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore = "microbench"]
	fn erased_begin_host_collect_microbench() {
		const HOST_COUNT: usize = 4096;
		const SCAN_LIMIT: usize = 192;
		const SCANS: usize = 2_000;

		let hosts: Vec<(u64, u64)> = (0..HOST_COUNT).map(|i| (i as u64, i as u64)).collect();
		let start = 0usize;

		let full_start = std::time::Instant::now();
		for _ in 0..SCANS {
			let collected: Vec<_> = hosts.iter().copied().collect();
			std::hint::black_box(collected);
		}
		let full_elapsed = full_start.elapsed();

		let capped_start = std::time::Instant::now();
		for _ in 0..SCANS {
			let mut batch = Vec::with_capacity(SCAN_LIMIT);
			for offset in 0..SCAN_LIMIT.min(hosts.len()) {
				batch.push(hosts[(start + offset) % hosts.len()]);
			}
			std::hint::black_box(batch);
		}
		let capped_elapsed = capped_start.elapsed();

		eprintln!(
			"erased_begin_host_collect_microbench: {HOST_COUNT} hosts × {SCANS} scans (scan_limit={SCAN_LIMIT}) — full collect {:?}, capped batch {:?} ({:.2}x)",
			full_elapsed,
			capped_elapsed,
			full_elapsed.as_secs_f64() / capped_elapsed.as_secs_f64().max(1e-9),
		);
	}
}
