use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
	AimPolicy, AimSnapshot, AttackDefinition, AttackId, AttackPresentationKind, FriendlyFire,
	HazardPose, HazardSpec, HitCandidate, PresentationPhase, TelegraphSpec,
};

/// Opaque live-instance key. Presentation tracks effects by this id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttackInstanceId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackStage {
	Windup,
	Execute,
	Recover,
}

impl AttackStage {
	pub fn next(self) -> Option<Self> {
		match self {
			Self::Windup => Some(Self::Execute),
			Self::Execute => Some(Self::Recover),
			Self::Recover => None,
		}
	}
}

/// Bookkeeping on an encounter or summoned owner.
#[derive(Component, Clone, Debug, Default)]
pub struct AttackUser {
	pub cooldown_remaining: HashMap<AttackId, f32>,
}

impl AttackUser {
	pub fn ready(&self, id: AttackId) -> bool {
		self.cooldown_remaining.get(&id).copied().unwrap_or(0.0) <= 1e-4
	}

	pub fn start_cooldown(&mut self, id: AttackId, seconds: f32) {
		self.cooldown_remaining.insert(id, seconds.max(0.0));
	}

	pub fn tick_cooldowns(&mut self, dt: f32) {
		for remaining in self.cooldown_remaining.values_mut() {
			*remaining = (*remaining - dt).max(0.0);
		}
		self.cooldown_remaining.retain(|_, remaining| *remaining > 1e-4);
	}
}

/// One in-flight attack. Lives as its own entity so owner despawn is detectable.
#[derive(Component, Clone, Debug)]
pub struct AttackInstance {
	pub id: AttackInstanceId,
	pub definition: AttackId,
	pub owner: Entity,
	pub stage: AttackStage,
	pub age: f32,
	pub stage_age: f32,
	pub snapshot: AimSnapshot,
	pub live_target: Option<Entity>,
	pub allies: Vec<Entity>,
	pub hit_clock: HashMap<Entity, f32>,
	pub friendly_fire: FriendlyFire,
	pub aim: AimPolicy,
}

pub struct TickOutcome {
	pub events: Vec<AttackPresentationKind>,
	pub pulse: bool,
	pub completed: bool,
}

impl AttackInstance {
	pub fn new(
		id: AttackInstanceId,
		definition: &AttackDefinition,
		owner: Entity,
		snapshot: AimSnapshot,
		live_target: Option<Entity>,
		allies: Vec<Entity>,
	) -> Self {
		Self {
			id,
			definition: definition.id,
			owner,
			stage: AttackStage::Windup,
			age: 0.0,
			stage_age: 0.0,
			snapshot,
			live_target,
			allies,
			hit_clock: HashMap::new(),
			friendly_fire: definition.friendly_fire,
			aim: definition.aim,
		}
	}

	pub fn refresh_aim(&mut self, target_point: Option<Vec3>, owner_origin: Vec3) {
		if !self.snapshot.tracks(self.aim, self.stage) {
			return;
		}
		let Some(point) = target_point else {
			return;
		};
		self.snapshot = AimSnapshot::toward(owner_origin, point, self.live_target);
	}

	pub fn telegraph_pose(&self, telegraph: TelegraphSpec) -> HazardPose {
		HazardSpec {
			shape: telegraph.shape,
			origin: telegraph.origin,
			damage: crate::DamageSpec::once(0.0),
		}
		.pose(self.snapshot, self.stage_age)
	}

	pub fn hazard_pose(&self, spec: HazardSpec) -> HazardPose {
		spec.pose(self.snapshot, self.stage_age)
	}

	pub fn collect_hits(
		&mut self,
		now: f32,
		spec: HazardSpec,
		candidates: &[HitCandidate],
	) -> Vec<damage::Hit> {
		crate::hazard::HazardHitPass {
			owner: self.owner,
			friendly_fire: self.friendly_fire,
			allies: &self.allies,
			now,
			spec,
			pose: self.hazard_pose(spec),
			candidates,
			hit_clock: &mut self.hit_clock,
		}
		.collect()
	}

	/// Advance stages. Large `dt` still visits Execute so one hitch cannot skip
	/// the pulse or leave hazards spawned.
	pub fn tick(&mut self, dt: f32, definition: &AttackDefinition) -> TickOutcome {
		self.age += dt.max(0.0);
		self.stage_age += dt.max(0.0);
		let mut outcome = TickOutcome { events: Vec::new(), pulse: false, completed: false };
		loop {
			let limit = definition.timing.duration(self.stage);
			if self.stage_age < limit {
				self.emit_active(&mut outcome, definition);
				break;
			}
			let overflow = self.stage_age - limit;
			if !self.cross_stage(&mut outcome, definition) {
				break;
			}
			self.stage_age = overflow;
		}
		outcome
	}

	fn emit_active(&self, outcome: &mut TickOutcome, definition: &AttackDefinition) {
		match self.stage {
			AttackStage::Windup => {
				if let Some(telegraph) = definition.telegraph {
					outcome.events.push(AttackPresentationKind::Telegraph {
						phase: PresentationPhase::Updated,
						pose: self.telegraph_pose(telegraph),
						shape: telegraph.shape,
					});
				}
			}
			AttackStage::Execute => {
				outcome.pulse = true;
				if let Some(telegraph) =
					definition.telegraph.filter(|telegraph| telegraph.persist_into_execute)
				{
					outcome.events.push(AttackPresentationKind::Telegraph {
						phase: PresentationPhase::Updated,
						pose: self.telegraph_pose(telegraph),
						shape: telegraph.shape,
					});
				}
				for (index, spec) in definition.hazards.iter().enumerate() {
					outcome.events.push(AttackPresentationKind::Hazard {
						index,
						phase: PresentationPhase::Updated,
						pose: self.hazard_pose(*spec),
						shape: spec.shape,
					});
				}
			}
			AttackStage::Recover => {}
		}
	}

	fn cross_stage(&mut self, outcome: &mut TickOutcome, definition: &AttackDefinition) -> bool {
		let from = self.stage;
		let Some(to) = from.next() else {
			outcome.completed = true;
			return false;
		};
		match (from, to) {
			(AttackStage::Windup, AttackStage::Execute) => {
				if let Some(telegraph) = definition.telegraph {
					let phase = if telegraph.persist_into_execute {
						PresentationPhase::Updated
					} else {
						PresentationPhase::Despawned
					};
					outcome.events.push(AttackPresentationKind::Telegraph {
						phase,
						pose: self.telegraph_pose(telegraph),
						shape: telegraph.shape,
					});
				}
				for (index, spec) in definition.hazards.iter().enumerate() {
					outcome.events.push(AttackPresentationKind::Hazard {
						index,
						phase: PresentationPhase::Spawned,
						pose: self.hazard_pose(*spec),
						shape: spec.shape,
					});
				}
				outcome.pulse = true;
			}
			(AttackStage::Execute, AttackStage::Recover) => {
				outcome.pulse = true;
				self.emit_end_volumes(outcome, definition);
			}
			_ => {}
		}
		outcome
			.events
			.push(AttackPresentationKind::Stage { from, to, snapshot: self.snapshot });
		self.stage = to;
		true
	}

	fn emit_end_volumes(&self, outcome: &mut TickOutcome, definition: &AttackDefinition) {
		if let Some(telegraph) = definition.telegraph {
			outcome.events.push(AttackPresentationKind::Telegraph {
				phase: PresentationPhase::Despawned,
				pose: self.telegraph_pose(telegraph),
				shape: telegraph.shape,
			});
		}
		for (index, spec) in definition.hazards.iter().enumerate() {
			outcome.events.push(AttackPresentationKind::Hazard {
				index,
				phase: PresentationPhase::Despawned,
				pose: self.hazard_pose(*spec),
				shape: spec.shape,
			});
		}
	}

	pub fn begin_events(&self, definition: &AttackDefinition) -> Vec<AttackPresentationKind> {
		let mut events = vec![AttackPresentationKind::Stage {
			from: AttackStage::Windup,
			to: AttackStage::Windup,
			snapshot: self.snapshot,
		}];
		if let Some(telegraph) = definition.telegraph {
			events.push(AttackPresentationKind::Telegraph {
				phase: PresentationPhase::Spawned,
				pose: self.telegraph_pose(telegraph),
				shape: telegraph.shape,
			});
		}
		events
	}

	pub fn cancel_events(
		&self,
		definition: Option<&AttackDefinition>,
	) -> Vec<AttackPresentationKind> {
		let Some(definition) = definition else {
			return Vec::new();
		};
		let mut outcome = TickOutcome { events: Vec::new(), pulse: false, completed: false };
		self.emit_end_volumes(&mut outcome, definition);
		outcome.events
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{AttackId, HazardOrigin, HazardShape};

	fn column() -> AttackDefinition {
		AttackDefinition::with_authored_defaults(
			AttackId::new("column"),
			1.0,
			2.0,
			0.5,
			AimPolicy::TargetAtWindup,
		)
		.with_telegraph(TelegraphSpec {
			shape: HazardShape::Sphere { radius: 1.5 },
			origin: HazardOrigin::AimPoint,
			persist_into_execute: false,
		})
		.authored_hazard(HazardShape::Cylinder { radius: 1.5, height: 7.0 }, HazardOrigin::AimPoint)
	}

	#[test]
	fn hitch_still_visits_execute_and_clears_volumes() {
		let def = column();
		let mut instance = AttackInstance::new(
			AttackInstanceId(1),
			&def,
			Entity::from_bits(1),
			AimSnapshot { origin: Vec3::ZERO, direction: Vec3::NEG_Z, target: None },
			None,
			Vec::new(),
		);
		let outcome = instance.tick(10.0, &def);
		assert!(outcome.pulse);
		assert!(outcome.completed);
		assert!(outcome.events.iter().any(|event| matches!(
			event,
			AttackPresentationKind::Hazard { phase: PresentationPhase::Spawned, .. }
		)));
		assert!(outcome.events.iter().any(|event| matches!(
			event,
			AttackPresentationKind::Hazard { phase: PresentationPhase::Despawned, .. }
		)));
		assert!(outcome.events.iter().any(|event| matches!(
			event,
			AttackPresentationKind::Telegraph { phase: PresentationPhase::Despawned, .. }
		)));
	}

	#[test]
	fn stage_boundaries_wait_for_authored_durations() {
		let def = column();
		let mut instance = AttackInstance::new(
			AttackInstanceId(1),
			&def,
			Entity::from_bits(1),
			AimSnapshot { origin: Vec3::ZERO, direction: Vec3::NEG_Z, target: None },
			None,
			Vec::new(),
		);
		let windup = instance.tick(0.5, &def);
		assert_eq!(instance.stage, AttackStage::Windup);
		assert!(!windup.pulse);
		let execute = instance.tick(0.6, &def);
		assert_eq!(instance.stage, AttackStage::Execute);
		assert!(execute.pulse);
		let recover = instance.tick(2.0, &def);
		assert_eq!(instance.stage, AttackStage::Recover);
		assert!(recover.pulse);
	}
}
