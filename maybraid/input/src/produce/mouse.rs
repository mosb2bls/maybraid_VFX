//! Mouse motion → [`VirtualPad::look_stick`]; LMB → [`VirtualPad::trigger_fire`].

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::pad::VirtualPad;

fn command_held(keys: &ButtonInput<KeyCode>) -> bool {
	keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)
}

pub fn produce_mouse(
	keyboard: Res<ButtonInput<KeyCode>>,
	mouse: Res<ButtonInput<MouseButton>>,
	mut mouse_motion: MessageReader<MouseMotion>,
	mut pad: ResMut<VirtualPad>,
) {
	if command_held(&keyboard) {
		mouse_motion.clear();
		return;
	}
	let mut delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		delta += event.delta;
	}
	pad.add_look(delta);
	if mouse.pressed(MouseButton::Left) {
		pad.max_triggers(0.0, 1.0);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn left_click_sets_trigger_fire() -> anyhow::Result<()> {
		let mut app = App::new();
		app.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<ButtonInput<MouseButton>>()
			.init_resource::<VirtualPad>()
			.add_message::<MouseMotion>()
			.add_systems(Update, produce_mouse);
		app.world_mut()
			.resource_mut::<ButtonInput<MouseButton>>()
			.press(MouseButton::Left);
		app.update();
		assert_eq!(app.world().resource::<VirtualPad>().trigger_fire, 1.0);
		Ok(())
	}
}
