//! character-specific typed character menus.

pub mod character;
pub mod characters;
pub mod event;
pub mod focus;
pub mod inventory_layout;
pub mod section_open;
pub mod shared;

pub use character::{CharacterMenu, ConceptSpecies, SectionOpenState, CHARACTER_NAME_MAX_LEN};
pub use characters::{
	braidman::BraidmanMenu, brenal::BrenalMenu, brodler::BrodlerMenu, brokker::BrokkerMenu,
	caole::CaoleMenu, chupri::ChupriMenu, claber::ClaberMenu, croconot::CroconotMenu, dui::DuiMenu,
	epiphant::EpiphantMenu, grener::GrenerMenu, hars::HarsMenu, kaller::KallerMenu,
	kappler::KapplerMenu, kispar::KisparMenu, lero::LeroMenu, lidder::LidderMenu,
	mistler::MistlerMenu, mygr::MygrMenu, sonyak::SonyakMenu, spibmom::SpibmomMenu, tapp::TappMenu,
	thumplus::ThumplusMenu, tipple::TippleMenu, topple::ToppleMenu, tuberwaber::TuberwaberMenu,
	wumbus::WumbusMenu, ylter::YilterMenu,
};
pub use event::{AssetValue, CharacterField, MenuEvent, SectionId, SwatchValue};
pub use focus::{
	spin_reveal_firearm_focus, spin_reveal_focus, spin_reveal_grenade_focus, BODY_FOCUS,
	SPIN_REVEAL_FOCUS,
};
pub use inventory_layout::{
	InventoryLayout, InventoryLayoutMode, INVENTORY_LABEL, LUBIKAN_LABEL, STARTER_KIT_LABEL,
	TOOLS_LABEL,
};

pub use character_ui_menu::{AssetOption, LabelOption, ListValues, StringIdentified, SwatchOption};

pub(crate) fn cycle_value<T: ListValues>(value: T, delta: i32) -> T {
	let values = T::values();
	let current = values.iter().position(|candidate| *candidate == value).unwrap_or(0);
	let next = (current as i32 + delta).rem_euclid(values.len() as i32) as usize;
	values[next]
}

#[cfg(test)]
mod tests;
