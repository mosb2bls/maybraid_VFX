//! On-screen HID → pad dump. Enable with [`crate::VirtualPadConfig::debug_overlay`].

use bevy::prelude::*;

use crate::config::VirtualPadConfig;
use crate::gate::PadGameplayEnabled;
use crate::pad::VirtualPad;
use crate::produce::gamepad::GamepadAxes;
use crate::surface::menu::MenuNavPad;

#[derive(Component)]
struct VirtualPadDebugText;

pub fn configure_debug(app: &mut App) {
	app.add_systems(Startup, spawn_debug_overlay)
		.add_systems(Update, update_debug_overlay);
}

fn spawn_debug_overlay(config: Res<VirtualPadConfig>, mut commands: Commands) {
	if !config.debug_overlay {
		return;
	}
	commands.spawn((
		VirtualPadDebugText,
		Text::new("pad debug"),
		TextFont { font_size: FontSize::Px(13.0), ..default() },
		TextColor(Color::srgb(0.95, 0.9, 0.45)),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(8.0),
			left: Val::Px(8.0),
			width: Val::Px(720.0),
			..default()
		},
		GlobalZIndex(i32::MAX),
		Pickable::IGNORE,
	));
}

fn update_debug_overlay(
	config: Res<VirtualPadConfig>,
	enabled: Res<PadGameplayEnabled>,
	pad: Res<VirtualPad>,
	nav: Res<MenuNavPad>,
	gamepads: Query<(Entity, &Gamepad)>,
	mut lines: Query<&mut Text, With<VirtualPadDebugText>>,
) {
	if !config.debug_overlay {
		return;
	}
	let Ok(mut text) = lines.single_mut() else {
		return;
	};
	let mut out = String::new();
	out.push_str(&format!(
		"enabled={} nav={:?} move=({:.2},{:.2}) dpad=({:.2},{:.2}) A={} B={} Start={} Select={}\n",
		enabled.is_enabled(),
		nav.events,
		pad.move_stick.x,
		pad.move_stick.y,
		pad.dpad.x,
		pad.dpad.y,
		pad.pressed(crate::PadButton::A),
		pad.pressed(crate::PadButton::B),
		pad.pressed(crate::PadButton::Start),
		pad.pressed(crate::PadButton::Select),
	));
	if gamepads.is_empty() {
		out.push_str("no Gamepad components\n");
	}
	for (entity, gamepad) in &gamepads {
		let left = gamepad.left_stick();
		let right = gamepad.right_stick();
		let dpad = gamepad.dpad();
		let pressed: Vec<_> = gamepad.get_pressed().copied().collect();
		out.push_str(&format!(
			"{entity} left=({:.2},{:.2}) right=({:.2},{:.2}) dpad=({:.2},{:.2}) pressed={pressed:?}\n",
			left.x, left.y, right.x, right.y, dpad.x, dpad.y
		));
		let analog = GamepadAxes::analog_dump(gamepad);
		if !analog.is_empty() {
			out.push_str(&format!("  analog {analog}\n"));
		}
	}
	text.0 = out;
}
