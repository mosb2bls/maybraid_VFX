//! Explicit membership linking runtime parts to their [`VfxInstance`] root.

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;

use crate::spawn::{VfxEmitter, VfxEmitterArmed, VfxEmitterBurst, VfxInstance, VfxLayerLife};

/// Links one spawned part to its owning [`VfxInstance`].
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = VfxInstanceMembers)]
pub struct VfxMemberOf(pub Entity);

impl VfxMemberOf {
	pub fn root(self) -> Entity {
		self.0
	}

	/// Whether the owning instance has passed the emitter readiness gate.
	pub fn instance_armed(&self, instances: &Query<&VfxInstance>) -> bool {
		instances.get(self.root()).map(|instance| instance.armed).unwrap_or(true)
	}

	/// Same as [`Self::instance_armed`] for unit tests that hold a [`World`].
	#[cfg(test)]
	pub fn instance_armed_in_world(&self, world: &World) -> bool {
		world
			.get::<VfxInstance>(self.root())
			.map(|instance| instance.armed)
			.unwrap_or(true)
	}
}

/// All runtime parts registered for one [`VfxInstance`].
#[derive(Component, Debug)]
#[relationship_target(relationship = VfxMemberOf)]
pub struct VfxInstanceMembers(Vec<Entity>);

impl VfxInstanceMembers {
	/// Every registered emitter is GPU-ready.
	pub fn emitters_ready(
		&self,
		emitters: &Query<(&VfxEmitter, Has<VfxEmitterArmed>, Has<VfxEmitterBurst>)>,
	) -> bool {
		self.emitters_ready_with(|member| emitters.get(member).ok().map(|(_, armed, _)| armed))
	}

	/// Every registered layer life has finished its clock.
	pub fn layers_finished(&self, lives: &Query<&VfxLayerLife>) -> bool {
		self.layers_finished_with(|member| lives.get(member).ok().cloned())
	}

	fn emitters_ready_with(&self, mut armed: impl FnMut(Entity) -> Option<bool>) -> bool {
		for member in self.iter() {
			if armed(member) == Some(false) {
				return false;
			}
		}
		true
	}

	fn layers_finished_with(&self, mut life: impl FnMut(Entity) -> Option<VfxLayerLife>) -> bool {
		let mut saw = false;
		for member in self.iter() {
			let Some(life) = life(member) else {
				continue;
			};
			saw = true;
			if life.waiting_for_emitter || life.age + 1e-3 < life.duration {
				return false;
			}
		}
		saw
	}
}

#[cfg(test)]
impl VfxInstanceMembers {
	pub fn emitters_ready_in_world(&self, world: &World) -> bool {
		self.emitters_ready_with(|member| {
			if world.get::<VfxEmitter>(member).is_none() {
				return None;
			}
			Some(world.get::<VfxEmitterArmed>(member).is_some())
		})
	}

	pub fn layers_finished_in_world(&self, world: &World) -> bool {
		self.layers_finished_with(|member| world.get::<VfxLayerLife>(member).cloned())
	}
}
