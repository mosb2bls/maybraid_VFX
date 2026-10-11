use bevy::prelude::*;
use damage::{Health, Hit};

use crate::{AimSnapshot, AttackDefaults, FriendlyFire};

/// How a hazard is placed relative to the aim snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HazardOrigin {
	Owner,
	AimPoint,
	AlongAim { distance: f32 },
}

impl HazardOrigin {
	pub fn resolve(self, snapshot: AimSnapshot) -> Vec3 {
		match self {
			Self::Owner | Self::AimPoint => snapshot.origin,
			Self::AlongAim { distance } => snapshot.origin + snapshot.direction * distance.max(0.0),
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
	/// Moving disc. Pose origin is the current wavefront center.
	Wavefront {
		radius: f32,
		speed: f32,
		length: f32,
	},
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
			Self::Sphere { radius } | Self::Wavefront { radius, .. } => {
				point.distance(pose.origin) <= radius.max(0.0)
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
			Self::Sphere { .. } | Self::Wavefront { .. } => point.distance(pose.origin),
			Self::Cylinder { .. } => {
				let local = pose.local_point(point);
				Vec2::new(local.x, local.z).length()
			}
			Self::Box { .. } | Self::Line { .. } => pose.local_point(point).length(),
		}
	}

	pub fn wavefront_travel(self, stage_age: f32) -> f32 {
		match self {
			Self::Wavefront { speed, length, .. } => {
				(speed.max(0.0) * stage_age.max(0.0)).min(length.max(0.0))
			}
			_ => 0.0,
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
		let mut origin = self.origin.resolve(snapshot);
		origin += snapshot.direction * self.shape.wavefront_travel(stage_age);
		HazardPose::new(origin, snapshot.rotation())
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
}

impl HitCandidate {
	pub fn resolved_target(self) -> Entity {
		Health::entity_or_parent(self.entity, self.parent, self.has_health, self.parent_has_health)
	}
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
}

impl HazardHitPass<'_> {
	pub fn collect(self) -> Vec<Hit> {
		collect_hazard_hits(self)
	}
}

fn collect_hazard_hits(pass: HazardHitPass<'_>) -> Vec<Hit> {
	let mut best: std::collections::HashMap<Entity, (f32, Vec3)> = std::collections::HashMap::new();
	for candidate in pass.candidates {
		let target = candidate.resolved_target();
		if !candidate.has_health && !candidate.parent_has_health {
			continue;
		}
		if !pass.friendly_fire.admits(pass.owner, target, pass.allies) {
			continue;
		}
		if !pass.spec.shape.contains(pass.pose, candidate.point) {
			continue;
		}
		let distance = pass.spec.shape.sample_distance(pass.pose, candidate.point);
		match best.get(&target) {
			Some((current, _)) if *current <= distance => {}
			_ => {
				best.insert(target, (distance, candidate.point));
			}
		}
	}

	let mut hits = Vec::new();
	for (target, (distance, point)) in best {
		if !cadence_ready(pass.hit_clock.get(&target).copied(), pass.now, pass.spec.damage.cadence)
		{
			continue;
		}
		let amount = pass.spec.damage.amount_at(distance);
		if amount <= 1e-3 {
			continue;
		}
		pass.hit_clock.insert(target, pass.now);
		hits.push(Hit { target, source: Some(pass.owner), amount, point });
	}
	hits
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
		HitCandidate { entity, parent: None, has_health: true, parent_has_health: false, point }
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

	#[test]
	fn wavefront_advances_along_aim() {
		let snapshot = AimSnapshot { origin: Vec3::ZERO, direction: Vec3::Z, target: None };
		let spec = HazardSpec {
			shape: HazardShape::Wavefront { radius: 1.0, speed: 4.0, length: 10.0 },
			origin: HazardOrigin::AimPoint,
			damage: DamageSpec::authored(),
		};
		let pose = spec.pose(snapshot, 0.5);
		assert!((pose.origin - Vec3::new(0.0, 0.0, 2.0)).length() < 1e-4);
		let capped = spec.pose(snapshot, 8.0);
		assert!((capped.origin - Vec3::new(0.0, 0.0, 10.0)).length() < 1e-4);
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
			},
			HitCandidate {
				entity: b,
				parent: Some(body),
				has_health: false,
				parent_has_health: true,
				point: Vec3::new(0.4, 0.0, 0.0),
			},
		];
		let mut clock = std::collections::HashMap::new();
		let hits = HazardHitPass {
			owner: Entity::from_bits(1),
			friendly_fire: FriendlyFire::ExcludeOwner,
			allies: &[],
			now: 0.0,
			spec,
			pose,
			candidates: &candidates,
			hit_clock: &mut clock,
		}
		.collect();
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
		let first = HazardHitPass {
			owner: Entity::from_bits(1),
			friendly_fire: FriendlyFire::ExcludeOwner,
			allies: &[],
			now: 0.0,
			spec,
			pose,
			candidates: &candidates,
			hit_clock: &mut clock,
		}
		.collect();
		let second = HazardHitPass {
			owner: Entity::from_bits(1),
			friendly_fire: FriendlyFire::ExcludeOwner,
			allies: &[],
			now: 0.25,
			spec,
			pose,
			candidates: &candidates,
			hit_clock: &mut clock,
		}
		.collect();
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
