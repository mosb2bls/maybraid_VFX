//! Kenney input chips for inventory actions. Glyphs follow the live device
//! and the real Select / drop bindings — not hard-coded button letters.

use bevy::prelude::*;

use crate::HudFonts;
use crate::theme::{
	PANEL_GROUP_FONT_SIZE, PANEL_ROW_GAP, PROMPT_KEY_DROP, PROMPT_KEY_SELECT, PROMPT_PAD_DROP,
	PROMPT_PAD_SELECT, TEXT_YELLOW,
};

/// Last device that produced a press. Starts on keyboard so headless shots
/// show key chips until a pad speaks.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputPromptDevice {
	#[default]
	Keyboard,
	Pad,
}

impl InputPromptDevice {
	pub fn select_icon(self) -> &'static str {
		match self {
			Self::Pad => PROMPT_PAD_SELECT,
			Self::Keyboard => PROMPT_KEY_SELECT,
		}
	}

	pub fn drop_icon(self) -> &'static str {
		match self {
			Self::Pad => PROMPT_PAD_DROP,
			Self::Keyboard => PROMPT_KEY_DROP,
		}
	}
}

/// Which inventory action a chip illustrates.
///
/// Select toggles wear / queue, so equip and unequip share one chip.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryPromptKind {
	Equip,
	Drop,
}

impl InventoryPromptKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::Equip => "Equip/Unequip",
			Self::Drop => "Drop",
		}
	}

	pub fn icon(self, device: InputPromptDevice) -> &'static str {
		match self {
			Self::Equip => device.select_icon(),
			Self::Drop => device.drop_icon(),
		}
	}
}

const INVENTORY_PROMPT_KINDS: [InventoryPromptKind; 2] =
	[InventoryPromptKind::Equip, InventoryPromptKind::Drop];

/// Image on an inventory action chip so a device change can swap the glyph.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryPromptGlyph(pub InventoryPromptKind);

/// Footer row under the interactive inventory scroll pane.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InventoryActionPrompts;

pub fn spawn_inventory_action_prompts(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	assets: &AssetServer,
	device: InputPromptDevice,
) {
	parent
		.spawn((
			InventoryActionPrompts,
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(16.0),
				row_gap: Val::Px(PANEL_ROW_GAP),
				align_items: AlignItems::Center,
				padding: UiRect::top(Val::Px(4.0)),
				flex_shrink: 0.0,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for kind in INVENTORY_PROMPT_KINDS {
				spawn_prompt_chip(row, fonts, assets, device, kind);
			}
		});
}

fn spawn_prompt_chip(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	assets: &AssetServer,
	device: InputPromptDevice,
	kind: InventoryPromptKind,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::Center,
				column_gap: Val::Px(8.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|chip| {
			chip.spawn((
				InventoryPromptGlyph(kind),
				ImageNode::new(assets.load(kind.icon(device))),
				Node { width: Val::Px(28.0), height: Val::Px(28.0), flex_shrink: 0.0, ..default() },
				Pickable::IGNORE,
			));
			chip.spawn((
				Text::new(kind.label()),
				fonts.item(PANEL_GROUP_FONT_SIZE),
				TextColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
		});
}

pub fn sync_input_prompt_device(
	keys: Res<ButtonInput<KeyCode>>,
	gamepads: Query<&Gamepad>,
	mut device: ResMut<InputPromptDevice>,
) {
	let key_used = keys.get_just_pressed().next().is_some();
	let pad_used = gamepads.iter().any(gamepad_just_pressed);
	if pad_used {
		*device = InputPromptDevice::Pad;
	} else if key_used {
		*device = InputPromptDevice::Keyboard;
	}
}

fn gamepad_just_pressed(gamepad: &Gamepad) -> bool {
	[
		GamepadButton::South,
		GamepadButton::East,
		GamepadButton::West,
		GamepadButton::North,
		GamepadButton::Start,
		GamepadButton::Select,
	]
	.into_iter()
	.any(|button| gamepad.just_pressed(button))
}

pub fn sync_inventory_prompt_glyphs(
	device: Res<InputPromptDevice>,
	assets: Res<AssetServer>,
	mut glyphs: Query<(&InventoryPromptGlyph, &mut ImageNode)>,
) {
	if !device.is_changed() && !glyphs.is_empty() {
		// Still refresh when the resource is first seen after spawn.
	}
	for (glyph, mut image) in &mut glyphs {
		let path = glyph.0.icon(*device);
		let handle = assets.load(path);
		if image.image != handle {
			image.image = handle;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{INVENTORY_PROMPT_KINDS, InputPromptDevice, InventoryPromptKind};
	use crate::theme::{PROMPT_KEY_DROP, PROMPT_KEY_SELECT, PROMPT_PAD_DROP, PROMPT_PAD_SELECT};
	use bevy::prelude::KeyCode;
	use maybraid_input::{
		INVENTORY_DROP_KEYS, INVENTORY_DROP_PAD, MENU_SELECT_KEYS, MENU_SELECT_PAD, PadButton,
	};

	#[test]
	fn select_glyphs_follow_the_menu_select_binding() {
		assert_eq!(MENU_SELECT_PAD, [PadButton::A, PadButton::Start]);
		assert_eq!(MENU_SELECT_KEYS, [KeyCode::Enter]);
		assert_eq!(InputPromptDevice::Pad.select_icon(), PROMPT_PAD_SELECT);
		assert_eq!(InputPromptDevice::Keyboard.select_icon(), PROMPT_KEY_SELECT);
		assert_eq!(InventoryPromptKind::Equip.label(), "Equip/Unequip");
		assert_eq!(InventoryPromptKind::Equip.icon(InputPromptDevice::Pad), PROMPT_PAD_SELECT);
		assert_eq!(InventoryPromptKind::Equip.icon(InputPromptDevice::Keyboard), PROMPT_KEY_SELECT);
		assert_eq!(INVENTORY_PROMPT_KINDS, [InventoryPromptKind::Equip, InventoryPromptKind::Drop]);
	}

	#[test]
	fn drop_glyphs_follow_the_inventory_drop_binding() {
		assert_eq!(INVENTORY_DROP_PAD, PadButton::X);
		assert_eq!(INVENTORY_DROP_KEYS, [KeyCode::KeyE, KeyCode::KeyX]);
		assert_eq!(InputPromptDevice::Pad.drop_icon(), PROMPT_PAD_DROP);
		assert_eq!(InputPromptDevice::Keyboard.drop_icon(), PROMPT_KEY_DROP);
		assert_eq!(InventoryPromptKind::Drop.icon(InputPromptDevice::Pad), PROMPT_PAD_DROP);
	}
}
