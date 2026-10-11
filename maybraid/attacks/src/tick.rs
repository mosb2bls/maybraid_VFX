use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use damage::{Health, Hit};

use crate::{
	aim::{owner_forward, owner_point, resolve_target, targeting_allies},
	cleanup::end_instance,
	AttackCatalog, AttackEndReason, AttackEnded, AttackInstance, AttackInstanceId,
	AttackPresentationBinding, AttackPresentationEvent, AttackUser, BeginAttack, HitCandidate,
};

#[derive(Resource, Default)]
pub struct AttackInstanceIds(u64);

impl AttackInstanceIds {
	pub fn next(&mut self) -> AttackInstanceId {
		self.0 += 1;
		AttackInstanceId(self.0)
	}
}

pub fn tick_user_cooldowns(time: Res<Time>, mut users: Query<&mut AttackUser>) {
	let dt = time.delta_secs();
	for mut user in &mut users {
		user.tick_cooldowns(dt);
	}
}

#[allow(clippy::too_many_arguments)]
pub fn begin_attacks(
	mut begins: MessageReader<BeginAttack>,
	catalog: Res<AttackCatalog>,
	mut ids: ResMut<AttackInstanceIds>,
	transforms: Query<(&Transform, Option<&GlobalTransform>)>,
	targeting: Query<&CombatTargeting>,
	existing: Query<&AttackInstance>,
	mut users: Query<&mut AttackUser>,
	mut presented: MessageWriter<AttackPresentationEvent>,
	mut commands: Commands,
) {
	for begin in begins.read() {
		let Some(definition) = catalog.get(begin.definition).cloned() else {
			continue;
		};
		let Ok(user) = users.get_mut(begin.owner) else {
			continue;
		};
		if !user.ready(definition.id) {
			continue;
		}
		let already_busy = existing.iter().any(|instance| instance.owner == begin.owner);
		if already_busy && !definition.concurrent {
			continue;
		}
		let exists = |entity| transforms.contains(entity);
		let targeting = targeting.get(begin.owner).ok();
		let live_target = resolve_target(begin.target, targeting, exists);
		let Ok((owner_transform, owner_global)) = transforms.get(begin.owner) else {
			continue;
		};
		let origin = owner_point(owner_transform, owner_global);
		let emission = begin
			.emission
			.or_else(|| {
				begin.emission_socket.and_then(|socket| {
					transforms.get(socket).ok().map(|(transform, global)| owner_point(transform, global))
				})
			})
			.unwrap_or(origin);
		let snapshot = match live_target.and_then(|target| transforms.get(target).ok()) {
			Some((target_transform, target_global)) => crate::AimSnapshot::toward(
				origin,
				owner_point(target_transform, target_global),
				live_target,
			),
			None => crate::AimSnapshot::owner_facing(
				origin,
				owner_forward(owner_transform, owner_global),
			),
		}
		.with_emission(emission);
		let mut allies = begin.allies.clone();
		for ally in targeting_allies(targeting) {
			if !allies.contains(&ally) {
				allies.push(ally);
			}
		}
		let id = ids.next();
		let instance = AttackInstance::new(id, &definition, begin.owner, snapshot, live_target, allies)
			.with_emission_socket(begin.emission_socket);
		for kind in instance.begin_events(&definition) {
			presented.write(AttackPresentationEvent::with_kind(
				id,
				begin.owner,
				definition.id,
				kind,
			));
		}
		commands.spawn((Name::new(format!("attack-{}", definition.id.as_str())), instance));
	}
}

#[allow(clippy::too_many_arguments)]
pub fn tick_attacks(
	time: Res<Time>,
	catalog: Res<AttackCatalog>,
	transforms: Query<(&Transform, Option<&GlobalTransform>)>,
	health: Query<(), With<Health>>,
	parents: Query<&ChildOf>,
	bodies: Query<(Entity, &Transform, Option<&GlobalTransform>, Option<&ChildOf>)>,
	mut users: Query<&mut AttackUser>,
	mut instances: Query<(Entity, &mut AttackInstance)>,
	bindings: Query<(Entity, &AttackPresentationBinding)>,
	mut hits: MessageWriter<Hit>,
	mut ended: MessageWriter<AttackEnded>,
	mut presented: MessageWriter<AttackPresentationEvent>,
	mut commands: Commands,
) {
	let dt = time.delta_secs();
	let now = time.elapsed_secs();
	let candidates = collect_candidates(&bodies, &health, &parents);
	let mut completed = Vec::new();
	for (entity, mut instance) in &mut instances {
		let Some(definition) = catalog.get(instance.definition).cloned() else {
			completed.push((entity, AttackEndReason::Cancelled));
			continue;
		};
		if let Some(target) = instance.live_target {
			if !transforms.contains(target) {
				instance.live_target = None;
			}
		}
		if let Ok((owner_transform, owner_global)) = transforms.get(instance.owner) {
			let origin = owner_point(owner_transform, owner_global);
			let target_point = instance.live_target.and_then(|target| {
				transforms
					.get(target)
					.ok()
					.map(|(transform, global)| owner_point(transform, global))
			});
			instance.refresh_aim(target_point, origin);
			if let Some(socket) = instance.emission_socket {
				if let Ok((socket_transform, socket_global)) = transforms.get(socket) {
					instance.refresh_emission(owner_point(socket_transform, socket_global));
				}
			}
		}
		let outcome = instance.tick(dt, &definition);
		if outcome.pulse {
			for (index, spec) in definition.hazards.iter().enumerate() {
				for hit in instance.collect_hits(now, index, *spec, &candidates) {
					hits.write(hit);
				}
			}
		}
		for kind in outcome.events {
			presented.write(AttackPresentationEvent::with_kind(
				instance.id,
				instance.owner,
				instance.definition,
				kind,
			));
		}
		if outcome.completed {
			if let Ok(mut user) = users.get_mut(instance.owner) {
				user.start_cooldown(definition.id, definition.timing.cooldown);
			}
			completed.push((entity, AttackEndReason::Completed));
		}
	}
	for (entity, reason) in completed {
		if let Ok((_, instance)) = instances.get(entity) {
			end_instance(
				&mut commands,
				entity,
				instance,
				catalog.get(instance.definition),
				reason,
				&bindings,
				&mut ended,
				&mut presented,
			);
		}
	}
}

fn collect_candidates(
	bodies: &Query<(Entity, &Transform, Option<&GlobalTransform>, Option<&ChildOf>)>,
	health: &Query<(), With<Health>>,
	parents: &Query<&ChildOf>,
) -> Vec<HitCandidate> {
	let mut candidates = Vec::new();
	for (entity, transform, global, child) in bodies {
		let parent = child
			.map(ChildOf::parent)
			.or_else(|| parents.get(entity).ok().map(ChildOf::parent));
		let has_health = health.contains(entity);
		let parent_has_health = parent.is_some_and(|parent| health.contains(parent));
		if !has_health && !parent_has_health {
			continue;
		}
		candidates.push(HitCandidate {
			entity,
			parent,
			has_health,
			parent_has_health,
			point: owner_point(transform, global),
		});
	}
	candidates
}
