//! Select declared contributions into [`ResolvedMotorRequest`].

use bevy::prelude::*;

use super::contract::{MotorIntent, MotorSelection, ResolvedMotorRequest};

/// Select this frame's contributions into [`ResolvedMotorRequest`].
pub fn compose_motor(
	mut bodies: Query<(&MotorIntent, &mut MotorSelection, &mut ResolvedMotorRequest)>,
) {
	for (intent, mut selection, mut resolved) in &mut bodies {
		let next = intent.selected_base().map(|(contributor, _)| contributor);
		*resolved = intent.compose(selection.base());
		selection.remember(if intent.suppressed() { None } else { next });
	}
}
