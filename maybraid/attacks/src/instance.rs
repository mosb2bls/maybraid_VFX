use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
	AimPolicy, AimSnapshot, AttackDefinition, AttackId, AttackPresentationKind, FriendlyFire,
	HazardOrigin, HazardPose, HazardShape, HazardSpec, HitCandidate, PresentationPhase,
	TelegraphSpec, WaveHitClock, WavefrontPresentation, WavefrontSpec, WavefrontSweep,
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
	pub wave_hits: HashMap<(usize, u8, Entity), f32>,
	pub wave_emissions: HashMap<(usize, u8), (Vec3, Vec3)>,
	pub emission_socket: Option<Entity>,
	pub execute_age: f32,
	pub pulse_from_age: f32,
	pub friendly_fire: FriendlyFire,
	pub aim: AimPolicy,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WaveMotion {
	pub emission: Vec3,
	pub heading: Vec3,
	pub from: f32,
	pub to: f32,
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
			wave_hits: HashMap::new(),
			wave_emissions: HashMap::new(),
			emission_socket: None,
			execute_age: 0.0,
			pulse_from_age: 0.0,
			friendly_fire: definition.friendly_fire,
			aim: definition.aim,
		}
	}

	pub fn with_emission_socket(mut self, socket: Option<Entity>) -> Self {
		self.emission_socket = socket;
		self
	}

	pub fn refresh_emission(&mut self, emission: Vec3) {
		if !self.snapshot.tracks(self.aim, self.stage) {
			return;
		}
		self.snapshot.emission = emission;
	}

	pub fn refresh_aim(&mut self, target_point: Option<Vec3>, owner_origin: Vec3) {
		if !self.snapshot.tracks(self.aim, self.stage) {
			return;
		}
		let Some(point) = target_point else {
			return;
		};
		let emission = self.snapshot.emission;
		self.snapshot = AimSnapshot::toward(owner_origin, point, self.live_target).with_emission(emission);
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
		spec_index: usize,
		spec: HazardSpec,
		candidates: &[HitCandidate],
	) -> Vec<damage::Hit> {
		match spec.shape {
			HazardShape::Wavefront(wave) => self.collect_wave_hits(now, spec_index, spec, wave, candidates),
			_ => crate::hazard::HazardHitPass {
				owner: self.owner,
				friendly_fire: self.friendly_fire,
				allies: &self.allies,
				now,
				spec,
				pose: self.hazard_pose(spec),
				candidates,
				hit_clock: &mut self.hit_clock,
				sweep: None,
				wave: None,
			}
			.collect(),
		}
	}

	#[allow(clippy::too_many_arguments)]
	pub(crate) fn collect_one_wave(
		&mut self,
		now: f32,
		spec_index: usize,
		spec: HazardSpec,
		wave: crate::WavefrontSpec,
		wave_index: u8,
		motion: WaveMotion,
		candidates: &[HitCandidate],
	) -> Vec<damage::Hit> {
		crate::hazard::HazardHitPass {
			owner: self.owner,
			friendly_fire: self.friendly_fire,
			allies: &self.allies,
			now,
			spec,
			pose: wave.pose(motion.emission, motion.heading, motion.to),
			candidates,
			hit_clock: &mut self.hit_clock,
			sweep: Some(WavefrontSweep {
				emission: motion.emission,
				heading: motion.heading,
				from: motion.from,
				to: motion.to,
			}),
			wave: Some(WaveHitClock { spec_index, wave_index, hits: &mut self.wave_hits }),
		}
		.collect()
	}

	fn collect_wave_hits(
		&mut self,
		now: f32,
		spec_index: usize,
		spec: HazardSpec,
		wave: WavefrontSpec,
		candidates: &[HitCandidate],
	) -> Vec<damage::Hit> {
		let prev = self.pulse_from_age;
		let execute_age = self.execute_age;
		let mut motions = Vec::new();
		for wave_index in 0..wave.wave_count() {
			if let Some(motion) =
				self.wave_motion(spec_index, spec.origin, wave, wave_index, prev, execute_age)
			{
				motions.push((wave_index, motion));
			}
		}
		let mut hits = Vec::new();
		for (wave_index, motion) in motions {
			hits.extend(
				crate::hazard::HazardHitPass {
					owner: self.owner,
					friendly_fire: self.friendly_fire,
					allies: &self.allies,
					now,
					spec,
					pose: wave.pose(motion.emission, motion.heading, motion.to),
					candidates,
					hit_clock: &mut self.hit_clock,
					sweep: Some(WavefrontSweep {
						emission: motion.emission,
						heading: motion.heading,
						from: motion.from,
						to: motion.to,
					}),
					wave: Some(WaveHitClock {
						spec_index,
						wave_index,
						hits: &mut self.wave_hits,
					}),
				}
				.collect(),
			);
		}
		hits
	}

	fn wave_spawn(&mut self, spec_index: usize, origin: HazardOrigin, wave: WavefrontSpec, wave_index: u8) -> (Vec3, Vec3) {
		*self.wave_emissions.entry((spec_index, wave_index)).or_insert_with(|| {
			(origin.resolve(self.snapshot), wave.heading(self.snapshot))
		})
	}

	pub(crate) fn live_wave_motions(
		&mut self,
		spec_index: usize,
		spec: HazardSpec,
		wave: WavefrontSpec,
	) -> Vec<(u8, WaveMotion)> {
		let prev = self.pulse_from_age;
		let execute_age = self.execute_age;
		let mut motions = Vec::new();
		for wave_index in 0..wave.wave_count() {
			if let Some(motion) =
				self.wave_motion(spec_index, spec.origin, wave, wave_index, prev, execute_age)
			{
				motions.push((wave_index, motion));
			}
		}
		motions
	}

	fn wave_motion(
		&mut self,
		spec_index: usize,
		origin: HazardOrigin,
		wave: WavefrontSpec,
		wave_index: u8,
		prev_execute_age: f32,
		execute_age: f32,
	) -> Option<WaveMotion> {
		let emit = wave.emit_age(wave_index);
		if execute_age < emit {
			return None;
		}
		let (emission, heading) = self.wave_spawn(spec_index, origin, wave, wave_index);
		let prev_age = (prev_execute_age - emit).max(0.0);
		let age = execute_age - emit;
		let from = wave.travel(prev_age);
		let raw = wave.speed.max(0.0) * age;
		if raw > wave.range.max(0.0) && wave.speed.max(0.0) * prev_age >= wave.range.max(0.0) && prev_execute_age >= emit
		{
			return None;
		}
		Some(WaveMotion { emission, heading, from, to: wave.travel(age) })
	}

	/// Advance stages. Large `dt` still visits Execute so one hitch cannot skip
	/// the pulse or leave hazards spawned.
	pub fn tick(&mut self, dt: f32, definition: &AttackDefinition) -> TickOutcome {
		self.age += dt.max(0.0);
		self.stage_age += dt.max(0.0);
		self.pulse_from_age = if self.stage == AttackStage::Execute { self.execute_age } else { 0.0 };
		let mut previous_execute_age =
			if self.stage == AttackStage::Execute { self.execute_age } else { -1.0 };
		let mut outcome = TickOutcome { events: Vec::new(), pulse: false, completed: false };
		loop {
			let limit = definition.timing.duration(self.stage);
			if self.stage == AttackStage::Execute {
				self.execute_age = self.stage_age.min(limit);
				self.emit_execute_hazards(&mut outcome, definition, previous_execute_age);
				previous_execute_age = self.execute_age;
				outcome.pulse = true;
			}
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
				if let Some(telegraph) =
					definition.telegraph.filter(|telegraph| telegraph.persist_into_execute)
				{
					outcome.events.push(AttackPresentationKind::Telegraph {
						phase: PresentationPhase::Updated,
						pose: self.telegraph_pose(telegraph),
						shape: telegraph.shape,
					});
				}
			}
			AttackStage::Recover => {}
		}
	}

	fn emit_execute_hazards(
		&mut self,
		outcome: &mut TickOutcome,
		definition: &AttackDefinition,
		previous_execute_age: f32,
	) {
		for (index, spec) in definition.hazards.iter().enumerate() {
			self.emit_hazard_events(outcome, index, *spec, previous_execute_age, self.execute_age);
		}
	}

	fn emit_hazard_events(
		&mut self,
		outcome: &mut TickOutcome,
		index: usize,
		spec: HazardSpec,
		previous_execute_age: f32,
		execute_age: f32,
	) {
		match spec.shape {
			HazardShape::Wavefront(wave) => {
				for wave_index in 0..wave.wave_count() {
					let emit = wave.emit_age(wave_index);
					if execute_age < emit {
						continue;
					}
					let (emission, heading) = self.wave_spawn(index, spec.origin, wave, wave_index);
					let travel = wave.travel(execute_age - emit);
					let pose = wave.pose(emission, heading, travel);
					let presentation =
						WavefrontPresentation::from_wave(wave, emission, heading, travel, wave_index);
					let was_emitted = previous_execute_age >= emit;
					let retired_now = wave.retired(execute_age, wave_index);
					let retired_before = wave.retired(previous_execute_age, wave_index);
					if !was_emitted {
						outcome.events.push(AttackPresentationKind::Hazard {
							index,
							phase: PresentationPhase::Spawned,
							pose,
							shape: spec.shape,
							wave: Some(presentation),
						});
					} else if !retired_now {
						outcome.events.push(AttackPresentationKind::Hazard {
							index,
							phase: PresentationPhase::Updated,
							pose,
							shape: spec.shape,
							wave: Some(presentation),
						});
					}
					if retired_now && !retired_before {
						outcome.events.push(AttackPresentationKind::Hazard {
							index,
							phase: PresentationPhase::Despawned,
							pose,
							shape: spec.shape,
							wave: Some(presentation),
						});
					}
				}
			}
			_ => {
				let phase = if previous_execute_age <= 1e-5 {
					PresentationPhase::Spawned
				} else {
					PresentationPhase::Updated
				};
				outcome.events.push(AttackPresentationKind::Hazard {
					index,
					phase,
					pose: self.hazard_pose(spec),
					shape: spec.shape,
					wave: None,
				});
			}
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
			match spec.shape {
				HazardShape::Wavefront(wave) => {
					for wave_index in 0..wave.wave_count() {
						let (emission, heading) = self
							.wave_emissions
							.get(&(index, wave_index))
							.copied()
							.unwrap_or((spec.origin.resolve(self.snapshot), wave.heading(self.snapshot)));
						let travel = wave.travel(
							wave.wave_age(self.execute_age, wave_index).unwrap_or(0.0),
						);
						outcome.events.push(AttackPresentationKind::Hazard {
							index,
							phase: PresentationPhase::Despawned,
							pose: wave.pose(emission, heading, travel),
							shape: spec.shape,
							wave: Some(WavefrontPresentation::from_wave(
								wave,
								emission,
								heading,
								travel,
								wave_index,
							)),
						});
					}
				}
				_ => outcome.events.push(AttackPresentationKind::Hazard {
					index,
					phase: PresentationPhase::Despawned,
					pose: self.hazard_pose(*spec),
					shape: spec.shape,
					wave: None,
				}),
			}
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
			AimSnapshot::owner_facing(Vec3::ZERO, Vec3::NEG_Z),
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
			AimSnapshot::owner_facing(Vec3::ZERO, Vec3::NEG_Z),
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

	#[test]
	fn wavefront_events_include_origin_direction_speed_range_radius_and_index() -> anyhow::Result<()> {
		let def = AttackDefinition::with_authored_defaults(
			AttackId::new("wave"),
			0.2,
			1.0,
			0.2,
			AimPolicy::TargetAtWindup,
		)
		.authored_hazard(
			HazardShape::Wavefront(WavefrontSpec::new(1.25, 5.0, 8.0).with_waves(2, 0.35)),
			HazardOrigin::Emission,
		);
		let mut instance = AttackInstance::new(
			AttackInstanceId(1),
			&def,
			Entity::from_bits(1),
			AimSnapshot::toward(Vec3::ZERO, Vec3::Z, None).with_emission(Vec3::new(0.0, 0.4, 0.2)),
			None,
			Vec::new(),
		);
		let entered = instance.tick(0.25, &def);
		let first = entered.events.iter().find_map(|event| match event {
			AttackPresentationKind::Hazard { wave: Some(wave), phase: PresentationPhase::Spawned, .. } => {
				Some(*wave)
			}
			_ => None,
		});
		let Some(first) = first else {
			return Err(anyhow::anyhow!("wave 0 should spawn on execute"));
		};
		if first.wave_index != 0 {
			return Err(anyhow::anyhow!("expected wave_index 0"));
		}
		if (first.origin - Vec3::new(0.0, 0.4, 0.2)).length() >= 1e-4 {
			return Err(anyhow::anyhow!("origin should be the emission"));
		}
		if (first.direction - Vec3::Z).length() >= 1e-4 {
			return Err(anyhow::anyhow!("direction should be the heading"));
		}
		if (first.speed - 5.0).abs() >= 1e-4 || (first.range - 8.0).abs() >= 1e-4 {
			return Err(anyhow::anyhow!("speed/range should be authored"));
		}
		if (first.radius - 1.25).abs() >= 1e-4 {
			return Err(anyhow::anyhow!("radius should be constant ring size"));
		}
		if (first.thickness - 0.7).abs() >= 1e-4 {
			return Err(anyhow::anyhow!("thickness should be 2 * half_depth"));
		}
		if (first.position - (first.origin + first.direction * 5.0 * 0.05)).length() >= 1e-3 {
			return Err(anyhow::anyhow!("position should be origin + direction * travel"));
		}
		let second = instance.tick(0.4, &def);
		if !second.events.iter().any(|event| matches!(
			event,
			AttackPresentationKind::Hazard { wave: Some(wave), phase: PresentationPhase::Spawned, .. }
			if wave.wave_index == 1
		)) {
			return Err(anyhow::anyhow!("wave 1 should spawn after the interval"));
		}
		Ok(())
	}
}
