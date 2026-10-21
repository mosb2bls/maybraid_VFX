//! Explicit controller-lane stages. Plugin install order is not the contract.

use bevy::prelude::*;

/// Capture claims written before actor samples are produced.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureResolveSystems;

/// Update-schedule stages of the controller lane.
///
/// Target order:
/// `VirtualPad` (PreUpdate) → [`Bind`](Self::Bind) → camera / item branch →
/// [`ResetMotorRequests`](Self::ResetMotorRequests) → [`MotorRequest`](Self::MotorRequest) →
/// [`Motor`](Self::Motor).
///
/// Pad production stays on `VirtualPadSystems` in PreUpdate. Camera look and
/// item intents read after [`Self::Bind`] and must not write the shared motor
/// wish.
///
/// Shared stages stay generic. Domain ordering (ordinary locomotion, Tanz,
/// world filters) lives beside those domain systems, not in this crate.
///
/// [`CaptureResolveSystems`] runs first inside [`Self::Bind`] so actor
/// delivery sees this frame's capture. [`Self::ResetMotorRequests`] clears
/// per-actor motor slots so writers in [`Self::MotorRequest`] declare this
/// frame only.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControllerPipeline {
	/// Pad → routed character / menu samples.
	Bind,
	/// Clear motor slots. Composition happens later; this stage does not apply.
	ResetMotorRequests,
	/// Declared motor requests / modifiers. Domain sets order writers.
	MotorRequest,
	/// One compose + apply, then the shared capsule motor (`PlayerSystems::Body`).
	Motor,
}

impl ControllerPipeline {
	/// Install the stage graph. Safe to call from every plugin on the lane.
	pub fn configure(app: &mut App) {
		app.configure_sets(
			Update,
			(
				Self::Bind,
				Self::ResetMotorRequests.after(Self::Bind),
				Self::MotorRequest.after(Self::ResetMotorRequests),
				Self::Motor.after(Self::MotorRequest),
			)
				.chain(),
		)
		.configure_sets(Update, CaptureResolveSystems.in_set(Self::Bind));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Resource, Default)]
	struct StageOrder(Vec<&'static str>);

	fn note(label: &'static str) -> impl FnMut(ResMut<StageOrder>) {
		move |mut order: ResMut<StageOrder>| order.0.push(label)
	}

	#[test]
	fn stages_run_bind_reset_motor_request_motor() {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<StageOrder>().add_systems(
			Update,
			(
				note("bind").in_set(ControllerPipeline::Bind),
				note("reset").in_set(ControllerPipeline::ResetMotorRequests),
				note("request").in_set(ControllerPipeline::MotorRequest),
				note("motor").in_set(ControllerPipeline::Motor),
			),
		);
		app.update();
		assert_eq!(
			app.world().resource::<StageOrder>().0,
			["bind", "reset", "request", "motor"]
		);
	}

	#[test]
	fn capture_resolve_runs_inside_bind() {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<StageOrder>().add_systems(
			Update,
			(
				note("capture").in_set(CaptureResolveSystems),
				note("bind").in_set(ControllerPipeline::Bind).after(CaptureResolveSystems),
			),
		);
		app.update();
		assert_eq!(app.world().resource::<StageOrder>().0, ["capture", "bind"]);
	}
}
