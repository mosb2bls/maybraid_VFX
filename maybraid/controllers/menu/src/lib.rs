//! Menu controller: pad nav → one focused menu in a screen subtree.

pub mod controller;
pub mod dispatch;

pub use controller::MenuController;
pub use dispatch::{exclusive_nav_menu, ExclusiveNav};

use bevy::prelude::*;
use maybraid_input::{
	KeyboardPadMap, MenuOwnsBack, PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems,
};
use menu_components::{
	KeyboardMenuNav, MenuComponentsPlugin, ShortTextModal, TextMenuInputLock, TextMenuSystems,
};

pub struct MenuControllerPlugin;

impl Plugin for MenuControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.init_resource::<maybraid_input::MenuOwnsBack>()
			.insert_resource(KeyboardMenuNav(false))
			.add_systems(PreUpdate, sync_pad_from_menu_lock.before(VirtualPadSystems::Produce));
		app.add_systems(
			Update,
			(dispatch::refresh_menu_focus, dispatch::dispatch_menu_nav)
				.chain()
				.in_set(TextMenuSystems::Navigate),
		);
	}
}

fn sync_pad_from_menu_lock(
	lock: Res<TextMenuInputLock>,
	modal: Res<ShortTextModal>,
	owns: Res<MenuOwnsBack>,
	mut enabled: ResMut<PadGameplayEnabled>,
	mut keys: ResMut<KeyboardPadMap>,
) {
	enabled.set_menu(!lock.0 || modal.is_open());
	*keys = if modal.is_open() || lock.0 || owns.0 {
		KeyboardPadMap::Menu
	} else {
		KeyboardPadMap::Gameplay
	};
}

#[cfg(test)]
mod tests {
	use super::sync_pad_from_menu_lock;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_input::{KeyboardPadMap, MenuOwnsBack, PadGameplayEnabled};
	use menu_components::controls::short_text::ShortTextSession;
	use menu_components::{ShortTextModal, TextMenuInputLock};

	fn world() -> World {
		let mut world = World::new();
		world.init_resource::<TextMenuInputLock>();
		world.init_resource::<ShortTextModal>();
		world.init_resource::<MenuOwnsBack>();
		world.init_resource::<PadGameplayEnabled>();
		world.init_resource::<KeyboardPadMap>();
		world
	}

	#[test]
	fn open_name_modal_keeps_the_pad_and_uses_the_menu_map() {
		let mut world = world();
		world.resource_mut::<TextMenuInputLock>().0 = true;
		world.resource_mut::<ShortTextModal>().session = Some(ShortTextSession {
			key: "name",
			source: Entity::PLACEHOLDER,
			value: String::new(),
			original: String::new(),
			max_len: 16,
			shift: false,
		});
		world.run_system_once(sync_pad_from_menu_lock).expect("sync");
		assert!(world.resource::<PadGameplayEnabled>().is_enabled());
		assert!(world.resource::<PadGameplayEnabled>().menu_allows());
		assert_eq!(*world.resource::<KeyboardPadMap>(), KeyboardPadMap::Menu);
	}

	#[test]
	fn console_lock_without_a_modal_suppresses_the_pad() {
		let mut world = world();
		world.resource_mut::<TextMenuInputLock>().0 = true;
		world.run_system_once(sync_pad_from_menu_lock).expect("sync");
		assert!(!world.resource::<PadGameplayEnabled>().is_enabled());
		assert_eq!(*world.resource::<KeyboardPadMap>(), KeyboardPadMap::Menu);
	}

	#[test]
	fn a_focused_menu_switches_the_keyboard_map() {
		let mut world = world();
		world.resource_mut::<MenuOwnsBack>().0 = true;
		world.run_system_once(sync_pad_from_menu_lock).expect("sync");
		assert!(world.resource::<PadGameplayEnabled>().is_enabled());
		assert_eq!(*world.resource::<KeyboardPadMap>(), KeyboardPadMap::Menu);
	}

	#[test]
	fn text_lane_survives_the_menu_writer() {
		let mut world = world();
		world.resource_mut::<PadGameplayEnabled>().set_text(false);
		world.run_system_once(sync_pad_from_menu_lock).expect("sync");
		assert!(!world.resource::<PadGameplayEnabled>().text_allows());
		assert!(world.resource::<PadGameplayEnabled>().menu_allows());
		assert!(!world.resource::<PadGameplayEnabled>().is_enabled());
	}
}
