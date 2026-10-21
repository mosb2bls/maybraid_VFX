//! Ordinary locomotion: character intents → actor-local [`player::MotorIntent`].

pub mod standard;

pub use maybraid_controllers_core::{ActorTarget, InputRouter};
pub use standard::interpret_standard_locomotion;

use bevy::prelude::*;
use maybraid_character_controller::CharacterIntent;
use maybraid_controllers_core::ControllersCorePlugin;
use player::MotorRequestStage;

/// Writes the Standard motor slot for the routed local actor.
pub struct LocomotionControllerPlugin;

impl Plugin for LocomotionControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		MotorRequestStage::configure(app);
		app.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<CharacterIntent>()
			.add_systems(Update, interpret_standard_locomotion.in_set(MotorRequestStage::Standard));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::{anyhow, Result};
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_character_controller::CharacterIntent;
	use maybraid_controllers_core::{
		ActorTarget, CaptureKind, ControllerPipeline, InputRouter, InputRouting, InputSource,
		RoutingEvent, ScopeOwner, UiTarget,
	};
	use player::{
		apply_motor, apply_routing_hold_effects, compose_motor, CharacterController, MotorIntent,
		MotorSelection, MoveWish, Player, PlayerLook, ResolvedMotorRequest, Sprinting,
	};

	fn assign_local(app: &mut App, entity: Entity) -> Result<()> {
		let actor = ActorTarget::new(entity);
		app.world_mut()
			.run_system_once(move |mut routing: InputRouting| {
				routing.assign_actor(InputSource::local_pad(), actor);
			})
			.map_err(|err| anyhow!("assign local actor: {err}"))?;
		Ok(())
	}

	fn publish_hold(
		app: &mut App,
		apply: impl FnOnce(&mut InputRouter) + Send + Sync + 'static,
	) -> Result<()> {
		let mut apply = Some(apply);
		app.world_mut()
			.run_system_once(move |mut routing: InputRouting| {
				if let Some(apply) = apply.take() {
					apply(&mut routing);
				}
			})
			.map_err(|err| anyhow!("publish routing hold: {err}"))?;
		Ok(())
	}

	fn motor_app() -> App {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		MotorRequestStage::configure(&mut app);
		app.init_resource::<ButtonInput<MouseButton>>()
			.init_resource::<InputRouter>()
			.add_message::<CharacterIntent>()
			.add_message::<RoutingEvent>()
			.add_systems(
				Update,
				(
					interpret_standard_locomotion.in_set(MotorRequestStage::Standard),
					apply_routing_hold_effects
						.in_set(ControllerPipeline::MotorRequest)
						.after(MotorRequestStage::WorldFilter),
					compose_motor.in_set(ControllerPipeline::Motor),
					apply_motor.in_set(ControllerPipeline::Motor).after(compose_motor),
				),
			);
		app
	}

	fn motor_bundle() -> impl Bundle {
		(
			CharacterController,
			Player,
			MoveWish::default(),
			MotorIntent::default(),
			MotorSelection::default(),
			ResolvedMotorRequest::default(),
			PlayerLook::default(),
		)
	}

	#[test]
	fn produce_to_apply_writes_one_actor_wish() -> anyhow::Result<()> {
		let mut app = motor_app();
		let player = app.world_mut().spawn(motor_bundle()).id();
		let other = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish(Vec3::X),
				MotorIntent::default(),
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
				PlayerLook::default(),
			))
			.id();
		assign_local(&mut app, player)?;
		app.world_mut().write_message(CharacterIntent::Move(Vec2::Y));
		app.update();
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(-Vec3::Z));
		assert!(
			app.world()
				.get::<MotorIntent>(other)
				.is_some_and(|intent| intent.standard().is_none()),
			"pad locomotion addresses InputRouter::delivered_local_actor"
		);
		assert_eq!(
			app.world().get::<MoveWish>(other).map(|wish| wish.0),
			Some(Vec3::ZERO),
			"unassigned actor releases to idle instead of keeping a stale wish"
		);
		Ok(())
	}

	#[test]
	fn release_semantics_drop_sprint_when_focus_is_held() -> anyhow::Result<()> {
		let mut app = motor_app();
		let player = app.world_mut().spawn((motor_bundle(), Sprinting)).id();
		assign_local(&mut app, player)?;
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
		MotorRequestStage::configure(&mut app);
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
	fn disconnect_releases_standard_holds_via_message_reader() -> Result<()> {
		let mut app = motor_app();
		let player = app.world_mut().spawn((motor_bundle(), Sprinting)).id();
		assign_local(&mut app, player)?;
		app.world_mut().write_message(CharacterIntent::StartSprint);
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			anyhow::bail!("precondition: sprint hold");
		}
		publish_hold(&mut app, |router| {
			router.disconnect(InputSource::local_pad());
		})?;
		app.world_mut().write_message(CharacterIntent::Move(Vec2::Y));
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			anyhow::bail!("disconnect ReleaseNeutral must drop Sprinting");
		}
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		Ok(())
	}

	#[test]
	fn capture_releases_standard_holds_via_message_reader() -> Result<()> {
		let mut app = motor_app();
		let player = app.world_mut().spawn((motor_bundle(), Sprinting)).id();
		let menu = app.world_mut().spawn_empty().id();
		assign_local(&mut app, player)?;
		app.world_mut().write_message(CharacterIntent::StartSprint);
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			anyhow::bail!("precondition: sprint hold");
		}
		publish_hold(&mut app, move |router| {
			router.acquire_capture(
				InputSource::local_pad(),
				ScopeOwner::ui(UiTarget::new(menu)),
				CaptureKind::MenuNav,
			);
		})?;
		app.world_mut().write_message(CharacterIntent::Move(Vec2::Y));
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			anyhow::bail!("capture ReleaseNeutral must drop Sprinting");
		}
		assert_eq!(
			app.world().resource::<InputRouter>().local_actor(),
			Some(ActorTarget::new(player)),
			"capture keeps assignment"
		);
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		Ok(())
	}

	#[test]
	fn plugin_installs_without_a_second_character_plugin() {
		let mut app = App::new();
		app.add_plugins(LocomotionControllerPlugin);
		assert!(app.is_plugin_added::<ControllersCorePlugin>());
	}
}
