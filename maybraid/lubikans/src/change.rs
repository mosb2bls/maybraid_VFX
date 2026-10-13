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
			1 => "hit",
			2 => "kill",
			10 => "occupancy",
			20 => "activation",
			30 => "adjustment",
			_ => "custom",
		}
	}
}

/// Ask the Award-set owner to apply a signed charge delta.
///
/// Occupancy, combat, and later producers write this. Exactly one system
/// applies it and emits one [`LubikanChargeChanged`]. `lubikan: None` is the
/// recipient's current selection (Self fallback).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct LubikanChargeRequest {
	pub recipient: Entity,
	pub lubikan: Option<LubikanAccountId>,
	pub requested: f32,
	pub reason: LubikanChargeReason,
	pub origin: Option<Vec3>,
}

impl LubikanChargeRequest {
	pub fn new(
		recipient: Entity,
		lubikan: Option<LubikanAccountId>,
		requested: f32,
		reason: LubikanChargeReason,
		origin: Option<Vec3>,
	) -> Self {
		Self { recipient, lubikan, requested, reason, origin }
	}

	pub fn equipped(
		recipient: Entity,
		requested: f32,
		reason: LubikanChargeReason,
		origin: Option<Vec3>,
	) -> Self {
		Self::new(recipient, None, requested, reason, origin)
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
	/// Signed applied amount after the stock cap. Zero is a valid clamped result.
	pub amount: f32,
	/// Signed points produced by the interaction, before overflow was discarded.
	///
	/// The point feed reads this so earned points still appear when charge is
	/// already at the stock cap (`amount` may be `0`).
	pub points_earned: f32,
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
		Self { recipient, lubikan, amount, points_earned: amount, reason, origin }
	}

	pub fn with_points_earned(self, points_earned: f32) -> Self {
		Self { points_earned, ..self }
	}

	/// Charge that actually landed after the stock cap. Same as [`Self::amount`].
	pub fn charge_applied(self) -> f32 {
		self.amount
	}

	/// Positive earn that added no charge because the stock cap was already full.
	pub fn discarded_at_stock_cap(self) -> bool {
		self.points_earned > 0.0 && self.amount == 0.0
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
		assert_eq!(LubikanChargeReason::COMBAT_HIT.label(), "hit");
		assert_eq!(LubikanChargeReason::COMBAT_KILL.label(), "kill");
		assert_eq!(LubikanChargeReason::from_raw(4_001).label(), "custom");
	}

	#[test]
	fn new_defaults_points_earned_to_applied_amount() {
		let change = LubikanChargeChanged::new(
			Entity::from_bits(1),
			LubikanAccountId::SELF,
			4.0,
			LubikanChargeReason::COMBAT_HIT,
			None,
		);
		assert_eq!(change.points_earned, 4.0);
		assert_eq!(change.charge_applied(), 4.0);
		assert!(!change.discarded_at_stock_cap());
		let capped = change.with_points_earned(9.0);
		assert_eq!(capped.amount, 4.0);
		assert_eq!(capped.points_earned, 9.0);
		let full = LubikanChargeChanged::new(
			Entity::from_bits(1),
			LubikanAccountId::SELF,
			0.0,
			LubikanChargeReason::COMBAT_HIT,
			None,
		)
		.with_points_earned(3.0);
		assert!(full.discarded_at_stock_cap());
	}
}
