//! Capsule motor recovery when a planner [`player::MotorIntent`] is not becoming motion.
//!
//! Not a planner: it does not change [`movement_intelligence::MovementIntelligence::plan`].
//! After strafe → hop → backup it inserts [`ReplanMovement`] so intelligence can pick a new path.

mod policy;

use avian3d::prelude::LinearVelocity;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use maybraid_controllers_core::ControllerPipeline;
use movement_intelligence::{
	MovementBody, MovementIntelligence, MovementIntelligenceSystems, ReplanMovement,
};
use player::{
	CharacterController, Grounded, Jumping, MotorIntent, MotorRecovery, Player, PlayerSystems,
};

pub use policy::{
	backup_dir, strafe_dir, MovementRealization, RealizationCommand, RealizationPhase,
	RealizationSample,
};

/// After intelligence drive, before capsule accel.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementRealizationSystems {
	Unstick,
}

pub struct MovementRealizationPlugin;

impl Plugin for MovementRealizationPlugin {
	fn build(&self, app: &mut App) {
		ControllerPipeline::configure(app);
		app.configure_sets(
			Update,
			MovementRealizationSystems::Unstick
				.after(MovementIntelligenceSystems::Drive)
				.in_set(ControllerPipeline::MotorRequest)
				.before(PlayerSystems::Body),
		)
		.add_systems(
			Update,
			(ensure_realization, realize_movement)
				.chain()
				.in_set(MovementRealizationSystems::Unstick),
		);
	}
}

fn ensure_realization(
	mut commands: Commands,
	movers: Query<
		Entity,
		(
			With<CharacterController>,
			With<MotorIntent>,
			Without<Player>,
			Without<MovementRealization>,
		),
	>,
) {
	for entity in &movers {
		commands.entity(entity).insert(MovementRealization::default());
	}
}

fn realize_movement(
	time: Res<Time>,
	mut movers: Query<
		(
			Entity,
			&Transform,
			&LinearVelocity,
			&mut MotorIntent,
			&mut MovementRealization,
			Option<&MovementIntelligence>,
			Has<Grounded>,
			Has<Jumping>,
		),
		(With<CharacterController>, Without<Player>),
	>,
	mut commands: Commands,
) {
	let dt = time.delta_secs();
	for (entity, transform, velocity, mut intent, mut motor, brain, grounded, jumping) in
		&mut movers
	{
		let max_jump = brain.map(|b| b.ability.max_jump()).unwrap_or(0.0);
		let planned = intent.planned_translation();
		let cmd = motor.tick(RealizationSample {
			dt,
			position: transform.translation,
			velocity: Vec3::new(velocity.x, velocity.y, velocity.z),
			wish: planned,
			grounded,
			jumping,
			max_jump,
		});
		intent.set_recovery(MotorRecovery { wish_override: cmd.wish_override, jump: cmd.jump });
		if cmd.replan {
			commands.entity(entity).insert(ReplanMovement);
		}
	}
}
