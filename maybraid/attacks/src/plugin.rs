use bevy::ecs::schedule::ApplyDeferred;
use bevy::prelude::*;
use damage::Hit;

use crate::{
	cleanup::{apply_cancel_requests, interrupt_unavailable_owners},
	tick::{begin_attacks, tick_attacks, tick_user_cooldowns, AttackInstanceIds},
	AttackCatalog, AttackEnded, AttackPresentationEvent, BeginAttack, CancelAttacks,
};

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackSystems {
	Command,
	Interrupt,
	Advance,
}

/// Gameplay plugin. Does not register world/HUD/input or depend on VFX.
pub struct AttacksPlugin;

impl Plugin for AttacksPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<AttackCatalog>()
			.init_resource::<AttackInstanceIds>()
			.add_message::<BeginAttack>()
			.add_message::<CancelAttacks>()
			.add_message::<AttackEnded>()
			.add_message::<AttackPresentationEvent>()
			.add_message::<Hit>()
			.configure_sets(
				Update,
				(AttackSystems::Command, AttackSystems::Interrupt, AttackSystems::Advance).chain(),
			)
			.add_systems(
				Update,
				(
					(tick_user_cooldowns, begin_attacks).in_set(AttackSystems::Command),
					ApplyDeferred,
					(apply_cancel_requests, interrupt_unavailable_owners)
						.in_set(AttackSystems::Interrupt),
					ApplyDeferred,
					tick_attacks.in_set(AttackSystems::Advance),
				)
					.chain(),
			);
	}
}
