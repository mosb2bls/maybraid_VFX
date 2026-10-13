use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement, ReplanThreshold,
};

use crate::avian::AvianRouteProbe;
use crate::user::RoutingIntelligenceUser;

const REPLAN_THRESHOLD: ReplanThreshold =
	ReplanThreshold { refresh_distance: 1.2, check_radius: true, check_vantage_weights: false };

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoutingSystems {
	Plan,
	Write,
}

pub struct RoutingPlugin;

impl Plugin for RoutingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(RoutingSystems::Plan, RoutingSystems::Write)
				.chain()
				.before(MovementIntelligenceSystems::Replan),
		)
		.add_systems(Update, plan_routes.in_set(RoutingSystems::Plan))
		.add_systems(Update, write_route_objectives.in_set(RoutingSystems::Write));
	}
}

pub fn plan_routes(
	spatial: SpatialQuery,
	mut users: Query<(Entity, &Transform, &mut RoutingIntelligenceUser)>,
) {
	for (entity, transform, mut routing) in &mut users {
		if !routing.needs_plan(transform.translation) {
			continue;
		}
		let probe = AvianRouteProbe::new(&spatial, &[entity]);
		routing.replan(transform.translation, &probe);
	}
}

pub fn write_route_objectives(
	mut users: Query<(Entity, &Transform, &mut RoutingIntelligenceUser, &mut MovementIntelligence)>,
	mut commands: Commands,
) {
	for (entity, transform, mut routing, mut movement) in &mut users {
		routing.advance(transform.translation);
		let Some(hop) = routing.current_hop(transform.translation) else {
			continue;
		};
		let next = MovementObjective::Reach(MovementLocation::new(
			hop,
			routing.settings.arrival_radius.max(movement.ability.agent_radius),
		));
		if !movement.objective.needs_replan(next, REPLAN_THRESHOLD) {
			continue;
		}
		movement.objective = next;
		commands.entity(entity).insert(ReplanMovement);
	}
}
