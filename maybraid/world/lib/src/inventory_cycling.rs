//! World gameplay gates for equipment selection. Category and queue ownership
//! live in `maybraid-equipment`. Cross-domain schedule constraints live in
//! `maybraid-gameplay`.

use game_commands::command::TextEntryFocus;

use crate::control::WorldGameplayEnabled;
use crate::map_view::WorldMapView;

pub use maybraid_equipment::ActiveInventoryCyclingCategory;

pub(crate) fn gameplay_cycling_allowed(
	gameplay: &WorldGameplayEnabled,
	text_focus: &TextEntryFocus,
	map: Option<&WorldMapView>,
) -> bool {
	gameplay.accepts_character_input(text_focus.0, map.is_some_and(|map| map.open))
}
