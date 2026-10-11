use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use damage::{Health, Hit};

use crate::{
	aim::{owner_forward, owner_point, resolve_aim_contact, targeting_allies},
	cleanup::end_instance,
	overlap::gather_overlaps,
	AttackCatalog, AttackEndReason, AttackEnded, AttackInstance, AttackInstanceId,
	AttackPresentationBinding, AttackPresentationEvent, AttackUser, BeginAttack, HazardShape,
	HazardSpec, WavefrontSweep,
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
		let targeting = targeting.get(begin.owner).ok();
		let contact = resolve_aim_contact(begin.target, targeting);
		let live_target = contact.map(|contact| contact.subject);
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
		let snapshot = match contact {
			Some(contact) => crate::AimSnapshot::toward(origin, contact.aim_point(false), live_target),
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn tick_attacks(
	time: Res<Time>,
	catalog: Res<AttackCatalog>,
	spatial: Option<SpatialQuery>,
	transforms: Query<(&Transform, Option<&GlobalTransform>)>,
	health: Query<(), With<Health>>,
	parents: Query<&ChildOf>,
	targeting: Query<&CombatTargeting>,
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
	let mut completed = Vec::new();
	for (entity, mut instance) in &mut instances {
		let Some(definition) = catalog.get(instance.definition).cloned() else {
			completed.push((entity, AttackEndReason::Cancelled));
			continue;
		};
		if let Some(target) = instance.live_target {
			let still_known = targeting
				.get(instance.owner)
				.ok()
				.and_then(|targeting| targeting.contact(target))
				.is_some();
			if !still_known {
				instance.live_target = None;
			}
		}
		if let Ok((owner_transform, owner_global)) = transforms.get(instance.owner) {
			let origin = owner_point(owner_transform, owner_global);
			let target_point = targeting.get(instance.owner).ok().and_then(|targeting| {
				instance
					.live_target
					.and_then(|target| targeting.contact(target))
					.or_else(|| targeting.best_contact())
					.map(|contact| contact.aim_point(false))
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
				for hit in pulse_hits(
					&mut instance,
					now,
					index,
					*spec,
					spatial.as_ref(),
					&health,
					&parents,
					&transforms,
				) {
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

#[allow(clippy::too_many_arguments)]
fn pulse_hits(
	instance: &mut AttackInstance,
	now: f32,
	index: usize,
	spec: HazardSpec,
	spatial: Option<&SpatialQuery>,
	health: &Query<(), With<Health>>,
	parents: &Query<&ChildOf>,
	transforms: &Query<(&Transform, Option<&GlobalTransform>)>,
) -> Vec<Hit> {
	let Some(spatial) = spatial else {
		return Vec::new();
	};
	match spec.shape {
		HazardShape::Wavefront(wave) => {
			let motions = instance.live_wave_motions(index, spec, wave);
			let mut hits = Vec::new();
			for (wave_index, motion) in motions {
				let sweep = WavefrontSweep {
					emission: motion.emission,
					heading: motion.heading,
					from: motion.from,
					to: motion.to,
				};
				let pose = wave.pose(motion.emission, motion.heading, motion.to);
				let candidates = gather_overlaps(
					spatial,
					spec,
					pose,
					Some(sweep),
					instance.owner,
					health,
					parents,
					transforms,
				);
				hits.extend(instance.collect_one_wave(
					now, index, spec, wave, wave_index, motion, &candidates,
				));
			}
			hits
		}
		_ => {
			let pose = instance.hazard_pose(spec);
			let candidates =
				gather_overlaps(spatial, spec, pose, None, instance.owner, health, parents, transforms);
			instance.collect_hits(now, index, spec, &candidates)
		}
	}
}
