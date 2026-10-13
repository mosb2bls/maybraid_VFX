//! Maps applied injury onto directed threat knowledge.
//!
//! The victim antagonizes the attacker's individual group, then writes a
//! [`ThreatObservation`](threat_intelligence::ThreatObservation). Classification
//! still runs through affiliation gating; this adapter only supplies the stimulus.

use bevy::prelude::*;
use damage::{DamageApplied, DamageSystems};
use threat_intelligence::{
	AffiliationStrength, Affiliations, ThreatGroupId, ThreatId, ThreatIntelligencePlugin,
	ThreatIntelligenceUser, ThreatObservation, ThreatSource, ThreatSubject,
};

/// Seconds until a damage-learned individual antagonism halves.
const DAMAGE_ANTAGONISM_HALF_LIFE: f32 = 30.0;

pub struct ThreatIntelligenceDamagePlugin;

impl Plugin for ThreatIntelligenceDamagePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ThreatIntelligencePlugin>() {
			app.add_plugins(ThreatIntelligencePlugin);
		}
		app.add_systems(PostUpdate, ingest_damage_threats.after(DamageSystems::Apply));
	}
}

pub fn ingest_damage_threats(
	time: Res<Time>,
	mut applied: MessageReader<DamageApplied>,
	subjects: Query<&ThreatSubject>,
	mut victims: Query<(&mut Affiliations, Has<ThreatIntelligenceUser>)>,
	mut observations: MessageWriter<ThreatObservation>,
) {
	let now = time.elapsed_secs();
	for event in applied.read() {
		let Some(source) = event.source else {
			continue;
		};
		if source == event.target {
			continue;
		}
		let source_id = subjects
			.get(source)
			.map(|subject| subject.id)
			.unwrap_or(ThreatId(source.to_bits()));
		let Ok((mut affiliations, has_user)) = victims.get_mut(event.target) else {
			continue;
		};
		affiliations.antagonize(
			ThreatGroupId::individual(source_id),
			AffiliationStrength::decaying(1.0, now, DAMAGE_ANTAGONISM_HALF_LIFE),
		);
		if has_user {
			observations.write(ThreatObservation::new(
				event.target,
				source_id,
				ThreatSource::RECEIVED_DAMAGE,
				1.0,
			));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use threat_intelligence::{
		ThreatDiscoveryPolicy, ThreatIntelligenceUser, ThreatKnowledge, ThreatRegistry,
	};

	#[test]
	fn damage_antagonizes_the_attacker_and_enters_knowledge() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ThreatIntelligenceDamagePlugin))
			.add_message::<DamageApplied>();
		let attacker_id = ThreatId(1);
		let victim_id = ThreatId(2);
		let attacker = app
			.world_mut()
			.spawn((
				ThreatSubject::new(attacker_id),
				Affiliations::with_self(attacker_id),
				GlobalTransform::from_translation(Vec3::X),
			))
			.id();
		let victim = app
			.world_mut()
			.spawn((
				ThreatSubject::new(victim_id),
				Affiliations::with_self(victim_id),
				ThreatIntelligenceUser::new(ThreatDiscoveryPolicy {
					threat_threshold: 0.2,
					..default()
				}),
				ThreatKnowledge::default(),
				GlobalTransform::default(),
			))
			.id();
		app.update();
		assert!(app.world().resource::<ThreatRegistry>().get(attacker_id).is_some());
		app.world_mut().write_message(DamageApplied {
			target: victim,
			source: Some(attacker),
			amount: 10.0,
			applied: 10.0,
			remaining: 90.0,
			point: Vec3::ZERO,
		});
		app.update();
		assert!(app.world().get::<Affiliations>(victim).is_some_and(|affiliations| {
			affiliations
				.known_antagonists
				.contains_key(&ThreatGroupId::individual(attacker_id))
		}));
		app.update();
		assert!(app
			.world()
			.get::<ThreatKnowledge>(victim)
			.is_some_and(|knowledge| knowledge.get(attacker_id).is_some()));
		Ok(())
	}
}
