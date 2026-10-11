//! Thin occupancy → charge adapter.
//!
//! Uses the shared [`lubikans`] types from
//! [#1107](https://github.com/ramate-io/maybraid/issues/1107) /
//! [PR #1142](https://github.com/ramate-io/maybraid/pull/1142). Occupancy
//! awards become [`OccupancyChargeRequest`] values in those types: recipient,
//! credited [`LubikanAccountId`], signed `requested` amount,
//! [`LubikanChargeReason::OCCUPANCY`], and optional world origin.
//!
//! Apply a request with [`LubikanLedger::apply_to`] /
//! [`LubikanCharge::apply`] to obtain a [`LubikanChargeMutation`]. Authoritative
//! [`LubikanChargeChanged`] routing is [#1108](https://github.com/ramate-io/maybraid/issues/1108)
//! and is **not** emitted here.

use bevy::prelude::{Entity, Message, Vec3};
use lubikans::{
	LubikanAccountId, LubikanCharge, LubikanChargeChanged, LubikanChargeMutation,
	LubikanChargeReason, LubikanLedger, LubikanSelection,
};

use crate::occupancy::OccupancyAward;

/// Occupancy award as a signed charge-mutation request in #1107 types.
///
/// This is the request [`LubikanCharge::apply`] / [`LubikanLedger::apply_to`]
/// accept. #1108 will consume these and emit [`LubikanChargeChanged`] exactly
/// once.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct OccupancyChargeRequest {
	pub recipient: Entity,
	pub lubikan: LubikanAccountId,
	pub requested: f32,
	pub reason: LubikanChargeReason,
	pub origin: Option<Vec3>,
}

impl OccupancyChargeRequest {
	pub fn apply_to_charge(self, charge: &mut LubikanCharge) -> LubikanChargeMutation {
		charge.apply(self.requested)
	}

	pub fn apply_to_ledger(self, ledger: &mut LubikanLedger) -> Option<LubikanChargeMutation> {
		ledger.apply_to(self.lubikan, self.requested)
	}

	/// Build the authoritative result #1108 will emit after applying `mutation`.
	pub fn into_changed(self, mutation: LubikanChargeMutation) -> LubikanChargeChanged {
		LubikanChargeChanged::new(
			self.recipient,
			self.lubikan,
			mutation.applied,
			self.reason,
			self.origin,
		)
	}
}

/// Map one occupancy award onto a shared-type mutation request.
///
/// `selection` is snapshotted at award time (`LubikanSelection::resolved`,
/// Self when nothing external is equipped).
pub fn adapt_award(award: &OccupancyAward, selection: LubikanSelection) -> OccupancyChargeRequest {
	OccupancyChargeRequest {
		recipient: award.recipient,
		lubikan: selection.resolved(),
		requested: award.amount,
		reason: LubikanChargeReason::OCCUPANCY,
		origin: Some(award.origin),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::identity::{GroveTileId, OccupancyMatchKey};
	use crate::rule::RuleId;
	use lubikans::LubikanSelfConfig;

	#[test]
	fn adapter_uses_shared_account_reason_and_origin() -> anyhow::Result<()> {
		let award = OccupancyAward {
			recipient: Entity::from_bits(7),
			rule: RuleId::SELF_DEMONSTRATION,
			cell: OccupancyMatchKey::Grove(GroveTileId { ix: 8, iz: 8 }),
			amount: 3.0,
			origin: Vec3::new(1.0, 2.0, 3.0),
		};
		let request = adapt_award(&award, LubikanSelection::default());
		assert_eq!(request.recipient, award.recipient);
		assert_eq!(request.lubikan, LubikanAccountId::SELF);
		assert_eq!(request.requested, 3.0);
		assert_eq!(request.reason, LubikanChargeReason::OCCUPANCY);
		assert_eq!(request.origin, Some(award.origin));
		Ok(())
	}

	#[test]
	fn request_apply_produces_shared_mutation() -> anyhow::Result<()> {
		let award = OccupancyAward {
			recipient: Entity::from_bits(8),
			rule: RuleId::SELF_DEMONSTRATION,
			cell: OccupancyMatchKey::Grove(GroveTileId { ix: 8, iz: 8 }),
			amount: 4.0,
			origin: Vec3::ZERO,
		};
		let request = adapt_award(&award, LubikanSelection::unequipped());
		let mut ledger = LubikanLedger::with_self(&LubikanSelfConfig::default());
		let mutation = request
			.apply_to_ledger(&mut ledger)
			.ok_or_else(|| anyhow::anyhow!("Self account must accept the apply"))?;
		assert_eq!(mutation.requested, 4.0);
		assert_eq!(mutation.applied, 4.0);
		assert_eq!(mutation.new_value, 4.0);
		assert!(!mutation.clamped_floor);
		assert!(!mutation.clamped_capacity);
		let changed = request.into_changed(mutation);
		assert_eq!(changed.recipient, award.recipient);
		assert_eq!(changed.lubikan, LubikanAccountId::SELF);
		assert_eq!(changed.amount, 4.0);
		assert_eq!(changed.reason, LubikanChargeReason::OCCUPANCY);
		Ok(())
	}
}
