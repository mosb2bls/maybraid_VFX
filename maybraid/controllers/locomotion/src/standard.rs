//! Broadcast [`CharacterIntent`] → one actor-local Standard [`MotorIntent`].

use bevy::prelude::*;
use maybraid_character_controller::{
	CharacterIntent, OrdinaryLocomotion, OrdinaryLocomotionEnabled,
};
use maybraid_controllers_core::LocalActor;
use player::{
	CharacterController, MotorIntent, MotorSample, Player, PlayerLook, SpeedCapRequest,
	SprintRequest, StanceRequest,
};

const FOCUS_EPS: f32 = 1e-6;

/// Interpret pad locomotion for the local [`Player`] only.
pub fn interpret_standard_locomotion(
	mouse: Res<ButtonInput<MouseButton>>,
	ordinary_pad: Option<Res<OrdinaryLocomotionEnabled>>,
	local: Option<Res<LocalActor>>,
	mut intents: MessageReader<CharacterIntent>,
	mut actors: Query<
		(Entity, &mut MotorIntent, &PlayerLook, Option<&OrdinaryLocomotion>),
		(With<CharacterController>, With<Player>),
	>,
) {
	let pad_ordinary = ordinary_pad.is_none_or(|enabled| enabled.0);
	let mut move_stick = Vec2::ZERO;
	let mut jump = false;
	let mut focusing = mouse.pressed(MouseButton::Right);
	let mut start_sprint = false;
	let mut stop_sprint = false;
	let mut change_squat = false;
	let mut change_prone = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Move(value) => move_stick = value,
			CharacterIntent::Jump => jump = true,
			CharacterIntent::Focus(value) if value > FOCUS_EPS => focusing = true,
			CharacterIntent::StartSprint => start_sprint = true,
			CharacterIntent::StopSprint => stop_sprint = true,
			CharacterIntent::ChangeSquat => change_squat = true,
			CharacterIntent::ChangeProne => change_prone = true,
			_ => {}
		}
	}

	let Some(target) = targeted_player(local.as_deref(), &actors) else {
		return;
	};
	let Ok((_, mut intent, look, ordinary)) = actors.get_mut(target) else {
		return;
	};
	let ordinary = pad_ordinary && ordinary.is_none_or(|enabled| enabled.0);
	let sprint = if focusing {
		SprintRequest::Release
	} else if ordinary && start_sprint {
		SprintRequest::Start
	} else if ordinary && stop_sprint {
		SprintRequest::Stop
	} else {
		SprintRequest::Neutral
	};
	let stance = if ordinary && start_sprint {
		StanceRequest::Stand
	} else if ordinary && change_prone {
		StanceRequest::ChangeProne
	} else if ordinary && change_squat {
		StanceRequest::ChangeSquat
	} else {
		StanceRequest::Neutral
	};
	intent.set_standard(MotorSample {
		translation: if ordinary { look.wish_from_stick(move_stick) } else { Vec3::ZERO },
		jump: ordinary && jump,
		sprint,
		stance,
		speed_cap: SpeedCapRequest::Release,
	});
}

fn targeted_player(
	local: Option<&LocalActor>,
	actors: &Query<
		(Entity, &mut MotorIntent, &PlayerLook, Option<&OrdinaryLocomotion>),
		(With<CharacterController>, With<Player>),
	>,
) -> Option<Entity> {
	if let Some(local) = local {
		if actors.contains(local.entity()) {
			return Some(local.entity());
		}
	}
	actors.iter().map(|(entity, ..)| entity).min_by_key(|entity| entity.to_bits())
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::{anyhow, Result};
	use player::MoveWish;

	fn loco_app() -> (App, Entity, Entity) {
		let mut app = App::new();
		app.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<CharacterIntent>()
			.add_systems(Update, interpret_standard_locomotion);
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
				MoveWish::default(),
				MotorIntent::default(),
				PlayerLook::default(),
			))
			.id();
		(app, player, other)
	}

	fn write_intents(app: &mut App, intents: &[CharacterIntent]) {
		for intent in intents {
			app.world_mut().write_message(*intent);
		}
	}

	#[test]
	fn writes_only_the_local_actor() -> Result<()> {
		let (mut app, player, other) = loco_app();
		app.world_mut().insert_resource(LocalActor::new(player));
		write_intents(&mut app, &[CharacterIntent::Move(Vec2::Y)]);
		app.update();
		let targeted = app.world().get::<MotorIntent>(player).ok_or_else(|| anyhow!("player"))?;
		let ignored = app.world().get::<MotorIntent>(other).ok_or_else(|| anyhow!("other"))?;
		assert_eq!(targeted.standard().map(|sample| sample.translation), Some(-Vec3::Z));
		assert!(ignored.standard().is_none());
		Ok(())
	}

	#[test]
	fn focus_releases_sprint() -> Result<()> {
		let (mut app, player, _) = loco_app();
		app.world_mut().insert_resource(LocalActor::new(player));
		write_intents(&mut app, &[CharacterIntent::Focus(1.0), CharacterIntent::StartSprint]);
		app.update();
		let sample = app
			.world()
			.get::<MotorIntent>(player)
			.and_then(MotorIntent::standard)
			.ok_or_else(|| anyhow!("standard sample"))?;
		assert_eq!(sample.sprint, SprintRequest::Release);
		Ok(())
	}

	#[test]
	fn disabled_ordinary_writes_a_neutral_standard_slot() -> Result<()> {
		let (mut app, player, _) = loco_app();
		app.world_mut().insert_resource(LocalActor::new(player));
		app.world_mut().entity_mut(player).insert(OrdinaryLocomotion(false));
		write_intents(&mut app, &[CharacterIntent::Move(Vec2::Y), CharacterIntent::Jump]);
		app.update();
		let sample = app
			.world()
			.get::<MotorIntent>(player)
			.and_then(MotorIntent::standard)
			.ok_or_else(|| anyhow!("standard sample"))?;
		assert_eq!(sample.translation, Vec3::ZERO);
		assert!(!sample.jump);
		Ok(())
	}
}
