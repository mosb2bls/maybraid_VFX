//! Ordinary locomotion: character intents → actor-local [`player::MotorIntent`].

pub mod standard;

pub use maybraid_controllers_core::{ActorTarget, LocalActor};
pub use standard::interpret_standard_locomotion;

use bevy::prelude::*;
use maybraid_character_controller::{
	ActorControlSample, CharacterBindSystems, CharacterControlSystems, CharacterIntent,
	RoutedActorControl,
};
use maybraid_controllers_core::{
	CaptureResolveSystems, CaptureStack, ControllerPipeline, ControllersCorePlugin, InputSource,
	MotorRequestStage, Routed,
};
use maybraid_input::VirtualPad;
use player::Player;

/// Writes the Standard motor slot for the local player.
pub struct LocomotionControllerPlugin;

impl Plugin for LocomotionControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		app.init_resource::<ButtonInput<MouseButton>>()
			.init_resource::<RoutedActorControl>()
			.add_message::<CharacterIntent>()
			.add_systems(
				Update,
				route_actor_control_samples
					.in_set(ControllerPipeline::Bind)
					.after(CaptureResolveSystems)
					.before(CharacterControlSystems)
					.before(CharacterBindSystems::Route),
			)
			.add_systems(Update, interpret_standard_locomotion.in_set(MotorRequestStage::Standard));
	}
}

/// Route the local pad onto one player. Hosts may set [`LocalActor`]; otherwise
/// this is the lowest-id [`Player`].
pub fn route_actor_control_samples(
	pad: Option<Res<VirtualPad>>,
	local: Option<Res<LocalActor>>,
	capture: Option<Res<CaptureStack>>,
	players: Query<Entity, With<Player>>,
	mut routed: ResMut<RoutedActorControl>,
) {
	let source = InputSource::local_pad();
	let assigned = LocalActor::resolve(local.as_deref(), players.iter()).map(ActorTarget::new);
	let delivered = capture
		.as_deref()
		.map(|stack| stack.delivered_actor(source, assigned))
		.unwrap_or(assigned);
	let Some(target) = delivered else {
		routed.clear();
		return;
	};
	let Some(pad) = pad else {
		routed.clear();
		return;
	};
	routed.set(Routed::to_actor(source, target, ActorControlSample::from_pad(&pad)));
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_character_controller::CharacterIntent;
	use maybraid_controllers_core::ControllerPipeline;
	use player::{
		CharacterController, MotorIntent, MoveWish, Player, PlayerLook, Sprinting, apply_motor,
	};

	fn motor_app() -> App {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<CharacterIntent>()
			.add_systems(
				Update,
				(
					interpret_standard_locomotion.in_set(MotorRequestStage::Standard),
					apply_motor.in_set(ControllerPipeline::Motor),
				),
			);
		app
	}

	#[test]
	fn produce_to_apply_writes_one_actor_wish() -> anyhow::Result<()> {
		let mut app = motor_app();
		let player = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish::default(),
				MotorIntent::default(),
				PlayerLook::default(),
			))
			.id();
		let other = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish(Vec3::X),
				MotorIntent::default(),
				PlayerLook::default(),
			))
			.id();
		app.world_mut().insert_resource(LocalActor::new(player));
		app.world_mut().write_message(CharacterIntent::Move(Vec2::Y));
		app.update();
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(-Vec3::Z));
		assert!(
			app.world()
				.get::<MotorIntent>(other)
				.is_some_and(|intent| intent.standard().is_none()),
			"pad locomotion addresses one ActorTarget"
		);
		assert_eq!(
			app.world().get::<MoveWish>(other).map(|wish| wish.0),
			Some(Vec3::ZERO),
			"untargeted actor releases to idle instead of keeping a stale wish"
		);
		Ok(())
	}

	#[test]
	fn release_semantics_drop_sprint_when_focus_is_held() -> anyhow::Result<()> {
		let mut app = motor_app();
		let player = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish::default(),
				MotorIntent::default(),
				PlayerLook::default(),
				Sprinting,
			))
			.id();
		app.world_mut().insert_resource(LocalActor::new(player));
		app.world_mut().write_message(CharacterIntent::Focus(1.0));
		app.world_mut().write_message(CharacterIntent::StartSprint);
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			anyhow::bail!("Focus must release Sprinting on the apply path");
		}
		Ok(())
	}

	#[test]
	fn pipeline_keeps_standard_before_apply() {
		#[derive(Resource, Default)]
		struct StageOrder(Vec<&'static str>);

		fn note(label: &'static str) -> impl FnMut(ResMut<StageOrder>) {
			move |mut order: ResMut<StageOrder>| order.0.push(label)
		}

		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<StageOrder>().add_systems(
			Update,
			(
				note("standard").in_set(MotorRequestStage::Standard),
				note("tanz").in_set(MotorRequestStage::Tanz),
				note("world").in_set(MotorRequestStage::WorldFilter),
				note("motor").in_set(ControllerPipeline::Motor),
			),
		);
		app.update();
		assert_eq!(app.world().resource::<StageOrder>().0, ["standard", "tanz", "world", "motor"]);
	}

	#[test]
	fn plugin_installs_without_a_second_character_plugin() {
		let mut app = App::new();
		app.add_plugins(LocomotionControllerPlugin);
		assert!(app.is_plugin_added::<ControllersCorePlugin>());
	}

	#[test]
	fn route_samples_address_the_local_player_only() -> anyhow::Result<()> {
		use maybraid_character_controller::RoutedActorControl;
		use maybraid_input::{PadButton, VirtualPad};

		let mut app = App::new();
		app.init_resource::<RoutedActorControl>()
			.init_resource::<VirtualPad>()
			.add_systems(Update, route_actor_control_samples);
		let player = app.world_mut().spawn(Player).id();
		let other = app.world_mut().spawn(Player).id();
		app.world_mut().insert_resource(LocalActor::new(player));
		{
			let mut pad = app.world_mut().resource_mut::<VirtualPad>();
			pad.begin_frame();
			pad.add_move(Vec2::NEG_X);
			pad.add_look_throw(Vec2::Y);
			pad.hold_digital(PadButton::A);
			pad.finish_digital();
		}
		app.update();
		let routed = app.world().resource::<RoutedActorControl>();
		assert!(routed.addresses(player));
		assert!(!routed.addresses(other));
		let sample = routed.payload().ok_or_else(|| anyhow::anyhow!("sample"))?;
		assert_eq!(sample.move_stick(), Vec2::NEG_X);
		assert_eq!(sample.look_throw(), Vec2::Y);
		assert!(sample.a().just_pressed());
		Ok(())
	}
}
