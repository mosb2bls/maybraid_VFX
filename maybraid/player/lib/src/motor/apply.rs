//! Realize the composed request and apply routing hold effects.

use bevy::prelude::*;
use maybraid_controllers_core::{HoldEffect, RoutingEvent};

use crate::body::{CharacterController, Jumping, MoveWish};
use crate::stance::CharacterStance;

use super::contract::{MotorIntent, MotorSample, ResolvedMotorRequest, SprintRequest};

/// Apply [`HoldEffect::ReleaseNeutral`] from [`RoutingEvent`] before compose.
///
/// Production consumers use [`MessageReader`]; `take_events` is the unpublished
/// buffer only. Each event is applied to the actor it names. Disconnect,
/// first capture, reassignment, and removal drop the Standard contribution
/// on [`RoutingEvent::lost_actor`].
pub fn apply_routing_hold_effects(
	mut events: MessageReader<RoutingEvent>,
	mut actors: Query<&mut MotorIntent, With<CharacterController>>,
) {
	for event in events.read() {
		match event.hold() {
			HoldEffect::ReleaseNeutral => {
				let Some(actor) = event.lost_actor() else {
					continue;
				};
				if let Ok(mut intent) = actors.get_mut(actor.entity()) {
					intent.clear_standard();
				}
			}
			HoldEffect::Freeze => {
				let Some(actor) = event.previous() else {
					continue;
				};
				if let Ok(mut intent) = actors.get_mut(actor.entity()) {
					if intent.selected_base().is_none() {
						intent.set_standard(MotorSample {
							sprint: SprintRequest::Neutral,
							..MotorSample::default()
						});
					}
				}
			}
			HoldEffect::None => {}
		}
	}
}

/// One apply path. Realizes the composed request; does not inspect producers.
pub fn apply_motor(
	mut commands: Commands,
	mut bodies: Query<
		(
			Entity,
			&ResolvedMotorRequest,
			&mut MoveWish,
			Option<&mut CharacterStance>,
			Option<&Jumping>,
		),
		With<CharacterController>,
	>,
) {
	for (entity, resolved, mut wish, mut stance, jumping) in &mut bodies {
		resolved.apply(entity, &mut wish, stance.as_deref_mut(), jumping, &mut commands);
	}
}
