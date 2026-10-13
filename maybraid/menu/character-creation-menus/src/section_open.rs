use character_ui_menu::SectionOpen;

use crate::{event::SectionId, SectionOpenState};

impl SectionOpen for SectionOpenState {
	fn is_open(&self, label: &'static str) -> bool {
		let section = match label {
			"Presets" => SectionId::Presets,
			"Head" => SectionId::Head,
			"Body" => SectionId::Body,
			"Head & Features" => SectionId::HeadFeatures,
			"Hair" => SectionId::Hair,
			"Clothing" => SectionId::Clothing,
			"Weapons" => SectionId::Weapons,
			"Skill Maps" => SectionId::SkillMaps,
			"Loadout" => SectionId::Loadout,
			"Animation" => SectionId::Animation,
			_ => return true,
		};
		SectionOpenState::is_section_open(*self, section)
	}
}
