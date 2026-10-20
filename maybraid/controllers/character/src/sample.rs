//! Actor-addressed pad sample. Analog stays on this value, not a Message.
//!
//! Physical face buttons live here so Tanz can read A / B without consuming
//! semantic [`crate::CharacterIntent::Jump`].

use bevy::prelude::*;
use maybraid_controllers_core::{ActorTarget, InputSource, LocalActor, Routed};
use maybraid_input::{PadButton, VirtualPad};

/// One frame of analog sticks and physical face / bumper state for one actor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActorControlSample {
	move_stick: Vec2,
	look_throw: Vec2,
	a: FaceButtonSample,
	b: FaceButtonSample,
	bumper_focus: bool,
	bumper_fire: bool,
}

/// Rising / hold state for one physical face button.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FaceButtonSample {
	pressed: bool,
	just_pressed: bool,
	just_released: bool,
	hold_secs: f32,
}

/// Bind-stage snapshot: the local pad routed to one [`ActorTarget`].
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct RoutedActorControl(Option<Routed<ActorControlSample>>);

impl FaceButtonSample {
	pub fn from_pad(pad: &VirtualPad, button: PadButton) -> Self {
		Self {
			pressed: pad.pressed(button),
			just_pressed: pad.just_pressed(button),
			just_released: pad.just_released(button),
			hold_secs: pad.hold_secs(button),
		}
	}

	pub const fn new(
		pressed: bool,
		just_pressed: bool,
		just_released: bool,
		hold_secs: f32,
	) -> Self {
		Self { pressed, just_pressed, just_released, hold_secs }
	}

	pub const fn pressed(self) -> bool {
		self.pressed
	}

	pub const fn just_pressed(self) -> bool {
		self.just_pressed
	}

	pub const fn just_released(self) -> bool {
		self.just_released
	}

	pub const fn hold_secs(self) -> f32 {
		self.hold_secs
	}
}

impl ActorControlSample {
	pub fn from_pad(pad: &VirtualPad) -> Self {
		Self {
			move_stick: pad.move_stick,
			look_throw: pad.look_throw,
			a: FaceButtonSample::from_pad(pad, PadButton::A),
			b: FaceButtonSample::from_pad(pad, PadButton::B),
			bumper_focus: pad.pressed(PadButton::BumperFocus),
			bumper_fire: pad.pressed(PadButton::BumperFire),
		}
	}

	pub const fn move_stick(self) -> Vec2 {
		self.move_stick
	}

	pub const fn look_throw(self) -> Vec2 {
		self.look_throw
	}

	pub const fn a(self) -> FaceButtonSample {
		self.a
	}

	pub const fn b(self) -> FaceButtonSample {
		self.b
	}

	pub const fn bumper_focus(self) -> bool {
		self.bumper_focus
	}

	pub const fn bumper_fire(self) -> bool {
		self.bumper_fire
	}

	pub const fn bumper_chord(self) -> bool {
		self.bumper_focus && self.bumper_fire
	}

	/// Xbox **RB** / keyboard **V** without the Tanzer bumper chord.
	pub const fn skill_map_steer_held(self) -> bool {
		self.bumper_fire && !self.bumper_chord()
	}

	pub fn with_move_stick(mut self, stick: Vec2) -> Self {
		self.move_stick = stick;
		self
	}

	pub fn with_look_throw(mut self, stick: Vec2) -> Self {
		self.look_throw = stick;
		self
	}

	pub fn with_a(mut self, a: FaceButtonSample) -> Self {
		self.a = a;
		self
	}

	pub fn with_b(mut self, b: FaceButtonSample) -> Self {
		self.b = b;
		self
	}

	pub fn with_bumpers(mut self, focus: bool, fire: bool) -> Self {
		self.bumper_focus = focus;
		self.bumper_fire = fire;
		self
	}
}

impl RoutedActorControl {
	pub fn clear(&mut self) {
		self.0 = None;
	}

	pub fn set(&mut self, routed: Routed<ActorControlSample>) {
		self.0 = Some(routed);
	}

	pub fn get(&self) -> Option<&Routed<ActorControlSample>> {
		self.0.as_ref()
	}

	pub fn payload(&self) -> Option<ActorControlSample> {
		self.0.as_ref().map(Routed::payload).copied()
	}

	pub fn actor(&self) -> Option<ActorTarget> {
		self.0.as_ref().and_then(|routed| match routed.target() {
			maybraid_controllers_core::RouteTarget::Actor(target) => Some(target),
			maybraid_controllers_core::RouteTarget::Ui(_) => None,
		})
	}

	pub fn addresses(&self, entity: Entity) -> bool {
		self.actor().is_some_and(|target| target.matches(entity))
	}

	pub fn from_local_pad(target: ActorTarget, pad: &VirtualPad) -> Self {
		Self(Some(Routed::to_actor(
			InputSource::local_pad(),
			target,
			ActorControlSample::from_pad(pad),
		)))
	}

	pub fn from_sample(target: ActorTarget, sample: ActorControlSample) -> Self {
		Self(Some(Routed::to_actor(InputSource::local_pad(), target, sample)))
	}
}

/// Copy the local pad onto [`RoutedActorControl`] when a [`LocalActor`] is set.
///
/// Locomotion fills the same resource with the Player fallback when hosts have
/// not installed [`LocalActor`]. This system does not clear a fallback write.
pub fn produce_actor_control_samples(
	pad: Option<Res<VirtualPad>>,
	local: Option<Res<LocalActor>>,
	mut routed: ResMut<RoutedActorControl>,
) {
	let Some(local) = local else {
		return;
	};
	let Some(pad) = pad else {
		routed.clear();
		return;
	};
	routed.set(Routed::to_actor(
		InputSource::local_pad(),
		local.target(),
		ActorControlSample::from_pad(&pad),
	));
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::produce::collect;
	use crate::CharacterIntent;
	use maybraid_input::VirtualPad;

	#[test]
	fn physical_a_is_not_jump() {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		let sample = ActorControlSample::from_pad(&pad);
		assert!(sample.a().just_pressed());
		assert!(collect(&pad, 0.5, 0.0, true, true).contains(&CharacterIntent::Jump));
		assert!(!collect(&pad, 0.5, 0.0, true, false).contains(&CharacterIntent::Jump));
	}

	#[test]
	fn look_throw_is_the_raw_stick_not_camera_look() {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_look(Vec2::new(40.0, -12.0));
		pad.add_look_throw(Vec2::Y);
		pad.finish_digital();
		let sample = ActorControlSample::from_pad(&pad);
		assert_eq!(sample.look_throw(), Vec2::Y);
		assert_ne!(sample.look_throw(), pad.look_stick);
	}

	#[test]
	fn steer_hold_is_rb_without_the_chord() {
		let rb = ActorControlSample::default().with_bumpers(false, true);
		assert!(rb.skill_map_steer_held());
		let chord = ActorControlSample::default().with_bumpers(true, true);
		assert!(chord.bumper_chord());
		assert!(!chord.skill_map_steer_held());
	}

	#[test]
	fn routed_control_addresses_one_actor() {
		let actor = ActorTarget::new(Entity::from_bits(4));
		let other = Entity::from_bits(5);
		let routed = RoutedActorControl::from_sample(actor, ActorControlSample::default());
		assert!(routed.addresses(actor.entity()));
		assert!(!routed.addresses(other));
	}
}
