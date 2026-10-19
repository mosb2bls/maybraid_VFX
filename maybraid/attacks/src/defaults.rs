use crate::{DamageSpec, FriendlyFire, InterruptionPolicy};

/// Explicit authored defaults for reusable attacks.
///
/// Encounter and summoned owners copy these unless they author their own
/// numbers. Nothing in the runtime invents a fallback when a field is omitted:
/// [`AttackDefinition`](crate::AttackDefinition) requires the policy fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackDefaults {
	pub damage: f32,
	pub cadence: f32,
	pub cooldown: f32,
	pub interruption: InterruptionPolicy,
	pub friendly_fire: FriendlyFire,
}

impl AttackDefaults {
	/// Shared published defaults. Damage is below firearm [`damage::DEFAULT_HIT`]
	/// so a standing hazard is serious but not an instant dump; cadence is a
	/// half-second pulse; cooldown leaves space between authored casts;
	/// interruption tears down on cancel, death, sleep, and retirement;
	/// friendly fire never includes the owner or listed allies.
	pub const AUTHORED: Self = Self {
		damage: 20.0,
		cadence: 0.5,
		cooldown: 8.0,
		interruption: InterruptionPolicy::AUTHORED,
		friendly_fire: FriendlyFire::ExcludeOwnerAndAllies,
	};

	pub const fn damage_spec(self) -> DamageSpec {
		DamageSpec {
			amount: self.damage,
			cadence: self.cadence,
			falloff: crate::DamageFalloff::None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn authored_defaults_are_explicit() {
		let defaults = AttackDefaults::AUTHORED;
		assert_eq!(defaults.damage, 20.0);
		assert_eq!(defaults.cadence, 0.5);
		assert_eq!(defaults.cooldown, 8.0);
		assert_eq!(defaults.interruption, InterruptionPolicy::AUTHORED);
		assert_eq!(defaults.friendly_fire, FriendlyFire::ExcludeOwnerAndAllies);
		assert!(defaults.interruption.downed);
		assert!(defaults.interruption.suspended);
		assert!(defaults.interruption.retired);
		assert!(defaults.interruption.cancel);
	}
}
