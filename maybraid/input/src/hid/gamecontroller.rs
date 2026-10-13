//! Poll Apple `GCController` into Bevy raw gamepad messages.

use std::collections::{HashMap, HashSet};
use std::ptr::from_ref;
use std::time::Duration;

use bevy::input::gamepad::{
	GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent, GamepadRumbleIntensity,
	GamepadRumbleRequest, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
	RawGamepadEvent,
};
use bevy::input::InputSystems;
use bevy::prelude::*;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{msg_send, AnyThread, Message};
use objc2_core_haptics::{
	CHHapticDynamicParameter, CHHapticEngine, CHHapticEvent, CHHapticEventParameter,
	CHHapticEventParameterIDHapticIntensity, CHHapticEventParameterIDHapticSharpness,
	CHHapticEventTypeHapticContinuous, CHHapticPattern, CHHapticPatternPlayer,
	CHHapticTimeImmediate,
};
use objc2_foundation::{ns_string, NSArray, NSString};
use objc2_game_controller::{
	GCController, GCControllerButtonInput, GCControllerDirectionPad, GCDevice, GCDeviceHaptics,
	GCExtendedGamepad, GCHapticsLocality, GCHapticsLocalityDefault, GCHapticsLocalityHandles,
	GCInputButtonOptions, GCInputButtonShare, GCSystemGestureState, GCXboxGamepad,
};

use crate::rumble::PadRumbleSystems;

fn value_changed(previous: Option<f32>, value: f32) -> bool {
	const EPS: f32 = 1e-4;
	previous.is_none_or(|old| (old - value).abs() > EPS)
}

pub struct GameControllerPlugin;

impl Plugin for GameControllerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GameControllerPads>()
			.add_systems(PreStartup, (enable_background_events, poll_game_controllers).chain())
			.add_systems(PreUpdate, poll_game_controllers.before(InputSystems))
			.add_systems(PostUpdate, play_gamecontroller_rumble.in_set(PadRumbleSystems::Play));
	}
}

#[derive(Default, Resource)]
struct GameControllerPads {
	pads: HashMap<usize, PadState>,
}

/// GameController / Core Haptics objects are main-thread only. This plugin
/// already polls on the Bevy thread that pumps CFRunLoop.
struct MainThreadRc<T>(Retained<T>);

// SAFETY: `GameControllerPlugin` systems run on the main thread that owns these
// Objective-C objects. They are never sent to another thread.
unsafe impl<T> Send for MainThreadRc<T> {}
unsafe impl<T> Sync for MainThreadRc<T> {}

struct LivePulse {
	player: MainThreadRc<ProtocolObject<dyn CHHapticPatternPlayer>>,
	until: f32,
}

struct PadState {
	entity: Entity,
	controller: MainThreadRc<GCController>,
	engine: Option<MainThreadRc<CHHapticEngine>>,
	pulses: Vec<LivePulse>,
	axes: HashMap<GamepadAxis, f32>,
	buttons: HashMap<GamepadButton, f32>,
	select: Option<MainThreadRc<GCControllerButtonInput>>,
	logged_select: bool,
}

fn controller_id(controller: &GCController) -> usize {
	from_ref(controller) as usize
}

fn enable_background_events() {
	// SAFETY: class method; Bevy runs this on the main thread that pumps CFRunLoop.
	unsafe {
		GCController::setShouldMonitorBackgroundEvents(true);
	}
}

fn poll_game_controllers(
	mut commands: Commands,
	mut pads: ResMut<GameControllerPads>,
	mut connection_events: MessageWriter<GamepadConnectionEvent>,
	mut raw_events: MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: `controllers()` is the GameController inventory for this process.
	let controllers = unsafe { GCController::controllers() };
	let mut seen = HashSet::new();
	for index in 0..controllers.count() {
		let controller = controllers.objectAtIndex(index);
		let id = controller_id(&controller);
		seen.insert(id);
		sync_controller(
			&controller,
			id,
			&mut commands,
			&mut pads,
			&mut connection_events,
			&mut raw_events,
		);
	}

	let gone: Vec<usize> = pads.pads.keys().copied().filter(|id| !seen.contains(id)).collect();
	for id in gone {
		let Some(state) = pads.pads.remove(&id) else {
			continue;
		};
		let event = GamepadConnectionEvent::new(state.entity, GamepadConnection::Disconnected);
		connection_events.write(event.clone());
		raw_events.write(event.into());
	}
}

fn sync_controller(
	controller: &GCController,
	id: usize,
	commands: &mut Commands,
	pads: &mut GameControllerPads,
	connection_events: &mut MessageWriter<GamepadConnectionEvent>,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: live `GCController` from `controllers()`.
	let Some(extended) = (unsafe { controller.extendedGamepad() }) else {
		return;
	};
	let state = pads.pads.entry(id).or_insert_with(|| {
		let entity = commands.spawn_empty().id();
		let event = GamepadConnectionEvent::new(
			entity,
			GamepadConnection::Connected {
				name: controller_name(controller),
				vendor_id: None,
				product_id: None,
			},
		);
		connection_events.write(event.clone());
		raw_events.write(event.into());
		let name = controller_name(controller);
		// SAFETY: live controller; `haptics` is nil when the OS has no rumble path.
		let has_haptics = unsafe { controller.haptics() }.is_some();
		debug!("pad_rumble: connected name={name} entity={entity:?} haptics={has_haptics}");
		if name == "Controller" {
			debug!(
				"pad_rumble: generic 'Controller' name is usually USB Xbox; Apple rumble often only works over Bluetooth"
			);
		}
		let select = unsafe { select_button(controller, &extended) };
		if let Some(select) = select.as_ref() {
			unsafe {
				claim_for_app(select);
			}
		}
		PadState {
			entity,
			controller: MainThreadRc(controller.retain()),
			engine: None,
			pulses: Vec::new(),
			axes: HashMap::new(),
			buttons: HashMap::new(),
			select: select.map(MainThreadRc),
			logged_select: false,
		}
	});
	emit_extended(state, &extended, raw_events);
}

fn controller_name(controller: &GCController) -> String {
	// SAFETY: `vendorName` / `productCategory` are read-only NSString copies.
	unsafe {
		if let Some(vendor) = controller.vendorName() {
			let name = vendor.to_string();
			if !name.is_empty() {
				return name;
			}
		}
		controller.productCategory().to_string()
	}
}

fn emit_extended(
	state: &mut PadState,
	pad: &GCExtendedGamepad,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: `pad` is the extended profile of a connected controller.
	unsafe {
		emit_stick(
			state,
			raw_events,
			GamepadAxis::LeftStickX,
			GamepadAxis::LeftStickY,
			&pad.leftThumbstick(),
		);
		emit_stick(
			state,
			raw_events,
			GamepadAxis::RightStickX,
			GamepadAxis::RightStickY,
			&pad.rightThumbstick(),
		);
		emit_button(state, raw_events, GamepadButton::South, &pad.buttonA());
		emit_button(state, raw_events, GamepadButton::East, &pad.buttonB());
		emit_button(state, raw_events, GamepadButton::West, &pad.buttonX());
		emit_button(state, raw_events, GamepadButton::North, &pad.buttonY());
		emit_button(state, raw_events, GamepadButton::LeftTrigger, &pad.leftShoulder());
		emit_button(state, raw_events, GamepadButton::RightTrigger, &pad.rightShoulder());
		emit_button(state, raw_events, GamepadButton::LeftTrigger2, &pad.leftTrigger());
		emit_button(state, raw_events, GamepadButton::RightTrigger2, &pad.rightTrigger());
		emit_button(state, raw_events, GamepadButton::Start, &pad.buttonMenu());
		emit_select(state, pad, raw_events);
		if let Some(home) = pad.buttonHome() {
			emit_button(state, raw_events, GamepadButton::Mode, &home);
		}
		if let Some(click) = pad.leftThumbstickButton() {
			emit_button(state, raw_events, GamepadButton::LeftThumb, &click);
		}
		if let Some(click) = pad.rightThumbstickButton() {
			emit_button(state, raw_events, GamepadButton::RightThumb, &click);
		}
		let dpad = pad.dpad();
		emit_button(state, raw_events, GamepadButton::DPadUp, &dpad.up());
		emit_button(state, raw_events, GamepadButton::DPadDown, &dpad.down());
		emit_button(state, raw_events, GamepadButton::DPadLeft, &dpad.left());
		emit_button(state, raw_events, GamepadButton::DPadRight, &dpad.right());
	}
}

/// Xbox View (opposite Menu/Start). `buttonOptions` is optional and often nil on
/// USB Xbox; macOS also binds Options to a screenshot gesture unless disabled.
fn emit_select(
	state: &mut PadState,
	_pad: &GCExtendedGamepad,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	if !state.logged_select {
		state.logged_select = true;
		let name = controller_name(&state.controller.0);
		match state.select.as_ref() {
			Some(button) => {
				let bound = unsafe { button.0.isBoundToSystemGesture() };
				debug!("pad_select: {name} View/Options claimed bound_to_system={bound}");
			}
			None => {
				let names = unsafe { physical_button_names(&state.controller.0) };
				debug!("pad_select: {name} no View/Options/Share in profile buttons={names}");
			}
		}
	}
	let Some(select) = state.select.as_ref().map(|select| select.0.clone()) else {
		return;
	};
	unsafe {
		emit_button(state, raw_events, GamepadButton::Select, &select);
	}
}

/// # Safety
/// `controller` / `pad` must be a live connected pair.
unsafe fn select_button(
	controller: &GCController,
	pad: &GCExtendedGamepad,
) -> Option<Retained<GCControllerButtonInput>> {
	if let Some(options) = unsafe { pad.buttonOptions() } {
		return Some(options);
	}
	if let Some(options) = unsafe { profile_button(controller, GCInputButtonOptions) } {
		return Some(options);
	}
	if let Some(options) = unsafe { profile_button_named(controller, ns_string!("Button Options")) }
	{
		return Some(options);
	}
	// USB Xbox sometimes exposes View only as Share. Prefer Options when present
	// so the Series capture button stays a system screenshot.
	if let Some(share) = unsafe { profile_button(controller, GCInputButtonShare) } {
		return Some(share);
	}
	if let Some(share) = unsafe { profile_button_named(controller, ns_string!("Button Share")) } {
		return Some(share);
	}
	if let Some(xbox) = pad.downcast_ref::<GCXboxGamepad>() {
		return unsafe { xbox.buttonShare() };
	}
	None
}

/// # Safety
/// `controller` must be a live `GCController`.
unsafe fn profile_button(
	controller: &GCController,
	name: Option<&NSString>,
) -> Option<Retained<GCControllerButtonInput>> {
	unsafe { profile_button_named(controller, name?) }
}

/// # Safety
/// `controller` must be a live `GCController`.
unsafe fn profile_button_named(
	controller: &GCController,
	name: &NSString,
) -> Option<Retained<GCControllerButtonInput>> {
	let element = unsafe { controller.physicalInputProfile().objectForKeyedSubscript(name) }?;
	element.downcast().ok()
}

/// # Safety
/// `controller` must be a live `GCController`.
unsafe fn physical_button_names(controller: &GCController) -> String {
	let buttons = unsafe { controller.physicalInputProfile().buttons() };
	let keys = buttons.allKeys();
	(0..keys.count())
		.map(|index| keys.objectAtIndex(index).to_string())
		.collect::<Vec<_>>()
		.join(",")
}

/// # Safety
/// `input` must be a live button from a connected controller.
unsafe fn claim_for_app(input: &GCControllerButtonInput) {
	unsafe { input.setPreferredSystemGestureState(GCSystemGestureState::Disabled) };
}

/// # Safety
/// `stick` must be a live direction pad from a connected `GCExtendedGamepad`.
unsafe fn emit_stick(
	state: &mut PadState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	axis_x: GamepadAxis,
	axis_y: GamepadAxis,
	stick: &GCControllerDirectionPad,
) {
	emit_axis(state, raw_events, axis_x, unsafe { stick.xAxis().value() });
	emit_axis(state, raw_events, axis_y, unsafe { stick.yAxis().value() });
}

fn emit_axis(
	state: &mut PadState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	axis: GamepadAxis,
	value: f32,
) {
	if !value_changed(state.axes.get(&axis).copied(), value) {
		return;
	}
	state.axes.insert(axis, value);
	raw_events.write(RawGamepadAxisChangedEvent::new(state.entity, axis, value).into());
}

/// # Safety
/// `input` must be a live button from a connected `GCExtendedGamepad`.
unsafe fn emit_button(
	state: &mut PadState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	button: GamepadButton,
	input: &GCControllerButtonInput,
) {
	let value = unsafe { input.value() };
	if !value_changed(state.buttons.get(&button).copied(), value) {
		return;
	}
	state.buttons.insert(button, value);
	raw_events.write(RawGamepadButtonChangedEvent::new(state.entity, button, value).into());
}

fn play_gamecontroller_rumble(
	time: Res<Time>,
	mut requests: MessageReader<GamepadRumbleRequest>,
	mut pads: ResMut<GameControllerPads>,
) {
	let now = time.elapsed_secs();
	for state in pads.pads.values_mut() {
		expire_pulses(state, now);
	}
	for request in requests.read() {
		match request {
			GamepadRumbleRequest::Stop { gamepad } => {
				if let Some(state) = pad_mut(&mut pads, *gamepad) {
					clear_pulses(state);
					state.engine = None;
				}
			}
			GamepadRumbleRequest::Add { gamepad, duration, intensity } => {
				let Some(state) = pad_mut(&mut pads, *gamepad) else {
					let known: Vec<Entity> = pads.pads.values().map(|state| state.entity).collect();
					warn!("pad_rumble: no PadState for Gamepad {gamepad:?} (known {known:?})");
					continue;
				};
				play_pulse(state, now, *intensity, *duration);
			}
		}
	}
}

fn expire_pulses(state: &mut PadState, now: f32) {
	let mut keep = Vec::new();
	for pulse in state.pulses.drain(..) {
		if now < pulse.until {
			keep.push(pulse);
			continue;
		}
		let _ = unsafe { pulse.player.0.stopAtTime_error(CHHapticTimeImmediate) };
	}
	state.pulses = keep;
}

fn clear_pulses(state: &mut PadState) {
	for pulse in state.pulses.drain(..) {
		let _ = unsafe { pulse.player.0.stopAtTime_error(CHHapticTimeImmediate) };
	}
}

fn pad_mut(pads: &mut GameControllerPads, gamepad: Entity) -> Option<&mut PadState> {
	pads.pads.values_mut().find(|state| state.entity == gamepad)
}

fn ensure_engine(state: &mut PadState) -> Option<Retained<CHHapticEngine>> {
	if let Some(engine) = &state.engine {
		return Some(engine.0.clone());
	}
	let engine = start_engine(&state.controller.0)?;
	state.engine = Some(MainThreadRc(engine.clone()));
	Some(engine)
}

fn start_engine(controller: &GCController) -> Option<Retained<CHHapticEngine>> {
	let name = controller_name(controller);
	// SAFETY: `controller` is a live `GCController` retained on `PadState`.
	let Some(haptics) = (unsafe { controller.haptics() }) else {
		warn!("pad_rumble: {name} has no GCDevice.haptics (USB Xbox on Mac often nil)");
		return None;
	};
	let Some(engine) = create_engine(&haptics) else {
		return None;
	};
	// SAFETY: new engine; `playsHapticsOnly` must be set before start.
	unsafe {
		engine.setPlaysHapticsOnly(true);
		engine.setAutoShutdownEnabled(false);
		if let Err(err) = engine.startAndReturnError() {
			warn!("pad_rumble: {name} engine start failed: {err:?}");
			return None;
		}
	}
	debug!("pad_rumble: {name} haptic engine started");
	Some(engine)
}

/// `createEngineWithLocality:` is macOS 11+, but objc2 0.3.2 only binds it on
/// iOS / tvOS / visionOS. Call it directly so Xbox pads rumble on Mac too.
fn create_engine(haptics: &GCDeviceHaptics) -> Option<Retained<CHHapticEngine>> {
	// SAFETY: Apple exports these locality strings from GameController.framework.
	let localities =
		unsafe { [("Handles", GCHapticsLocalityHandles), ("Default", GCHapticsLocalityDefault)] };
	for (label, locality) in localities {
		if let Some(engine) = create_engine_at(haptics, locality) {
			debug!("pad_rumble: engine locality={label}");
			return Some(engine);
		}
		warn!("pad_rumble: createEngineWithLocality:{label} failed");
	}
	None
}

fn create_engine_at(
	haptics: &GCDeviceHaptics,
	locality: &GCHapticsLocality,
) -> Option<Retained<CHHapticEngine>> {
	// SAFETY: GameController documents this selector on `GCDeviceHaptics`.
	let engine: Option<Retained<NSObject>> =
		unsafe { msg_send![haptics, createEngineWithLocality: locality] };
	let object = engine?;
	object.downcast().ok()
}

fn play_pulse(
	state: &mut PadState,
	now: f32,
	intensity: GamepadRumbleIntensity,
	duration: Duration,
) {
	let name = controller_name(&state.controller.0);
	let Some(engine) = ensure_engine(state) else {
		warn!("pad_rumble: no haptic engine for {name}");
		return;
	};
	let weak = intensity.weak_motor.clamp(0.0, 1.0);
	let strong = intensity.strong_motor.clamp(0.0, 1.0);
	let total = weak + strong;
	if total <= 0.0 || duration.is_zero() {
		warn!("pad_rumble: {name} play skipped (zero intensity or duration)");
		return;
	}
	let haptic_intensity = total.min(1.0);
	// Xbox rumble motors want low sharpness; 0.7 reads as a click, not a shake.
	let sharpness = (0.12 + 0.25 * (weak / total)).clamp(0.0, 0.45);
	let secs = duration.as_secs_f64().max(0.08);
	let Some(pattern) = haptic_pattern(haptic_intensity, sharpness, secs) else {
		warn!("pad_rumble: {name} pattern build failed");
		return;
	};
	// SAFETY: keep the player retained until `until` or Core Haptics cancels on drop.
	let result = unsafe { engine.createPlayerWithPattern_error(&pattern) };
	let player = match result {
		Ok(player) => player,
		Err(err) => {
			warn!("pad_rumble: {name} player create failed: {err:?}");
			return;
		}
	};
	if let Err(err) = unsafe { player.startAtTime_error(CHHapticTimeImmediate) } {
		warn!("pad_rumble: {name} play failed: {err:?}");
		return;
	}
	let until = now + duration.as_secs_f32().max(0.08);
	debug!(
		"pad_rumble: {name} play ok duration={duration:?} intensity={haptic_intensity:.2} sharpness={sharpness:.2} live_until={until:.3}"
	);
	state.pulses.push(LivePulse { player: MainThreadRc(player), until });
}

fn haptic_pattern(
	intensity: f32,
	sharpness: f32,
	duration: f64,
) -> Option<Retained<CHHapticPattern>> {
	// SAFETY: Core Haptics inits copy the parameter values into the event.
	unsafe {
		let intensity = CHHapticEventParameter::initWithParameterID_value(
			CHHapticEventParameter::alloc(),
			CHHapticEventParameterIDHapticIntensity,
			intensity,
		);
		let sharpness = CHHapticEventParameter::initWithParameterID_value(
			CHHapticEventParameter::alloc(),
			CHHapticEventParameterIDHapticSharpness,
			sharpness,
		);
		let params = NSArray::from_retained_slice(&[intensity, sharpness]);
		let event = CHHapticEvent::initWithEventType_parameters_relativeTime_duration(
			CHHapticEvent::alloc(),
			CHHapticEventTypeHapticContinuous,
			&params,
			0.0,
			duration,
		);
		let events = NSArray::from_retained_slice(&[event]);
		let parameters = NSArray::<CHHapticDynamicParameter>::from_retained_slice(&[]);
		CHHapticPattern::initWithEvents_parameters_error(
			CHHapticPattern::alloc(),
			&events,
			&parameters,
		)
		.map_err(|err| {
			warn!("pad_rumble: CHHapticPattern init failed: {err:?}");
			err
		})
		.ok()
	}
}

#[cfg(test)]
mod value_changed_tests {
	use super::value_changed;

	#[test]
	fn first_sample_always_emits() -> anyhow::Result<()> {
		assert!(value_changed(None, 0.0));
		Ok(())
	}

	#[test]
	fn tiny_noise_is_ignored() -> anyhow::Result<()> {
		assert!(!value_changed(Some(0.5), 0.5 + 1e-5));
		Ok(())
	}

	#[test]
	fn stick_deflection_emits() -> anyhow::Result<()> {
		assert!(value_changed(Some(0.0), 0.2));
		Ok(())
	}
}
