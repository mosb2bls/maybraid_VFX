//! Write live pose/health back onto the roster and request respawns.

use bevy::prelude::*;
use damage::{DespawnAfter, Downed, Health};
use poi_intelligence::{
	mix_seed, place_nearby_among, PoiId, PoiInterests, PoiRegistry, AGENT_SEPARATION,
};

use crate::host::{Mob, MobId};
use crate::member::MemberOf;
use crate::roster::{
	MobInterests, MobMemberNeeded, MobRespawn, MobRespawnAt, MobRoster, RosterMember,
};

type DownedMember<'a> =
	(Entity, &'a MemberOf, &'a Transform, &'a GlobalTransform, Has<ChildOf>, Option<&'a Health>);
type LiveMember<'a> = DownedMember<'a>;
type RespawnHost<'a> = (
	Entity,
	&'a MobId,
	&'a Transform,
	&'a mut MobRoster,
	Option<&'a MobRespawn>,
	Option<&'a MobInterests>,
);

pub(crate) fn policy(respawn: Option<&MobRespawn>) -> MobRespawn {
	respawn.copied().unwrap_or_else(MobRespawn::never)
}

/// Death starts the replacement clock and queues the corpse. Cull does not.
pub(crate) fn queue_downed_member_deaths(
	time: Res<Time>,
	mut commands: Commands,
	downed: Query<DownedMember<'_>, Added<Downed>>,
	mut rosters: Query<(&mut MobRoster, Option<&MobRespawn>), With<Mob>>,
) {
	let now = time.elapsed_secs();
	for (entity, membership, transform, global, parented, health) in &downed {
		let policy = rosters
			.get(membership.mob)
			.ok()
			.map(|(_, respawn)| policy(respawn))
			.unwrap_or_else(MobRespawn::never);
		// Always queue the corpse. Roster misses used to `continue` first, so a
		// stale live pointer left a 0-HP plant that `apply_hits` then ignored.
		commands.entity(entity).try_insert(DespawnAfter::seconds(policy.corpse_secs));
		let Ok((mut roster, _)) = rosters.get_mut(membership.mob) else {
			continue;
		};
		let Some(member) = roster.get_mut(membership.slot) else {
			continue;
		};
		if member.entity != Some(entity) {
			continue;
		}
		member.pose = member_translation(transform, global, parented);
		if let Some(health) = health {
			member.health = *health;
		}
		schedule_respawn(member, membership.slot, policy, now);
	}
}

pub(crate) fn write_back_mob_roster(
	members: Query<LiveMember<'_>>,
	mut rosters: Query<(Entity, &mut MobRoster), With<Mob>>,
) {
	let live: Vec<_> = members
		.iter()
		.map(|(entity, membership, transform, global, parented, health)| {
			(
				entity,
				membership.mob,
				membership.slot,
				member_translation(transform, global, parented),
				health.copied(),
			)
		})
		.collect();

	for (host, mut roster) in &mut rosters {
		if !roster_needs_writeback(&roster, host, &live) {
			continue;
		}
		for (slot, member) in roster.iter_mut() {
			if let Some((_, _, _, pose, health)) =
				live.iter().find(|(entity, mob, live_slot, ..)| {
					*mob == host && *live_slot == slot && member.entity == Some(*entity)
				}) {
				if member.pose != *pose {
					member.pose = *pose;
				}
				if let Some(health) = health {
					if member.health != *health {
						member.health = *health;
					}
				}
				continue;
			}
			if member.entity.is_some() {
				member.entity = None;
			}
		}
	}
}

fn roster_needs_writeback(
	roster: &MobRoster,
	host: Entity,
	live: &[(Entity, Entity, u16, Vec3, Option<Health>)],
) -> bool {
	roster.iter().any(|(slot, member)| match live_snapshot(live, host, slot, member.entity) {
		Some((pose, health)) => {
			member.pose != pose || health.is_some_and(|health| member.health != health)
		}
		None => member.entity.is_some(),
	})
}

fn live_snapshot(
	live: &[(Entity, Entity, u16, Vec3, Option<Health>)],
	host: Entity,
	slot: u16,
	entity: Option<Entity>,
) -> Option<(Vec3, Option<Health>)> {
	live.iter().find_map(|(live_entity, mob, live_slot, pose, health)| {
		(*mob == host && *live_slot == slot && entity == Some(*live_entity))
			.then_some((*pose, *health))
	})
}

fn member_translation(transform: &Transform, global: &GlobalTransform, parented: bool) -> Vec3 {
	if parented {
		global.translation()
	} else {
		transform.translation
	}
}

pub(crate) fn respawn_mob_members(
	time: Res<Time>,
	registry: Option<Res<PoiRegistry>>,
	mut needed: MessageWriter<MobMemberNeeded>,
	mut rosters: Query<RespawnHost<'_>, With<Mob>>,
) {
	let now = time.elapsed_secs();
	for (host, id, transform, mut roster, respawn, interests) in &mut rosters {
		let policy = policy(respawn);
		let ready: Vec<u16> = roster
			.iter()
			.filter(|(_, member)| {
				member.entity.is_none()
					&& !member.spawn_requested
					&& member.respawn_at.is_some_and(|at| now >= at)
			})
			.map(|(slot, _)| slot)
			.collect();
		let mut occupied = roster.occupied_poses(None);
		for slot in ready {
			let Some(member) = roster.get_mut(slot) else {
				continue;
			};
			member.spawn_requested = true;
			member.health.current = member.health.max;
			let excluded = member.excluded_pois().to_vec();
			let last = member.pose;
			let used = member.replacements_used;
			let (pose, poi) = policy.placement(
				transform.translation,
				last,
				*id,
				slot,
				used,
				&excluded,
				&occupied,
				registry.as_deref(),
				interests.map(|interests| &interests.0),
			);
			member.remember_respawn_poi(poi);
			member.pose = pose;
			occupied.push(pose);
			needed.write(MobMemberNeeded { mob: host, id: *id, slot, pose });
		}
	}
}

pub(crate) fn spawn_pose(at: MobRespawnAt, host: Vec3, last: Vec3) -> Vec3 {
	match at {
		MobRespawnAt::Host | MobRespawnAt::Poi => {
			let y = if last.y.abs() > 1e-3 { last.y } else { host.y };
			Vec3::new(host.x, y, host.z)
		}
		MobRespawnAt::LastPose => last,
	}
}

impl MobRespawn {
	#[allow(clippy::too_many_arguments)]
	fn placement(
		self,
		host: Vec3,
		last: Vec3,
		mob: MobId,
		slot: u16,
		generation: u32,
		excluded: &[PoiId],
		occupied: &[Vec3],
		registry: Option<&PoiRegistry>,
		interests: Option<&PoiInterests>,
	) -> (Vec3, Option<PoiId>) {
		if self.at != MobRespawnAt::Poi {
			return (spawn_pose(self.at, host, last), None);
		}
		let seed = respawn_seed(mob, slot, generation);
		let placed = place_nearby_among(
			registry,
			host,
			self.nearby_query(),
			interests,
			excluded,
			seed,
			self.fallback,
			occupied,
			AGENT_SEPARATION,
		);
		(with_member_height(placed.position, host, last, placed.poi.is_some()), placed.poi)
	}

	pub fn delay_for_slot(self, slot: u16) -> f32 {
		self.delay_secs.max(0.0) + f32::from(slot) * self.slot_stagger_secs.max(0.0)
	}
}

fn with_member_height(placed: Vec3, host: Vec3, last: Vec3, at_poi: bool) -> Vec3 {
	if at_poi {
		return placed + Vec3::Y * (last.y - host.y).clamp(0.75, 3.0);
	}
	let y = if last.y.abs() > 1e-3 { last.y } else { host.y };
	Vec3::new(placed.x, y, placed.z)
}

fn respawn_seed(mob: MobId, slot: u16, generation: u32) -> u64 {
	mix_seed(mob.0 ^ u64::from(slot).rotate_left(21) ^ u64::from(generation).rotate_left(43))
}

fn schedule_respawn(member: &mut RosterMember, slot: u16, policy: MobRespawn, now: f32) {
	if member.respawn_at.is_some() || member.spawn_requested {
		return;
	}
	if !policy.allows(member.replacements_used) {
		return;
	}
	member.replacements_used = member.replacements_used.saturating_add(1);
	member.respawn_at = Some(now + policy.delay_for_slot(slot));
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use poi_intelligence::{NearbyFallback, Poi, PoiInterest, PoiKind, DEFAULT_NEARBY_RADIUS};

	use super::*;

	const CAMP: PoiKind = PoiKind::new("test/respawn-camp");

	fn registry_with_two_camps() -> Result<PoiRegistry> {
		let mut registry = PoiRegistry::default();
		registry.upsert(
			Entity::from_bits(11),
			Poi::new(PoiId(11), CAMP).with_arrival_radius(8.0),
			Vec3::X * 20.0,
			true,
			false,
		)?;
		registry.upsert(
			Entity::from_bits(12),
			Poi::new(PoiId(12), CAMP).with_arrival_radius(8.0),
			Vec3::Z * 20.0,
			true,
			false,
		)?;
		Ok(registry)
	}

	#[test]
	fn poi_respawn_avoids_immediately_repeating_a_destination() -> Result<()> {
		let registry = registry_with_two_camps()?;
		let interests = PoiInterests::new([PoiInterest::new(CAMP, 1.0)]);
		let selected =
			registry.choose_nearby(Vec3::ZERO, DEFAULT_NEARBY_RADIUS, &interests, &[PoiId(11)], 42);
		assert_eq!(selected.map(|poi| poi.id), Some(PoiId(12)));
		Ok(())
	}

	#[test]
	fn poi_respawn_fallback_varies_around_the_host() {
		let policy = MobRespawn::default();
		let first = policy.placement(Vec3::ZERO, Vec3::Y, MobId(3), 0, 1, &[], &[], None, None).0;
		let second = policy.placement(Vec3::ZERO, Vec3::Y, MobId(3), 0, 2, &[], &[], None, None).0;
		assert_ne!(first, second);
		assert!((policy.fallback.min_radius..=policy.fallback.max_radius)
			.contains(&first.xz().length()));
		assert!((policy.fallback.min_radius..=policy.fallback.max_radius)
			.contains(&second.xz().length()));
	}

	#[test]
	fn default_respawn_keeps_corpses_readable_and_uses_pois() {
		let policy = MobRespawn::default();
		assert_eq!(policy.corpse_secs, 4.0);
		assert_eq!(policy.at, MobRespawnAt::Poi);
		assert_eq!(policy.poi_radius, DEFAULT_NEARBY_RADIUS);
		assert_eq!(policy.min_radius, 0.0);
		assert_eq!(policy.fallback, NearbyFallback::HOST);
		assert_eq!(policy.slot_stagger_secs, crate::roster::RESPAWN_SLOT_STAGGER_SECS);
		assert!((policy.delay_for_slot(3) - (8.0 + 1.2)).abs() < 1e-4);
	}

	#[test]
	fn death_replace_keeps_siblings_apart_on_an_empty_registry() {
		let policy = MobRespawn::default();
		let first = policy.placement(Vec3::ZERO, Vec3::Y, MobId(3), 0, 1, &[], &[], None, None).0;
		let second = policy
			.placement(Vec3::ZERO, Vec3::Y, MobId(3), 1, 1, &[], &[first], None, None)
			.0;
		assert!((first.xz() - second.xz()).length() >= AGENT_SEPARATION - 1e-3);
	}
}
