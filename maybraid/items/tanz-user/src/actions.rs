//! Starter Tanz definitions. Effects compose existing movement / health APIs.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use characters::AnimClip;

use crate::input::{TanzButton, TANZ_MIN_COHERENCE, TANZ_MIN_MAGNITUDE};
use crate::stack::{TanzDefinition, TanzId, TanzInputRange, TanzStack, TanzTiming};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TanzAction {
	Move { local: Vec2 },
	Heal { amount: f32 },
}

pub const HEALING_SALUTE_AMOUNT: f32 = 25.0;

const STRONG: RangeInclusive<f32> = TANZ_MIN_MAGNITUDE..=1.5;
const SECTOR: f32 = 40_f32 * std::f32::consts::PI / 180.0;

const MOVE_TIMING: TanzTiming = TanzTiming {
	startup: 0.04,
	active: 0.22,
	recovery: 0.12,
	cooldown: 0.35,
	chain_window: 0.22,
	buffer_ttl: 0.40,
};

const SALUTE_TIMING: TanzTiming = TanzTiming {
	startup: 0.08,
	active: 0.28,
	recovery: 0.20,
	cooldown: 0.80,
	chain_window: 0.24,
	buffer_ttl: 0.40,
};

fn strong_sector(direction: Vec2, button: TanzButton) -> TanzInputRange {
	TanzInputRange {
		direction,
		half_angle: SECTOR,
		magnitude: STRONG,
		min_coherence: TANZ_MIN_COHERENCE,
		button,
	}
}

pub fn healing_salute() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::HealingSalute,
		input: strong_sector(Vec2::Y, TanzButton::APress),
		action: TanzAction::Heal { amount: HEALING_SALUTE_AMOUNT },
		timing: SALUTE_TIMING,
		clip: AnimClip::jab(),
	}
}

pub fn left_strafe() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::LeftStrafe,
		input: strong_sector(Vec2::NEG_X, TanzButton::None),
		action: TanzAction::Move { local: Vec2::NEG_X },
		timing: MOVE_TIMING,
		clip: AnimClip::leap(),
	}
}

pub fn right_strafe() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::RightStrafe,
		input: strong_sector(Vec2::X, TanzButton::None),
		action: TanzAction::Move { local: Vec2::X },
		timing: MOVE_TIMING,
		clip: AnimClip::leap(),
	}
}

pub fn forward_dash() -> TanzDefinition {
	TanzDefinition {
		id: TanzId::ForwardDash,
		input: strong_sector(Vec2::Y, TanzButton::None),
		action: TanzAction::Move { local: Vec2::Y },
		timing: MOVE_TIMING,
		clip: AnimClip::power_dash(),
	}
}

pub fn starter_stack() -> TanzStack {
	TanzStack::starter()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn salute_uses_jab_and_heal() -> anyhow::Result<()> {
		let salute = healing_salute();
		assert_eq!(salute.clip.id(), AnimClip::jab().id());
		assert_eq!(salute.action, TanzAction::Heal { amount: HEALING_SALUTE_AMOUNT });
		assert_eq!(salute.input.button, TanzButton::APress);
		Ok(())
	}

	#[test]
	fn strafe_tanzes_use_leap() -> anyhow::Result<()> {
		assert_eq!(left_strafe().clip.id(), AnimClip::leap().id());
		assert_eq!(right_strafe().clip.id(), AnimClip::leap().id());
		assert_eq!(left_strafe().action, TanzAction::Move { local: Vec2::NEG_X });
		Ok(())
	}

	#[test]
	fn forward_dash_uses_power_dash() -> anyhow::Result<()> {
		assert_eq!(forward_dash().clip.id(), AnimClip::power_dash().id());
		assert_eq!(forward_dash().action, TanzAction::Move { local: Vec2::Y });
		Ok(())
	}
}
