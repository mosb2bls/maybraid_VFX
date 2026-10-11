//! Pack-wide semantic memory. Not visual contact. Not a mixer.

use std::collections::BTreeSet;

use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};
use threat_intelligence::{
	AffiliationStrength, Affiliations, ThreatGroupId, ThreatId, ThreatIntelligenceUser,
	ThreatKnowledge, ThreatRecord, ThreatRegistry, ThreatSource,
};

use crate::member::MemberOf;
use crate::roster::MobRoster;
use crate::Mob;

/// Seconds until a pack-shared individual antagonism halves.
const SHARE_ANTAGONISM_HALF_LIFE: f32 = 30.0;

const SHAREABLE_TARGETS: TargetSource = TargetSource::from_bits(
	TargetSource::OBJECTIVE.bits()
		| TargetSource::RECEIVED_FIRE.bits()
		| TargetSource::ENEMYSHIP.bits()
		| TargetSource::FIREARM.bits(),
);

/// Pack-wide semantic memory. Not visual contact. Not a mixer.
#[derive(Component, Clone, Debug, Default)]
pub struct MobKnowledge {
	pub threats: BTreeSet<ThreatId>,
	pub targets: BTreeSet<Entity>,
}

/// Host grant for pack write-up / fan-out. High cull keeps the board.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MobSharePolicy {
	pub enabled: bool,
}

impl MobSharePolicy {
	pub fn on() -> Self {
		Self { enabled: true }
	}
}

/// Live member that can ingest a shared threat finding.
pub struct ThreatShareRecipient<'a> {
	pub entity: Entity,
	pub affiliations: &'a mut Affiliations,
	pub user: &'a ThreatIntelligenceUser,
	pub knowledge: &'a mut ThreatKnowledge,
}

impl MobKnowledge {
	pub fn adopt_threat(&mut self, id: ThreatId) -> bool {
		self.threats.insert(id)
	}

	pub fn adopt_target(&mut self, entity: Entity) -> bool {
		self.targets.insert(entity)
	}

	pub fn adopt_first_hand_threats(
		&mut self,
		knowledge: &ThreatKnowledge,
		host: Entity,
		roster: &MobRoster,
		registry: &ThreatRegistry,
		members: &Query<&MemberOf>,
	) {
		for known in knowledge.iter() {
			if !known.sources.is_first_hand() {
				continue;
			}
			if subject_is_pack_mate_id(known.id, host, roster, registry, members) {
				continue;
			}
			self.adopt_threat(known.id);
		}
	}

	pub fn adopt_shareable_targets(&mut self, targeting: &CombatTargeting) {
		for (subject, target) in &targeting.active {
			if !target.active_sources().intersects(SHAREABLE_TARGETS) {
				continue;
			}
			self.adopt_target(*subject);
		}
	}

	pub fn share_threat_with(
		&self,
		id: ThreatId,
		recipient: Entity,
		affiliations: &mut Affiliations,
		knowledge: &mut ThreatKnowledge,
		user: &ThreatIntelligenceUser,
		registry: &ThreatRegistry,
		now: f32,
	) -> bool {
		let Some(record) = registry.get(id) else {
			return false;
		};
		if record.entity == recipient {
			return false;
		}
		if knowledge.get(id).is_some() {
			return false;
		}
		affiliations.antagonize(
			ThreatGroupId::individual(id),
			AffiliationStrength::decaying(1.0, now, SHARE_ANTAGONISM_HALF_LIFE),
		);
		knowledge
			.observe(
				record,
				affiliations,
				ThreatSource::SHARED,
				1.0,
				now,
				user.policy.threat_threshold,
			)
			.is_some()
	}

	/// Whether [`Self::share_into`] would write anything for `recipient`.
	///
	/// Mirrors the early returns of [`Self::share_threat_with`] so callers can
	/// leave a member's `Affiliations` / `ThreatKnowledge` untouched (and
	/// unchanged for change detection) when the board has nothing new for it.
	pub fn has_threats_for(
		&self,
		recipient: Entity,
		knowledge: &ThreatKnowledge,
		registry: &ThreatRegistry,
	) -> bool {
		self.threats.iter().any(|id| {
			registry.get(*id).is_some_and(|record| record.entity != recipient)
				&& knowledge.get(*id).is_none()
		})
	}

	pub fn share_target_with(&self, subject: Entity, targeting: &mut CombatTargeting) -> bool {
		if !self.targets.contains(&subject) {
			return false;
		}
		targeting.include(subject, TargetSource::SHARED)
	}

	pub fn share_into<'a>(
		&self,
		recipients: impl IntoIterator<Item = ThreatShareRecipient<'a>>,
		registry: &ThreatRegistry,
		now: f32,
	) {
		for recipient in recipients {
			for id in &self.threats {
				self.share_threat_with(
					*id,
					recipient.entity,
					recipient.affiliations,
					recipient.knowledge,
					recipient.user,
					registry,
					now,
				);
			}
		}
	}

	pub fn share_targets_into<'a>(
		&self,
		recipients: impl IntoIterator<Item = (Entity, &'a mut CombatTargeting)>,
	) {
		for (recipient, targeting) in recipients {
			for subject in &self.targets {
				if *subject == recipient {
					continue;
				}
				self.share_target_with(*subject, targeting);
			}
		}
	}

	pub fn drain_into(
		&self,
		recipient: Entity,
		affiliations: &mut Affiliations,
		user: &ThreatIntelligenceUser,
		knowledge: &mut ThreatKnowledge,
		targeting: Option<&mut CombatTargeting>,
		registry: &ThreatRegistry,
		now: f32,
	) {
		self.share_into(
			[ThreatShareRecipient { entity: recipient, affiliations, user, knowledge }],
			registry,
			now,
		);
		if let Some(targeting) = targeting {
			self.share_targets_into([(recipient, targeting)]);
		}
	}
}

fn subject_is_pack_mate(
	record: &ThreatRecord,
	host: Entity,
	members: &Query<&MemberOf>,
	roster: &MobRoster,
) -> bool {
	if members.get(record.entity).is_ok_and(|membership| membership.mob == host) {
		return true;
	}
	roster.iter().any(|(_, member)| member.entity == Some(record.entity))
}

#[cfg(test)]
mod tests;

fn subject_is_pack_mate_id(
	id: ThreatId,
	host: Entity,
	roster: &MobRoster,
	registry: &ThreatRegistry,
	members: &Query<&MemberOf>,
) -> bool {
	let Some(record) = registry.get(id) else {
		return false;
	};
	subject_is_pack_mate(record, host, members, roster)
}

/// `(mob, plant)` pairs sorted by mob; plants keep query order within a mob.
fn group_by_mob(
	pairs: &mut Vec<(Entity, Entity)>,
	members: impl Iterator<Item = (Entity, Entity)>,
) {
	pairs.clear();
	pairs.extend(members);
	pairs.sort_by_key(|&(mob, _)| mob);
}

fn plants_of(pairs: &[(Entity, Entity)], host: Entity) -> impl Iterator<Item = Entity> + '_ {
	let start = pairs.partition_point(|&(mob, _)| mob < host);
	pairs[start..]
		.iter()
		.take_while(move |&&(mob, _)| mob == host)
		.map(|&(_, plant)| plant)
}

pub(crate) fn share_mob_threats(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	mut boards: Query<(Entity, &MobRoster, &MobSharePolicy, &mut MobKnowledge), With<Mob>>,
	members: Query<&MemberOf>,
	mut recipients: Query<(
		Entity,
		&MemberOf,
		&mut Affiliations,
		&ThreatIntelligenceUser,
		&mut ThreatKnowledge,
	)>,
	mut by_mob: Local<Vec<(Entity, Entity)>>,
) {
	let now = time.elapsed_secs();
	group_by_mob(
		&mut by_mob,
		recipients.iter().map(|(plant, membership, ..)| (membership.mob, plant)),
	);
	for (host, roster, policy, mut board) in &mut boards {
		if !policy.enabled {
			continue;
		}
		for plant in plants_of(&by_mob, host) {
			let Ok((_, _, _, _, knowledge)) = recipients.get(plant) else {
				continue;
			};
			board.adopt_first_hand_threats(knowledge, host, roster, &registry, &members);
		}
		for plant in plants_of(&by_mob, host) {
			let Ok((plant, _, mut affiliations, user, mut knowledge)) = recipients.get_mut(plant)
			else {
				continue;
			};
			if !board.has_threats_for(plant, &knowledge, &registry) {
				continue;
			}
			board.share_into(
				[ThreatShareRecipient {
					entity: plant,
					affiliations: &mut affiliations,
					user,
					knowledge: &mut knowledge,
				}],
				&registry,
				now,
			);
		}
	}
}

pub(crate) fn share_mob_targets(
	mut boards: Query<(Entity, &MobSharePolicy, &mut MobKnowledge), With<Mob>>,
	mut recipients: Query<(Entity, &MemberOf, &mut CombatTargeting)>,
	mut by_mob: Local<Vec<(Entity, Entity)>>,
) {
	group_by_mob(
		&mut by_mob,
		recipients.iter().map(|(plant, membership, _)| (membership.mob, plant)),
	);
	for (host, policy, mut board) in &mut boards {
		if !policy.enabled {
			continue;
		}
		for plant in plants_of(&by_mob, host) {
			if let Ok((_, _, targeting)) = recipients.get(plant) {
				board.adopt_shareable_targets(targeting);
			}
		}
		for plant in plants_of(&by_mob, host) {
			let Ok((plant, _, mut targeting)) = recipients.get_mut(plant) else {
				continue;
			};
			if board.targets.iter().all(|subject| *subject == plant) {
				continue;
			}
			board.share_targets_into([(plant, &mut *targeting)]);
		}
	}
}
