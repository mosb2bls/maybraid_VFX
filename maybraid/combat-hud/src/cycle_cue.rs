//! D-pad inventory cycling category cue ([#1112](https://github.com/ramate-io/maybraid/issues/1112)).
//!
//! Gameplay sets this resource; the Lubikan charge plate only reads it.

use bevy::prelude::*;

/// Which equipped queue Y will cycle. Upstream inventory code owns updates.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombatHudInventoryCycleCategory {
	#[default]
	Weapons,
	SkillMaps,
	Lubikans,
}

impl CombatHudInventoryCycleCategory {
	pub const fn is_lubikans(self) -> bool {
		matches!(self, Self::Lubikans)
	}
}
