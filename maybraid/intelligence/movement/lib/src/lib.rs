//! Movement location, objective, and per-entity movement intelligence.
//!
//! Install [`MovementIntelligence`] on a capsule and register
//! [`MovementIntelligencePlugin`] with a [`MovementIntelligenceSurface`]
//! [`bevy::ecs::system::SystemParam`]. The brain fields [`MovementObjective`]
//! and writes [`player::MotorIntent`]; it does not own physics, combat policy, or
//! entity lock-on.

mod ability;
mod candidate;
mod location;
mod objective;
mod plugin;
mod step;
mod surface;
pub mod testing;
mod user;

use bevy::prelude::*;
use maybraid_controllers_core::ControllerPipeline;
use player::{MotorRequestStage, PlayerSystems};

pub use ability::{
	Covering, MovementAbility, MovementBody, MovementSheet, VantageStandoffs,
	WALK_DETOUR_WORK_FRACTION,
};
pub use candidate::{MovementCandidate, MovementCandidateHints};
pub use intelligence_lod::{
	due_by_rank, reserve_fairness, IntelligenceBand, IntelligenceLod, IntelligencePriority,
};
pub use location::MovementLocation;
pub use objective::{MovementObjective, ReplanThreshold};
pub use plugin::MovementIntelligencePlugin;
pub use step::{MovementDrive, MovementStep};
pub use surface::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligenceSurface, WalkProbeBudget,
};
pub use user::{
	MovementDriveResult, MovementIntelligence, MovementIntelligenceSettings, ReplanMovement,
};

/// Plan, then declare a [`player::MotorIntent`] NPC slot, before recovery and apply.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementIntelligenceSystems {
	Replan,
	Drive,
}

pub(crate) fn configure_movement_intelligence_sets(app: &mut App) {
	ControllerPipeline::configure(app);
	MotorRequestStage::configure(app);
	app.configure_sets(
		Update,
		(
			MovementIntelligenceSystems::Replan,
			MovementIntelligenceSystems::Drive
				.in_set(MotorRequestStage::Standard)
				.before(PlayerSystems::Body),
		)
			.chain(),
	);
}
