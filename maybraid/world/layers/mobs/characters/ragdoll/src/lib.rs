//! Persistent character corpses with a lightweight, world-space procedural ragdoll.

use std::collections::HashMap;

use avian3d::prelude::{
	Collider, ColliderDisabled, LinearVelocity, PhysicsPlugins, PhysicsSchedulePlugin,
	RigidBodyDisabled, ShapeCastConfig, SpatialQuery, SpatialQueryFilter,
};
use bevy::math::Affine3A;
use bevy::prelude::*;
use character_motion::{
	AnimBone, ApplyTerrainPitch, CharacterMotionSystems, CharacterRig, CharacterRigRole,
	RigSkeletonKind, SuspendAnimation,
};
use characters::{CharacterRoot, HeadCapsule, HitCapsule};
use damage::{DamageSystems, DespawnAfter, Downed};
use lod_avian::PhysicsInteractionLayer;
use player::{
	CameraFollow, CharacterController, HitVolume, MoveWish, Npc, Player, PlayerCameraAim,
	PlayerVisual, PlayerYawOwner,
};

const PARTICLE_SKIN: f32 = 0.01;
const MIN_COLLISION_MOTION_SQUARED: f32 = 1e-6;

const HUMANOID_BONES: &[&str] = &[
	"root",
	"lumbar",
	"midback",
	"upper_back",
	"lower_neck",
	"upper_neck",
	"shoulder.L",
	"humerus.L",
	"forearm.L",
	"shoulder.R",
	"humerus.R",
	"forearm.R",
	"pelvis.L",
	"femur.L",
	"shin.L",
	"pelvis.R",
	"femur.R",
	"shin.R",
];

const QUADRUPED_BONES: &[&str] = &[
	"back_ridge",
	"upper_back",
	"lumbar",
	"neck",
	"head_socket",
	"shoulder.L",
	"anterior_thigh.L",
	"anterior_shin.L",
	"shoulder.R",
	"anterior_thigh.R",
	"anterior_shin.R",
	"hip.L",
	"posterior_thigh.L",
	"posterior_shin.L",
	"hip.R",
	"posterior_thigh.R",
	"posterior_shin.R",
	"tailbone",
];

const FORELIMBED_BONES: &[&str] = &[
	"upper_mid_spine",
	"upper_spine",
	"head_socket",
	"lower_mid_spine",
	"lower_spine",
	"tailbone",
	"tail_socket",
	"back_ridge",
	"shoulder.L",
	"upper_arm.L",
	"lower_arm.L",
	"shoulder.R",
	"upper_arm.R",
	"lower_arm.R",
];

/// Detached character visual retained after its gameplay body is retired.
#[derive(Component, Clone, Copy, Debug)]
pub struct Corpse {
	pub source: Option<Entity>,
	pub point: Vec3,
	pub inherited_velocity: Vec3,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct CharacterRagdollSettings {
	pub corpse_lifetime_secs: f32,
	pub handoff_wait_secs: f32,
	pub solver_iterations: usize,
	pub simulation_hz: f32,
	pub max_simulation_secs: f32,
}

/// Which downed character roles this plugin should hand off to ragdolls.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterRagdollTargets {
	pub players: bool,
	pub npcs: bool,
	pub unmarked: bool,
}

impl Default for CharacterRagdollTargets {
	fn default() -> Self {
		Self { players: true, npcs: true, unmarked: true }
	}
}

impl Default for CharacterRagdollSettings {
	fn default() -> Self {
		Self {
			corpse_lifetime_secs: 15.0,
			handoff_wait_secs: 0.5,
			solver_iterations: 2,
			simulation_hz: 30.0,
			max_simulation_secs: 1.5,
		}
	}
}

/// Add velocity to an active corpse without exposing its solver representation.
#[derive(Message, Clone, Copy, Debug)]
pub struct RagdollImpulse {
	pub corpse: Entity,
	pub point: Vec3,
	pub impulse: Vec3,
}

#[derive(Clone, Debug)]
struct RagdollParticle {
	name: String,
	position: Vec3,
	previous_position: Vec3,
	initial_position: Vec3,
	rest_local: Transform,
	rest_world_rotation: Quat,
	parent_world_rotation: Quat,
	parent_world_inverse: Affine3A,
	parent: Option<usize>,
	primary_child: Option<usize>,
	inverse_mass: f32,
	collider: Option<RagdollColliderKind>,
}

#[derive(Clone, Copy, Debug)]
enum RagdollColliderKind {
	Core,
	Head,
	Limb,
}

#[derive(Resource)]
struct RagdollColliders {
	core: Collider,
	head: Collider,
	limb: Collider,
}

impl Default for RagdollColliders {
	fn default() -> Self {
		Self {
			core: Collider::sphere(0.13),
			head: Collider::sphere(0.11),
			limb: Collider::sphere(0.07),
		}
	}
}

impl RagdollColliders {
	fn get(&self, kind: RagdollColliderKind) -> &Collider {
		match kind {
			RagdollColliderKind::Core => &self.core,
			RagdollColliderKind::Head => &self.head,
			RagdollColliderKind::Limb => &self.limb,
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct DistanceConstraint {
	a: usize,
	b: usize,
	length: f32,
}

/// World-space articulated state; bone entities are resolved by name when rendered.
#[derive(Component, Clone, Debug)]
pub struct RagdollState {
	corpse: Entity,
	skeleton: RigSkeletonKind,
	particles: Vec<RagdollParticle>,
	constraints: Vec<DistanceConstraint>,
	initial_center: Vec3,
	corpse_initial_translation: Vec3,
	age: f32,
	still_for: f32,
	step_accumulator: f32,
	sleeping: bool,
	pose_dirty: bool,
}

/// Body-side acknowledgement that its retained visual entered corpse ownership.
#[derive(Component, Clone, Copy, Debug)]
struct CorpseHandoff {
	visual: Entity,
	started_at: f32,
}

/// Gameplay ownership already left this body. Distinct from [`DespawnAfter`],
/// which mobs stamp as a 4 s corpse clock before ragdoll handoff finishes.
#[derive(Component, Clone, Copy, Debug)]
struct CorpseRetired;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterRagdollSystems {
	Handoff,
	Initialize,
	Simulate,
	Marshal,
}

pub struct CharacterRagdollPlugin;

type PendingRigHosts<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static CharacterRig, &'static character_motion::BoneMap),
	(Without<RagdollState>, Without<AnimBone>),
>;

type DownedBodies<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static Downed,
		Option<&'static mut LinearVelocity>,
		Option<&'static mut MoveWish>,
		Has<Player>,
		Has<Npc>,
	),
	(Without<CorpseHandoff>, Without<CorpseRetired>),
>;

impl Plugin for CharacterRagdollPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		if !app.is_plugin_added::<damage::DamagePlugin>() {
			app.add_plugins(damage::DamagePlugin);
		}
		app.init_resource::<CharacterRagdollSettings>()
			.init_resource::<CharacterRagdollTargets>()
			.init_resource::<RagdollColliders>()
			.add_message::<RagdollImpulse>()
			.configure_sets(
				PostUpdate,
				(
					CharacterRagdollSystems::Marshal
						.after(rigs::RigSystems::Pose)
						.before(TransformSystems::Propagate),
					CharacterRagdollSystems::Handoff.after(DamageSystems::Down),
				),
			)
			.configure_sets(
				Update,
				(
					CharacterRagdollSystems::Initialize.before(CharacterRagdollSystems::Simulate),
					CharacterRagdollSystems::Simulate.before(CharacterMotionSystems::Anim),
				),
			)
			.add_systems(
				PostUpdate,
				(begin_corpse_handoffs, finish_corpse_handoffs)
					.chain()
					.in_set(CharacterRagdollSystems::Handoff),
			)
			.add_systems(Update, initialize_ragdolls.in_set(CharacterRagdollSystems::Initialize))
			.add_systems(Update, simulate_ragdolls.in_set(CharacterRagdollSystems::Simulate))
			.add_systems(PostUpdate, marshal_ragdolls.in_set(CharacterRagdollSystems::Marshal));
	}
}

fn begin_corpse_handoffs(
	time: Res<Time>,
	targets: Res<CharacterRagdollTargets>,
	mut commands: Commands,
	mut downed: DownedBodies,
	visuals: Query<(Entity, &ChildOf), With<CharacterRoot>>,
	hit_volumes: Query<(Entity, &ChildOf), With<HitVolume>>,
) {
	let now = time.elapsed_secs();
	for (body, downed, velocity, wish, player, npc) in &mut downed {
		let enabled = (player && targets.players)
			|| (npc && targets.npcs)
			|| (!player && !npc && targets.unmarked);
		if !enabled {
			continue;
		}
		let inherited = velocity.as_ref().map_or(Vec3::ZERO, |velocity| velocity.0);
		if let Some(mut velocity) = velocity {
			velocity.0 = Vec3::ZERO;
		}
		if let Some(mut wish) = wish {
			wish.0 = Vec3::ZERO;
		}
		if let Ok(mut entity) = commands.get_entity(body) {
			entity
				.remove::<(CharacterController, HitCapsule, HeadCapsule)>()
				.try_insert((RigidBodyDisabled, ColliderDisabled));
		}
		for (volume, child_of) in &hit_volumes {
			if child_of.parent() == body {
				if let Ok(mut entity) = commands.get_entity(volume) {
					entity.try_despawn();
				}
			}
		}
		for (visual, child_of) in &visuals {
			if child_of.parent() != body {
				continue;
			}
			if let Ok(mut entity) = commands.get_entity(visual) {
				entity.try_insert((
					Corpse {
						source: downed.source,
						point: downed.point,
						inherited_velocity: inherited,
					},
					SuspendAnimation,
				));
			}
			if let Ok(mut entity) = commands.get_entity(body) {
				entity.try_insert(CorpseHandoff { visual, started_at: now });
			}
			break;
		}
	}
}

fn finish_corpse_handoffs(
	time: Res<Time>,
	settings: Res<CharacterRagdollSettings>,
	mut commands: Commands,
	handoffs: Query<(Entity, &CorpseHandoff), Without<CorpseRetired>>,
	visuals: Query<(Option<&ChildOf>, Option<&Corpse>)>,
	ragdolls: Query<&RagdollState>,
	queued: Query<(), With<DespawnAfter>>,
) {
	let now = time.elapsed_secs();
	for (body, handoff) in &handoffs {
		let Ok((child_of, corpse)) = visuals.get(handoff.visual) else {
			retire_gameplay_body(
				&mut commands,
				body,
				queued.get(body).is_err(),
				settings.corpse_lifetime_secs,
			);
			continue;
		};
		let retained = child_of.is_some_and(|child_of| child_of.parent() == body);
		if !retained || corpse.is_none() {
			retire_gameplay_body(
				&mut commands,
				body,
				queued.get(body).is_err(),
				settings.corpse_lifetime_secs,
			);
			continue;
		}
		let ragdoll_ready = ragdolls.iter().any(|ragdoll| ragdoll.corpse == handoff.visual);
		let timed_out = now - handoff.started_at >= settings.handoff_wait_secs.max(0.0);
		if ragdoll_ready || timed_out {
			if let Ok(mut entity) = commands.get_entity(handoff.visual) {
				entity.try_remove::<(PlayerVisual, PlayerYawOwner, ApplyTerrainPitch)>();
			}
			retire_gameplay_body(
				&mut commands,
				body,
				queued.get(body).is_err(),
				settings.corpse_lifetime_secs,
			);
		}
	}
}

fn retire_gameplay_body(
	commands: &mut Commands,
	body: Entity,
	missing_ttl: bool,
	corpse_secs: f32,
) {
	let Ok(mut entity) = commands.get_entity(body) else {
		return;
	};
	entity
		.remove::<(Player, Npc, CameraFollow, PlayerCameraAim, CharacterController)>()
		.try_insert((RigidBodyDisabled, ColliderDisabled, CorpseRetired));
	if missing_ttl {
		entity.try_insert(DespawnAfter::seconds(corpse_secs));
	}
}

fn initialize_ragdolls(
	fixed_time: Res<Time<Fixed>>,
	mut commands: Commands,
	hosts: PendingRigHosts,
	corpses: Query<&Corpse>,
	parents: Query<&ChildOf>,
	transforms: Query<(&Transform, &GlobalTransform)>,
) {
	for (host, rig, bone_map) in &hosts {
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		let Some((corpse_entity, corpse)) = ancestor_corpse(host, &parents, &corpses) else {
			continue;
		};
		let Some(state) = build_state(
			corpse_entity,
			corpse,
			rig.skeleton,
			bone_map,
			&parents,
			&transforms,
			fixed_time.delta_secs().max(1e-4),
		) else {
			continue;
		};
		if let Ok(mut entity) = commands.get_entity(host) {
			entity.try_insert((state, SuspendAnimation));
		}
	}
}

fn ancestor_corpse(
	mut entity: Entity,
	parents: &Query<&ChildOf>,
	corpses: &Query<&Corpse>,
) -> Option<(Entity, Corpse)> {
	loop {
		if let Ok(corpse) = corpses.get(entity) {
			return Some((entity, *corpse));
		}
		entity = parents.get(entity).ok()?.parent();
	}
}

fn profile_bones(skeleton: RigSkeletonKind) -> &'static [&'static str] {
	match skeleton {
		RigSkeletonKind::Humanoid => HUMANOID_BONES,
		RigSkeletonKind::Quadruped => QUADRUPED_BONES,
		RigSkeletonKind::Forelimbed => FORELIMBED_BONES,
		RigSkeletonKind::Neck => &[],
	}
}

fn build_state(
	corpse_entity: Entity,
	corpse: Corpse,
	skeleton: RigSkeletonKind,
	bone_map: &character_motion::BoneMap,
	parents: &Query<&ChildOf>,
	transforms: &Query<(&Transform, &GlobalTransform)>,
	fixed_dt: f32,
) -> Option<RagdollState> {
	let mut resolved = Vec::new();
	for &name in profile_bones(skeleton) {
		let Some(&entity) = bone_map.by_name.get(name) else {
			continue;
		};
		let Ok((local, global)) = transforms.get(entity) else {
			continue;
		};
		let (_, world_rotation, position) = global.to_scale_rotation_translation();
		resolved.push((name.to_owned(), entity, *local, position, world_rotation, global.affine()));
	}
	if resolved.len() < 2 {
		return None;
	}

	let by_entity: HashMap<Entity, usize> = resolved
		.iter()
		.enumerate()
		.map(|(index, (_, entity, ..))| (*entity, index))
		.collect();
	let mut particles = Vec::with_capacity(resolved.len());
	for (name, entity, local, position, world_rotation, world_affine) in &resolved {
		let parent = simulated_parent(*entity, &by_entity, parents);
		let parent_world_rotation = *world_rotation * local.rotation.inverse();
		let parent_world_inverse = (*world_affine * local.compute_affine().inverse()).inverse();
		let away_from_hit = (*position - corpse.point).normalize_or_zero();
		let impulse = away_from_hit * 1.6 + Vec3::Y * 0.45;
		let velocity = corpse.inherited_velocity + impulse;
		particles.push(RagdollParticle {
			name: name.clone(),
			position: *position,
			previous_position: *position - velocity * fixed_dt,
			initial_position: *position,
			rest_local: *local,
			rest_world_rotation: *world_rotation,
			parent_world_rotation,
			parent_world_inverse,
			parent,
			primary_child: None,
			inverse_mass: inverse_mass(name),
			collider: particle_collider(name),
		});
	}

	let mut constraints = Vec::new();
	for child in 0..particles.len() {
		let Some(parent) = particles[child].parent else {
			continue;
		};
		let length = particles[parent].position.distance(particles[child].position);
		if length > 1e-4 {
			constraints.push(DistanceConstraint { a: parent, b: child, length });
		}
	}
	connect_roots(&particles, &mut constraints);
	assign_primary_children(&mut particles, &constraints);
	let initial_center = particle_center(&particles);
	let corpse_initial_translation = transforms.get(corpse_entity).ok()?.0.translation;

	Some(RagdollState {
		corpse: corpse_entity,
		skeleton,
		particles,
		constraints,
		initial_center,
		corpse_initial_translation,
		age: 0.0,
		still_for: 0.0,
		step_accumulator: 0.0,
		sleeping: false,
		pose_dirty: true,
	})
}

fn simulated_parent(
	entity: Entity,
	by_entity: &HashMap<Entity, usize>,
	parents: &Query<&ChildOf>,
) -> Option<usize> {
	let mut parent = parents.get(entity).ok().map(ChildOf::parent);
	while let Some(entity) = parent {
		if let Some(&index) = by_entity.get(&entity) {
			return Some(index);
		}
		parent = parents.get(entity).ok().map(ChildOf::parent);
	}
	None
}

fn connect_roots(particles: &[RagdollParticle], constraints: &mut Vec<DistanceConstraint>) {
	let roots: Vec<usize> = particles
		.iter()
		.enumerate()
		.filter_map(|(index, particle)| particle.parent.is_none().then_some(index))
		.collect();
	let Some(&anchor) = roots.first() else {
		return;
	};
	for &root in roots.iter().skip(1) {
		let length = particles[anchor].position.distance(particles[root].position);
		if length > 1e-4 {
			constraints.push(DistanceConstraint { a: anchor, b: root, length });
		}
	}
}

fn assign_primary_children(particles: &mut [RagdollParticle], constraints: &[DistanceConstraint]) {
	for constraint in constraints {
		if particles[constraint.b].parent != Some(constraint.a) {
			continue;
		}
		let replace = particles[constraint.a]
			.primary_child
			.map(|child| {
				particles[constraint.a]
					.initial_position
					.distance(particles[child].initial_position)
					< constraint.length
			})
			.unwrap_or(true);
		if replace {
			particles[constraint.a].primary_child = Some(constraint.b);
		}
	}
}

fn inverse_mass(name: &str) -> f32 {
	if name.contains("root")
		|| name.contains("back")
		|| name.contains("spine")
		|| name.contains("lumbar")
	{
		0.45
	} else {
		1.0
	}
}

fn particle_collider(name: &str) -> Option<RagdollColliderKind> {
	match name {
		"root" | "back_ridge" | "upper_mid_spine" | "upper_spine" | "upper_back" => {
			Some(RagdollColliderKind::Core)
		}
		"upper_neck" | "head_socket" => Some(RagdollColliderKind::Head),
		"forearm.L" | "forearm.R" | "lower_arm.L" | "lower_arm.R" | "shin.L" | "shin.R"
		| "anterior_shin.L" | "anterior_shin.R" | "posterior_shin.L" | "posterior_shin.R"
		| "tail_socket" => Some(RagdollColliderKind::Limb),
		_ => None,
	}
}

fn particle_center(particles: &[RagdollParticle]) -> Vec3 {
	if particles.is_empty() {
		return Vec3::ZERO;
	}
	particles.iter().map(|particle| particle.position).sum::<Vec3>() / particles.len() as f32
}

fn simulate_ragdolls(
	spatial: SpatialQuery,
	time: Res<Time>,
	settings: Res<CharacterRagdollSettings>,
	colliders: Res<RagdollColliders>,
	mut impulses: MessageReader<RagdollImpulse>,
	mut ragdolls: Query<&mut RagdollState>,
) {
	let pending: Vec<RagdollImpulse> = impulses.read().copied().collect();
	let step = settings.simulation_hz.max(1.0).recip();
	let frame_dt = time.delta_secs().min(step);
	if frame_dt <= 0.0 {
		return;
	}
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	for mut ragdoll in &mut ragdolls {
		let corpse = ragdoll.corpse;
		for impulse in pending.iter().filter(|impulse| impulse.corpse == corpse) {
			apply_impulse(&mut ragdoll, *impulse, step);
		}
		if ragdoll.sleeping {
			continue;
		}
		ragdoll.step_accumulator += frame_dt;
		if ragdoll.step_accumulator + f32::EPSILON < step {
			continue;
		}
		ragdoll.step_accumulator = 0.0;
		let gravity_scale = match ragdoll.skeleton {
			RigSkeletonKind::Forelimbed => 0.35,
			_ => 1.0,
		};
		let damping = match ragdoll.skeleton {
			RigSkeletonKind::Forelimbed => 0.92,
			_ => 0.985,
		};
		for particle in &mut ragdoll.particles {
			let velocity = (particle.position - particle.previous_position) * damping;
			particle.previous_position = particle.position;
			particle.position += velocity + Vec3::NEG_Y * 9.81 * gravity_scale * step * step;
		}
		for _ in 0..settings.solver_iterations {
			for index in 0..ragdoll.constraints.len() {
				let constraint = ragdoll.constraints[index];
				solve_distance(&mut ragdoll.particles, constraint);
			}
		}
		for particle in &mut ragdoll.particles {
			let Some(collider) = particle.collider.map(|kind| colliders.get(kind)) else {
				continue;
			};
			collide_particle(&spatial, &filter, collider, particle);
		}
		ragdoll.age += step;
		ragdoll.pose_dirty = true;
		let max_speed = ragdoll
			.particles
			.iter()
			.map(|particle| particle.position.distance(particle.previous_position) / step)
			.fold(0.0, f32::max);
		if max_speed < 0.04 {
			ragdoll.still_for += step;
		} else {
			ragdoll.still_for = 0.0;
		}
		ragdoll.sleeping = ragdoll.age >= settings.max_simulation_secs.max(step)
			|| (ragdoll.age > 0.35 && ragdoll.still_for > 0.35);
	}
}

fn apply_impulse(state: &mut RagdollState, impulse: RagdollImpulse, dt: f32) {
	for particle in &mut state.particles {
		let weight = 1.0 / (1.0 + particle.position.distance(impulse.point) * 4.0);
		particle.previous_position -= impulse.impulse * weight * dt;
	}
	state.sleeping = false;
	state.still_for = 0.0;
	state.pose_dirty = true;
}

fn solve_distance(particles: &mut [RagdollParticle], constraint: DistanceConstraint) {
	let Some((a, b)) = two_mut(particles, constraint.a, constraint.b) else {
		return;
	};
	let delta = b.position - a.position;
	let distance = delta.length();
	if distance <= 1e-5 {
		return;
	}
	let total_mass = a.inverse_mass + b.inverse_mass;
	if total_mass <= 1e-5 {
		return;
	}
	let correction = delta * ((distance - constraint.length) / distance);
	a.position += correction * (a.inverse_mass / total_mass);
	b.position -= correction * (b.inverse_mass / total_mass);
}

fn two_mut<T>(values: &mut [T], a: usize, b: usize) -> Option<(&mut T, &mut T)> {
	if a == b || a >= values.len() || b >= values.len() {
		return None;
	}
	if a < b {
		let (left, right) = values.split_at_mut(b);
		Some((&mut left[a], &mut right[0]))
	} else {
		let (left, right) = values.split_at_mut(a);
		Some((&mut right[0], &mut left[b]))
	}
}

fn collide_particle(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	shape: &Collider,
	particle: &mut RagdollParticle,
) {
	let delta = particle.position - particle.previous_position;
	if delta.length_squared() < MIN_COLLISION_MOTION_SQUARED {
		return;
	}
	let Ok(direction) = Dir3::new(delta) else {
		return;
	};
	let config = ShapeCastConfig::from_max_distance(delta.length());
	let Some(hit) = spatial.cast_shape(
		shape,
		particle.previous_position,
		Quat::IDENTITY,
		direction,
		&config,
		filter,
	) else {
		return;
	};
	let travel = (hit.distance - PARTICLE_SKIN).max(0.0);
	particle.position = particle.previous_position + *direction * travel;
	particle.previous_position = particle.position;
}

fn marshal_ragdolls(
	mut hosts: Query<(&character_motion::BoneMap, &mut RagdollState)>,
	mut transforms: Query<&mut Transform>,
) {
	for (bone_map, mut state) in &mut hosts {
		if !state.pose_dirty {
			continue;
		}
		let rotations = desired_world_rotations(&state);
		let corpse_displacement = particle_center(&state.particles) - state.initial_center;
		if let Ok(mut corpse) = transforms.get_mut(state.corpse) {
			corpse.translation = state.corpse_initial_translation + corpse_displacement;
		}
		for (index, particle) in state.particles.iter().enumerate() {
			let Some(&entity) = bone_map.by_name.get(&particle.name) else {
				continue;
			};
			let Ok(mut transform) = transforms.get_mut(entity) else {
				continue;
			};
			let (parent_rotation, translation) = if let Some(parent) = particle.parent {
				let parent_rotation = rotations[parent];
				(parent_rotation, particle.rest_local.translation)
			} else {
				let displacement =
					particle.position - particle.initial_position - corpse_displacement;
				let translation = particle.rest_local.translation
					+ particle.parent_world_inverse.transform_vector3(displacement);
				(particle.parent_world_rotation, translation)
			};
			transform.translation = translation;
			transform.rotation = parent_rotation.inverse() * rotations[index];
			transform.scale = particle.rest_local.scale;
		}
		state.pose_dirty = false;
	}
}

fn desired_world_rotations(state: &RagdollState) -> Vec<Quat> {
	let mut rotations = vec![Quat::IDENTITY; state.particles.len()];
	let mut resolved = vec![false; state.particles.len()];
	for index in 0..state.particles.len() {
		resolve_rotation(index, state, &mut rotations, &mut resolved);
	}
	rotations
}

fn resolve_rotation(
	index: usize,
	state: &RagdollState,
	rotations: &mut [Quat],
	resolved: &mut [bool],
) -> Quat {
	if resolved[index] {
		return rotations[index];
	}
	let particle = &state.particles[index];
	let rotation = if let Some(child) = particle.primary_child {
		let rest = state.particles[child].initial_position - particle.initial_position;
		let current = state.particles[child].position - particle.position;
		if rest.length_squared() > 1e-8 && current.length_squared() > 1e-8 {
			Quat::from_rotation_arc(rest.normalize(), current.normalize())
				* particle.rest_world_rotation
		} else {
			particle.rest_world_rotation
		}
	} else if let Some(parent) = particle.parent {
		let parent_rotation = resolve_rotation(parent, state, rotations, resolved);
		let rest_relative =
			state.particles[parent].rest_world_rotation.inverse() * particle.rest_world_rotation;
		parent_rotation * rest_relative
	} else {
		particle.rest_world_rotation
	};
	rotations[index] = rotation.normalize();
	resolved[index] = true;
	rotations[index]
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;

	fn particle(position: Vec3) -> RagdollParticle {
		RagdollParticle {
			name: String::new(),
			position,
			previous_position: position,
			initial_position: position,
			rest_local: Transform::IDENTITY,
			rest_world_rotation: Quat::IDENTITY,
			parent_world_rotation: Quat::IDENTITY,
			parent_world_inverse: Affine3A::IDENTITY,
			parent: None,
			primary_child: None,
			inverse_mass: 1.0,
			collider: None,
		}
	}

	#[test]
	fn distance_solver_restores_segment_length() {
		let mut particles = vec![particle(Vec3::ZERO), particle(Vec3::X * 2.0)];
		solve_distance(&mut particles, DistanceConstraint { a: 0, b: 1, length: 1.0 });
		assert!((particles[0].position.distance(particles[1].position) - 1.0).abs() < 1e-5);
	}

	#[test]
	fn distance_solver_ignores_a_self_constraint() {
		let mut particles = vec![particle(Vec3::ZERO), particle(Vec3::X)];
		solve_distance(&mut particles, DistanceConstraint { a: 1, b: 1, length: 1.0 });
		assert_eq!(particles[1].position, Vec3::X);
	}

	#[test]
	fn every_body_skeleton_has_a_ragdoll_profile() {
		assert!(!profile_bones(RigSkeletonKind::Humanoid).is_empty());
		assert!(!profile_bones(RigSkeletonKind::Quadruped).is_empty());
		assert!(!profile_bones(RigSkeletonKind::Forelimbed).is_empty());
	}

	#[test]
	fn profiles_use_only_a_small_collision_proxy_set() {
		for skeleton in
			[RigSkeletonKind::Humanoid, RigSkeletonKind::Quadruped, RigSkeletonKind::Forelimbed]
		{
			let colliders = profile_bones(skeleton)
				.iter()
				.filter(|name| particle_collider(name).is_some())
				.count();
			assert!((5..=8).contains(&colliders), "{skeleton:?} has {colliders} colliders");
		}
	}

	#[test]
	fn profile_names_exist_on_each_imported_rig() {
		use character_rigs::rigs::{
			forelimbed_v0::forelimbed_v0_bone_names, humanoid_v0::humanoid_v0_bone_names,
			quadruped_v0::quadruped_v0_bone_names,
		};

		assert_profile_names(HUMANOID_BONES, humanoid_v0_bone_names());
		assert_profile_names(QUADRUPED_BONES, quadruped_v0_bone_names());
		assert_profile_names(FORELIMBED_BONES, forelimbed_v0_bone_names());
	}

	#[test]
	fn particle_center_tracks_world_motion() {
		let particles = vec![particle(Vec3::ZERO), particle(Vec3::X * 2.0)];
		assert_eq!(particle_center(&particles), Vec3::X);
	}

	#[test]
	fn handoff_keeps_visual_attached_until_corpse_expiry() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(CharacterRagdollSettings { handoff_wait_secs: 0.0, ..default() })
			.init_resource::<CharacterRagdollTargets>()
			.add_systems(
				Update,
				(begin_corpse_handoffs, finish_corpse_handoffs, damage::tick_queued_despawns)
					.chain(),
			);
		let body = app
			.world_mut()
			.spawn((
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
				Npc,
				avian3d::prelude::RigidBody::Dynamic,
				Collider::sphere(0.4),
			))
			.id();
		let visual = app
			.world_mut()
			.spawn((
				CharacterRoot,
				PlayerVisual,
				PlayerYawOwner::Wish,
				ApplyTerrainPitch,
				Transform::IDENTITY,
				GlobalTransform::IDENTITY,
				ChildOf(body),
			))
			.id();

		for _ in 0..2 {
			app.update();
		}

		let world = app.world();
		assert!(world.entities().contains(body));
		assert!(world.entities().contains(visual));
		assert!(world.get::<DespawnAfter>(body).is_some());
		assert!(world.get::<Npc>(body).is_none());
		assert!(world.get::<RigidBodyDisabled>(body).is_some());
		assert!(world.get::<ColliderDisabled>(body).is_some());
		assert!(world.get::<avian3d::prelude::RigidBody>(body).is_some());
		assert!(world.get::<Collider>(body).is_some());
		assert!(world.get::<DespawnAfter>(visual).is_none());
		assert!(world.get::<Corpse>(visual).is_some());
		assert!(world.get::<ChildOf>(visual).is_some_and(|child_of| child_of.parent() == body));
		assert!(world.get::<PlayerVisual>(visual).is_none());
		assert!(world.get::<ApplyTerrainPitch>(visual).is_none());
		Ok(())
	}

	#[test]
	fn handoff_starts_even_when_a_corpse_timer_is_already_set() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<CharacterRagdollTargets>();
		let body = world
			.spawn((
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
				Npc,
				CharacterController,
				MoveWish(Vec3::X),
				LinearVelocity(Vec3::Z),
				DespawnAfter::seconds(4.0),
			))
			.id();
		let visual = world
			.spawn((CharacterRoot, Transform::IDENTITY, GlobalTransform::IDENTITY, ChildOf(body)))
			.id();

		world
			.run_system_once(begin_corpse_handoffs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<Corpse>(visual).is_some());
		assert!(world.get::<CorpseHandoff>(body).is_some());
		assert!(world.get::<CharacterController>(body).is_none());
		assert!(world.get::<MoveWish>(body).is_some_and(|wish| wish.0 == Vec3::ZERO));
		assert!(world
			.get::<LinearVelocity>(body)
			.is_some_and(|velocity| velocity.0 == Vec3::ZERO));
		assert!(world.get::<DespawnAfter>(body).is_some());
		Ok(())
	}

	#[test]
	fn multi_hit_kill_then_handoff_does_not_command_despawned_volumes() -> anyhow::Result<()> {
		use damage::{apply_hits, mark_downed, DamageApplied, Health, Hit};

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<CharacterRagdollTargets>()
			.add_message::<Hit>()
			.add_message::<DamageApplied>()
			.add_message::<damage::Died>()
			.add_systems(Update, (apply_hits, mark_downed, begin_corpse_handoffs).chain());
		let body = app
			.world_mut()
			.spawn((
				Health::from_max(10.0),
				Npc,
				HitCapsule { radius: 0.4, length: 0.8, along: 0.0 },
			))
			.id();
		let volume = app.world_mut().spawn((HitVolume, ChildOf(body))).id();
		for _ in 0..6 {
			app.world_mut().write_message(Hit {
				target: volume,
				source: None,
				amount: 4.0,
				point: Vec3::new(0.0, 1.2, 0.0),
				allow_self: false,
			});
		}
		app.update();
		app.update();
		assert_eq!(app.world().get::<Health>(body).map(|health| health.current), Some(0.0));
		assert!(app.world().get::<Downed>(body).is_some());
		assert!(app.world().get_entity(volume).is_err());
		Ok(())
	}

	#[test]
	fn handoff_strips_hit_volumes_and_disables_the_body() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<CharacterRagdollTargets>();
		let body = world
			.spawn((
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
				Npc,
				HitCapsule { radius: 0.4, length: 0.8, along: 0.0 },
			))
			.id();
		let volume = world.spawn((HitVolume, ChildOf(body))).id();

		world
			.run_system_once(begin_corpse_handoffs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();

		assert!(world.get_entity(volume).is_err());
		assert!(world.get::<HitCapsule>(body).is_none());
		assert!(world.get::<ColliderDisabled>(body).is_some());
		Ok(())
	}

	#[test]
	fn missing_visual_still_disables_combat_and_a_fallback_clock_despawns() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<CharacterRagdollTargets>()
			.init_resource::<CharacterRagdollSettings>()
			.add_systems(
				Update,
				(begin_corpse_handoffs, finish_corpse_handoffs, damage::ensure_downed_have_despawn)
					.chain(),
			);
		let body = app
			.world_mut()
			.spawn((Downed { source: None, point: Vec3::ZERO, at: 0.0 }, Npc))
			.id();

		app.update();

		assert!(app.world().get::<ColliderDisabled>(body).is_some());
		assert!(app.world().get::<DespawnAfter>(body).is_some());
		assert!(app.world().get::<CorpseHandoff>(body).is_none());
		Ok(())
	}

	#[test]
	fn lost_handoff_visual_still_retires_the_body() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<CharacterRagdollSettings>();
		let visual = world.spawn_empty().id();
		let body = world
			.spawn((
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
				Npc,
				CorpseHandoff { visual, started_at: 0.0 },
			))
			.id();
		world.entity_mut(visual).despawn();

		world
			.run_system_once(finish_corpse_handoffs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();

		assert!(world.get::<Npc>(body).is_none());
		assert!(world.get::<DespawnAfter>(body).is_some());
		Ok(())
	}

	fn assert_profile_names<'a>(profile: &[&str], defined: impl IntoIterator<Item = &'a str>) {
		let defined: std::collections::HashSet<&str> = defined.into_iter().collect();
		for name in profile {
			assert!(defined.contains(name), "missing ragdoll bone {name}");
		}
	}
}
