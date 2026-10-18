//! Firearm-specific aim attempts built on generic sight probes.

use std::collections::BTreeMap;

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use firearm_user::FirearmUser;
use firearms::{muzzle_world, BoneMap, FirearmMembers, RigRoot};
use lod_avian::PhysicsInteractionLayer;
use spotting_intelligence::{SpotFeature, SpotSubject};
use spotting_intelligence_avian::clear_segment;

use crate::combat::{gun_landmark, FirearmIntelligence};

const AIM_CACHE_SECS: f32 = 0.1;
const ENDPOINT_EPSILON_SQUARED: f32 = 1e-6;
const MAX_VALIDATED_TARGETS: usize = 2;

fn allocate_aim_samples(vision: usize, candidates: usize, focus: f32) -> Vec<usize> {
	if candidates == 0 {
		return Vec::new();
	}
	let decay = 1.0 - focus.clamp(0.0, 1.0);
	let weights: Vec<f32> = (0..candidates)
		.map(|index| if index == 0 { 1.0 } else { decay.powi(index as i32) })
		.collect();
	let sum = weights.iter().sum::<f32>().max(1e-6);
	let mut grants = vec![0; candidates];
	let mut remainder = Vec::with_capacity(candidates);
	let mut used = 0;
	for (index, weight) in weights.into_iter().enumerate() {
		let exact = vision as f32 * weight / sum;
		let whole = exact.floor() as usize;
		grants[index] = whole;
		used += whole;
		remainder.push((index, exact - whole as f32));
	}
	remainder.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
	for (index, _) in remainder.into_iter().take(vision.saturating_sub(used)) {
		grants[index] += 1;
	}
	grants
}

/// One sampled trajectory from the current muzzle pose to a target feature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimTrajectory {
	pub target: Entity,
	pub feature: SpotFeature,
	pub aim_point: Vec3,
	pub center: Vec3,
	pub distance: f32,
	pub clear: bool,
	pub checked_at: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AimCacheStamp {
	target: Entity,
	muzzle: Vec3,
	target_position: Vec3,
	last_spotted_at: f32,
	checked_at: f32,
}

impl AimCacheStamp {
	fn reusable(
		self,
		target: Entity,
		muzzle: Vec3,
		target_position: Vec3,
		last_spotted_at: f32,
		now: f32,
	) -> bool {
		self.target == target
			&& self.muzzle.distance_squared(muzzle) <= ENDPOINT_EPSILON_SQUARED
			&& self.target_position.distance_squared(target_position) <= ENDPOINT_EPSILON_SQUARED
			&& self.last_spotted_at == last_spotted_at
			&& now - self.checked_at <= AIM_CACHE_SECS
	}
}

/// Cached aim attempts for one firearm combatant.
#[derive(Component, Clone, Debug, Default)]
pub struct FirearmTargeting {
	pub trajectories: Vec<AimTrajectory>,
	stamps: BTreeMap<Entity, AimCacheStamp>,
}

impl FirearmTargeting {
	pub fn for_target(&self, target: Entity) -> impl Iterator<Item = &AimTrajectory> {
		self.trajectories.iter().filter(move |trajectory| trajectory.target == target)
	}

	pub fn select(
		&self,
		target: Entity,
		prefer_head: bool,
		allow_blocked: bool,
	) -> Option<&AimTrajectory> {
		let acceptable = |trajectory: &&AimTrajectory| trajectory.clear || allow_blocked;
		if prefer_head {
			if let Some(head) = self
				.for_target(target)
				.filter(acceptable)
				.find(|trajectory| trajectory.feature.is_head())
			{
				return Some(head);
			}
		}
		self.for_target(target)
			.filter(acceptable)
			.find(|trajectory| !trajectory.feature.is_head())
			.or_else(|| {
				self.for_target(target).find(|trajectory| trajectory.clear || allow_blocked)
			})
	}
}

/// Validate target samples from the fully posed muzzle and update firearm opportunity.
pub(crate) fn validate_firearm_aim_trajectories(
	spatial: SpatialQuery,
	time: Res<Time>,
	mut combatants: Query<(
		&FirearmIntelligence,
		&FirearmUser,
		&mut CombatTargeting,
		&mut FirearmTargeting,
	)>,
	guns: Query<&FirearmMembers>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	subjects: Query<(&GlobalTransform, &SpotSubject)>,
) {
	let now = time.elapsed_secs();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	for (brain, user, mut targeting, mut firearm_targeting) in &mut combatants {
		let vision = usize::from(brain.settings.vision.max(1));
		let ranked: Vec<Entity> = targeting
			.ranked
			.iter()
			.filter(|target| targeting.contact(target.entity).is_some())
			.take(MAX_VALIDATED_TARGETS.min(vision))
			.map(|target| target.entity)
			.collect();
		if ranked.is_empty() {
			firearm_targeting.trajectories.clear();
			firearm_targeting.stamps.clear();
			continue;
		}
		let grants = allocate_aim_samples(vision, ranked.len(), brain.settings.focus);
		let validated: Vec<Entity> = ranked
			.iter()
			.zip(&grants)
			.filter_map(|(entity, grant)| (*grant > 0).then_some(*entity))
			.collect();
		let reset_opportunity: Vec<Entity> = targeting
			.active
			.keys()
			.filter(|entity| !validated.contains(entity))
			.copied()
			.collect();
		for entity in reset_opportunity {
			if let Some(active) = targeting.active_target(entity) {
				let mut factors = active.factors;
				factors.opportunity = 0.0;
				targeting.set_factors(entity, factors);
			}
		}
		let Some(global) = gun_landmark(user.held, "barrel", &guns, &maps, &globals) else {
			continue;
		};
		let (muzzle, _) = muzzle_world(global);
		firearm_targeting
			.trajectories
			.retain(|trajectory| validated.contains(&trajectory.target));
		firearm_targeting.stamps.retain(|target, _| validated.contains(target));

		for (target, sample_count) in ranked.into_iter().zip(grants) {
			if sample_count == 0 {
				continue;
			}
			let Some(contact) = targeting.contact(target).copied() else {
				continue;
			};
			let Ok((target_transform, subject)) = subjects.get(target) else {
				continue;
			};
			let target_position = target_transform.translation();
			if firearm_targeting.stamps.get(&target).is_some_and(|stamp| {
				stamp.reusable(target, muzzle, target_position, contact.last_spotted_at, now)
			}) {
				continue;
			}
			firearm_targeting.trajectories.retain(|trajectory| trajectory.target != target);

			let center = brain.settings.perceive_motion(
				subject.bounds.center_mass(target_position),
				contact.movement_vector,
			);
			let samples = subject.bounds.samples(muzzle, target_position);
			let sample_count = sample_count.min(samples.len());
			for sample in samples.into_iter().take(sample_count) {
				let aim_point =
					brain.settings.perceive_motion(sample.point, contact.movement_vector);
				firearm_targeting.trajectories.push(AimTrajectory {
					target,
					feature: sample.feature,
					aim_point,
					center,
					distance: muzzle.distance(center),
					clear: clear_segment(muzzle, aim_point, &spatial, &filter),
					checked_at: now,
				});
			}
			firearm_targeting.stamps.insert(
				target,
				AimCacheStamp {
					target,
					muzzle,
					target_position,
					last_spotted_at: contact.last_spotted_at,
					checked_at: now,
				},
			);

			if let Some(active) = targeting.active_target(target) {
				let mut factors = active.factors;
				factors.opportunity =
					if firearm_targeting.for_target(target).any(|trajectory| trajectory.clear) {
						1.0 / (1.0 + muzzle.distance(center))
					} else {
						-1.0
					};
				targeting.set_factors(target, factors);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn trajectory(target: Entity, feature: SpotFeature, clear: bool) -> AimTrajectory {
		AimTrajectory {
			target,
			feature,
			aim_point: Vec3::Z,
			center: Vec3::Z,
			distance: 1.0,
			clear,
			checked_at: 0.0,
		}
	}

	#[test]
	fn selection_prefers_a_clear_requested_feature() {
		let target = Entity::from_bits(1);
		let targeting = FirearmTargeting {
			trajectories: vec![
				trajectory(target, SpotFeature::CenterMass, true),
				trajectory(target, SpotFeature::Head, true),
			],
			stamps: BTreeMap::new(),
		};
		assert!(targeting.select(target, true, false).is_some_and(|aim| aim.feature.is_head()));
	}

	#[test]
	fn focus_moves_the_aim_budget_to_the_top_target() {
		assert_eq!(allocate_aim_samples(4, 2, 0.0), vec![2, 2]);
		assert_eq!(allocate_aim_samples(4, 2, 1.0), vec![4, 0]);
	}

	#[test]
	fn cache_invalidates_when_either_endpoint_moves() {
		let target = Entity::from_bits(1);
		let stamp = AimCacheStamp {
			target,
			muzzle: Vec3::ZERO,
			target_position: Vec3::Z,
			last_spotted_at: 1.0,
			checked_at: 1.0,
		};
		assert!(stamp.reusable(target, Vec3::ZERO, Vec3::Z, 1.0, 1.05));
		assert!(!stamp.reusable(target, Vec3::X, Vec3::Z, 1.0, 1.05));
		assert!(!stamp.reusable(target, Vec3::ZERO, Vec3::Z * 2.0, 1.0, 1.05));
		assert!(!stamp.reusable(target, Vec3::ZERO, Vec3::Z, 1.0, 1.2));
	}
}
