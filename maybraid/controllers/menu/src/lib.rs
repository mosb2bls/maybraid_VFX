//! Menu controller: pad nav → one focused menu in a screen subtree.

pub mod controller;
pub mod dispatch;

pub use controller::MenuController;
pub use dispatch::{exclusive_nav_menu, ExclusiveNav, MenuNavCapture};
pub use maybraid_controllers_core::{
	CaptureKind, CaptureToken, InputRouter, InputRouting, InputSource, RoutingEvent, ScopeOwner,
	UiTarget,
};

use bevy::prelude::*;
use maybraid_controllers_core::{CaptureResolveSystems, ControllersCorePlugin};
use maybraid_input::{PadInputPolicy, VirtualPadPlugin, VirtualPadSystems};
use menu_components::{
	KeyboardMenuNav, MenuComponentsPlugin, ShortTextModal, TextMenuInputLock, TextMenuSystems,
};

pub struct MenuControllerPlugin;

impl Plugin for MenuControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.init_resource::<MenuNavCapture>()
			.init_resource::<PadInputPolicy>()
			.insert_resource(KeyboardMenuNav(false))
			.add_systems(
				PreUpdate,
				(sync_text_menu_capture, sync_pad_from_router)
					.chain()
					.before(VirtualPadSystems::Produce),
			);
		app.add_systems(
			Update,
			dispatch::refresh_menu_focus
				.in_set(CaptureResolveSystems)
				.before(TextMenuSystems::Navigate),
		);
		app.add_systems(Update, dispatch::dispatch_menu_nav.in_set(TextMenuSystems::Navigate));
	}
}

fn sync_text_menu_capture(
	lock: Res<TextMenuInputLock>,
	modal: Res<ShortTextModal>,
	mut router: InputRouting,
	mut menu_capture: ResMut<MenuNavCapture>,
) {
	router.replace_owned_capture(
		&mut menu_capture.text,
		maybraid_controllers_core::InputSource::local_pad(),
		(lock.0 && !modal.is_open()).then_some(ScopeOwner::text_lock()),
		CaptureKind::TextEntry,
	);
}

fn sync_pad_from_router(router: Res<InputRouter>, mut policy: ResMut<PadInputPolicy>) {
	*policy = router.pad_policy();
}

#[cfg(test)]
mod tests {
	use super::{sync_pad_from_router, sync_text_menu_capture};
	use crate::dispatch::MenuNavCapture;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_controllers_core::{
		CaptureKind, InputRouter, InputSource, RoutingEvent, ScopeOwner,
	};
	use maybraid_input::PadInputPolicy;
	use menu_components::controls::short_text::ShortTextSession;
	use menu_components::{ShortTextModal, TextMenuInputLock};

	fn world() -> World {
		let mut world = World::new();
		world.init_resource::<TextMenuInputLock>();
		world.init_resource::<ShortTextModal>();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<RoutingEvent>>();
		world.init_resource::<MenuNavCapture>();
		world.init_resource::<PadInputPolicy>();
		world
	}

	fn sync(world: &mut World) {
		world.run_system_once(sync_text_menu_capture).expect("text capture");
		world.run_system_once(sync_pad_from_router).expect("pad policy");
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
		sync(&mut world);
		assert_eq!(*world.resource::<PadInputPolicy>(), PadInputPolicy::gameplay());
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(maybraid_controllers_core::UiTarget::new(Entity::from_bits(1))),
			CaptureKind::MenuNav,
		);
		world.run_system_once(sync_pad_from_router).expect("pad policy");
		assert_eq!(*world.resource::<PadInputPolicy>(), PadInputPolicy::menu());
	}

	#[test]
	fn console_lock_without_a_modal_suppresses_the_pad() {
		let mut world = world();
		world.resource_mut::<TextMenuInputLock>().0 = true;
		sync(&mut world);
		assert_eq!(*world.resource::<PadInputPolicy>(), PadInputPolicy::console());
		assert!(world
			.resource::<InputRouter>()
			.captures()
			.suppresses_pad(InputSource::local_pad()));
	}

	#[test]
	fn a_focused_menu_switches_the_keyboard_map() {
		let mut world = world();
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(maybraid_controllers_core::UiTarget::new(Entity::from_bits(2))),
			CaptureKind::MenuNav,
		);
		sync(&mut world);
		assert_eq!(*world.resource::<PadInputPolicy>(), PadInputPolicy::menu());
	}

	#[test]
	fn console_capture_survives_a_later_menu_layer() {
		let mut world = world();
		world.resource_mut::<TextMenuInputLock>().0 = true;
		sync(&mut world);
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(maybraid_controllers_core::UiTarget::new(Entity::from_bits(3))),
			CaptureKind::MenuNav,
		);
		world.run_system_once(sync_pad_from_router).expect("pad policy");
		assert_eq!(*world.resource::<PadInputPolicy>(), PadInputPolicy::console());
	}
}
