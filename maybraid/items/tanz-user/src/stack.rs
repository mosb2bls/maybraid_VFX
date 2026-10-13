//! Tanz definitions, input regions, and per-player equipment.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use characters::AnimClip;

use crate::actions::{forward_dash, healing_salute, left_strafe, right_strafe};
use crate::input::{TanzButton, TanzInput};

/// Starter Tanz identities. Priority is equipment order, not this enum order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TanzId {
	HealingSalute,
	LeftStrafe,
	RightStrafe,
	ForwardDash,
}

impl TanzId {
	pub const STARTER: [Self; 4] =
		[Self::HealingSalute, Self::LeftStrafe, Self::RightStrafe, Self::ForwardDash];

	pub fn definition(self) -> TanzDefinition {
		match self {
			Self::HealingSalute => healing_salute(),
			Self::LeftStrafe => left_strafe(),
			Self::RightStrafe => right_strafe(),
			Self::ForwardDash => forward_dash(),
		}
	}

	pub fn script_id(self) -> &'static str {
		match self {
			Self::HealingSalute => "healing_salute",
			Self::LeftStrafe => "left_strafe",
			Self::RightStrafe => "right_strafe",
			Self::ForwardDash => "forward_dash",
		}
	}

	pub fn from_script_id(raw: &str) -> Option<Self> {
		match raw.trim().to_ascii_lowercase().as_str() {
			"healing_salute" | "salute" => Some(Self::HealingSalute),
			"left_strafe" | "left" => Some(Self::LeftStrafe),
			"right_strafe" | "right" => Some(Self::RightStrafe),
			"forward_dash" | "dash" | "forward" => Some(Self::ForwardDash),
			_ => None,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct TanzInputRange {
	pub direction: Vec2,
	pub half_angle: f32,
	pub magnitude: RangeInclusive<f32>,
	pub min_coherence: f32,
	pub button: TanzButton,
}

impl TanzInputRange {
	pub fn matches(&self, input: &TanzInput) -> bool {
		if self.button != input.button {
			return false;
		}
		if !self.magnitude.contains(&input.magnitude) {
			return false;
		}
		if input.coherence < self.min_coherence {
			return false;
		}
		if self.direction.length_squared() <= 1e-8 {
			return input.direction.length_squared() <= 1e-8;
		}
		if input.direction.length_squared() <= 1e-8 {
			return false;
		}
		angle_between(self.direction, input.direction) <= self.half_angle
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TanzTiming {
	pub startup: f32,
	pub active: f32,
	pub recovery: f32,
	pub cooldown: f32,
	pub chain_window: f32,
	pub buffer_ttl: f32,
}

impl TanzTiming {
	pub fn total(self) -> f32 {
		self.startup + self.active + self.recovery
	}

	pub fn startup_end(self) -> f32 {
		self.startup
	}

	pub fn active_end(self) -> f32 {
		self.startup + self.active
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct TanzDefinition {
	pub id: TanzId,
	pub input: TanzInputRange,
	pub action: crate::TanzAction,
	pub timing: TanzTiming,
	pub clip: AnimClip,
}

/// Equipped Tanzes, highest priority first.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct TanzStack {
	pub equipped: Vec<TanzId>,
}

impl TanzStack {
	pub fn starter() -> Self {
		Self { equipped: TanzId::STARTER.to_vec() }
	}

	pub fn contains(&self, id: TanzId) -> bool {
		self.equipped.contains(&id)
	}

	pub fn equip(&mut self, id: TanzId) {
		if !self.contains(id) {
			self.equipped.push(id);
		}
	}

	pub fn unequip(&mut self, id: TanzId) {
		self.equipped.retain(|equipped| *equipped != id);
	}

	pub fn reorder(&mut self, id: TanzId, index: usize) {
		let Some(from) = self.equipped.iter().position(|equipped| *equipped == id) else {
			return;
		};
		let item = self.equipped.remove(from);
		self.equipped.insert(index.min(self.equipped.len()), item);
	}

	/// First equipped Tanz whose input region matches and is not cooling down.
	pub fn match_input(
		&self,
		input: &TanzInput,
		cooling: impl Fn(TanzId) -> bool,
	) -> Option<TanzId> {
		self.equipped
			.iter()
			.copied()
			.find(|id| !cooling(*id) && id.definition().input.matches(input))
	}
}

fn angle_between(a: Vec2, b: Vec2) -> f32 {
	if a.length_squared() <= 1e-8 || b.length_squared() <= 1e-8 {
		return std::f32::consts::PI;
	}
	a.normalize().dot(b.normalize()).clamp(-1.0, 1.0).acos()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::input::TANZ_MIN_COHERENCE;

	fn input(direction: Vec2, magnitude: f32, button: TanzButton) -> TanzInput {
		TanzInput { user: Entity::from_bits(1), direction, magnitude, coherence: 1.0, button }
	}

	#[test]
	fn starter_priority_prefers_salute_over_dash() -> anyhow::Result<()> {
		assert_eq!(TanzId::from_script_id("left_strafe"), Some(TanzId::LeftStrafe));
		assert_eq!(TanzId::from_script_id("DASH"), Some(TanzId::ForwardDash));
		assert_eq!(TanzId::LeftStrafe.script_id(), "left_strafe");
		assert!(TanzId::from_script_id("spin").is_none());
		let stack = TanzStack::starter();
		let hit = stack.match_input(&input(Vec2::Y, 0.9, TanzButton::APress), |_| false);
		assert_eq!(hit, Some(TanzId::HealingSalute));
		let dash = stack.match_input(&input(Vec2::Y, 0.9, TanzButton::None), |_| false);
		assert_eq!(dash, Some(TanzId::ForwardDash));
		Ok(())
	}

	#[test]
	fn cooldown_falls_through_to_a_lower_priority_match() -> anyhow::Result<()> {
		let mut stack = TanzStack::starter();
		stack.reorder(TanzId::ForwardDash, 0);
		stack.reorder(TanzId::HealingSalute, 1);
		let cooling = |id: TanzId| id == TanzId::ForwardDash;
		let hit = stack.match_input(&input(Vec2::Y, 0.9, TanzButton::None), cooling);
		assert_eq!(hit, None);
		let left = stack.match_input(&input(Vec2::NEG_X, 0.9, TanzButton::None), cooling);
		assert_eq!(left, Some(TanzId::LeftStrafe));
		Ok(())
	}

	#[test]
	fn equip_unequip_and_reorder() -> anyhow::Result<()> {
		let mut stack = TanzStack { equipped: vec![TanzId::LeftStrafe] };
		stack.equip(TanzId::RightStrafe);
		stack.equip(TanzId::LeftStrafe);
		assert_eq!(stack.equipped, vec![TanzId::LeftStrafe, TanzId::RightStrafe]);
		stack.reorder(TanzId::RightStrafe, 0);
		assert_eq!(stack.equipped, vec![TanzId::RightStrafe, TanzId::LeftStrafe]);
		stack.unequip(TanzId::RightStrafe);
		assert_eq!(stack.equipped, vec![TanzId::LeftStrafe]);
		Ok(())
	}

	#[test]
	fn unmatched_input_selects_nothing() -> anyhow::Result<()> {
		let stack = TanzStack::starter();
		assert!(stack
			.match_input(&input(Vec2::ZERO, 0.0, TanzButton::APress), |_| false)
			.is_none());
		assert!(stack
			.match_input(
				&TanzInput {
					user: Entity::from_bits(1),
					direction: Vec2::NEG_X,
					magnitude: 0.9,
					coherence: TANZ_MIN_COHERENCE - 0.2,
					button: TanzButton::None,
				},
				|_| false,
			)
			.is_none());
		Ok(())
	}

	#[test]
	fn stacks_are_independent() -> anyhow::Result<()> {
		let left = TanzStack { equipped: vec![TanzId::LeftStrafe] };
		let right = TanzStack { equipped: vec![TanzId::RightStrafe] };
		assert_eq!(
			left.match_input(&input(Vec2::NEG_X, 0.9, TanzButton::None), |_| false),
			Some(TanzId::LeftStrafe)
		);
		assert!(right
			.match_input(&input(Vec2::NEG_X, 0.9, TanzButton::None), |_| false)
			.is_none());
		Ok(())
	}
}
