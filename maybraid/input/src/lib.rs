//! Shared virtual pad for Maybraid: device producers, current-frame snapshot,
//! optional history, and derived surfaces (menu nav, cursor).

pub mod analog;
pub mod button;
pub mod config;
pub mod debug;
pub mod gate;
pub mod hid;
pub mod history;
pub mod overlay;
pub mod pad;
pub mod produce;
pub mod rumble;
pub mod surface;

pub use analog::{Cardinal, Deadzone};
pub use button::{ButtonPhase, ButtonStroke, PadButton, PAD_BUTTON_COUNT};
pub use config::VirtualPadConfig;
pub use gate::{KeyboardPadMap, MenuOwnsBack, PadGameplayEnabled};
pub use hid::{with_pad_hid, PadHidPlugins};
pub use history::{PadEdge, PadHistory, PadSnapshot, Timed};
pub use overlay::PadOverlay;
pub use pad::VirtualPad;
pub use rumble::{PadRumble, PadRumbleSystems};
pub use surface::cursor::PadCursor;
pub use surface::menu::{
	MenuNav, MenuNavImpulse, MenuNavPad, INVENTORY_DROP_KEYS, INVENTORY_DROP_PAD, MENU_SELECT_KEYS,
	MENU_SELECT_PAD,
};

use bevy::input::InputSystems;
use bevy::prelude::*;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VirtualPadSystems {
	Produce,
	Derive,
}

#[derive(Default)]
pub struct VirtualPadPlugin {
	pub config: VirtualPadConfig,
}

impl VirtualPadPlugin {
	pub fn new(config: VirtualPadConfig) -> Self {
		Self { config }
	}
}

impl Plugin for VirtualPadPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			PostUpdate,
			rumble::PadRumbleSystems::Play.after(rumble::PadRumbleSystems::FanOut),
		);
		#[cfg(not(any(target_os = "macos", target_os = "ios")))]
		{
			app.configure_sets(
				PostUpdate,
				bevy::gilrs::RumbleSystems.after(rumble::PadRumbleSystems::FanOut),
			);
		}
		hid::configure_backend(app);
		app.insert_resource(self.config.clone())
			.init_resource::<VirtualPad>()
			.init_resource::<PadHistory>()
			.init_resource::<PadGameplayEnabled>()
			.init_resource::<KeyboardPadMap>()
			.init_resource::<MenuOwnsBack>()
			.init_resource::<MenuNavPad>()
			.init_resource::<PadCursor>()
			.add_message::<rumble::PadRumble>()
			.init_resource::<rumble::DelayedPadRumble>()
			.configure_sets(
				PreUpdate,
				VirtualPadSystems::Produce.after(InputSystems).before(VirtualPadSystems::Derive),
			)
			.configure_sets(PreUpdate, VirtualPadSystems::Derive.after(VirtualPadSystems::Produce));
		produce::configure_produce(app);
		debug::configure_debug(app);
		app.add_systems(
			PreUpdate,
			(
				history::push_history,
				surface::menu::derive_menu_nav,
				surface::cursor::integrate_cursor,
			)
				.chain()
				.in_set(VirtualPadSystems::Derive),
		);
		app.add_systems(
			PostUpdate,
			rumble::fan_out_pad_rumble.in_set(rumble::PadRumbleSystems::FanOut),
		);
	}
}
