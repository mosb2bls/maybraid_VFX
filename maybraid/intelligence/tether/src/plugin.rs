use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement, ReplanThreshold,
};
use routing_intelligence::{RoutingIntelligenceUser, RoutingSystems};

use crate::memory::TetherMemory;
use crate::user::{Tether, TetherAction, TetherIntelligenceUser};

const REPLAN_THRESHOLD: ReplanThreshold =
	ReplanThreshold { refresh_distance: 1.2, check_radius: true, check_vantage_weights: false };

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TetherSystems {
	Write,
}

pub struct TetherPlugin;

impl Plugin for TetherPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			TetherSystems::Write
				.before(RoutingSystems::Plan)
				.before(MovementIntelligenceSystems::Replan),
		)
		.add_systems(Update, write_tether_objectives.in_set(TetherSystems::Write));
	}
}

pub fn write_tether_objectives(
	time: Res<Time>,
	mut users: Query<(
		Entity,
		&Transform,
		&mut TetherIntelligenceUser,
		&mut TetherMemory,
		&mut MovementIntelligence,
		Option<&mut RoutingIntelligenceUser>,
	)>,
	anchors: Query<&Transform, With<Tether>>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (entity, transform, mut tether, mut memory, mut movement, routing) in &mut users {
		let dt = if memory.last_checked_at > 0.0 {
			(now - memory.last_checked_at).max(0.0)
		} else {
			time.delta_secs()
		};
		let subject_id = tether.objective.subject();
		let Ok(anchor) = anchors.get(subject_id) else {
			continue;
		};
		let action =
			tether.evaluate(&mut memory, transform.translation, anchor.translation, dt, now);
		apply_action(entity, transform.translation, action, &mut movement, routing, &mut commands);
	}
}

fn apply_action(
	entity: Entity,
	at: Vec3,
	action: TetherAction,
	movement: &mut MovementIntelligence,
	mut routing: Option<Mut<'_, RoutingIntelligenceUser>>,
	commands: &mut Commands,
) {
	match action {
		TetherAction::None => {}
		TetherAction::Hold => {
			if let Some(routing) = routing.as_deref_mut() {
				routing.clear_destination();
			}
			movement.hold_at(at);
			commands.entity(entity).remove::<ReplanMovement>();
		}
		TetherAction::Local(next) => {
			if let Some(routing) = routing.as_deref_mut() {
				routing.clear_destination();
			}
			if movement.objective.needs_replan(next, REPLAN_THRESHOLD) {
				movement.objective = next;
				commands.entity(entity).insert(ReplanMovement);
			}
		}
		TetherAction::Route(point) => {
			if let Some(routing) = routing.as_deref_mut() {
				routing.set_destination(point);
			} else {
				let next = MovementObjective::Reach(MovementLocation::new(
					point,
					movement.ability.agent_radius,
				));
				if movement.objective.needs_replan(next, REPLAN_THRESHOLD) {
					movement.objective = next;
					commands.entity(entity).insert(ReplanMovement);
				}
			}
		}
	}
}
