//! Actor-local motor contract. Composition selects contributions; apply realizes.

mod apply;
mod compose;
mod contract;

use bevy::prelude::*;
use maybraid_controllers_core::ControllerPipeline;

use crate::body::CharacterController;

pub use apply::{apply_motor, apply_routing_hold_effects};
pub use compose::compose_motor;
pub use contract::{
	MotorContributor, MotorIntent, MotorModifier, MotorRecovery, MotorSample, MotorSelection,
	MotorSuppressionToken, ResolvedMotorRequest, SpeedCapRequest, SprintRequest, StanceRequest,
};

/// Domain order inside [`ControllerPipeline::MotorRequest`].
///
/// Registered beside locomotion / Tanz / world systems, not in generic core.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MotorRequestStage {
	/// Ordinary locomotion + NPC planning write base movement.
	Standard,
	/// Tanzer execution writes a base movement request.
	Tanz,
	/// Pause / text / map declare scoped suppression.
	WorldFilter,
}

impl MotorRequestStage {
	/// Install Standard → Tanz → WorldFilter inside [`ControllerPipeline::MotorRequest`].
	pub fn configure(app: &mut App) {
		ControllerPipeline::configure(app);
		app.configure_sets(
			Update,
			(
				Self::Standard.in_set(ControllerPipeline::MotorRequest),
				Self::Tanz.in_set(ControllerPipeline::MotorRequest).after(Self::Standard),
				Self::WorldFilter.in_set(ControllerPipeline::MotorRequest).after(Self::Tanz),
			),
		);
	}
}

pub(crate) fn ensure_motor_intents(
	mut commands: Commands,
	missing_intent: Query<Entity, (With<CharacterController>, Without<MotorIntent>)>,
	missing_selection: Query<Entity, (With<CharacterController>, Without<MotorSelection>)>,
	missing_resolved: Query<Entity, (With<CharacterController>, Without<ResolvedMotorRequest>)>,
) {
	for entity in &missing_intent {
		commands.entity(entity).insert(MotorIntent::default());
	}
	for entity in &missing_selection {
		commands.entity(entity).insert(MotorSelection::default());
	}
	for entity in &missing_resolved {
		commands.entity(entity).insert(ResolvedMotorRequest::default());
	}
}

pub(crate) fn reset_motor_intents(mut intents: Query<&mut MotorIntent>) {
	for mut intent in &mut intents {
		intent.clear();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::{JumpWish, MoveSpeedCap, MoveWish, Sprinting};
	use crate::identity::{Npc, Player};
	use crate::stance::{CharacterStance, StanceKind};
	use anyhow::{Result, anyhow};
	use characters::LocomotionCapsule;
	use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource, RoutingEvent};

	#[test]
	fn compose_prefers_tanz_over_npc_and_standard() {
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample::translation(Vec3::X));
		intent.set_npc(MotorSample::translation(Vec3::Y));
		intent.set_tanz(MotorSample {
			translation: Vec3::NEG_X,
			sprint: SprintRequest::Release,
			speed_cap: SpeedCapRequest::Set(12.0),
			..MotorSample::default()
		});
		let resolved = intent.compose(None);
		assert!((resolved.translation.x + 1.0).abs() < 1e-5);
		assert_eq!(resolved.sprint, SprintRequest::Release);
		assert_eq!(resolved.speed_cap, SpeedCapRequest::Set(12.0));
		assert_eq!(intent.planned_translation(), resolved.translation);
		assert_eq!(intent.selected_base().map(|(who, _)| who), Some(MotorContributor::Tanz));
	}

	#[test]
	fn planned_translation_matches_compose_for_npc_over_standard() {
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample::translation(Vec3::X));
		intent.set_npc(MotorSample::translation(Vec3::Z));
		let resolved = intent.compose(None);
		assert_eq!(intent.planned_translation(), resolved.translation);
		assert_eq!(resolved.translation, Vec3::Z);
		assert_eq!(intent.selected_base().map(|(who, _)| who), Some(MotorContributor::Npc));
	}

	#[test]
	fn unused_base_releases_the_cap() {
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample::translation(Vec3::Z));
		let resolved = intent.compose(None);
		assert_eq!(resolved.translation, Vec3::Z);
		assert_eq!(resolved.speed_cap, SpeedCapRequest::Release);
	}

	#[test]
	fn suppress_releases_sprint_jump_and_cap() {
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample {
			translation: Vec3::X,
			jump: true,
			sprint: SprintRequest::Start,
			speed_cap: SpeedCapRequest::Set(9.0),
			..MotorSample::default()
		});
		intent.suppress(MotorSuppressionToken::WORLD_GAMEPLAY);
		assert_eq!(intent.compose(None), ResolvedMotorRequest::released());
	}

	#[test]
	fn scoped_suppression_is_token_owned() {
		let other = MotorSuppressionToken::from_raw(2);
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample::translation(Vec3::X));
		intent.suppress(MotorSuppressionToken::WORLD_GAMEPLAY);
		intent.release_suppression(other);
		assert!(intent.suppressed_by(MotorSuppressionToken::WORLD_GAMEPLAY));
		assert_eq!(intent.compose(None), ResolvedMotorRequest::released());
		intent.release_suppression(MotorSuppressionToken::WORLD_GAMEPLAY);
		assert!(!intent.suppressed());
		assert_eq!(intent.compose(None).translation, Vec3::X);
	}

	#[test]
	fn vanished_contributor_releases_unclaimed_holds() {
		let mut intent = MotorIntent::default();
		intent.set_tanz(MotorSample {
			translation: Vec3::NEG_Z,
			sprint: SprintRequest::Release,
			speed_cap: SpeedCapRequest::Set(12.0),
			..MotorSample::default()
		});
		assert_eq!(intent.compose(None).speed_cap, SpeedCapRequest::Set(12.0));

		let mut next = MotorIntent::default();
		next.set_standard(MotorSample {
			translation: Vec3::X,
			sprint: SprintRequest::Neutral,
			..MotorSample::default()
		});
		let resolved = next.compose(Some(MotorContributor::Tanz));
		assert_eq!(resolved.translation, Vec3::X);
		assert_eq!(resolved.sprint, SprintRequest::Release);
		assert_eq!(resolved.speed_cap, SpeedCapRequest::Release);
	}

	#[test]
	fn no_request_does_not_preserve_stale_ownership() {
		let empty = MotorIntent::default();
		assert_eq!(
			empty.compose(Some(MotorContributor::Standard)),
			ResolvedMotorRequest::released()
		);
		assert_eq!(empty.planned_translation(), Vec3::ZERO);
	}

	#[test]
	fn recovery_and_separation_compose_on_the_selected_sample() {
		let mut intent = MotorIntent::default();
		intent.set_npc(MotorSample::translation(Vec3::X));
		intent.set_recovery(MotorRecovery { wish_override: Some(Vec3::X), jump: true });
		intent.add_separation(Vec2::X * 0.5);
		let resolved = intent.compose(None);
		assert!(resolved.translation.z.abs() > 1e-4, "{:?}", resolved.translation);
		assert!(resolved.jump);
		assert_eq!(intent.planned_translation(), Vec3::X);
	}

	fn motor_app() -> (App, Entity) {
		let mut app = App::new();
		app.add_systems(Update, (compose_motor, apply_motor).chain());
		let player = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish::default(),
				MotorIntent::default(),
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
				crate::identity::PlayerLook::default(),
				CharacterStance::settled(StanceKind::Stand),
			))
			.id();
		(app, player)
	}

	fn write_standard(app: &mut App, player: Entity, sample: MotorSample) -> Result<()> {
		let mut intent = app
			.world_mut()
			.get_mut::<MotorIntent>(player)
			.ok_or_else(|| anyhow!("motor intent"))?;
		intent.set_standard(sample);
		Ok(())
	}

	#[test]
	fn start_inserts_stop_removes() -> Result<()> {
		let (mut app, player) = motor_app();
		write_standard(
			&mut app,
			player,
			MotorSample { sprint: SprintRequest::Start, ..default() },
		)?;
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("Start should insert Sprinting"));
		}
		write_standard(&mut app, player, MotorSample { sprint: SprintRequest::Stop, ..default() })?;
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("Stop should remove Sprinting"));
		}
		Ok(())
	}

	#[test]
	fn disappearing_tanz_drops_stale_sprint_and_cap() -> Result<()> {
		let (mut app, player) = motor_app();
		{
			let mut intent = app
				.world_mut()
				.get_mut::<MotorIntent>(player)
				.ok_or_else(|| anyhow!("motor intent"))?;
			intent.set_tanz(MotorSample {
				translation: Vec3::NEG_Z,
				sprint: SprintRequest::Release,
				speed_cap: SpeedCapRequest::Set(12.0),
				..MotorSample::default()
			});
		}
		app.world_mut()
			.entity_mut(player)
			.insert((Sprinting, MoveSpeedCap(12.0), JumpWish));
		app.update();
		app.world_mut()
			.entity_mut(player)
			.insert((Sprinting, MoveSpeedCap(12.0), JumpWish));
		{
			let mut intent = app
				.world_mut()
				.get_mut::<MotorIntent>(player)
				.ok_or_else(|| anyhow!("motor intent reset"))?;
			intent.clear();
		}
		write_standard(
			&mut app,
			player,
			MotorSample { translation: Vec3::X, sprint: SprintRequest::Neutral, ..default() },
		)?;
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("vanished Tanz must release Sprinting"));
		}
		if app.world().get::<MoveSpeedCap>(player).is_some() {
			return Err(anyhow!("vanished Tanz must release MoveSpeedCap"));
		}
		if app.world().get::<JumpWish>(player).is_some() {
			return Err(anyhow!("vanished Tanz must drop JumpWish"));
		}
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::X));
		Ok(())
	}

	#[test]
	fn start_sprint_stands_and_stop_does_not_crouch() -> Result<()> {
		let (mut app, player) = motor_app();
		app.world_mut()
			.entity_mut(player)
			.insert(CharacterStance::settled(StanceKind::Squat));
		write_standard(
			&mut app,
			player,
			MotorSample { sprint: SprintRequest::Start, stance: StanceRequest::Stand, ..default() },
		)?;
		app.update();
		let stance = app.world().get::<CharacterStance>(player).ok_or_else(|| anyhow!("stance"))?;
		if stance.kind != StanceKind::Stand {
			return Err(anyhow!("StartSprint should stand"));
		}
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("StartSprint should still grant Sprinting"));
		}
		write_standard(&mut app, player, MotorSample { sprint: SprintRequest::Stop, ..default() })?;
		app.update();
		let stance = app.world().get::<CharacterStance>(player).ok_or_else(|| anyhow!("stance"))?;
		if stance.kind != StanceKind::Stand {
			return Err(anyhow!("StopSprint must not re-crouch"));
		}
		Ok(())
	}

	#[test]
	fn change_squat_toggles_and_prone_enters() -> Result<()> {
		let (mut app, player) = motor_app();
		write_standard(
			&mut app,
			player,
			MotorSample { stance: StanceRequest::ChangeSquat, ..default() },
		)?;
		app.update();
		assert_eq!(
			app.world().get::<CharacterStance>(player).map(|stance| stance.kind),
			Some(StanceKind::Squat)
		);
		write_standard(
			&mut app,
			player,
			MotorSample { stance: StanceRequest::ChangeSquat, ..default() },
		)?;
		app.update();
		assert_eq!(
			app.world().get::<CharacterStance>(player).map(|stance| stance.kind),
			Some(StanceKind::Stand)
		);
		write_standard(
			&mut app,
			player,
			MotorSample { stance: StanceRequest::ChangeProne, ..default() },
		)?;
		app.update();
		assert_eq!(
			app.world().get::<CharacterStance>(player).map(|stance| stance.kind),
			Some(StanceKind::Prone)
		);
		Ok(())
	}

	#[test]
	fn overlapping_npc_modifiers_compose_opposing_wishes() -> Result<()> {
		let mut app = App::new();
		app.add_systems(
			Update,
			(crate::separation::declare_npc_soft_bump, compose_motor, apply_motor).chain(),
		);
		let a = app
			.world_mut()
			.spawn((
				Npc,
				CharacterController,
				Transform::from_xyz(0.0, 0.0, 0.0),
				MoveWish(Vec3::X),
				{
					let mut intent = MotorIntent::default();
					intent.set_npc(MotorSample::translation(Vec3::X));
					intent
				},
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
				LocomotionCapsule::HUMANOID,
			))
			.id();
		let b = app
			.world_mut()
			.spawn((
				Npc,
				CharacterController,
				Transform::from_xyz(0.5, 0.0, 0.0),
				MoveWish(Vec3::X),
				{
					let mut intent = MotorIntent::default();
					intent.set_npc(MotorSample::translation(Vec3::X));
					intent
				},
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
				LocomotionCapsule::HUMANOID,
			))
			.id();
		let player = app
			.world_mut()
			.spawn((
				Player,
				CharacterController,
				Transform::from_xyz(0.25, 0.0, 0.0),
				MoveWish(Vec3::X),
				{
					let mut intent = MotorIntent::default();
					intent.set_standard(MotorSample::translation(Vec3::X));
					intent
				},
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
				LocomotionCapsule::HUMANOID,
			))
			.id();
		app.update();
		let wish_a = app.world().get::<MoveWish>(a).map(|wish| wish.0);
		let wish_b = app.world().get::<MoveWish>(b).map(|wish| wish.0);
		let wish_player = app.world().get::<MoveWish>(player).map(|wish| wish.0);
		let (wish_a, wish_b, wish_player) = match (wish_a, wish_b, wish_player) {
			(Some(a), Some(b), Some(player)) => (a, b, player),
			_ => return Err(anyhow!("missing wishes")),
		};
		assert!((wish_player - Vec3::X).length() < 1e-5, "{wish_player:?}");
		let delta_a = Vec2::new(wish_a.x - 1.0, wish_a.z);
		let delta_b = Vec2::new(wish_b.x - 1.0, wish_b.z);
		assert!(delta_a.length() > 1e-4, "{wish_a:?}");
		assert!(delta_a.dot(delta_b) < 0.0, "opposing {wish_a:?} {wish_b:?}");
		Ok(())
	}

	fn routing_motor_app() -> (App, Entity) {
		let mut app = App::new();
		MotorRequestStage::configure(&mut app);
		app.init_resource::<InputRouter>().add_message::<RoutingEvent>().add_systems(
			Update,
			(
				apply_routing_hold_effects
					.in_set(ControllerPipeline::MotorRequest)
					.after(MotorRequestStage::WorldFilter),
				(compose_motor, apply_motor).chain().in_set(ControllerPipeline::Motor),
			),
		);
		let player = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish::default(),
				MotorIntent::default(),
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
				crate::identity::PlayerLook::default(),
				CharacterStance::settled(StanceKind::Stand),
			))
			.id();
		(app, player)
	}

	fn publish_hold(
		app: &mut App,
		apply: impl FnOnce(&mut InputRouter) + Send + Sync + 'static,
	) -> Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_controllers_core::InputRouting;
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

	#[test]
	fn disconnect_event_releases_standard_contributor() -> Result<()> {
		let (mut app, player) = routing_motor_app();
		publish_hold(&mut app, move |router| {
			router.assign_actor(InputSource::local_pad(), ActorTarget::new(player));
		})?;
		write_standard(
			&mut app,
			player,
			MotorSample { translation: Vec3::X, sprint: SprintRequest::Start, ..default() },
		)?;
		app.update();
		if app.world().get::<Sprinting>(player).is_none() {
			return Err(anyhow!("precondition: sprint hold"));
		}
		publish_hold(&mut app, |router| {
			router.disconnect(InputSource::local_pad());
		})?;
		write_standard(
			&mut app,
			player,
			MotorSample { translation: Vec3::X, sprint: SprintRequest::Neutral, ..default() },
		)?;
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("disconnect must release Sprinting"));
		}
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		Ok(())
	}

	#[test]
	fn capture_event_releases_standard_contributor() -> Result<()> {
		use maybraid_controllers_core::{CaptureKind, ScopeOwner, UiTarget};
		let (mut app, player) = routing_motor_app();
		let menu = app.world_mut().spawn_empty().id();
		publish_hold(&mut app, move |router| {
			router.assign_actor(InputSource::local_pad(), ActorTarget::new(player));
		})?;
		write_standard(
			&mut app,
			player,
			MotorSample { sprint: SprintRequest::Start, ..default() },
		)?;
		app.update();
		publish_hold(&mut app, move |router| {
			router.acquire_capture(
				InputSource::local_pad(),
				ScopeOwner::ui(UiTarget::new(menu)),
				CaptureKind::MenuNav,
			);
		})?;
		write_standard(
			&mut app,
			player,
			MotorSample { translation: Vec3::Z, sprint: SprintRequest::Neutral, ..default() },
		)?;
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("capture must release Sprinting"));
		}
		assert_eq!(
			app.world().resource::<InputRouter>().local_actor(),
			Some(ActorTarget::new(player))
		);
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		Ok(())
	}

	#[test]
	fn clear_standard_is_contributor_loss() {
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample::translation(Vec3::X));
		intent.clear_standard();
		assert!(intent.standard().is_none());
		assert_eq!(
			intent.compose(Some(MotorContributor::Standard)),
			ResolvedMotorRequest::released()
		);
	}
}
