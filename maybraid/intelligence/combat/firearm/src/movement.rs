//! Firearm movement brain: where to stand relative to combat targets.

use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use damage::Downed;
use intelligence_lod::{IntelligenceBand, IntelligenceLod};
use movement_intelligence::{
	MovementIntelligence, MovementLocation, MovementObjective, ReplanMovement, ReplanThreshold,
};

use crate::combat::FirearmIntelligence;

const REPLAN_THRESHOLD: ReplanThreshold =
	ReplanThreshold { refresh_distance: 0.6, check_radius: true, check_vantage_weights: true };

/// How a firearm combatant stands relative to its targets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirearmMovementIntelligenceSettings {
	/// `(preferred_distance, strength)`. Distance is the vantage / edge radius.
	/// Strength scales how much that standoff matters versus other hints.
	pub range: (f32, f32),
	/// How strongly to seek cover (scales hide on [`MovementObjective::VantageOn`]).
	pub cover: f32,
	/// `(trigger_distance, flee_radius)`. Inside the trigger, write [`MovementObjective::FleeFrom`].
	pub flee: (f32, f32),
	/// `(hide, sightline)` weights written onto [`MovementObjective::VantageOn`].
	pub vantage: (f32, f32),
}

impl Default for FirearmMovementIntelligenceSettings {
	fn default() -> Self {
		Self { range: (1.4, 1.0), cover: 1.0, flee: (1.2, 8.0), vantage: (10.0, 14.0) }
	}
}

/// Per-user firearm movement policy. Writes [`MovementIntelligence::objective`].
#[derive(Component, Debug, Clone)]
pub struct FirearmMovementIntelligence {
	pub settings: FirearmMovementIntelligenceSettings,
	driving: bool,
}

impl FirearmMovementIntelligence {
	pub fn new() -> Self {
		Self { settings: FirearmMovementIntelligenceSettings::default(), driving: false }
	}

	pub fn compose(&self, from: Vec3, target: Vec3) -> MovementObjective {
		self.compose_sight(from, target, true)
	}

	pub fn compose_sight(&self, from: Vec3, target: Vec3, freshly_seen: bool) -> MovementObjective {
		let dist = Vec2::new(from.x, from.z).distance(Vec2::new(target.x, target.z));
		let (flee_at, flee_radius) = self.settings.flee;
		if flee_at > 0.0 && dist < flee_at {
			return MovementObjective::FleeFrom(MovementLocation::new(
				target,
				flee_radius.max(0.5),
			));
		}
		let (standoff, strength) = self.settings.range;
		let radius = standoff.max(0.4);
		let hide = self.settings.vantage.0 * self.settings.cover.max(0.0) * strength.max(0.0);
		let sightline = self.settings.vantage.1 * strength.max(0.0);
		let (hide, sightline) =
			if freshly_seen { (hide, sightline) } else { (hide * 0.35, sightline * 2.2) };
		MovementObjective::VantageOn {
			location: MovementLocation::new(target, radius),
			hide_weight: hide,
			sightline_weight: sightline,
		}
	}
}

impl Default for FirearmMovementIntelligence {
	fn default() -> Self {
		Self::new()
	}
}

pub(crate) fn write_firearm_movement_objectives(
	time: Res<Time>,
	mut combatants: Query<
		(
			Entity,
			&Transform,
			&FirearmIntelligence,
			&CombatTargeting,
			&mut FirearmMovementIntelligence,
			&mut MovementIntelligence,
			Option<&IntelligenceLod>,
		),
		Without<Downed>,
	>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (entity, transform, combat, targeting, mut brain, mut movement, lod) in &mut combatants {
		let Some(target) = targeting.best_contact() else {
			if brain.driving {
				brain.driving = false;
				movement.objective = MovementObjective::Reach(MovementLocation::new(
					transform.translation,
					movement.ability.agent_radius,
				));
				movement.adopt_plan(Vec::new());
				commands.entity(entity).remove::<ReplanMovement>();
			}
			continue;
		};
		let fresh = target.is_fresh(now, combat.settings.fire_spotting_freshness);
		if IntelligenceLod::band_or_near(lod) == IntelligenceBand::Far && !fresh {
			continue;
		}
		brain.driving = true;
		let next = brain.compose_sight(transform.translation, target.position, fresh);
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

	#[test]
	fn compose_vantage_when_outside_flee_range() -> anyhow::Result<()> {
		let brain = FirearmMovementIntelligence::new();
		let objective = brain.compose(Vec3::ZERO, Vec3::X * 6.0);
		assert!(objective.is_vantage_on());
		assert!((objective.location().radius - 1.4).abs() < 1e-4);
		assert!((objective.hide_weight() - 10.0).abs() < 1e-4);
		assert!((objective.sightline_weight() - 14.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn compose_flee_when_inside_trigger_distance() -> anyhow::Result<()> {
		let brain = FirearmMovementIntelligence::new();
		let objective = brain.compose(Vec3::ZERO, Vec3::X * 0.5);
		assert!(matches!(objective, MovementObjective::FleeFrom(_)));
		assert!((objective.location().radius - 8.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn cover_scales_hide_but_not_sightline() -> anyhow::Result<()> {
		let mut brain = FirearmMovementIntelligence::new();
		brain.settings.cover = 0.5;
		let objective = brain.compose(Vec3::ZERO, Vec3::X * 6.0);
		assert!((objective.hide_weight() - 5.0).abs() < 1e-4);
		assert!((objective.sightline_weight() - 14.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn lost_sightline_boosts_search_over_cover() {
		let brain = FirearmMovementIntelligence::new();
		let seen = brain.compose_sight(Vec3::ZERO, Vec3::X * 6.0, true);
		let hunt = brain.compose_sight(Vec3::ZERO, Vec3::X * 6.0, false);
		assert!(hunt.sightline_weight() > seen.sightline_weight());
		assert!(hunt.hide_weight() < seen.hide_weight());
		assert!(seen.needs_replan(hunt, REPLAN_THRESHOLD));
	}

	fn combat_contact(subject: Entity, spotted_at: f32) -> combat_targeting::CombatContact {
		combat_targeting::CombatContact {
			subject,
			position: Vec3::X * 6.0,
			movement_vector: Vec3::ZERO,
			visible_point: Vec3::X * 6.0,
			visible_head: None,
			last_spotted_at: spotted_at,
		}
	}

	fn targeting_with(contact: combat_targeting::CombatContact) -> CombatTargeting {
		let mut targeting = CombatTargeting::default();
		targeting.upsert_contact(contact);
		targeting.rebalance(0.0);
		targeting
	}

	fn firearm_writer_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_systems(Update, write_firearm_movement_objectives);
		app
	}

	#[test]
	fn far_stale_contact_does_not_enqueue_covering() {
		let mut app = firearm_writer_app();
		let target = Entity::from_bits(7);
		let combatant = app
			.world_mut()
			.spawn((
				Transform::default(),
				FirearmIntelligence::new(),
				targeting_with(combat_contact(target, -1.0)),
				FirearmMovementIntelligence::new(),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					Vec3::ZERO,
					0.4,
				))),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();

		app.update();
		assert!(app.world().get::<ReplanMovement>(combatant).is_none());
		assert!(!app
			.world()
			.get::<MovementIntelligence>(combatant)
			.is_some_and(|movement| movement.objective.is_vantage_on()));
	}

	#[test]
	fn near_stale_and_far_fresh_still_enqueue_covering() {
		let mut app = firearm_writer_app();
		let target = Entity::from_bits(7);
		let near = app
			.world_mut()
			.spawn((
				Transform::default(),
				FirearmIntelligence::new(),
				targeting_with(combat_contact(target, -1.0)),
				FirearmMovementIntelligence::new(),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					Vec3::ZERO,
					0.4,
				))),
				IntelligenceLod::missing(),
			))
			.id();
		let far_fresh = app
			.world_mut()
			.spawn((
				Transform::from_xyz(1.0, 0.0, 0.0),
				FirearmIntelligence::new(),
				targeting_with(combat_contact(target, 0.0)),
				FirearmMovementIntelligence::new(),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					Vec3::ZERO,
					0.4,
				))),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();

		app.update();
		assert!(app.world().get::<ReplanMovement>(near).is_some());
		assert!(app.world().get::<ReplanMovement>(far_fresh).is_some());
		assert!(app
			.world()
			.get::<MovementIntelligence>(near)
			.is_some_and(|movement| movement.objective.is_vantage_on()));
	}
}
