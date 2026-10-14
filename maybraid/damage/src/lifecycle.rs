//! One-shot downing and component-backed deferred despawn.

use bevy::prelude::*;

use crate::{Died, Health};

/// Fallback corpse lifetime when no consumer queues a tighter [`DespawnAfter`].
/// Matches the default ragdoll corpse clock so a missed handoff still expires.
pub const DEFAULT_CORPSE_SECS: f32 = 15.0;

/// A damageable entity whose health crossed zero.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Downed {
	pub source: Option<Entity>,
	pub point: Vec3,
	pub at: f32,
}

/// Queue an entity hierarchy for despawn after a game-time delay.
#[derive(Component, Clone, Debug)]
pub struct DespawnAfter(Timer);

impl DespawnAfter {
	pub fn seconds(seconds: f32) -> Self {
		Self(Timer::from_seconds(seconds.max(0.0), TimerMode::Once))
	}

	pub fn remaining_secs(&self) -> f32 {
		self.0.remaining_secs()
	}
}

/// Materialize [`Died`] messages as durable, queryable state.
pub fn mark_downed(
	time: Res<Time>,
	mut died: MessageReader<Died>,
	mut commands: Commands,
	already_downed: Query<(), With<Downed>>,
	queued: Query<(), With<DespawnAfter>>,
) {
	let now = time.elapsed_secs();
	for death in died.read() {
		if already_downed.contains(death.entity) {
			continue;
		}
		let Ok(mut entity) = commands.get_entity(death.entity) else {
			continue;
		};
		entity.try_insert(Downed { source: death.source, point: death.point, at: now });
		if queued.get(death.entity).is_err() {
			entity.try_insert(DespawnAfter::seconds(DEFAULT_CORPSE_SECS));
		}
	}
}

/// Zero-HP bodies that never emitted [`Died`] still become [`Downed`].
///
/// [`crate::apply_hits`] ignores further hits once `Health` is already dead, so a
/// missed death message would otherwise leave a visible, un-injurable body.
pub fn down_zero_health(
	time: Res<Time>,
	mut commands: Commands,
	mut died: MessageWriter<Died>,
	living: Query<(Entity, &Health), Without<Downed>>,
	queued: Query<(), With<DespawnAfter>>,
) {
	let now = time.elapsed_secs();
	for (entity, health) in &living {
		if !health.is_dead() {
			continue;
		}
		died.write(Died { entity, source: None, point: Vec3::ZERO });
		let Ok(mut commands) = commands.get_entity(entity) else {
			continue;
		};
		commands.try_insert(Downed { source: None, point: Vec3::ZERO, at: now });
		if queued.get(entity).is_err() {
			commands.try_insert(DespawnAfter::seconds(DEFAULT_CORPSE_SECS));
		}
	}
}

/// Any [`Downed`] body without a corpse clock would linger after hits stop applying.
pub fn ensure_downed_have_despawn(
	mut commands: Commands,
	downed: Query<Entity, (With<Downed>, Without<DespawnAfter>)>,
) {
	for entity in &downed {
		if let Ok(mut commands) = commands.get_entity(entity) {
			commands.try_insert(DespawnAfter::seconds(DEFAULT_CORPSE_SECS));
		}
	}
}

/// Drain queued despawns in [`Last`] so the hierarchy remains readable for the frame.
pub fn tick_queued_despawns(
	time: Res<Time>,
	mut commands: Commands,
	mut queued: Query<(Entity, &mut DespawnAfter)>,
) {
	for (entity, mut despawn) in &mut queued {
		despawn.0.tick(time.delta());
		if despawn.0.is_finished() {
			if let Ok(mut entity) = commands.get_entity(entity) {
				entity.try_despawn();
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn despawn_delay_clamps_at_zero() {
		let queued = DespawnAfter::seconds(-1.0);
		assert_eq!(queued.remaining_secs(), 0.0);
	}

	#[test]
	fn death_becomes_durable_downed_state() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<Died>()
			.add_systems(Update, mark_downed);
		let entity = app.world_mut().spawn_empty().id();
		let point = Vec3::new(1.0, 2.0, 3.0);
		app.world_mut().write_message(Died { entity, source: None, point });

		app.update();

		let downed = app.world().get::<Downed>(entity);
		assert!(downed.is_some_and(|downed| downed.point == point));
		assert!(app.world().get::<DespawnAfter>(entity).is_some());
	}

	#[test]
	fn zero_health_without_died_still_downs_and_queues_despawn() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<Died>()
			.add_systems(Update, (down_zero_health, ensure_downed_have_despawn).chain());
		let entity = app.world_mut().spawn(Health { current: 0.0, max: 40.0 }).id();

		app.update();

		assert!(app.world().get::<Downed>(entity).is_some());
		assert!(app.world().get::<DespawnAfter>(entity).is_some());
	}

	#[test]
	fn killing_blow_downs_queues_despawn_and_later_hits_do_nothing() {
		use crate::{apply_hits, DamageApplied, Health, Hit};

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<Hit>()
			.add_message::<DamageApplied>()
			.add_message::<Died>()
			.add_systems(
				Update,
				(apply_hits, mark_downed, down_zero_health, ensure_downed_have_despawn).chain(),
			);
		let entity = app.world_mut().spawn(Health::from_max(10.0)).id();
		app.world_mut().write_message(Hit {
			target: entity,
			source: None,
			amount: 10.0,
			point: Vec3::new(1.0, 2.0, 3.0),
			allow_self: false,
		});
		app.update();
		assert_eq!(app.world().get::<Health>(entity).map(|health| health.current), Some(0.0));
		assert!(app.world().get::<Downed>(entity).is_some_and(|downed| downed.point.y == 2.0));
		assert!(app.world().get::<DespawnAfter>(entity).is_some());

		app.world_mut().write_message(Hit {
			target: entity,
			source: None,
			amount: 25.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();
		assert_eq!(app.world().get::<Health>(entity).map(|health| health.current), Some(0.0));
		assert!(app.world().entities().contains(entity));
	}

	#[test]
	fn burst_kill_in_one_frame_downs_once_and_ignores_overkill() {
		use crate::{apply_hits, DamageApplied, Health, Hit};

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<Hit>()
			.add_message::<DamageApplied>()
			.add_message::<Died>()
			.add_systems(
				Update,
				(apply_hits, mark_downed, down_zero_health, ensure_downed_have_despawn).chain(),
			);
		let entity = app.world_mut().spawn(Health::from_max(12.0)).id();
		for index in 0..6 {
			app.world_mut().write_message(Hit {
				target: entity,
				source: None,
				amount: 5.0,
				point: Vec3::new(index as f32, 1.5, 0.0),
				allow_self: false,
			});
		}
		app.update();
		assert_eq!(app.world().get::<Health>(entity).map(|health| health.current), Some(0.0));
		assert!(app.world().get::<Downed>(entity).is_some());
		assert!(app.world().get::<DespawnAfter>(entity).is_some());
		assert!(app.world().entities().contains(entity));
	}

	#[test]
	fn mark_downed_skips_a_despawned_victim() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<Died>()
			.add_systems(Update, mark_downed);
		let entity = app.world_mut().spawn_empty().id();
		app.world_mut().write_message(Died { entity, source: None, point: Vec3::ZERO });
		app.world_mut().despawn(entity);
		app.update();
	}

	#[test]
	fn downed_without_a_timer_gets_a_fallback_corpse_clock() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, ensure_downed_have_despawn);
		let entity =
			app.world_mut().spawn(Downed { source: None, point: Vec3::ZERO, at: 0.0 }).id();

		app.update();

		assert!(app.world().get::<DespawnAfter>(entity).is_some());
		assert!(
			(app.world().get::<DespawnAfter>(entity).unwrap().remaining_secs()
				- DEFAULT_CORPSE_SECS)
				.abs() < 1e-4
		);
	}

	#[test]
	fn zero_delay_despawns_on_the_next_drain() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, tick_queued_despawns);
		let entity = app.world_mut().spawn(DespawnAfter::seconds(0.0)).id();

		app.update();

		assert!(!app.world().entities().contains(entity));
	}
}
