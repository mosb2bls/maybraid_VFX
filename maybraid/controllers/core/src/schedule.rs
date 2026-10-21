//! Explicit controller-lane stages. Plugin install order is not the contract.

use bevy::prelude::*;

/// Update-schedule stages of the controller lane.
///
/// Target order:
/// `VirtualPad` (PreUpdate) → [`Bind`](Self::Bind) → camera / item branch →
/// [`Interpret`](Self::Interpret) → [`MotorRequest`](Self::MotorRequest) →
/// [`Motor`](Self::Motor).
///
/// Pad production stays on `VirtualPadSystems` in PreUpdate. Camera look and
/// item intents read after [`Self::Bind`] and must not write the shared motor
/// wish.
///
/// Shared stages stay generic. Domain ordering (ordinary locomotion, Tanz,
/// world filters) lives beside those domain systems, not in this crate.
///
/// [`Self::Interpret`] is reserved for later per-actor interpreters that do
/// not also write `MoveWish`. Phase 1 registers the graph so later PRs can
/// move writers into the named sets.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControllerPipeline {
	/// Pad → routed character / menu samples.
	Bind,
	/// Per-actor interpreters. Empty in phase 1.
	Interpret,
	/// One resolved motor request per actor (`MoveWish` / `JumpWish` / stance).
	MotorRequest,
	/// Shared capsule motor (`PlayerSystems::Body`).
	Motor,
}

impl ControllerPipeline {
	/// Install the stage graph. Safe to call from every plugin on the lane.
	pub fn configure(app: &mut App) {
		app.configure_sets(
			Update,
			(
				Self::Bind,
				Self::Interpret.after(Self::Bind),
				Self::MotorRequest.after(Self::Interpret),
				Self::Motor.after(Self::MotorRequest),
			)
				.chain(),
		);
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
	fn stages_run_bind_interpret_motor_request_motor() {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<StageOrder>().add_systems(
			Update,
			(
				note("bind").in_set(ControllerPipeline::Bind),
				note("interpret").in_set(ControllerPipeline::Interpret),
				note("request").in_set(ControllerPipeline::MotorRequest),
				note("motor").in_set(ControllerPipeline::Motor),
			),
		);
		app.update();
		assert_eq!(
			app.world().resource::<StageOrder>().0,
			["bind", "interpret", "request", "motor"]
		);
	}
}
