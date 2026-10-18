//! Gameplay intents derived from [`maybraid_input::VirtualPad`].

use bevy::prelude::*;

/// One frame of character control. Analog variants may repeat while held.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum CharacterIntent {
	Move(Vec2),
	Look(Vec2),
	Focus(f32),
	/// Cheek-weld ADS with iron-sight FOV. Left bumper / middle mouse.
	Ads(f32),
	/// Unused hold chord. Discover maps now read stick flicks, not bumpers.
	SkillMap,
	UseItem(f32),
	StartSprint,
	StopSprint,
	SwapPov,
	Jump,
	/// Tap **B** (release before the hold threshold). Toggles stand ↔ squat; prone steps to squat.
	ChangeSquat,
	/// Hold **B** across the stance threshold. Enters prone from stand or squat.
	ChangeProne,
	ExitInteraction,
	StartInteraction,
	/// Cycle the equipped queue for the active inventory category (pad **Y** / keyboard **Y**).
	SwapActive,
	/// Cycle the equipped queue for the active category. **[** / **]** keyboard equivalents.
	/// Negative steps backward; non-negative advances.
	CycleEquippedQueue(i8),
	/// Select which equipped queue **Y** cycles. Pad **D-Pad** left / right / down.
	SelectInventoryCyclingCategory(InventoryCyclingCategory),
	InGameMenu,
	Inventory,
	PowerUseItem,
	/// Toggle the world overhead map. Keyboard **M** / Xbox **View** (pad Select).
	ToggleMap,
}

/// Which equipped inventory queue responds to **Y** / **[** / **]**.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InventoryCyclingCategory {
	#[default]
	Weapons,
	SkillMaps,
	Lubikans,
}

impl InventoryCyclingCategory {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Weapons => "Weapons",
			Self::SkillMaps => "Skill Maps",
			Self::Lubikans => "Lubikans",
		}
	}
}

impl CharacterIntent {
	pub fn label(self) -> &'static str {
		match self {
			Self::Move(_) => "move",
			Self::Look(_) => "look",
			Self::Focus(_) => "focus",
			Self::Ads(_) => "ads",
			Self::SkillMap => "skill-map",
			Self::UseItem(_) => "use-item",
			Self::StartSprint => "start-sprint",
			Self::StopSprint => "stop-sprint",
			Self::SwapPov => "swap-pov",
			Self::Jump => "jump",
			Self::ChangeSquat => "change-squat",
			Self::ChangeProne => "change-prone",
			Self::ExitInteraction => "exit-interaction",
			Self::StartInteraction => "start-interaction",
			Self::SwapActive => "swap-active",
			Self::CycleEquippedQueue(_) => "cycle-equipped-queue",
			Self::SelectInventoryCyclingCategory(_) => "select-inventory-cycling-category",
			Self::InGameMenu => "in-game-menu",
			Self::Inventory => "inventory",
			Self::PowerUseItem => "power-use-item",
			Self::ToggleMap => "toggle-map",
		}
	}
}
