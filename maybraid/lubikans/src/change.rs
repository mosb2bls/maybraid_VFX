//! Authoritative charge-change result consumed by HUD and late readers.

use bevy::prelude::*;

use crate::identity::LubikanAccountId;

/// Why a [`LubikanChargeChanged`] was emitted.
///
/// New well-known codes can be added as associated constants without breaking
/// consumers that match on the raw value. Content crates should pick codes
/// at or above [`Self::CUSTOM`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LubikanChargeReason(pub u16);

impl LubikanChargeReason {
	/// Non-lethal combat credit.
	pub const COMBAT_HIT: Self = Self(1);
	/// Lethal combat credit. Amount includes a stacked hit weight when the
	/// equipped policy stacks hit and kill.
	pub const COMBAT_KILL: Self = Self(2);
	/// Semantic cell occupancy (#1111).
	pub const OCCUPANCY: Self = Self(10);
	/// Activation spend (#1110).
	pub const ACTIVATION: Self = Self(20);
	/// Manual, debug, or test adjustment.
	pub const ADJUSTMENT: Self = Self(30);
	/// First code reserved for other crates.
	pub const CUSTOM: Self = Self(1_000);

	pub const fn from_raw(raw: u16) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u16 {
		self.0
	}

	pub const fn label(self) -> &'static str {
		match self.0 {
			1 => "combat-hit",
			2 => "combat-kill",
			10 => "occupancy",
			20 => "activation",
			30 => "adjustment",
			_ => "custom",
		}
	}
}

/// One resolved award or spend. Emit exactly once per interaction; UI does
/// not re-evaluate rules from this message.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct LubikanChargeChanged {
	/// Owner who received or spent the charge.
	pub recipient: Entity,
	/// Account credited or debited at the moment the event resolved.
	pub lubikan: LubikanAccountId,
	/// Signed applied amount after clamping. Zero is a valid clamped result.
	pub amount: f32,
	pub reason: LubikanChargeReason,
	/// World-space origin of the interaction, when the producer has one.
	pub origin: Option<Vec3>,
}

impl LubikanChargeChanged {
	pub fn new(
		recipient: Entity,
		lubikan: LubikanAccountId,
		amount: f32,
		reason: LubikanChargeReason,
		origin: Option<Vec3>,
	) -> Self {
		Self { recipient, lubikan, amount, reason, origin }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn well_known_reasons_keep_stable_codes() {
		assert_eq!(LubikanChargeReason::COMBAT_HIT.raw(), 1);
		assert_eq!(LubikanChargeReason::COMBAT_KILL.raw(), 2);
		assert_eq!(LubikanChargeReason::OCCUPANCY.raw(), 10);
		assert_eq!(LubikanChargeReason::ACTIVATION.raw(), 20);
		assert_eq!(LubikanChargeReason::CUSTOM.raw(), 1_000);
		assert_eq!(LubikanChargeReason::COMBAT_KILL.label(), "combat-kill");
		assert_eq!(LubikanChargeReason::from_raw(4_001).label(), "custom");
	}
}
