use bevy::prelude::*;
use damage::{Health, Hit};

use crate::{AimSnapshot, AttackDefaults, FriendlyFire};

/// How a hazard is placed relative to the aim snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HazardOrigin {
	Owner,
	AimPoint,
	AlongAim { distance: f32 },
	/// Mouth / socket sampled into [`crate::AimSnapshot::emission`].
	Emission,
}

impl HazardOrigin {
	pub fn resolve(self, snapshot: AimSnapshot) -> Vec3 {
		match self {
			Self::Owner => snapshot.origin,
			Self::AimPoint => snapshot.aim_point,
			Self::AlongAim { distance } => snapshot.origin + snapshot.direction * distance.max(0.0),
			Self::Emission => snapshot.emission,
		}
	}
}

/// Authoritative hazard / telegraph volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HazardShape {
	Sphere {
		radius: f32,
	},
	/// Vertical column. Local Y is up, base at the pose origin.
	Cylinder {
		radius: f32,
		height: f32,
	},
	Box {
		half_extents: Vec3,
	},
	/// Corridor warning: local +Z from the pose origin.
	Line {
		length: f32,
		half_width: f32,
	},
	/// Discrete ring / disc that travels along a heading at constant size.
	/// This is not a radially expanding ground ring.
	Wavefront(WavefrontSpec),
}

/// Constant-size moving volume for sequential sonic-style waves.
///
/// The ring is spawned at an emission transform and travels in world space
/// along a horizontal heading (optional pitch) at [`Self::speed`] until
/// [`Self::range`]. Local +Z is travel; the disc lies in XY.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WavefrontSpec {
	pub radius: f32,
	/// Half-thickness along travel. A short capsule / disc, not a sphere.
	pub half_depth: f32,
	pub speed: f32,
	pub range: f32,
	/// Seconds between sequential waves. `<= 0` emits every wave at execute start.
	pub interval: f32,
	/// How many rings to emit. `0` is treated as one.
	pub waves: u8,
	/// Extra pitch above the flattened heading, in radians.
	pub pitch: f32,
}

impl WavefrontSpec {
	pub const fn new(radius: f32, speed: f32, range: f32) -> Self {
		Self { radius, half_depth: 0.35, speed, range, interval: 0.0, waves: 1, pitch: 0.0 }
	}

	pub const fn with_half_depth(mut self, half_depth: f32) -> Self {
		self.half_depth = half_depth;
		self
	}

	pub const fn with_waves(mut self, waves: u8, interval: f32) -> Self {
		self.waves = waves;
		self.interval = interval;
		self
	}

	pub const fn with_pitch(mut self, pitch: f32) -> Self {
		self.pitch = pitch;
		self
	}

	pub const fn wave_count(self) -> u8 {
		if self.waves == 0 { 1 } else { self.waves }
	}

	pub fn heading(self, snapshot: crate::AimSnapshot) -> Vec3 {
		snapshot.travel_direction(self.pitch)
	}

	pub fn emit_age(self, wave_index: u8) -> f32 {
		self.interval.max(0.0) * f32::from(wave_index)
	}

	pub fn wave_age(self, execute_age: f32, wave_index: u8) -> Option<f32> {
		let age = execute_age - self.emit_age(wave_index);
		(age >= 0.0).then_some(age)
	}

	pub fn travel(self, wave_age: f32) -> f32 {
		(self.speed.max(0.0) * wave_age.max(0.0)).min(self.range.max(0.0))
	}

	pub fn retired(self, execute_age: f32, wave_index: u8) -> bool {
		self.wave_age(execute_age, wave_index)
			.is_some_and(|age| self.speed.max(0.0) * age >= self.range.max(0.0))
	}

	pub fn pose(self, emission: Vec3, heading: Vec3, travel: f32) -> HazardPose {
		let heading = if heading.length_squared() <= 1e-8 { Vec3::NEG_Z } else { heading.normalize() };
		HazardPose::new(emission + heading * travel, Quat::from_rotation_arc(Vec3::Z, heading))
	}

	pub fn contains_disc(self, emission: Vec3, heading: Vec3, travel: f32, point: Vec3) -> bool {
		let heading = if heading.length_squared() <= 1e-8 { Vec3::NEG_Z } else { heading.normalize() };
		let center = emission + heading * travel;
		let offset = point - center;
		let along = offset.dot(heading);
		let radial = (offset - heading * along).length();
		along.abs() <= self.half_depth.max(0.0) && radial <= self.radius.max(0.0)
	}

	/// Swept disc from `from` to `to` along the heading so a hitch still hits.
	pub fn contains_swept(
		self,
		emission: Vec3,
		heading: Vec3,
		from: f32,
		to: f32,
		point: Vec3,
	) -> bool {
		let heading = if heading.length_squared() <= 1e-8 { Vec3::NEG_Z } else { heading.normalize() };
		let lo = from.min(to);
		let hi = from.max(to);
		let along = (point - emission).dot(heading);
		if along < lo - self.half_depth.max(0.0) || along > hi + self.half_depth.max(0.0) {
			return false;
		}
		let closest = along.clamp(lo, hi);
		let center = emission + heading * closest;
		let offset = point - center;
		(offset - heading * offset.dot(heading)).length() <= self.radius.max(0.0)
	}

	pub fn sample_distance(self, emission: Vec3, heading: Vec3, travel: f32, point: Vec3) -> f32 {
		let heading = if heading.length_squared() <= 1e-8 { Vec3::NEG_Z } else { heading.normalize() };
		let center = emission + heading * travel;
		let offset = point - center;
		(offset - heading * offset.dot(heading)).length()
	}
}

/// World pose for a telegraph or hazard this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HazardPose {
	pub origin: Vec3,
	pub rotation: Quat,
}

impl HazardPose {
	pub fn new(origin: Vec3, rotation: Quat) -> Self {
		Self { origin, rotation }
	}

	pub fn local_point(self, point: Vec3) -> Vec3 {
		self.rotation.inverse() * (point - self.origin)
	}
}

impl HazardShape {
	pub fn contains(self, pose: HazardPose, point: Vec3) -> bool {
		match self {
			Self::Sphere { radius } => point.distance(pose.origin) <= radius.max(0.0),
			Self::Wavefront(wave) => {
				wave.contains_disc(pose.origin, pose.rotation * Vec3::Z, 0.0, point)
			}
			Self::Cylinder { radius, height } => {
				let local = pose.local_point(point);
				let radial = Vec2::new(local.x, local.z).length();
				radial <= radius.max(0.0) && local.y >= 0.0 && local.y <= height.max(0.0)
			}
			Self::Box { half_extents } => {
				let local = pose.local_point(point);
				local.x.abs() <= half_extents.x
					&& local.y.abs() <= half_extents.y
					&& local.z.abs() <= half_extents.z
			}
			Self::Line { length, half_width } => {
				let local = pose.local_point(point);
				local.z >= 0.0
					&& local.z <= length.max(0.0)
					&& local.x.abs() <= half_width.max(0.0)
					&& local.y.abs() <= half_width.max(0.0)
			}
		}
	}

	pub fn sample_distance(self, pose: HazardPose, point: Vec3) -> f32 {
		match self {
			Self::Sphere { .. } => point.distance(pose.origin),
			Self::Wavefront(wave) => {
				wave.sample_distance(pose.origin, pose.rotation * Vec3::Z, 0.0, point)
			}
			Self::Cylinder { .. } => {
				let local = pose.local_point(point);
				Vec2::new(local.x, local.z).length()
			}
			Self::Box { .. } | Self::Line { .. } => pose.local_point(point).length(),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageFalloff {
	None,
	Linear { radius: f32 },
}

impl DamageFalloff {
	pub fn amount(self, base: f32, distance: f32) -> f32 {
		match self {
			Self::None => base.max(0.0),
			Self::Linear { radius } => {
				if base <= 0.0 || radius <= 1e-4 || distance > radius {
					0.0
				} else {
					base * (1.0 - distance / radius).clamp(0.0, 1.0)
				}
			}
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageSpec {
	pub amount: f32,
	/// Seconds between Hits on one resolved health entity. `<= 0` means once.
	pub cadence: f32,
	pub falloff: DamageFalloff,
}

impl DamageSpec {
	pub const fn authored() -> Self {
		AttackDefaults::AUTHORED.damage_spec()
	}

	pub const fn once(amount: f32) -> Self {
		Self { amount, cadence: 0.0, falloff: DamageFalloff::None }
	}

	pub fn amount_at(self, distance: f32) -> f32 {
		self.falloff.amount(self.amount, distance)
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HazardSpec {
	pub shape: HazardShape,
	pub origin: HazardOrigin,
	pub damage: DamageSpec,
}

impl HazardSpec {
	pub fn pose(self, snapshot: AimSnapshot, stage_age: f32) -> HazardPose {
		match self.shape {
			HazardShape::Wavefront(wave) => {
				let emission = self.origin.resolve(snapshot);
				let heading = wave.heading(snapshot);
				wave.pose(emission, heading, wave.travel(stage_age))
			}
			_ => HazardPose::new(self.origin.resolve(snapshot), snapshot.rotation()),
		}
	}

	pub fn wave_pose(
		self,
		snapshot: AimSnapshot,
		emission: Vec3,
		heading: Vec3,
		travel: f32,
	) -> HazardPose {
		match self.shape {
			HazardShape::Wavefront(wave) => wave.pose(emission, heading, travel),
			_ => self.pose(snapshot, 0.0),
		}
	}
}

/// One overlap candidate. Child colliders remount onto parent [`Health`].
#[derive(Clone, Copy, Debug)]
pub struct HitCandidate {
	pub entity: Entity,
	pub parent: Option<Entity>,
	pub has_health: bool,
	pub parent_has_health: bool,
	pub point: Vec3,
	/// Spatial overlap already confirmed the collider intersects the hazard.
	pub confirmed: bool,
}

impl HitCandidate {
	pub fn resolved_target(self) -> Entity {
		Health::entity_or_parent(self.entity, self.parent, self.has_health, self.parent_has_health)
	}
}

/// Optional swept ring used so a hitch still registers a passing wave.
#[derive(Clone, Copy, Debug)]
pub struct WavefrontSweep {
	pub emission: Vec3,
	pub heading: Vec3,
	pub from: f32,
	pub to: f32,
}

/// Per-wave hit clock key. Cadence `<= 0` means once per wave.
pub struct WaveHitClock<'a> {
	pub spec_index: usize,
	pub wave_index: u8,
	pub hits: &'a mut std::collections::HashMap<(usize, u8, Entity), f32>,
}

pub struct HazardHitPass<'a> {
	pub owner: Entity,
	pub friendly_fire: FriendlyFire,
	pub allies: &'a [Entity],
	pub now: f32,
	pub spec: HazardSpec,
	pub pose: HazardPose,
	pub candidates: &'a [HitCandidate],
	pub hit_clock: &'a mut std::collections::HashMap<Entity, f32>,
	pub sweep: Option<WavefrontSweep>,
	pub wave: Option<WaveHitClock<'a>>,
}

impl HazardHitPass<'_> {
	pub fn collect(self) -> Vec<Hit> {
		collect_hazard_hits(self)
	}
}

fn collect_hazard_hits(pass: HazardHitPass<'_>) -> Vec<Hit> {
	let HazardHitPass {
		owner,
		friendly_fire,
		allies,
		now,
		spec,
		pose,
		candidates,
		hit_clock,
		sweep,
		mut wave,
	} = pass;
	let mut best: std::collections::HashMap<Entity, (f32, Vec3)> = std::collections::HashMap::new();
	for candidate in candidates {
		let target = candidate.resolved_target();
		if !candidate.has_health && !candidate.parent_has_health {
			continue;
		}
		if !friendly_fire.admits(owner, target, allies) {
			continue;
		}
		if !candidate.confirmed && !volume_contains(spec, pose, sweep, candidate.point) {
			continue;
		}
		let distance = volume_distance(spec, pose, sweep, candidate.point);
		match best.get(&target) {
			Some((current, _)) if *current <= distance => {}
			_ => {
				best.insert(target, (distance, candidate.point));
			}
		}
	}

	let mut hits = Vec::new();
	for (target, (distance, point)) in best {
		let last = match wave.as_ref() {
			Some(wave) => wave.hits.get(&(wave.spec_index, wave.wave_index, target)).copied(),
			None => hit_clock.get(&target).copied(),
		};
		let cadence = if wave.is_some() { 0.0 } else { spec.damage.cadence };
		if !cadence_ready(last, now, cadence) {
			continue;
		}
		let amount = spec.damage.amount_at(distance);
		if amount <= 1e-3 {
			continue;
		}
		if let Some(wave) = wave.as_mut() {
			wave.hits.insert((wave.spec_index, wave.wave_index, target), now);
		} else {
			hit_clock.insert(target, now);
		}
		hits.push(Hit {
			target,
			source: Some(owner),
			amount,
			point,
			allow_self: target == owner,
		});
	}
	hits
}

fn volume_contains(
	spec: HazardSpec,
	pose: HazardPose,
	sweep: Option<WavefrontSweep>,
	point: Vec3,
) -> bool {
	match (spec.shape, sweep) {
		(HazardShape::Wavefront(wave), Some(sweep)) => {
			wave.contains_swept(sweep.emission, sweep.heading, sweep.from, sweep.to, point)
		}
		_ => spec.shape.contains(pose, point),
	}
}

fn volume_distance(
	spec: HazardSpec,
	pose: HazardPose,
	sweep: Option<WavefrontSweep>,
	point: Vec3,
) -> f32 {
	match (spec.shape, sweep) {
		(HazardShape::Wavefront(wave), Some(sweep)) => {
			let along = (point - sweep.emission).dot(sweep.heading);
			let travel = along.clamp(sweep.from.min(sweep.to), sweep.from.max(sweep.to));
			wave.sample_distance(sweep.emission, sweep.heading, travel, point)
		}
		_ => spec.shape.sample_distance(pose, point),
	}
}

fn cadence_ready(last: Option<f32>, now: f32, cadence: f32) -> bool {
	match last {
		None => true,
		Some(_) if cadence <= 0.0 => false,
		Some(last) => now - last >= cadence,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn candidate(entity: Entity, point: Vec3) -> HitCandidate {
		HitCandidate {
			entity,
			parent: None,
			has_health: true,
			parent_has_health: false,
			point,
			confirmed: false,
		}
	}

	#[test]
	fn cylinder_contains_shaft_and_rejects_outside() {
		let pose = HazardPose::new(Vec3::ZERO, Quat::IDENTITY);
		let shape = HazardShape::Cylinder { radius: 1.5, height: 7.0 };
		assert!(shape.contains(pose, Vec3::new(0.0, 3.0, 0.0)));
		assert!(shape.contains(pose, Vec3::new(1.4, 6.9, 0.0)));
		assert!(!shape.contains(pose, Vec3::new(1.6, 3.0, 0.0)));
		assert!(!shape.contains(pose, Vec3::new(0.0, -0.1, 0.0)));
	}

	fn hit_pass<'a>(
		spec: HazardSpec,
		pose: HazardPose,
		candidates: &'a [HitCandidate],
		clock: &'a mut std::collections::HashMap<Entity, f32>,
		now: f32,
	) -> HazardHitPass<'a> {
		HazardHitPass {
			owner: Entity::from_bits(1),
			friendly_fire: FriendlyFire::ExcludeOwner,
			allies: &[],
			now,
			spec,
			pose,
			candidates,
			hit_clock: clock,
			sweep: None,
			wave: None,
		}
	}

	#[test]
	fn origin_variants_keep_owner_aim_and_emission_distinct() {
		let snapshot = AimSnapshot::toward(Vec3::new(0.0, 1.0, 4.0), Vec3::new(3.0, 2.0, -2.0), None)
			.with_emission(Vec3::new(0.1, 1.6, 3.4));
		assert!((HazardOrigin::Owner.resolve(snapshot) - snapshot.origin).length() < 1e-4);
		assert!((HazardOrigin::AimPoint.resolve(snapshot) - snapshot.aim_point).length() < 1e-4);
		assert!((HazardOrigin::Emission.resolve(snapshot) - snapshot.emission).length() < 1e-4);
		assert!((snapshot.aim_point - snapshot.origin).length() > 1.0);
		let along = HazardOrigin::AlongAim { distance: 2.0 }.resolve(snapshot);
		assert!((along - (snapshot.origin + snapshot.direction * 2.0)).length() < 1e-4);
	}

	#[test]
	fn wavefront_is_a_constant_size_disc_not_a_sphere() {
		let wave = WavefrontSpec::new(1.0, 4.0, 10.0).with_half_depth(0.25);
		let pose = wave.pose(Vec3::ZERO, Vec3::Z, 2.0);
		assert!(wave.contains_disc(Vec3::ZERO, Vec3::Z, 2.0, Vec3::new(0.9, 0.0, 2.0)));
		assert!(!wave.contains_disc(Vec3::ZERO, Vec3::Z, 2.0, Vec3::new(1.2, 0.0, 2.0)));
		assert!(!wave.contains_disc(Vec3::ZERO, Vec3::Z, 2.0, Vec3::new(0.0, 0.0, 3.0)));
		assert!(!HazardShape::Wavefront(wave).contains(pose, Vec3::new(0.0, 0.0, 2.9)));
		assert!((pose.origin - Vec3::new(0.0, 0.0, 2.0)).length() < 1e-4);
	}

	#[test]
	fn wavefront_advances_from_emission_and_caps_at_range() {
		let snapshot = AimSnapshot::toward(Vec3::ZERO, Vec3::Z, None)
			.with_emission(Vec3::new(0.0, 0.4, 0.2));
		let spec = HazardSpec {
			shape: HazardShape::Wavefront(WavefrontSpec::new(1.0, 4.0, 10.0)),
			origin: HazardOrigin::Emission,
			damage: DamageSpec::authored(),
		};
		let pose = spec.pose(snapshot, 0.5);
		assert!((pose.origin - Vec3::new(0.0, 0.4, 2.2)).length() < 1e-4);
		let capped = spec.pose(snapshot, 8.0);
		assert!((capped.origin - Vec3::new(0.0, 0.4, 10.2)).length() < 1e-4);
	}

	#[test]
	fn wavefront_pitch_tilts_off_the_horizontal() {
		let snapshot = AimSnapshot::toward(Vec3::ZERO, Vec3::Z, None);
		let wave = WavefrontSpec::new(1.0, 4.0, 10.0).with_pitch(std::f32::consts::FRAC_PI_6);
		let heading = wave.heading(snapshot);
		assert!((heading.y.abs() - 0.5).abs() < 1e-3);
		assert!(heading.z > 0.8);
	}

	#[test]
	fn sequential_waves_emit_on_interval_and_hit_once_each() {
		let wave = WavefrontSpec::new(1.2, 4.0, 8.0).with_waves(3, 0.4).with_half_depth(0.3);
		assert_eq!(wave.wave_count(), 3);
		assert!(!wave.retired(0.0, 0));
		assert!(wave.wave_age(0.2, 1).is_none());
		assert!(wave.wave_age(0.4, 1).is_some());
		assert!(wave.retired(2.1, 0));

		let target = Entity::from_bits(3);
		let spec = HazardSpec {
			shape: HazardShape::Wavefront(wave),
			origin: HazardOrigin::Emission,
			damage: DamageSpec::once(20.0),
		};
		let candidates = [candidate(target, Vec3::new(0.0, 0.0, 2.0))];
		let mut clock = std::collections::HashMap::new();
		let mut wave_hits = std::collections::HashMap::new();
		let first = HazardHitPass {
			sweep: Some(WavefrontSweep {
				emission: Vec3::ZERO,
				heading: Vec3::Z,
				from: 0.0,
				to: 2.1,
			}),
			wave: Some(WaveHitClock { spec_index: 0, wave_index: 0, hits: &mut wave_hits }),
			..hit_pass(spec, wave.pose(Vec3::ZERO, Vec3::Z, 2.0), &candidates, &mut clock, 0.5)
		}
		.collect();
		let second_same_wave = HazardHitPass {
			sweep: Some(WavefrontSweep {
				emission: Vec3::ZERO,
				heading: Vec3::Z,
				from: 2.1,
				to: 3.0,
			}),
			wave: Some(WaveHitClock { spec_index: 0, wave_index: 0, hits: &mut wave_hits }),
			..hit_pass(spec, wave.pose(Vec3::ZERO, Vec3::Z, 3.0), &candidates, &mut clock, 0.8)
		}
		.collect();
		let second_wave = HazardHitPass {
			sweep: Some(WavefrontSweep {
				emission: Vec3::ZERO,
				heading: Vec3::Z,
				from: 0.0,
				to: 2.1,
			}),
			wave: Some(WaveHitClock { spec_index: 0, wave_index: 1, hits: &mut wave_hits }),
			..hit_pass(spec, wave.pose(Vec3::ZERO, Vec3::Z, 2.0), &candidates, &mut clock, 1.0)
		}
		.collect();
		assert_eq!(first.len(), 1);
		assert!(second_same_wave.is_empty());
		assert_eq!(second_wave.len(), 1);
	}

	#[test]
	fn multiple_colliders_yield_one_hit() {
		let body = Entity::from_bits(10);
		let a = Entity::from_bits(11);
		let b = Entity::from_bits(12);
		let spec = HazardSpec {
			shape: HazardShape::Sphere { radius: 3.0 },
			origin: HazardOrigin::AimPoint,
			damage: DamageSpec::authored(),
		};
		let pose = HazardPose::new(Vec3::ZERO, Quat::IDENTITY);
		let candidates = [
			HitCandidate {
				entity: a,
				parent: Some(body),
				has_health: false,
				parent_has_health: true,
				point: Vec3::new(0.2, 0.0, 0.0),
				confirmed: false,
			},
			HitCandidate {
				entity: b,
				parent: Some(body),
				has_health: false,
				parent_has_health: true,
				point: Vec3::new(0.4, 0.0, 0.0),
				confirmed: false,
			},
		];
		let mut clock = std::collections::HashMap::new();
		let hits = hit_pass(spec, pose, &candidates, &mut clock, 0.0).collect();
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].target, body);
		assert_eq!(clock.len(), 1);
	}

	#[test]
	fn cadence_is_per_target_and_once_when_zero() {
		let target = Entity::from_bits(3);
		let spec = HazardSpec {
			shape: HazardShape::Sphere { radius: 2.0 },
			origin: HazardOrigin::AimPoint,
			damage: DamageSpec::once(15.0),
		};
		let pose = HazardPose::new(Vec3::ZERO, Quat::IDENTITY);
		let candidates = [candidate(target, Vec3::ZERO)];
		let mut clock = std::collections::HashMap::new();
		let first = hit_pass(spec, pose, &candidates, &mut clock, 0.0).collect();
		let second = hit_pass(spec, pose, &candidates, &mut clock, 0.25).collect();
		assert_eq!(first.len(), 1);
		assert!(second.is_empty());
	}

	#[test]
	fn linear_falloff_matches_blast_style() {
		assert!((DamageFalloff::Linear { radius: 8.0 }.amount(80.0, 0.0) - 80.0).abs() < 1e-4);
		assert!((DamageFalloff::Linear { radius: 8.0 }.amount(80.0, 4.0) - 40.0).abs() < 1e-4);
		assert_eq!(DamageFalloff::Linear { radius: 8.0 }.amount(80.0, 8.1), 0.0);
	}
}
