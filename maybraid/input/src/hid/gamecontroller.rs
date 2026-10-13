//! Poll Apple `GCController` into Bevy raw gamepad messages.
//!
//! Native GameController and Core Haptics objects live in thread-local storage on
//! the main thread. ECS only tracks per-controller routing (entities and last
//! input samples).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ptr::from_ref;
use std::time::Duration;

use bevy::ecs::system::NonSendMarker;
use bevy::input::gamepad::{
	GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent, GamepadRumbleIntensity,
	GamepadRumbleRequest, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
	RawGamepadEvent,
};
use bevy::input::InputSystems;
use bevy::prelude::*;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{msg_send, AnyThread, MainThreadMarker, Message};
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

thread_local! {
	static NATIVE_PADS: RefCell<HashMap<usize, PadNativeState>> = RefCell::new(HashMap::new());
}

fn with_native_pads<R>(f: impl FnOnce(&mut HashMap<usize, PadNativeState>) -> R) -> R {
	NATIVE_PADS.with(|cell| f(&mut *cell.borrow_mut()))
}

fn value_changed(previous: Option<f32>, value: f32) -> bool {
	const EPS: f32 = 1e-4;
	previous.is_none_or(|old| (old - value).abs() > EPS)
}

#[inline]
fn assert_apple_main_thread() {
	let _mtm = MainThreadMarker::new().expect("GameController backend must run on the main thread");
	#[cfg(test)]
	scheduling_tests::mark_main_thread();
}

pub struct GameControllerPlugin;

impl Plugin for GameControllerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GameControllerRouting>()
			.add_systems(
				PreStartup,
				(reset_gamecontroller_backend, enable_background_events, poll_game_controllers)
					.chain(),
			)
			.add_systems(PreUpdate, poll_game_controllers.before(InputSystems))
			.add_systems(PostUpdate, play_gamecontroller_rumble.in_set(PadRumbleSystems::Play))
			.add_systems(Last, shutdown_gamecontroller_backend);
	}
}

/// Sendable routing state: Bevy entities and last-reported stick/button samples.
#[derive(Default, Resource)]
struct GameControllerRouting {
	by_controller_id: HashMap<usize, PadInputState>,
	entity_to_controller_id: HashMap<Entity, usize>,
}

struct PadInputState {
	entity: Entity,
	axes: HashMap<GamepadAxis, f32>,
	buttons: HashMap<GamepadButton, f32>,
}

struct PadNativeState {
	controller: Retained<GCController>,
	engine: Option<Retained<CHHapticEngine>>,
	pulses: Vec<LivePulse>,
	select: Option<Retained<GCControllerButtonInput>>,
	logged_select: bool,
}

struct LivePulse {
	player: Retained<ProtocolObject<dyn CHHapticPatternPlayer>>,
	until: f32,
}

fn controller_id(controller: &GCController) -> usize {
	from_ref(controller) as usize
}

fn reset_gamecontroller_backend(
	_non_send: NonSendMarker,
	mut routing: ResMut<GameControllerRouting>,
) {
	assert_apple_main_thread();
	teardown_all_native_pads();
	routing.by_controller_id.clear();
	routing.entity_to_controller_id.clear();
}

fn shutdown_gamecontroller_backend(_non_send: NonSendMarker) {
	assert_apple_main_thread();
	teardown_all_native_pads();
}

fn teardown_all_native_pads() {
	with_native_pads(|pads| {
		for (_, mut state) in pads.drain() {
			teardown_native_pad(&mut state);
		}
	});
}

fn teardown_native_pad(state: &mut PadNativeState) {
	clear_pulses(state);
	state.engine = None;
	state.select = None;
}

fn enable_background_events(_non_send: NonSendMarker) {
	assert_apple_main_thread();
	// SAFETY: class method; must run on the main thread.
	unsafe {
		GCController::setShouldMonitorBackgroundEvents(true);
	}
}

fn poll_game_controllers(
	_non_send: NonSendMarker,
	mut commands: Commands,
	mut routing: ResMut<GameControllerRouting>,
	mut connection_events: MessageWriter<GamepadConnectionEvent>,
	mut raw_events: MessageWriter<RawGamepadEvent>,
) {
	assert_apple_main_thread();
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
			&mut routing,
			&mut connection_events,
			&mut raw_events,
		);
	}

	let gone: Vec<usize> = routing
		.by_controller_id
		.keys()
		.copied()
		.filter(|id| !seen.contains(id))
		.collect();
	for id in gone {
		let Some(input) = routing.by_controller_id.remove(&id) else {
			continue;
		};
		routing.entity_to_controller_id.remove(&input.entity);
		with_native_pads(|pads| {
			if let Some(mut native) = pads.remove(&id) {
				teardown_native_pad(&mut native);
			}
		});
		let event = GamepadConnectionEvent::new(input.entity, GamepadConnection::Disconnected);
		connection_events.write(event.clone());
		raw_events.write(event.into());
	}
}

fn sync_controller(
	controller: &GCController,
	id: usize,
	commands: &mut Commands,
	routing: &mut GameControllerRouting,
	connection_events: &mut MessageWriter<GamepadConnectionEvent>,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: live `GCController` from `controllers()`.
	let Some(extended) = (unsafe { controller.extendedGamepad() }) else {
		return;
	};
	let is_new = !routing.by_controller_id.contains_key(&id);
	if is_new {
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
		routing.entity_to_controller_id.insert(entity, id);
		routing
			.by_controller_id
			.insert(id, PadInputState { entity, axes: HashMap::new(), buttons: HashMap::new() });
		with_native_pads(|pads| {
			pads.insert(
				id,
				PadNativeState {
					controller: controller.retain(),
					engine: None,
					pulses: Vec::new(),
					select,
					logged_select: false,
				},
			);
		});
	}
	let input = routing
		.by_controller_id
		.get_mut(&id)
		.expect("routing and native maps stay in sync for connected controllers");
	with_native_pads(|pads| {
		let native = pads.get_mut(&id).expect("native map matches routing");
		emit_extended(input, native, controller, &extended, raw_events);
	});
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
	input: &mut PadInputState,
	native: &mut PadNativeState,
	controller: &GCController,
	pad: &GCExtendedGamepad,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: `pad` is the extended profile of a connected controller.
	unsafe {
		emit_stick(
			input,
			raw_events,
			GamepadAxis::LeftStickX,
			GamepadAxis::LeftStickY,
			&pad.leftThumbstick(),
		);
		emit_stick(
			input,
			raw_events,
			GamepadAxis::RightStickX,
			GamepadAxis::RightStickY,
			&pad.rightThumbstick(),
		);
		emit_button(input, raw_events, GamepadButton::South, &pad.buttonA());
		emit_button(input, raw_events, GamepadButton::East, &pad.buttonB());
		emit_button(input, raw_events, GamepadButton::West, &pad.buttonX());
		emit_button(input, raw_events, GamepadButton::North, &pad.buttonY());
		emit_button(input, raw_events, GamepadButton::LeftTrigger, &pad.leftShoulder());
		emit_button(input, raw_events, GamepadButton::RightTrigger, &pad.rightShoulder());
		emit_button(input, raw_events, GamepadButton::LeftTrigger2, &pad.leftTrigger());
		emit_button(input, raw_events, GamepadButton::RightTrigger2, &pad.rightTrigger());
		emit_button(input, raw_events, GamepadButton::Start, &pad.buttonMenu());
		emit_select(input, native, controller, pad, raw_events);
		if let Some(home) = pad.buttonHome() {
			emit_button(input, raw_events, GamepadButton::Mode, &home);
		}
		if let Some(click) = pad.leftThumbstickButton() {
			emit_button(input, raw_events, GamepadButton::LeftThumb, &click);
		}
		if let Some(click) = pad.rightThumbstickButton() {
			emit_button(input, raw_events, GamepadButton::RightThumb, &click);
		}
		let dpad = pad.dpad();
		emit_button(input, raw_events, GamepadButton::DPadUp, &dpad.up());
		emit_button(input, raw_events, GamepadButton::DPadDown, &dpad.down());
		emit_button(input, raw_events, GamepadButton::DPadLeft, &dpad.left());
		emit_button(input, raw_events, GamepadButton::DPadRight, &dpad.right());
	}
}

/// Xbox View (opposite Menu/Start). `buttonOptions` is optional and often nil on
/// USB Xbox; macOS also binds Options to a screenshot gesture unless disabled.
fn emit_select(
	input: &mut PadInputState,
	native: &mut PadNativeState,
	controller: &GCController,
	_pad: &GCExtendedGamepad,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	if !native.logged_select {
		native.logged_select = true;
		let name = controller_name(controller);
		match native.select.as_ref() {
			Some(button) => {
				let bound = unsafe { button.isBoundToSystemGesture() };
				debug!("pad_select: {name} View/Options claimed bound_to_system={bound}");
			}
			None => {
				let names = unsafe { physical_button_names(controller) };
				debug!("pad_select: {name} no View/Options/Share in profile buttons={names}");
			}
		}
	}
	let Some(select) = native.select.as_ref().cloned() else {
		return;
	};
	unsafe {
		emit_button(input, raw_events, GamepadButton::Select, &select);
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
	input: &mut PadInputState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	axis_x: GamepadAxis,
	axis_y: GamepadAxis,
	stick: &GCControllerDirectionPad,
) {
	emit_axis(input, raw_events, axis_x, unsafe { stick.xAxis().value() });
	emit_axis(input, raw_events, axis_y, unsafe { stick.yAxis().value() });
}

fn emit_axis(
	input: &mut PadInputState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	axis: GamepadAxis,
	value: f32,
) {
	if !value_changed(input.axes.get(&axis).copied(), value) {
		return;
	}
	input.axes.insert(axis, value);
	raw_events.write(RawGamepadAxisChangedEvent::new(input.entity, axis, value).into());
}

/// # Safety
/// `input` must be a live button from a connected `GCExtendedGamepad`.
unsafe fn emit_button(
	input: &mut PadInputState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	button: GamepadButton,
	input_button: &GCControllerButtonInput,
) {
	let value = unsafe { input_button.value() };
	if !value_changed(input.buttons.get(&button).copied(), value) {
		return;
	}
	input.buttons.insert(button, value);
	raw_events.write(RawGamepadButtonChangedEvent::new(input.entity, button, value).into());
}

fn play_gamecontroller_rumble(
	_non_send: NonSendMarker,
	time: Res<Time>,
	mut requests: MessageReader<GamepadRumbleRequest>,
	routing: Res<GameControllerRouting>,
) {
	assert_apple_main_thread();
	let now = time.elapsed_secs();
	with_native_pads(|pads| {
		for state in pads.values_mut() {
			expire_pulses(state, now);
		}
	});
	for request in requests.read() {
		match request {
			GamepadRumbleRequest::Stop { gamepad } => {
				let Some(id) = routing.entity_to_controller_id.get(gamepad).copied() else {
					continue;
				};
				with_native_pads(|pads| {
					if let Some(state) = pads.get_mut(&id) {
						clear_pulses(state);
						state.engine = None;
					}
				});
			}
			GamepadRumbleRequest::Add { gamepad, duration, intensity } => {
				let Some(id) = routing.entity_to_controller_id.get(gamepad).copied() else {
					let known: Vec<Entity> =
						routing.by_controller_id.values().map(|state| state.entity).collect();
					warn!("pad_rumble: no PadState for Gamepad {gamepad:?} (known {known:?})");
					continue;
				};
				with_native_pads(|pads| {
					let Some(state) = pads.get_mut(&id) else {
						warn!("pad_rumble: missing native state for controller id {id}");
						return;
					};
					play_pulse(state, now, *intensity, *duration);
				});
			}
		}
	}
}

fn expire_pulses(state: &mut PadNativeState, now: f32) {
	let mut keep = Vec::new();
	for pulse in state.pulses.drain(..) {
		if now < pulse.until {
			keep.push(pulse);
			continue;
		}
		let _ = unsafe { pulse.player.stopAtTime_error(CHHapticTimeImmediate) };
	}
	state.pulses = keep;
}

fn clear_pulses(state: &mut PadNativeState) {
	for pulse in state.pulses.drain(..) {
		let _ = unsafe { pulse.player.stopAtTime_error(CHHapticTimeImmediate) };
	}
}

fn ensure_engine(state: &mut PadNativeState) -> Option<Retained<CHHapticEngine>> {
	if let Some(engine) = &state.engine {
		return Some(engine.clone());
	}
	let engine = start_engine(&state.controller)?;
	state.engine = Some(engine.clone());
	Some(engine)
}

fn start_engine(controller: &GCController) -> Option<Retained<CHHapticEngine>> {
	let name = controller_name(controller);
	// SAFETY: `controller` is a live `GCController` retained on the main thread.
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
	state: &mut PadNativeState,
	now: f32,
	intensity: GamepadRumbleIntensity,
	duration: Duration,
) {
	let name = controller_name(&state.controller);
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
	state.pulses.push(LivePulse { player, until });
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
mod scheduling_tests {
	use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

	use bevy::ecs::schedule::{MainThreadExecutor, MultiThreadedExecutor};
	use bevy::input::InputPlugin;
	use bevy::prelude::*;
	use bevy::tasks::{block_on, ComputeTaskPool};

	use super::{
		enable_background_events, play_gamecontroller_rumble, poll_game_controllers,
		reset_gamecontroller_backend, shutdown_gamecontroller_backend, GameControllerPlugin,
		GameControllerRouting,
	};
	use bevy::ecs::system::{IntoSystem, SystemInput};

	pub static BACKEND_MAIN_THREAD: AtomicBool = AtomicBool::new(false);
	static WORKER_TOUCHED: AtomicUsize = AtomicUsize::new(0);

	pub fn mark_main_thread() {
		BACKEND_MAIN_THREAD.store(true, Ordering::SeqCst);
	}

	fn touch_worker_counter() {
		WORKER_TOUCHED.fetch_add(1, Ordering::SeqCst);
	}

	fn on_pthread_main() -> bool {
		extern "C" {
			fn pthread_main_np() -> i32;
		}
		// SAFETY: documented thread-local query on Apple/Darwin.
		unsafe { pthread_main_np() == 1 }
	}

	fn native_system_names(schedule: &Schedule) -> Vec<String> {
		schedule
			.systems()
			.map(|systems| {
				systems
					.filter_map(|(_key, system)| {
						let name = system.name().to_string();
						if name.contains("poll_game_controllers")
							|| name.contains("enable_background_events")
							|| name.contains("play_gamecontroller_rumble")
							|| name.contains("reset_gamecontroller_backend")
							|| name.contains("shutdown_gamecontroller_backend")
						{
							Some(name)
						} else {
							None
						}
					})
					.collect()
			})
			.unwrap_or_default()
	}

	fn assert_initialized_system_is_non_send<M, I, O, S>(world: &mut World, label: &str, system: S)
	where
		I: SystemInput,
		S: IntoSystem<I, O, M>,
	{
		let mut system = IntoSystem::into_system(system);
		system.initialize(world);
		assert!(
			!system.is_send(),
			"{label} must carry NonSendMarker so the multithreaded executor keeps it off worker threads"
		);
	}

	#[test]
	fn registered_native_systems_are_non_send() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, InputPlugin));
		app.add_plugins(GameControllerPlugin);
		app.finish();
		let world = app.world_mut();
		world.init_resource::<GameControllerRouting>();
		assert_initialized_system_is_non_send(
			world,
			"reset_gamecontroller_backend",
			reset_gamecontroller_backend,
		);
		assert_initialized_system_is_non_send(
			world,
			"enable_background_events",
			enable_background_events,
		);
		assert_initialized_system_is_non_send(
			world,
			"poll_game_controllers",
			poll_game_controllers,
		);
		assert_initialized_system_is_non_send(
			world,
			"play_gamecontroller_rumble",
			play_gamecontroller_rumble,
		);
		assert_initialized_system_is_non_send(
			world,
			"shutdown_gamecontroller_backend",
			shutdown_gamecontroller_backend,
		);
	}

	#[test]
	fn native_systems_run_on_main_thread_with_multithreaded_executor() -> anyhow::Result<()> {
		if !on_pthread_main() {
			eprintln!(
				"skip native_systems_run_on_main_thread_with_multithreaded_executor: libtest worker is not the process main thread"
			);
			return Ok(());
		}

		BACKEND_MAIN_THREAD.store(false, Ordering::SeqCst);
		WORKER_TOUCHED.store(0, Ordering::SeqCst);

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, InputPlugin));
		app.insert_resource(MainThreadExecutor::new());
		let multithreaded = |schedule: &mut Schedule| {
			schedule.set_executor(MultiThreadedExecutor::new());
		};
		app.edit_schedule(PreStartup, multithreaded);
		app.edit_schedule(PreUpdate, multithreaded);
		app.edit_schedule(Update, multithreaded);
		app.edit_schedule(PostUpdate, multithreaded);
		app.edit_schedule(Last, multithreaded);
		app.add_systems(Update, touch_worker_counter);
		app.add_plugins(GameControllerPlugin);

		app.update();
		app.update();

		block_on(async {
			ComputeTaskPool::get()
				.spawn(async {
					touch_worker_counter();
				})
				.await;
		});

		let native_names: Vec<String> = app
			.world()
			.resource::<Schedules>()
			.get(PreStartup)
			.map(|schedule| native_system_names(schedule))
			.unwrap_or_default();
		assert!(
			!native_names.is_empty(),
			"PreStartup should register GameController systems: {native_names:?}"
		);
		assert!(
			BACKEND_MAIN_THREAD.load(Ordering::SeqCst),
			"GameController systems did not assert main-thread execution"
		);
		assert!(
			WORKER_TOUCHED.load(Ordering::SeqCst) > 0,
			"expected at least one Update system on the compute pool"
		);
		Ok(())
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
