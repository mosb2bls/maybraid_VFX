//! HP pool and a generic [`Hit`] seam. Weapons stamp [`HitPayload`]; this crate
//! does not know about firearms.

mod apply;
mod attribution;
mod from_projectiles;
mod history;
mod lifecycle;

use bevy::prelude::*;
use projectiles::{tick_flights, ProjectileContact};

pub use apply::apply_hits;
pub use attribution::{DamageActorId, DamageCredit, ResponsibleOwner};
pub use from_projectiles::contacts_to_hits;
pub use history::{
	maintain_damage_history, ClearDamageHistory, DamageHistory, DamageHistoryPolicy, DamageRecord,
	TimeInterval,
};
pub use lifecycle::{
	down_zero_health, ensure_downed_have_despawn, mark_downed, tick_queued_despawns, DespawnAfter,
	Downed, DEFAULT_CORPSE_SECS,
};

/// Default DPC used when a bolt is spawned without catalog stats.
pub const DEFAULT_HIT: f32 = 25.0;

/// Baseline max HP before clothing modifiers.
pub const DEFAULT_MAX_HEALTH: f32 = 100.0;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageSystems {
	Collect,
	Apply,
	History,
	Down,
}

/// Current / max hit points. Missing means the entity cannot be hurt this way.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Health {
	pub current: f32,
	pub max: f32,
}

impl Default for Health {
	fn default() -> Self {
		Self::from_max(DEFAULT_MAX_HEALTH)
	}
}

impl Health {
	pub fn from_max(max: f32) -> Self {
		let max = if max.is_finite() { max.max(1.0) } else { 1.0 };
		Self { current: max, max }
	}

	pub fn apply_damage(&mut self, damage: f32) {
		self.take_damage(damage);
	}

	/// Subtract damage and return the HP actually lost. Overkill is clamped.
	pub fn take_damage(&mut self, damage: f32) -> f32 {
		let before = if self.current.is_finite() { self.current.max(0.0) } else { 0.0 };
		self.current = (before - if damage.is_finite() { damage.max(0.0) } else { 0.0 }).max(0.0);
		before - self.current
	}

	/// Restore HP and return the amount actually gained. Never exceeds [`Self::max`].
	pub fn heal(&mut self, amount: f32) -> f32 {
		let before = if self.current.is_finite() { self.current.max(0.0) } else { 0.0 };
		let max = if self.max.is_finite() { self.max.max(0.0) } else { 0.0 };
		let gain = if amount.is_finite() { amount.max(0.0) } else { 0.0 };
		self.current = (before + gain).min(max);
		self.current - before
	}

	/// Restore HP toward `cap` without lowering current health.
	///
	/// A body already at or above `cap` gains nothing. `cap` is itself
	/// clamped to [`Self::max`].
	pub fn heal_toward(&mut self, amount: f32, cap: f32) -> f32 {
		let max = if self.max.is_finite() { self.max.max(0.0) } else { 0.0 };
		let cap = if cap.is_finite() { cap.clamp(0.0, max) } else { 0.0 };
		let before = if self.current.is_finite() { self.current.max(0.0) } else { 0.0 };
		if before >= cap {
			return 0.0;
		}
		self.heal(amount.min(cap - before))
	}

	pub fn is_dead(self) -> bool {
		self.current <= 0.0
	}

	/// Live hit-tests can land on a query-only child collider. One parent hop.
	pub fn entity_or_parent(
		entity: Entity,
		parent: Option<Entity>,
		self_has_health: bool,
		parent_has_health: bool,
	) -> Entity {
		if self_has_health {
			entity
		} else if parent_has_health {
			parent.unwrap_or(entity)
		} else {
			entity
		}
	}

	pub fn fraction(self) -> f32 {
		if !self.max.is_finite() || self.max <= 0.0 {
			0.0
		} else {
			let fraction = self.current / self.max;
			if fraction.is_finite() {
				fraction.clamp(0.0, 1.0)
			} else {
				0.0
			}
		}
	}
}

/// Local-Y band that scales incoming [`Hit`]s. Missing means no headshot bonus.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HeadshotBand {
	pub min_local_y: f32,
	pub multiplier: f32,
}

impl HeadshotBand {
	pub fn contains(self, target: &GlobalTransform, point: Vec3) -> bool {
		let local = target.affine().inverse().transform_point3(point);
		local.y >= self.min_local_y
	}

	pub fn scale(self, target: &GlobalTransform, point: Vec3, amount: f32) -> f32 {
		if self.contains(target, point) {
			amount * self.multiplier
		} else {
			amount
		}
	}
}

/// Amount copied onto a projectile (or read off a laser) at fire time.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HitPayload {
	pub amount: f32,
}

impl Default for HitPayload {
	fn default() -> Self {
		Self { amount: DEFAULT_HIT }
	}
}

/// Requested injury. Any producer may write this.
#[derive(Message, Clone, Copy, Debug)]
pub struct Hit {
	pub target: Entity,
	pub source: Option<Entity>,
	pub amount: f32,
	pub point: Vec3,
	/// Own-weapon volumes stay ignored. Blast / other authored self-hits set this.
	pub allow_self: bool,
}

/// One accepted apply. [`Self::amount`] is the requested/scaled injury;
/// [`Self::applied`] is the HP actually lost after clamping to remaining
/// health. Zero-effect hits keep `applied == 0` so they stay distinguishable
/// from overkill (`applied > 0 && applied < amount`).
#[derive(Message, Clone, Copy, Debug)]
pub struct DamageApplied {
	pub target: Entity,
	pub source: Option<Entity>,
	pub amount: f32,
	pub applied: f32,
	pub remaining: f32,
	pub point: Vec3,
}

impl DamageApplied {
	pub fn is_zero_effect(self) -> bool {
		!self.applied.is_finite() || self.applied <= 0.0
	}

	pub fn is_overkill(self) -> bool {
		self.applied.is_finite()
			&& self.amount.is_finite()
			&& self.applied > 0.0
			&& self.applied < self.amount
	}
}

/// `Health` crossed zero this apply.
#[derive(Message, Clone, Copy, Debug)]
pub struct Died {
	pub entity: Entity,
	pub source: Option<Entity>,
	pub point: Vec3,
}

pub struct DamagePlugin;

impl Plugin for DamagePlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<Hit>()
			.add_message::<DamageApplied>()
			.add_message::<Died>()
			.add_message::<ClearDamageHistory>()
			.add_message::<ProjectileContact>()
			.init_resource::<DamageHistory>()
			.init_resource::<DamageHistoryPolicy>()
			.configure_sets(
				PostUpdate,
				(
					DamageSystems::Collect.after(tick_flights),
					DamageSystems::Apply.after(DamageSystems::Collect),
					DamageSystems::History.after(DamageSystems::Apply),
					DamageSystems::Down.after(DamageSystems::History),
				),
			)
			.add_systems(
				PostUpdate,
				(
					contacts_to_hits.in_set(DamageSystems::Collect),
					apply_hits.in_set(DamageSystems::Apply),
					maintain_damage_history.in_set(DamageSystems::History),
				),
			)
			.add_systems(
				PostUpdate,
				(mark_downed, down_zero_health, ensure_downed_have_despawn)
					.chain()
					.in_set(DamageSystems::Down),
			)
			.add_systems(Last, tick_queued_despawns);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn damage_clamps_at_zero() {
		let mut health = Health::default();
		assert_eq!(health.take_damage(25.0), 25.0);
		assert_eq!(health.current, 75.0);
		assert_eq!(health.take_damage(100.0), 75.0);
		assert_eq!(health.current, 0.0);
		assert!(health.is_dead());
		assert_eq!(health.take_damage(10.0), 0.0);
	}

	#[test]
	fn heal_clamps_at_max_and_heal_toward_never_reduces() {
		let mut health = Health::from_max(100.0);
		health.take_damage(40.0);
		assert_eq!(health.heal(15.0), 15.0);
		assert_eq!(health.current, 75.0);
		assert_eq!(health.heal(100.0), 25.0);
		assert_eq!(health.current, 100.0);
		assert_eq!(health.heal_toward(10.0, 70.0), 0.0);
		assert_eq!(health.current, 100.0);
		health.take_damage(50.0);
		assert_eq!(health.heal_toward(20.0, 70.0), 20.0);
		assert_eq!(health.current, 70.0);
		assert_eq!(health.heal_toward(20.0, 70.0), 0.0);
		assert_eq!(health.current, 70.0);
	}

	#[test]
	fn fraction_tracks_remaining() {
		let mut health = Health::default();
		assert_eq!(health.fraction(), 1.0);
		health.apply_damage(50.0);
		assert!((health.fraction() - 0.5).abs() < 1e-5);
		health.apply_damage(50.0);
		assert_eq!(health.fraction(), 0.0);
	}

	#[test]
	fn from_max_floors_at_one() {
		let health = Health::from_max(0.0);
		assert_eq!(health.max, 1.0);
		assert_eq!(health.current, 1.0);
	}

	#[test]
	fn from_max_treats_nan_as_one() {
		let health = Health::from_max(f32::NAN);
		assert_eq!(health.max, 1.0);
		assert_eq!(health.current, 1.0);
		assert_eq!(health.fraction(), 1.0);
	}

	#[test]
	fn headshot_band_scales_only_above_the_cut() {
		let band = HeadshotBand { min_local_y: 0.7, multiplier: 1.25 };
		let target = GlobalTransform::from_translation(Vec3::new(2.0, 1.0, 0.0));
		assert!(band.contains(&target, Vec3::new(2.0, 1.75, 0.0)));
		assert!(!band.contains(&target, Vec3::new(2.0, 1.5, 0.0)));
		assert!((band.scale(&target, Vec3::new(2.0, 1.75, 0.0), 25.0) - 31.25).abs() < 1e-4);
		assert!((band.scale(&target, Vec3::new(2.0, 1.5, 0.0), 25.0) - 25.0).abs() < 1e-4);
	}

	#[test]
	fn health_remounts_child_onto_parent() {
		let body = Entity::from_bits(1);
		let child = Entity::from_bits(2);
		assert_eq!(Health::entity_or_parent(child, Some(body), false, true), body);
		assert_eq!(Health::entity_or_parent(body, None, true, false), body);
		assert_eq!(Health::entity_or_parent(child, Some(body), true, true), child);
		assert_eq!(Health::entity_or_parent(child, Some(body), false, false), child);
	}
}
