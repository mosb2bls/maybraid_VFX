//! Connected-pad rumble. Mouse and keyboard do not vibrate.

use std::time::Duration;

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;

/// Gameplay rumble pulse. [`fan_out_pad_rumble`] copies it onto every live
/// [`Gamepad`]. Linux / Windows play that through gilrs; Apple plays it through
/// GameController haptics.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PadRumble {
	pub duration: Duration,
	pub intensity: GamepadRumbleIntensity,
	/// Wait this long on the pad clock before starting the motors.
	pub delay: Duration,
}

impl PadRumble {
	pub fn motors(duration: Duration, weak: f32, strong: f32) -> Self {
		Self {
			duration,
			intensity: GamepadRumbleIntensity {
				weak_motor: weak.clamp(0.0, 1.0),
				strong_motor: strong.clamp(0.0, 1.0),
			},
			delay: Duration::ZERO,
		}
	}

	pub fn after(mut self, delay: Duration) -> Self {
		self.delay = delay;
		self
	}

	pub fn is_silent(self) -> bool {
		self.duration.is_zero()
			|| (self.intensity.weak_motor <= 0.0 && self.intensity.strong_motor <= 0.0)
	}
}

#[derive(Resource, Default)]
pub(crate) struct DelayedPadRumble {
	waiting: Vec<DelayedPulse>,
}

#[derive(Clone, Copy)]
pub(crate) struct DelayedPulse {
	remaining: f32,
	pulse: PadRumble,
}

/// Pulses whose delay has elapsed, oldest first. `dt` is not applied to a
/// brand-new zero-delay pulse so it can fan out on the same tick it arrives.
pub(crate) fn take_due_rumbles(waiting: &mut Vec<DelayedPulse>, dt: f32) -> Vec<PadRumble> {
	let mut due = Vec::new();
	waiting.retain_mut(|item| {
		if item.remaining <= 1e-4 {
			due.push(item.pulse);
			return false;
		}
		item.remaining -= dt;
		if item.remaining <= 1e-4 {
			due.push(item.pulse);
			return false;
		}
		true
	});
	due
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PadRumbleSystems {
	FanOut,
	Play,
}

pub(crate) fn fan_out_pad_rumble(
	time: Res<Time>,
	mut pulses: MessageReader<PadRumble>,
	mut delayed: ResMut<DelayedPadRumble>,
	pads: Query<Entity, With<Gamepad>>,
	mut rumble: MessageWriter<GamepadRumbleRequest>,
) {
	for pulse in pulses.read() {
		if pulse.is_silent() {
			debug!("pad_rumble: skipped silent pulse");
			continue;
		}
		delayed
			.waiting
			.push(DelayedPulse { remaining: pulse.delay.as_secs_f32(), pulse: *pulse });
	}
	let due = take_due_rumbles(&mut delayed.waiting, time.delta_secs());
	if due.is_empty() {
		return;
	}
	let pads: Vec<Entity> = pads.iter().collect();
	if pads.is_empty() {
		debug!("pad_rumble: fan_out had {} pulse(s) but 0 Gamepad entities", due.len());
		return;
	}
	for pulse in due {
		debug!(
			"pad_rumble: fan_out pads={} duration={:?} delay={:?} weak={:.2} strong={:.2}",
			pads.len(),
			pulse.duration,
			pulse.delay,
			pulse.intensity.weak_motor,
			pulse.intensity.strong_motor
		);
		for gamepad in &pads {
			rumble.write(GamepadRumbleRequest::Add {
				gamepad: *gamepad,
				duration: pulse.duration,
				intensity: pulse.intensity,
			});
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn motors_clamp_to_unit_interval() {
		let pulse = PadRumble::motors(Duration::from_millis(40), 1.5, -0.2);
		assert!((pulse.intensity.weak_motor - 1.0).abs() < 1e-5);
		assert!(pulse.intensity.strong_motor.abs() < 1e-5);
		assert!(!pulse.is_silent());
	}

	#[test]
	fn zero_duration_or_intensity_is_silent() {
		assert!(PadRumble::motors(Duration::ZERO, 0.4, 0.2).is_silent());
		assert!(PadRumble::motors(Duration::from_millis(40), 0.0, 0.0).is_silent());
	}

	#[test]
	fn fan_out_copies_onto_each_gamepad() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<DelayedPadRumble>()
			.add_message::<PadRumble>()
			.add_message::<GamepadRumbleRequest>()
			.add_systems(Update, fan_out_pad_rumble);
		let left = app.world_mut().spawn(Gamepad::default()).id();
		let right = app.world_mut().spawn(Gamepad::default()).id();
		app.world_mut()
			.write_message(PadRumble::motors(Duration::from_millis(40), 0.3, 0.1));
		app.update();

		let messages = app.world().resource::<Messages<GamepadRumbleRequest>>();
		let mut cursor = messages.get_cursor();
		let got: Vec<_> = cursor.read(messages).cloned().collect();
		assert_eq!(got.len(), 2);
		let entities: Vec<Entity> = got.iter().map(GamepadRumbleRequest::gamepad).collect();
		assert!(entities.contains(&left));
		assert!(entities.contains(&right));
		for request in &got {
			let GamepadRumbleRequest::Add { duration, intensity, .. } = request else {
				panic!("expected Add");
			};
			assert_eq!(*duration, Duration::from_millis(40));
			assert!((intensity.weak_motor - 0.3).abs() < 1e-5);
			assert!((intensity.strong_motor - 0.1).abs() < 1e-5);
		}
	}

	#[test]
	fn silent_pulse_does_not_fan_out() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<DelayedPadRumble>()
			.add_message::<PadRumble>()
			.add_message::<GamepadRumbleRequest>()
			.add_systems(Update, fan_out_pad_rumble);
		app.world_mut().spawn(Gamepad::default());
		app.world_mut()
			.write_message(PadRumble::motors(Duration::from_millis(40), 0.0, 0.0));
		app.update();

		let messages = app.world().resource::<Messages<GamepadRumbleRequest>>();
		let mut cursor = messages.get_cursor();
		assert_eq!(cursor.read(messages).count(), 0);
	}

	#[test]
	fn no_gamepad_means_no_rumble() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<DelayedPadRumble>()
			.add_message::<PadRumble>()
			.add_message::<GamepadRumbleRequest>()
			.add_systems(Update, fan_out_pad_rumble);
		app.world_mut()
			.write_message(PadRumble::motors(Duration::from_millis(40), 0.4, 0.2));
		app.update();

		let messages = app.world().resource::<Messages<GamepadRumbleRequest>>();
		let mut cursor = messages.get_cursor();
		assert_eq!(cursor.read(messages).count(), 0);
	}

	#[test]
	fn zero_delay_is_due_immediately() {
		let pulse = PadRumble::motors(Duration::from_millis(40), 0.3, 0.1);
		let mut waiting = vec![DelayedPulse { remaining: 0.0, pulse }];
		let due = take_due_rumbles(&mut waiting, 0.016);
		assert_eq!(due, vec![pulse]);
		assert!(waiting.is_empty());
	}

	#[test]
	fn delayed_pulse_waits_out_the_clock() {
		let pulse =
			PadRumble::motors(Duration::from_millis(40), 0.3, 0.1).after(Duration::from_millis(40));
		let mut waiting = vec![DelayedPulse { remaining: 0.04, pulse }];
		assert!(take_due_rumbles(&mut waiting, 0.016).is_empty());
		assert!(take_due_rumbles(&mut waiting, 0.016).is_empty());
		assert_eq!(take_due_rumbles(&mut waiting, 0.016), vec![pulse]);
	}
}
