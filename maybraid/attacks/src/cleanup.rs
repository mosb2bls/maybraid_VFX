use bevy::prelude::*;
use damage::Downed;

use crate::{
	AttackCatalog, AttackEndReason, AttackEnded, AttackInstance, AttackPresentationBinding,
	AttackPresentationEvent, AttackUser, CancelAttacks,
};

/// Generic sleep / stun hold. Bikanwarda sleep inserts this; the runtime stays content-free.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AttackSuspended;

/// Generic actor retirement. HCSG / summon teardown inserts this.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AttackRetired;

pub fn apply_cancel_requests(
	mut cancels: MessageReader<CancelAttacks>,
	catalog: Res<AttackCatalog>,
	mut instances: Query<(Entity, &AttackInstance)>,
	bindings: Query<(Entity, &AttackPresentationBinding)>,
	mut ended: MessageWriter<AttackEnded>,
	mut presented: MessageWriter<AttackPresentationEvent>,
	mut commands: Commands,
) {
	for cancel in cancels.read() {
		for (entity, instance) in &mut instances {
			if instance.owner != cancel.owner {
				continue;
			}
			if cancel.instance.is_some_and(|id| id != instance.id) {
				continue;
			}
			let definition = catalog.get(instance.definition);
			if !definition.is_none_or(|definition| definition.interruption.allows(cancel.reason)) {
				continue;
			}
			end_instance(
				&mut commands,
				entity,
				instance,
				definition,
				cancel.reason,
				&bindings,
				&mut ended,
				&mut presented,
			);
		}
	}
}

#[allow(clippy::type_complexity)]
pub fn interrupt_unavailable_owners(
	catalog: Res<AttackCatalog>,
	instances: Query<(Entity, &AttackInstance)>,
	owners: Query<
		(Option<&Downed>, Option<&AttackSuspended>, Option<&AttackRetired>),
		With<AttackUser>,
	>,
	bindings: Query<(Entity, &AttackPresentationBinding)>,
	mut ended: MessageWriter<AttackEnded>,
	mut presented: MessageWriter<AttackPresentationEvent>,
	mut commands: Commands,
) {
	for (entity, instance) in &instances {
		let reason = match owners.get(instance.owner) {
			Err(_) => Some(AttackEndReason::OwnerDespawned),
			Ok((downed, suspended, retired)) => {
				if downed.is_some() {
					Some(AttackEndReason::OwnerDowned)
				} else if suspended.is_some() {
					Some(AttackEndReason::OwnerSuspended)
				} else if retired.is_some() {
					Some(AttackEndReason::OwnerRetired)
				} else {
					None
				}
			}
		};
		let Some(reason) = reason else {
			continue;
		};
		let definition = catalog.get(instance.definition);
		if !definition.is_none_or(|definition| definition.interruption.allows(reason)) {
			continue;
		}
		end_instance(
			&mut commands,
			entity,
			instance,
			definition,
			reason,
			&bindings,
			&mut ended,
			&mut presented,
		);
	}
}

#[allow(clippy::too_many_arguments)]
pub fn end_instance(
	commands: &mut Commands,
	instance_entity: Entity,
	instance: &AttackInstance,
	definition: Option<&crate::AttackDefinition>,
	reason: AttackEndReason,
	bindings: &Query<(Entity, &AttackPresentationBinding)>,
	ended: &mut MessageWriter<AttackEnded>,
	presented: &mut MessageWriter<AttackPresentationEvent>,
) {
	for kind in instance.cancel_events(definition) {
		presented.write(AttackPresentationEvent::with_kind(
			instance.id,
			instance.owner,
			instance.definition,
			kind,
		));
	}
	presented.write(AttackPresentationEvent::ended(
		instance.id,
		instance.owner,
		instance.definition,
		reason,
	));
	ended.write(AttackEnded {
		instance: instance.id,
		owner: instance.owner,
		definition: instance.definition,
		reason,
	});
	despawn_bindings(commands, instance.id, bindings);
	commands.entity(instance_entity).try_despawn();
}

pub fn despawn_bindings(
	commands: &mut Commands,
	instance: crate::AttackInstanceId,
	bindings: &Query<(Entity, &AttackPresentationBinding)>,
) {
	for (entity, binding) in bindings {
		if binding.instance == instance {
			commands.entity(entity).try_despawn();
		}
	}
}
