//! Persistent Bevy system execution and once-per-frame message aging.
//!
//! `run_system_once` builds a fresh `MessageReader` cursor each call. If the
//! caller also skips `Messages::update`, unread messages accumulate and later
//! passes re-read earlier frames. Register once, run the same system, age
//! buffers once per simulated frame.

use anyhow::Result;
use bevy::ecs::message::{Message, Messages};
use bevy::ecs::system::{IntoSystem, SystemId};
use bevy::prelude::World;

/// Initialized system whose locals (including `MessageReader` cursors) persist.
pub struct PersistentSystem<Out: 'static> {
	id: SystemId<(), Out>,
}

impl<Out: 'static> PersistentSystem<Out> {
	pub fn initialize<M>(world: &mut World, system: impl IntoSystem<(), Out, M> + 'static) -> Self {
		Self { id: world.register_system(system) }
	}

	pub fn run(&self, world: &mut World) -> Result<Out> {
		world.run_system(self.id).map_err(|error| anyhow::anyhow!("{error}"))
	}
}

/// One simulated-frame aging of a message buffer (`Messages::update`).
pub struct MessageBuffers;

impl MessageBuffers {
	pub fn age<M: Message>(world: &mut World) {
		world.resource_mut::<Messages<M>>().update();
	}
}
