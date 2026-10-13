//! Hide actuator: move to a nearby low-vantage, low-occupancy pocket.

mod candidate;

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use evasion_intelligence::{EvasionIntelligenceUser, EvasionSystems};
use intelligence_lod::{IntelligenceBand, IntelligenceLod};
use lod_avian::PhysicsInteractionLayer;
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement, ReplanThreshold,
};
use spotting_intelligence::SpotSubject;
use spotting_intelligence_avian::clear_segment;

pub use candidate::{occupancy_at, pick_hide, HideCandidate, HideOccupant};

const REPLAN_THRESHOLD: ReplanThreshold =
	ReplanThreshold { refresh_distance: 0.8, check_radius: false, check_vantage_weights: false };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HidingSettings {
	pub horizon: f32,
	pub occupancy_radius: f32,
	pub azimuths: u32,
	pub standoffs: [f32; 2],
}

impl Default for HidingSettings {
	fn default() -> Self {
		Self { horizon: 14.0, occupancy_radius: 2.4, azimuths: 8, standoffs: [4.0, 8.0] }
	}
}

/// Claimed hide pocket so other hiders can treat it as occupied.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HideClaim {
	pub point: Vec3,
}

/// Per-user hide policy. Writes [`MovementObjective::Reach`] only while the
/// evasion signal is hide.
#[derive(Component, Clone, Debug)]
pub struct HidingUser {
	pub settings: HidingSettings,
	driving: bool,
}

impl HidingUser {
	pub fn new(settings: HidingSettings) -> Self {
		Self { settings, driving: false }
	}

	pub fn samples(&self, from: Vec3) -> Vec<Vec3> {
		let mut points = Vec::new();
		for radius in self.settings.standoffs {
			if radius > self.settings.horizon {
				continue;
			}
			points.extend(
				MovementLocation::ring_around(from, from.y, radius, self.settings.azimuths, 0.4)
					.into_iter()
					.map(|location| location.point),
			);
		}
		points
	}
}

impl Default for HidingUser {
	fn default() -> Self {
		Self::new(HidingSettings::default())
	}
}

pub struct HidingPlugin;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HidingSystems {
	Write,
}

impl Plugin for HidingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			HidingSystems::Write
				.after(EvasionSystems::Rank)
				.before(MovementIntelligenceSystems::Replan),
		)
		.add_systems(Update, write_hide_objectives.in_set(HidingSystems::Write));
	}
}

pub fn write_hide_objectives(
	time: Res<Time>,
	spatial: SpatialQuery,
	mut users: Query<(
		Entity,
		&Transform,
		&EvasionIntelligenceUser,
		&mut HidingUser,
		&mut MovementIntelligence,
		Option<&IntelligenceLod>,
	)>,
	subjects: Query<(Entity, &Transform, Option<&HideClaim>), With<SpotSubject>>,
	mut commands: Commands,
) {
	let occupants: Vec<HideOccupant> = subjects
		.iter()
		.flat_map(|(entity, transform, claim)| {
			let body = HideOccupant { entity, point: transform.translation };
			claim.map_or_else(
				|| vec![body],
				|claim| vec![body, HideOccupant { entity, point: claim.point }],
			)
		})
		.collect();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);

	let now = time.elapsed_secs();
	for (entity, transform, evasion, mut hiding, mut movement, lod) in &mut users {
		if !evasion.signal.is_hide() {
			let was_driving = hiding.driving;
			hiding.driving = false;
			commands.entity(entity).remove::<HideClaim>();
			if was_driving && evasion.signal.is_idle() {
				movement.hold_at(transform.translation);
				commands.entity(entity).remove::<ReplanMovement>();
			}
			continue;
		}
		let Some(contact) = evasion.best_contact() else {
			if hiding.driving {
				hiding.driving = false;
				commands.entity(entity).remove::<HideClaim>();
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
		let from = transform.translation;
		let threat = contact.position;
		let samples = hiding.samples(from);
		let Some(point) = pick_hide(
			from,
			threat,
			&samples,
			entity,
			&occupants,
			hiding.settings.occupancy_radius,
			|candidate| !clear_segment(threat, candidate, &spatial, &filter),
		) else {
			continue;
		};
		let next = MovementObjective::Reach(MovementLocation::new(
			point,
			movement.ability.agent_radius.max(0.4),
		));
		hiding.driving = true;
		commands.entity(entity).insert(HideClaim { point });
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
	use evasion_intelligence::{AssailantContact, EvasionActuator, EvasionSignal, RankedAssailant};
	use intelligence_lod::{IntelligenceBand, IntelligenceLod};

	fn hide_user(threat: Entity, last_known_at: f32) -> EvasionIntelligenceUser {
		let mut evasion = EvasionIntelligenceUser::default();
		evasion.memory.insert(
			threat,
			AssailantContact {
				subject: threat,
				position: Vec3::X * 12.0,
				movement_vector: Vec3::ZERO,
				last_known_at,
			},
		);
		evasion.ranked.push(RankedAssailant { entity: threat, weight: 1.0 });
		evasion.signal = EvasionSignal { actuator: EvasionActuator::Hide, threat: Some(threat) };
		evasion
	}

	#[test]
	fn samples_stay_inside_the_horizon() -> anyhow::Result<()> {
		let hiding = HidingUser::new(HidingSettings {
			horizon: 6.0,
			occupancy_radius: 2.0,
			azimuths: 4,
			standoffs: [4.0, 8.0],
		});
		let samples = hiding.samples(Vec3::ZERO);
		assert_eq!(samples.len(), 4);
		assert!(samples.iter().all(|point| point.length() <= 6.0 + 1e-3));
		Ok(())
	}

	#[test]
	fn far_stale_hide_does_not_enqueue_covering() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			avian3d::prelude::PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		))
		.add_systems(Update, write_hide_objectives);
		let threat = Entity::from_bits(4);
		let entity = app
			.world_mut()
			.spawn((
				Transform::default(),
				hide_user(threat, -10.0),
				HidingUser::default(),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					Vec3::ZERO,
					0.4,
				))),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();

		app.update();
		assert!(app.world().get::<ReplanMovement>(entity).is_none());
		assert!(app.world().get::<HideClaim>(entity).is_none());
	}

	#[test]
	fn near_hide_still_enqueues_covering() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			avian3d::prelude::PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		))
		.add_systems(Update, write_hide_objectives);
		let threat = Entity::from_bits(4);
		let entity = app
			.world_mut()
			.spawn((
				Transform::default(),
				hide_user(threat, 0.0),
				HidingUser::default(),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					Vec3::ZERO,
					0.4,
				))),
				IntelligenceLod::missing(),
			))
			.id();

		app.update();
		assert!(app.world().get::<ReplanMovement>(entity).is_some());
		assert!(app.world().get::<HideClaim>(entity).is_some());
	}
}
