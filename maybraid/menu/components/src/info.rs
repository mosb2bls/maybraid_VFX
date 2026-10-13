//! Non-interactive copy that sits with a menu.

pub mod brand;
pub mod description;
pub mod hint;
pub mod info_box;
pub mod objective;
pub mod text_card;

pub use brand::{set_brand_mode_title, BrandModeCorner, BrandModeLine, BrandModeTitle, BRAND_NAME};
pub use description::{set_description_for_menu, TextMenuDescription};
pub use hint::{set_hint_for_menu, TextMenuHint, TextMenuHintLabel};
pub use info_box::{set_info_box_for_menu, MenuInfoBox, MenuInfoBoxLabel};
pub use objective::{spawn_menu_objective, MenuObjective};
pub use text_card::{
	spawn_hud_text_card, spawn_hud_text_card_label, HudTextCard, HUD_TEXT_CARD_FACE_PX,
};
