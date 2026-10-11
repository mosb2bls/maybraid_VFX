//! [`ProjectileContact`] → [`Hit`] using the payload stamped on the flight.

use bevy::prelude::*;
use projectiles::ProjectileContact;

use crate::{Hit, HitPayload};

pub fn contacts_to_hits(
	mut contacts: MessageReader<ProjectileContact>,
	payloads: Query<&HitPayload>,
	mut hits: MessageWriter<Hit>,
) {
	for contact in contacts.read() {
		let Ok(payload) = payloads.get(contact.projectile) else {
			continue;
		};
		hits.write(Hit {
			target: contact.target,
			source: contact.source,
			amount: payload.amount,
			point: contact.point,
			allow_self: false,
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn missing_payload_is_not_a_hit() {
		assert_eq!(HitPayload::default().amount, crate::DEFAULT_HIT);
	}
}
