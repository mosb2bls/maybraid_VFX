//! Playable stick / trunk capsules for character physics.
//!
//! Forest plants are [`FlattenedComponentsOnly`] hosts — kit GLBs spawn as posed
//! content with no nested [`StickNode`] LOD hosts. A type-erased producer is
//! stamped when each source component is added, then one shared drain creates a
//! compound collider per host (every gated stick, no shape cap).
//!
//! Compounds live on the stick source entity (isolated `/show` host, or a posed
//! plant under a grove). Band flicker does not rebuild them. The first **High
//! or Medium** realization — on this entity or an ancestor [`LodSceneHost`] —
//! stamps High-IR capsules. Grove High/Medium pose plants without nested hosts;
//! the grove band is what makes the compound playable. Sources with no
//! collideable sticks still take [`StickPhysicsAttached`] so empty tuft / frond
//! plants do not starve the drain. Later Low / UltraLow leaves the compound in
//! place until the source is culled.

use std::collections::{HashSet, VecDeque};

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use lod::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;
use vegetation_components::{
	Placement, StickMember, StickNode, VegetationComponents, STICK_KIT_HALF,
};

/// Four inches. Gate and floor use world-space girth after plant [`Placement`] scale.
pub const MIN_STICK_COLLIDER_RADIUS_M: f32 = 4.0 * 0.0254;
/// 10cm. Gates the min length of the stick capsule.
pub const MIN_STICK_COLLIDER_LENGTH_M: f32 = 0.1;

/// How many pending High/Medium hosts may build compounds in one frame.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickPhysicsBudget {
	pub hosts_per_frame: u32,
	/// Unattached producers probed for a playable band when the queue needs work.
	///
	/// The drain only builds [`Self::hosts_per_frame`] compounds; a full producer
	/// walk every Update is wasted while thousands of Low / UltraLow plants wait.
	/// `u32::MAX` restores the old walk.
	pub inspect_per_frame: u32,
}

impl Default for StickPhysicsBudget {
	fn default() -> Self {
		Self { hosts_per_frame: 8, inspect_per_frame: 256 }
	}
}

type ProduceColliderPoses = fn(&World, Entity, LodSceneLevel) -> Vec<(Transform, f32, f32)>;

/// Type-erased source callback; all vegetation types share one runtime drain.
#[derive(Component, Clone, Copy)]
struct StickPhysicsProducer(ProduceColliderPoses);

#[derive(Component)]
pub(crate) struct StickPhysicsCompound;

/// This host was considered for a stick compound (empty or stamped).
///
/// Persists across LOD bands so we do not rebuild or re-queue.
#[derive(Component, Clone, Copy)]
pub(crate) struct StickPhysicsAttached;

#[derive(Resource, Default)]
struct StickPhysicsQueue {
	pending: VecDeque<Entity>,
	pending_entities: HashSet<Entity>,
	inspect_cursor: u32,
}

impl StickPhysicsQueue {
	fn enqueue(&mut self, entity: Entity) {
		if self.pending_entities.insert(entity) {
			self.pending.push_back(entity);
		}
	}

	fn pop_front(&mut self) -> Option<Entity> {
		let entity = self.pending.pop_front()?;
		self.pending_entities.remove(&entity);
		Some(entity)
	}
}

pub struct StickPhysicsPlugin;

impl Plugin for StickPhysicsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<StickPhysicsBudget>()
			.init_resource::<StickPhysicsQueue>()
			.add_observer(attach_stick_node_producer)
			.add_systems(Update, sync_stick_colliders);
	}
}

/// Register a flattened vegetation source without adding another update system.
pub(crate) fn register_vegetation_stick_colliders<T>(app: &mut App)
where
	T: VegetationComponents + Component,
{
	app.add_observer(attach_vegetation_producer::<T>);
}

fn attach_vegetation_producer<T>(insert: On<Insert, T>, mut commands: Commands)
where
	T: VegetationComponents + Component,
{
	if let Ok(mut entity) = commands.get_entity(insert.entity) {
		entity.insert(StickPhysicsProducer(|world, entity, level| {
			world
				.get::<T>(entity)
				.into_iter()
				.flat_map(|vegetation| vegetation.stick_nodes_for_level(level).flatten())
				.flat_map(|node| collider_poses(&node, level))
				.collect()
		}));
	}
}

fn attach_stick_node_producer(insert: On<Insert, StickNode>, mut commands: Commands) {
	if let Ok(mut entity) = commands.get_entity(insert.entity) {
		entity.insert(StickPhysicsProducer(|world, entity, level| {
			world
				.get::<StickNode>(entity)
				.map(|node| collider_poses(node, level))
				.unwrap_or_default()
		}));
	}
}

fn wants_playable_colliders(level: LodSceneLevel) -> bool {
	matches!(level, LodSceneLevel::High | LodSceneLevel::Medium)
}

/// Playable band on this entity, or an ancestor [`LodSceneHost`] (posed grove plants).
fn playable_level_for(world: &World, entity: Entity) -> Option<LodSceneLevel> {
	let mut current = Some(entity);
	while let Some(entity) = current {
		if let Some(level) = world.get::<LodSceneLevel>(entity).copied() {
			return wants_playable_colliders(level).then_some(level);
		}
		current = world.get::<ChildOf>(entity).map(|child| child.parent());
	}
	None
}

fn refill_pending_producers(world: &mut World, inspect_limit: u32) {
	if inspect_limit == 0 {
		return;
	}
	let start = world.resource::<StickPhysicsQueue>().inspect_cursor;
	let entities: Vec<Entity> = {
		let mut sources = world
			.query_filtered::<Entity, (With<StickPhysicsProducer>, Without<StickPhysicsAttached>)>(
			);
		sources.iter(world).collect()
	};
	let len = entities.len() as u32;
	if len == 0 {
		world.resource_mut::<StickPhysicsQueue>().inspect_cursor = 0;
		return;
	}
	let start = start % len;
	let take = inspect_limit.min(len);
	let mut found = Vec::new();
	for offset in 0..take {
		let entity = entities[((start + offset) % len) as usize];
		if playable_level_for(world, entity).is_some() {
			found.push(entity);
		}
	}
	let mut queue = world.resource_mut::<StickPhysicsQueue>();
	queue.inspect_cursor = if take < len { (start + take) % len } else { 0 };
	for entity in found {
		queue.enqueue(entity);
	}
}

fn sync_stick_colliders(world: &mut World) {
	let budget = *world.resource::<StickPhysicsBudget>();
	// Skip the producer walk while the drain already has a full frame of work.
	// Discovery resumes as soon as the queue drops below the budget; compounds
	// and attach stamps stay the same, only later hosts wait a few extra frames.
	if (world.resource::<StickPhysicsQueue>().pending.len() as u32) < budget.hosts_per_frame {
		refill_pending_producers(world, budget.inspect_per_frame);
	}
	for _ in 0..budget.hosts_per_frame {
		let Some(entity) = world.resource_mut::<StickPhysicsQueue>().pop_front() else {
			break;
		};
		if world.get_entity(entity).is_err() {
			continue;
		}
		if playable_level_for(world, entity).is_none() {
			continue;
		}
		if world.get::<StickPhysicsAttached>(entity).is_some() && has_compound(world, entity) {
			continue;
		}
		let Some(producer) = world.get::<StickPhysicsProducer>(entity).copied() else {
			continue;
		};
		// High stick IR even when the host is still Medium — same capsules as High visuals.
		let poses = (producer.0)(world, entity, LodSceneLevel::High);
		despawn_compound(world, entity);
		if world.get_entity(entity).is_err() {
			continue;
		}
		if poses.is_empty() {
			world.entity_mut(entity).insert(StickPhysicsAttached);
			continue;
		}
		let shapes = poses
			.into_iter()
			.map(|(transform, radius, cylinder)| {
				(transform.translation, transform.rotation, Collider::capsule(radius, cylinder))
			})
			.collect();
		world.spawn((
			StickPhysicsCompound,
			ChildOf(entity),
			Transform::IDENTITY,
			Visibility::Hidden,
			RigidBody::Static,
			Collider::compound(shapes),
			PhysicsInteractionLayer::fixed_layers(),
		));
		world.entity_mut(entity).insert(StickPhysicsAttached);
	}
}

fn has_compound(world: &World, entity: Entity) -> bool {
	world.get::<Children>(entity).is_some_and(|children| {
		children.iter().any(|child| world.get::<StickPhysicsCompound>(child).is_some())
	})
}

fn despawn_compound(world: &mut World, entity: Entity) {
	let Some(children) = world.get::<Children>(entity) else {
		return;
	};
	let compounds: Vec<_> = children
		.iter()
		.filter(|child| world.get::<StickPhysicsCompound>(*child).is_some())
		.collect();
	for child in compounds {
		world.despawn(child);
	}
}

fn authored_radius(placement: Placement) -> f32 {
	(placement.scale.x.abs() * STICK_KIT_HALF).max(placement.scale.z.abs() * STICK_KIT_HALF)
}

/// Trunks always; branches at least four inches in world space. Thinner High twigs stay visual-only.
fn should_collide_member(is_trunk: bool, placement: Placement) -> bool {
	is_trunk || authored_radius(placement) + 1e-5 >= MIN_STICK_COLLIDER_RADIUS_M
}

fn collider_poses(node: &StickNode, level: LodSceneLevel) -> Vec<(Transform, f32, f32)> {
	if let Some(collection) = &node.collection {
		let members = collection.members_for_level(level);
		let mut poses: Vec<_> = members
			.iter()
			.filter_map(|member| gated_member_pose(node.placement, member))
			.collect();
		if poses.is_empty() {
			poses = members
				.iter()
				.max_by(|a, b| {
					world_radius(node.placement, a)
						.partial_cmp(&world_radius(node.placement, b))
						.unwrap_or(std::cmp::Ordering::Equal)
				})
				.and_then(|member| {
					capsule_from_placement(world_member_placement(node.placement, member))
				})
				.into_iter()
				.collect();
		}
		return poses;
	}
	capsule_from_placement(node.placement).into_iter().collect()
}

fn world_member_placement(parent: Placement, member: &StickMember) -> Placement {
	parent.compose_child(member.placement)
}

fn world_radius(parent: Placement, member: &StickMember) -> f32 {
	authored_radius(world_member_placement(parent, member))
}

fn gated_member_pose(parent: Placement, member: &StickMember) -> Option<(Transform, f32, f32)> {
	let placed = world_member_placement(parent, member);
	if !should_collide_member(member.is_trunk(), placed) {
		return None;
	}
	capsule_from_placement(placed)
}

fn capsule_from_placement(placement: Placement) -> Option<(Transform, f32, f32)> {
	let length = placement.scale.y;
	if length < MIN_STICK_COLLIDER_LENGTH_M {
		return None;
	}
	let radius = authored_radius(placement).max(MIN_STICK_COLLIDER_RADIUS_M);
	let cylinder = (length - 2.0 * radius).max(0.02);
	let center = placement.translation + placement.rotation() * Vec3::new(0.0, length * 0.5, 0.0);
	Some((
		Transform::from_translation(center).with_rotation(placement.rotation()),
		radius,
		cylinder,
	))
}

#[cfg(test)]
mod tests {
	use super::*;
	use lod::LodSceneHost;
	use vegetation_components::{StickCollection, StickGeometry, StickMember};

	#[test]
	fn collection_keeps_trunks_and_drops_thin_twigs() {
		let trunk = StickMember {
			geometry: StickGeometry::Trunk,
			placement: Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)),
		};
		let twig = StickMember {
			geometry: StickGeometry::Segment,
			placement: Placement::new(Vec3::new(1.0, 2.0, 0.0), 0.0)
				.with_scale(Vec3::new(0.04, 1.0, 0.04)),
		};
		let node = StickNode::collection(
			StickCollection::new([trunk, twig]).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		let poses = collider_poses(&node, LodSceneLevel::High);
		assert_eq!(poses.len(), 1);
	}

	#[test]
	fn collection_keeps_scaled_limbs_that_are_thin_in_unit_space() {
		let trunk = StickMember {
			geometry: StickGeometry::Trunk,
			placement: Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)),
		};
		let limb = StickMember {
			geometry: StickGeometry::Segment,
			placement: Placement::new(Vec3::new(1.0, 2.0, 0.0), 0.0)
				.with_scale(Vec3::new(0.04, 1.0, 0.04)),
		};
		let node = StickNode::collection(
			StickCollection::new([trunk, limb]).bake_bounds_from_members(),
			Placement::IDENTITY.with_scale(Vec3::splat(30.0)),
		);
		let poses = collider_poses(&node, LodSceneLevel::High);
		assert_eq!(poses.len(), 2);
		let expected = 0.04 * STICK_KIT_HALF * 30.0;
		let limb_radius = poses.iter().map(|(_, radius, _)| *radius).fold(f32::INFINITY, f32::min);
		assert!((limb_radius - expected).abs() < 1e-4);
	}

	#[test]
	fn high_and_medium_want_playable_colliders() {
		assert!(wants_playable_colliders(LodSceneLevel::High));
		assert!(wants_playable_colliders(LodSceneLevel::Medium));
		assert!(!wants_playable_colliders(LodSceneLevel::Low));
		assert!(!wants_playable_colliders(LodSceneLevel::UltraLow));
	}

	#[test]
	fn collection_keeps_every_gated_member() {
		let count = 84;
		// World girth `scale.x * STICK_KIT_HALF` must meet the four-inch branch gate.
		let girth = MIN_STICK_COLLIDER_RADIUS_M / STICK_KIT_HALF + 0.01;
		let members = (0..count)
			.map(|index| StickMember {
				geometry: StickGeometry::Segment,
				placement: Placement::new(Vec3::new(index as f32, 0.0, 0.0), 0.0)
					.with_scale(Vec3::new(girth, 2.0, girth)),
			})
			.collect::<Vec<_>>();
		let node = StickNode::collection(
			StickCollection::new(members).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		assert_eq!(collider_poses(&node, LodSceneLevel::High).len(), count);
	}

	#[test]
	fn ungated_segments_keep_the_thickest_member() {
		let members = (0..8)
			.map(|index| StickMember {
				geometry: StickGeometry::Segment,
				placement: Placement::new(Vec3::new(index as f32, 0.0, 0.0), 0.0)
					.with_scale(Vec3::new(0.3, 2.0, 0.3)),
			})
			.collect::<Vec<_>>();
		let node = StickNode::collection(
			StickCollection::new(members).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		assert_eq!(collider_poses(&node, LodSceneLevel::High).len(), 1);
	}

	fn playable_trunk() -> StickNode {
		StickNode::collection(
			StickCollection::new([StickMember {
				geometry: StickGeometry::Trunk,
				placement: Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)),
			}])
			.bake_bounds_from_members(),
			Placement::IDENTITY,
		)
	}

	fn empty_sticks() -> StickNode {
		StickNode::collection(StickCollection::new([]), Placement::IDENTITY)
	}

	#[test]
	fn high_compound_attaches_after_playable_band_arrives() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let host = app.world_mut().spawn(playable_trunk()).id();
		app.update();
		assert!(!has_compound(app.world(), host));

		app.world_mut().entity_mut(host).insert(LodSceneLevel::High);
		app.update();
		assert!(has_compound(app.world(), host));
	}

	#[test]
	fn posed_plant_uses_ancestor_grove_band() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let grove = app.world_mut().spawn((LodSceneHost, LodSceneLevel::High)).id();
		let plant = app.world_mut().spawn((playable_trunk(), ChildOf(grove))).id();
		app.update();
		assert!(has_compound(app.world(), plant));
		assert!(!has_compound(app.world(), grove));
	}

	#[test]
	fn medium_host_stamps_high_ir_compound() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let host = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::Medium))
			.id();
		app.update();
		assert!(has_compound(app.world(), host));
	}

	#[test]
	fn compound_stays_on_the_host_after_leaving_playable_bands() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let host = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
			.id();
		app.update();
		assert!(has_compound(app.world(), host));

		*app.world_mut().get_mut::<LodSceneLevel>(host).expect("level") = LodSceneLevel::Low;
		app.update();
		assert!(has_compound(app.world(), host));
		assert!(app.world().get::<StickPhysicsAttached>(host).is_some());
	}

	#[test]
	fn empty_hosts_do_not_starve_later_playable_hosts() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		app.insert_resource(StickPhysicsBudget { hosts_per_frame: 1, ..Default::default() });
		for _ in 0..3 {
			app.world_mut().spawn((empty_sticks(), LodSceneHost, LodSceneLevel::High));
		}
		app.update();
		app.update();
		app.update();

		let tree = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
			.id();
		app.update();
		assert!(has_compound(app.world(), tree));
		assert!(app.world().get::<StickPhysicsAttached>(tree).is_some());
	}

	#[test]
	fn backlog_skips_rescan_until_queue_drops_below_budget() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		app.insert_resource(StickPhysicsBudget { hosts_per_frame: 1, ..Default::default() });
		let first: Vec<_> = (0..3)
			.map(|_| {
				app.world_mut()
					.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
					.id()
			})
			.collect();
		app.update();
		let attached = first
			.iter()
			.filter(|entity| app.world().get::<StickPhysicsAttached>(**entity).is_some())
			.count();
		assert_eq!(attached, 1);

		let late = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
			.id();
		// Two first-batch hosts remain queued (>= budget), so the late host waits.
		app.update();
		assert!(app.world().get::<StickPhysicsAttached>(late).is_none());
		app.update();
		app.update();
		// Queue is empty; the next scan discovers the late host.
		app.update();
		assert!(has_compound(app.world(), late));
	}

	#[test]
	fn inspect_budget_still_attaches_every_playable_host() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		app.insert_resource(StickPhysicsBudget { hosts_per_frame: 8, inspect_per_frame: 2 });
		let hosts: Vec<_> = (0..5)
			.map(|_| {
				app.world_mut()
					.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
					.id()
			})
			.collect();
		app.update();
		let attached = hosts
			.iter()
			.filter(|entity| app.world().get::<StickPhysicsAttached>(**entity).is_some())
			.count();
		assert_eq!(attached, 2);

		app.update();
		app.update();
		for host in hosts {
			assert!(has_compound(app.world(), host));
		}
	}
}
