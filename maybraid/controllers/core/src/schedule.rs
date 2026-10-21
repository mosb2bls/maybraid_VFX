//! Explicit controller-lane stages. Plugin install order is not the contract.

use bevy::prelude::*;

/// Capture claims written before actor samples are produced.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureResolveSystems;

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
/// [`CaptureResolveSystems`] runs first inside [`Self::Bind`] so actor
/// delivery sees this frame's capture. [`Self::Interpret`] resets per-actor
/// motor slots so writers in [`Self::MotorRequest`] declare this frame only.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControllerPipeline {
	/// Pad → routed character / menu samples.
	Bind,
	/// Clear motor slots. Selection happens in the motor resolver, not here.
	Interpret,
	/// Declared motor requests / modifiers. Named stages keep #1394 order.
	MotorRequest,
	/// One resolve + apply, then the shared capsule motor (`PlayerSystems::Body`).
	Motor,
}

/// Writers inside [`ControllerPipeline::MotorRequest`].
///
/// Order is the explicit #1394 stack: Standard, Tanz, WorldFilter. Phase 2
/// keeps the names and replaces overwrite with one resolver that reads
/// declared slots.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MotorRequestStage {
	/// Ordinary locomotion + NPC planning write `MotorIntent` slots.
	Standard,
	/// Tanzer execution writes the Tanz slot. It does not write `MoveWish`.
	Tanz,
	/// Pause / text / map declare suppress. No live vegetation motor write.
	WorldFilter,
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
		)
		.configure_sets(Update, CaptureResolveSystems.in_set(Self::Bind))
		.configure_sets(
			Update,
			(
				MotorRequestStage::Standard.in_set(Self::MotorRequest),
				MotorRequestStage::Tanz
					.in_set(Self::MotorRequest)
					.after(MotorRequestStage::Standard),
				MotorRequestStage::WorldFilter
					.in_set(Self::MotorRequest)
					.after(MotorRequestStage::Tanz),
			),
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
	fn stages_run_bind_interpret_standard_tanz_world_filter_motor() {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<StageOrder>().add_systems(
			Update,
			(
				note("bind").in_set(ControllerPipeline::Bind),
				note("interpret").in_set(ControllerPipeline::Interpret),
				note("standard").in_set(MotorRequestStage::Standard),
				note("tanz").in_set(MotorRequestStage::Tanz),
				note("world").in_set(MotorRequestStage::WorldFilter),
				note("motor").in_set(ControllerPipeline::Motor),
			),
		);
		app.update();
		assert_eq!(
			app.world().resource::<StageOrder>().0,
			["bind", "interpret", "standard", "tanz", "world", "motor"]
		);
	}
}
