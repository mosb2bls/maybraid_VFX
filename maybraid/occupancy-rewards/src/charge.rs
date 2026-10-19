//! Thin occupancy → [`LubikanChargeRequest`] adapter.
//!
//! [#1108](https://github.com/ramate-io/maybraid/issues/1108) /
//! [PR #1154](https://github.com/ramate-io/maybraid/pull/1154) owns apply:
//! occupancy writes one request; [`lubikans::LubikanSystems::Award`] applies
//! it once and emits one [`lubikans::LubikanChargeChanged`].

use lubikans::{LubikanChargeReason, LubikanChargeRequest, LubikanSelection};

use crate::occupancy::OccupancyAward;

/// Map one occupancy award onto the shared charge request.
///
/// `selection` is snapshotted at award time (`LubikanSelection::resolved`,
/// Self when nothing external is equipped).
pub fn adapt_award(award: &OccupancyAward, selection: LubikanSelection) -> LubikanChargeRequest {
	LubikanChargeRequest::new(
		award.recipient,
		Some(selection.resolved()),
		award.amount,
		LubikanChargeReason::OCCUPANCY,
		Some(award.origin),
	)
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{Entity, Vec3};

	use super::*;
	use crate::identity::{GroveTileId, OccupancyMatchKey};
	use crate::rule::RuleId;
	use lubikans::LubikanAccountId;

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
		assert_eq!(request.lubikan, Some(LubikanAccountId::SELF));
		assert_eq!(request.requested, 3.0);
		assert_eq!(request.reason, LubikanChargeReason::OCCUPANCY);
		assert_eq!(request.origin, Some(award.origin));
		Ok(())
	}
}
