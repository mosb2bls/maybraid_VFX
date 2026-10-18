//! Flee actuator: get away from the ranked assailant as quickly as possible.

use bevy::prelude::*;
use evasion_intelligence::{EvasionIntelligenceUser, EvasionSystems};
use intelligence_lod::{IntelligenceBand, IntelligenceLod};
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement, ReplanThreshold,
};

const REPLAN_THRESHOLD: ReplanThreshold =
	ReplanThreshold { refresh_distance: 0.8, check_radius: true, check_vantage_weights: false };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FleeingSettings {
	pub radius: f32,
}

impl Default for FleeingSettings {
	fn default() -> Self {
		Self { radius: 16.0 }
	}
}

/// Per-user flee policy. Writes [`MovementObjective::FleeFrom`] only while the
/// evasion signal is flee.
#[derive(Component, Clone, Debug)]
pub struct FleeingUser {
	pub settings: FleeingSettings,
	driving: bool,
}

impl FleeingUser {
	pub fn new(settings: FleeingSettings) -> Self {
		Self { settings, driving: false }
	}

	pub fn objective(&self, threat: Vec3) -> MovementObjective {
		MovementObjective::FleeFrom(MovementLocation::new(threat, self.settings.radius.max(0.5)))
	}

	/// True while this user is writing a flee objective.
	pub fn is_driving(&self) -> bool {
		self.driving
	}
}

impl Default for FleeingUser {
	fn default() -> Self {
		Self::new(FleeingSettings::default())
	}
}

pub struct FleeingPlugin;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FleeingSystems {
	Write,
}

impl Plugin for FleeingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			FleeingSystems::Write
				.after(EvasionSystems::Rank)
				.before(MovementIntelligenceSystems::Replan),
		)
		.add_systems(Update, write_flee_objectives.in_set(FleeingSystems::Write));
	}
}

pub fn write_flee_objectives(
	time: Res<Time>,
	mut users: Query<(
		Entity,
		&Transform,
		&EvasionIntelligenceUser,
		&mut FleeingUser,
		&mut MovementIntelligence,
		Option<&IntelligenceLod>,
	)>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (entity, transform, evasion, mut fleeing, mut movement, lod) in &mut users {
		if !evasion.signal.is_flee() {
			let was_driving = fleeing.driving;
			fleeing.driving = false;
			if was_driving && evasion.signal.is_idle() {
				movement.hold_at(transform.translation);
				commands.entity(entity).remove::<ReplanMovement>();
			}
			continue;
		}
		let Some(contact) = evasion.best_contact() else {
			if fleeing.driving {
				fleeing.driving = false;
				movement.hold_at(transform.translation);
				commands.entity(entity).remove::<ReplanMovement>();
			}
			continue;
		};
		if IntelligenceLod::band_or_near(lod) == IntelligenceBand::Far
			&& !contact.is_fresh(now, evasion.settings.memory_secs)
		{
			continue;
		}
		let next = fleeing.objective(contact.position);
		fleeing.driving = true;
		if !movement.objective.needs_replan(next, REPLAN_THRESHOLD) {
			continue;
		}
		movement.objective = next;
		commands.entity(entity).insert(ReplanMovement);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use evasion_intelligence::{
		AssailantContact, AssailantSource, EvasionActuator, EvasionSignal, RankedAssailant,
	};
	use intelligence_lod::{IntelligenceBand, IntelligenceLod};

	fn flee_user(threat: Entity, last_known_at: f32) -> EvasionIntelligenceUser {
		let mut evasion = EvasionIntelligenceUser::default();
		evasion.memory.insert(
			threat,
			AssailantContact {
				subject: threat,
				position: Vec3::X * 3.0,
				movement_vector: Vec3::ZERO,
				last_known_at,
			},
		);
		evasion.include(threat, AssailantSource::SPOTTING);
		evasion.ranked.push(RankedAssailant { entity: threat, weight: 1.0 });
		evasion.signal = EvasionSignal { actuator: EvasionActuator::Flee, threat: Some(threat) };
		evasion
	}

	#[test]
	fn flee_writes_a_disk_around_the_threat_snapshot() -> anyhow::Result<()> {
		let user = FleeingUser::new(FleeingSettings { radius: 12.0 });
		let objective = user.objective(Vec3::X * 4.0);
		assert!(matches!(objective, MovementObjective::FleeFrom(_)));
		assert!((objective.location().radius - 12.0).abs() < 1e-4);
		assert_eq!(objective.location().point, Vec3::X * 4.0);
		Ok(())
	}

	#[test]
	fn idle_signal_does_not_drive() -> anyhow::Result<()> {
		let threat = Entity::from_bits(1);
		let mut evasion = EvasionIntelligenceUser::default();
		evasion.include(threat, AssailantSource::ENEMYSHIP);
		assert!(!evasion.signal.is_flee());
		let _ = AssailantContact {
			subject: threat,
			position: Vec3::ZERO,
			movement_vector: Vec3::ZERO,
			last_known_at: 0.0,
		};
		Ok(())
	}

	#[test]
	fn far_stale_flee_does_not_enqueue_covering() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, write_flee_objectives);
		let threat = Entity::from_bits(3);
		let entity = app
			.world_mut()
			.spawn((
				Transform::default(),
				flee_user(threat, -10.0),
				FleeingUser::default(),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					Vec3::ZERO,
					0.4,
				))),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();

		app.update();
		assert!(app.world().get::<ReplanMovement>(entity).is_none());
		assert!(!app
			.world()
			.get::<MovementIntelligence>(entity)
			.is_some_and(|movement| matches!(movement.objective, MovementObjective::FleeFrom(_))));
	}

	#[test]
	fn near_flee_still_enqueues_covering() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, write_flee_objectives);
		let threat = Entity::from_bits(3);
		let entity = app
			.world_mut()
			.spawn((
				Transform::default(),
				flee_user(threat, 0.0),
				FleeingUser::default(),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					Vec3::ZERO,
					0.4,
				))),
				IntelligenceLod::missing(),
			))
			.id();

		app.update();
		assert!(app.world().get::<ReplanMovement>(entity).is_some());
		assert!(app
			.world()
			.get::<MovementIntelligence>(entity)
			.is_some_and(|movement| matches!(movement.objective, MovementObjective::FleeFrom(_))));
	}
}
