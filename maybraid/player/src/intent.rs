//! Move / jump / sprint / stance from [`CharacterIntent`]. Look and use-item belong to other crates.

use bevy::prelude::*;
use maybraid_character_controller::{CharacterIntent, OrdinaryLocomotionEnabled};

use crate::body::{CharacterController, JumpWish, Jumping, MoveWish, Sprinting};
use crate::identity::{Player, PlayerLook};
use crate::stance::{CharacterStance, StanceKind};

const FOCUS_EPS: f32 = 1e-6;

pub(crate) fn apply_move_intents(
	mouse: Res<ButtonInput<MouseButton>>,
	ordinary: Option<Res<OrdinaryLocomotionEnabled>>,
	mut commands: Commands,
	mut intents: MessageReader<CharacterIntent>,
	mut wishes: Query<
		(Entity, &mut MoveWish, &PlayerLook, Option<&mut CharacterStance>, Option<&Jumping>),
		(With<CharacterController>, With<Player>),
	>,
) {
	let ordinary = ordinary.is_none_or(|enabled| enabled.0);
	let mut move_stick = Vec2::ZERO;
	let mut jump = false;
	let mut focusing = mouse.pressed(MouseButton::Right);
	let mut start_sprint = false;
	let mut stop_sprint = false;
	let mut change_squat = false;
	let mut change_prone = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Move(value) if ordinary => move_stick = value,
			CharacterIntent::Jump if ordinary => jump = true,
			CharacterIntent::Focus(value) if value > FOCUS_EPS => focusing = true,
			CharacterIntent::StartSprint if ordinary => start_sprint = true,
			CharacterIntent::StopSprint if ordinary => stop_sprint = true,
			CharacterIntent::ChangeSquat if ordinary => change_squat = true,
			CharacterIntent::ChangeProne if ordinary => change_prone = true,
			_ => {}
		}
	}

	for (entity, mut wish, look, mut stance, jumping) in &mut wishes {
		wish.0 = look_wish(look.yaw, move_stick);
		let airborne = jumping.is_some_and(Jumping::airborne);
		if jump {
			commands.entity(entity).insert(JumpWish);
		}
		if !airborne && !start_sprint {
			apply_stance_change(
				entity,
				stance.as_deref_mut(),
				change_squat,
				change_prone,
				&mut commands,
			);
		}
		if focusing {
			commands.entity(entity).remove::<Sprinting>();
		} else if start_sprint {
			if let Some(stance) = stance.as_deref_mut() {
				stance.stand();
			} else {
				commands.entity(entity).insert(CharacterStance::settled(StanceKind::Stand));
			}
			commands.entity(entity).insert(Sprinting);
		} else if stop_sprint {
			commands.entity(entity).remove::<Sprinting>();
		}
	}
}

fn apply_stance_change(
	entity: Entity,
	stance: Option<&mut CharacterStance>,
	change_squat: bool,
	change_prone: bool,
	commands: &mut Commands,
) {
	if !change_squat && !change_prone {
		return;
	}
	if let Some(stance) = stance {
		if change_prone {
			stance.change_prone();
		} else {
			stance.change_squat();
		}
		return;
	}
	let kind = if change_prone { StanceKind::Prone } else { StanceKind::Squat };
	commands.entity(entity).insert(CharacterStance::settled(kind));
}

fn look_wish(yaw: f32, stick: Vec2) -> Vec3 {
	if stick == Vec2::ZERO {
		return Vec3::ZERO;
	}
	let yaw = Quat::from_axis_angle(Vec3::Y, yaw);
	let forward = yaw * -Vec3::Z;
	let right = yaw * Vec3::X;
	(right * stick.x + forward * stick.y).clamp_length_max(1.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::{anyhow, Result};

	fn sprint_app() -> (App, Entity) {
		let mut app = App::new();
		app.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<CharacterIntent>()
			.add_systems(Update, apply_move_intents);
		let player = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish::default(),
				PlayerLook::default(),
				CharacterStance::settled(StanceKind::Stand),
			))
			.id();
		(app, player)
	}

	fn write_intents(app: &mut App, intents: &[CharacterIntent]) {
		for intent in intents {
			app.world_mut().write_message(*intent);
		}
	}

	#[test]
	fn look_wish_is_camera_relative_xz() {
		let forward = look_wish(0.0, Vec2::Y);
		assert!((forward.z + 1.0).abs() < 1e-4, "{forward}");
		assert!(forward.y.abs() < 1e-6);
		let right = look_wish(0.0, Vec2::X);
		assert!((right.x - 1.0).abs() < 1e-4, "{right}");
		assert!(right.y.abs() < 1e-6);
		let half = look_wish(0.0, Vec2::Y * 0.5);
		assert!((half.length() - 0.5).abs() < 1e-4, "stick magnitude is the throttle: {half}");
	}

	#[test]
	fn start_inserts_stop_removes() -> Result<()> {
		let (mut app, player) = sprint_app();
		write_intents(&mut app, &[CharacterIntent::StartSprint]);
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("StartSprint without Focus should insert Sprinting"));
		}

		write_intents(&mut app, &[CharacterIntent::StopSprint]);
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("StopSprint should remove Sprinting"));
		}
		Ok(())
	}

	#[test]
	fn lt_focus_drops_sprint() -> Result<()> {
		let (mut app, player) = sprint_app();
		app.world_mut().entity_mut(player).insert(Sprinting);
		write_intents(&mut app, &[CharacterIntent::Focus(1.0), CharacterIntent::StartSprint]);
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("Focus > ε must drop Sprinting even with StartSprint"));
		}
		Ok(())
	}

	#[test]
	fn ads_does_not_remove_sprinting() -> Result<()> {
		let (mut app, player) = sprint_app();
		app.world_mut().entity_mut(player).insert(Sprinting);
		write_intents(&mut app, &[CharacterIntent::Ads(1.0)]);
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("iron Ads must leave Sprinting"));
		}
		Ok(())
	}

	#[test]
	fn sprint_resumes_after_focus() -> Result<()> {
		let (mut app, player) = sprint_app();
		write_intents(&mut app, &[CharacterIntent::StartSprint]);
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("held L3 should grant Sprinting"));
		}

		write_intents(&mut app, &[CharacterIntent::Focus(1.0), CharacterIntent::StartSprint]);
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("Focus should clear Sprinting"));
		}

		write_intents(&mut app, &[CharacterIntent::StartSprint]);
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("StartSprint after Focus ends should restore Sprinting"));
		}
		Ok(())
	}

	#[test]
	fn right_mouse_focus_drops_sprint() -> Result<()> {
		let (mut app, player) = sprint_app();
		app.world_mut().entity_mut(player).insert(Sprinting);
		app.world_mut()
			.resource_mut::<ButtonInput<MouseButton>>()
			.press(MouseButton::Right);
		write_intents(&mut app, &[CharacterIntent::StartSprint]);
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("RMB focus must drop Sprinting"));
		}
		Ok(())
	}

	#[test]
	fn start_sprint_stands_and_stop_does_not_crouch() -> Result<()> {
		let (mut app, player) = sprint_app();
		app.world_mut()
			.entity_mut(player)
			.insert(CharacterStance::settled(StanceKind::Squat));
		write_intents(&mut app, &[CharacterIntent::StartSprint]);
		app.update();
		let stance = app.world().get::<CharacterStance>(player).ok_or_else(|| anyhow!("stance"))?;
		if stance.kind != StanceKind::Stand {
			return Err(anyhow!("StartSprint should stand"));
		}
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("StartSprint should still grant Sprinting"));
		}

		write_intents(&mut app, &[CharacterIntent::StopSprint]);
		app.update();
		let stance = app.world().get::<CharacterStance>(player).ok_or_else(|| anyhow!("stance"))?;
		if stance.kind != StanceKind::Stand {
			return Err(anyhow!("StopSprint must not re-crouch"));
		}
		Ok(())
	}

	#[test]
	fn change_squat_toggles_and_prone_enters() -> Result<()> {
		let (mut app, player) = sprint_app();
		write_intents(&mut app, &[CharacterIntent::ChangeSquat]);
		app.update();
		assert_eq!(
			app.world().get::<CharacterStance>(player).map(|stance| stance.kind),
			Some(StanceKind::Squat)
		);
		write_intents(&mut app, &[CharacterIntent::ChangeSquat]);
		app.update();
		assert_eq!(
			app.world().get::<CharacterStance>(player).map(|stance| stance.kind),
			Some(StanceKind::Stand)
		);
		write_intents(&mut app, &[CharacterIntent::ChangeProne]);
		app.update();
		assert_eq!(
			app.world().get::<CharacterStance>(player).map(|stance| stance.kind),
			Some(StanceKind::Prone)
		);
		write_intents(&mut app, &[CharacterIntent::ChangeSquat]);
		app.update();
		assert_eq!(
			app.world().get::<CharacterStance>(player).map(|stance| stance.kind),
			Some(StanceKind::Squat)
		);
		Ok(())
	}
}
