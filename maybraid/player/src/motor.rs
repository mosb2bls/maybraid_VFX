//! Actor-local motor contract. Locomotion and Tanz write slots; apply owns realization.

use bevy::prelude::*;

use crate::body::{CharacterController, JumpWish, Jumping, MoveSpeedCap, MoveWish, Sprinting};
use crate::separation::SoftBump;
use crate::stance::{CharacterStance, StanceKind};

/// Declared motor requests and modifiers for one actor this frame.
///
/// Writers fill slots. [`Self::resolve`] selects and composes. [`apply_motor`]
/// is the only path that writes [`MoveWish`] / [`JumpWish`] / [`Sprinting`] /
/// [`MoveSpeedCap`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorIntent {
	standard: Option<MotorSample>,
	tanz: Option<MotorSample>,
	npc: Option<MotorSample>,
	recovery: MotorRecovery,
	separation: Vec2,
	suppress: bool,
}

/// One declared translation / jump / sprint / stance / cap sample.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorSample {
	pub translation: Vec3,
	pub jump: bool,
	pub sprint: SprintRequest,
	pub stance: StanceRequest,
	pub speed_cap: SpeedCapRequest,
}

impl MotorSample {
	pub fn translation(translation: Vec3) -> Self {
		Self { translation, ..Self::default() }
	}
}

/// Sprint edges for this frame. [`Self::Neutral`] leaves a live hold alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SprintRequest {
	#[default]
	Neutral,
	Start,
	Stop,
	/// Drop sprint (focus, unused lane, pause).
	Release,
}

/// Stance edges for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StanceRequest {
	#[default]
	Neutral,
	ChangeSquat,
	ChangeProne,
	Stand,
}

/// Absolute ground-speed cap, or release so jog/sprint own the throttle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SpeedCapRequest {
	#[default]
	Release,
	Set(f32),
}

/// Unstick override declared by movement-realization.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorRecovery {
	pub wish_override: Option<Vec3>,
	pub jump: bool,
}

/// Composed request after lane selection and modifiers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedMotorRequest {
	pub translation: Vec3,
	pub jump: bool,
	pub release_jump: bool,
	pub sprint: SprintRequest,
	pub stance: StanceRequest,
	pub speed_cap: SpeedCapRequest,
}

impl MotorIntent {
	pub fn clear(&mut self) {
		*self = Self::default();
	}

	pub fn set_standard(&mut self, sample: MotorSample) {
		self.standard = Some(sample);
	}

	pub fn set_tanz(&mut self, sample: MotorSample) {
		self.tanz = Some(sample);
	}

	pub fn set_npc(&mut self, sample: MotorSample) {
		self.npc = Some(sample);
	}

	pub fn set_npc_if_neq(&mut self, sample: MotorSample) -> bool {
		if self.npc == Some(sample) {
			return false;
		}
		self.npc = Some(sample);
		true
	}

	pub fn set_recovery(&mut self, recovery: MotorRecovery) {
		self.recovery = recovery;
	}

	pub fn add_separation(&mut self, push: Vec2) {
		self.separation += push;
	}

	pub fn set_suppress(&mut self, suppress: bool) {
		self.suppress = suppress;
	}

	pub fn standard(&self) -> Option<MotorSample> {
		self.standard
	}

	pub fn tanz(&self) -> Option<MotorSample> {
		self.tanz
	}

	pub fn npc(&self) -> Option<MotorSample> {
		self.npc
	}

	pub fn recovery(&self) -> MotorRecovery {
		self.recovery
	}

	pub fn separation(&self) -> Vec2 {
		self.separation
	}

	pub fn suppress(&self) -> bool {
		self.suppress
	}

	pub fn planned_translation(&self) -> Vec3 {
		self.npc
			.or(self.tanz)
			.or(self.standard)
			.map(|sample| sample.translation)
			.unwrap_or(Vec3::ZERO)
	}

	/// Select Tanz, else NPC, else Standard; then apply recovery / separation.
	pub fn resolve(&self) -> ResolvedMotorRequest {
		if self.suppress {
			return ResolvedMotorRequest::released();
		}
		let sample = if let Some(sample) = self.tanz {
			sample
		} else if let Some(sample) = self.npc {
			sample
		} else {
			self.standard.unwrap_or_default()
		};
		let mut translation = self.recovery.wish_override.unwrap_or(sample.translation);
		SoftBump::apply_to_wish(&mut translation, self.separation);
		let tanz = self.tanz.is_some();
		ResolvedMotorRequest {
			translation,
			jump: sample.jump || self.recovery.jump,
			release_jump: tanz,
			sprint: if tanz { SprintRequest::Release } else { sample.sprint },
			stance: sample.stance,
			speed_cap: if tanz { sample.speed_cap } else { SpeedCapRequest::Release },
		}
	}
}

impl ResolvedMotorRequest {
	pub fn released() -> Self {
		Self {
			translation: Vec3::ZERO,
			jump: false,
			release_jump: true,
			sprint: SprintRequest::Release,
			stance: StanceRequest::Neutral,
			speed_cap: SpeedCapRequest::Release,
		}
	}

	pub fn apply(
		self,
		entity: Entity,
		wish: &mut MoveWish,
		mut stance: Option<&mut CharacterStance>,
		jumping: Option<&Jumping>,
		commands: &mut Commands,
	) {
		if wish.0 != self.translation {
			wish.0 = self.translation;
		}
		let airborne = jumping.is_some_and(Jumping::airborne);
		if self.jump {
			commands.entity(entity).insert(JumpWish);
		} else if self.release_jump {
			commands.entity(entity).remove::<JumpWish>();
		}
		if !airborne && !matches!(self.sprint, SprintRequest::Start) {
			self.stance.apply(entity, stance.as_deref_mut(), commands);
		}
		match self.sprint {
			SprintRequest::Release | SprintRequest::Stop => {
				commands.entity(entity).remove::<Sprinting>();
			}
			SprintRequest::Start => {
				if let Some(stance) = stance.as_deref_mut() {
					stance.stand();
				} else {
					commands.entity(entity).insert(CharacterStance::settled(StanceKind::Stand));
				}
				commands.entity(entity).insert(Sprinting);
			}
			SprintRequest::Neutral => {}
		}
		match self.speed_cap {
			SpeedCapRequest::Release => {
				commands.entity(entity).remove::<MoveSpeedCap>();
			}
			SpeedCapRequest::Set(speed) => {
				commands.entity(entity).insert(MoveSpeedCap(speed));
			}
		}
	}
}

impl StanceRequest {
	pub fn apply(
		self,
		entity: Entity,
		stance: Option<&mut CharacterStance>,
		commands: &mut Commands,
	) {
		match self {
			Self::Neutral => {}
			Self::Stand => {
				if let Some(stance) = stance {
					stance.stand();
				} else {
					commands.entity(entity).insert(CharacterStance::settled(StanceKind::Stand));
				}
			}
			Self::ChangeSquat | Self::ChangeProne => {
				if let Some(stance) = stance {
					if matches!(self, Self::ChangeProne) {
						stance.change_prone();
					} else {
						stance.change_squat();
					}
					return;
				}
				let kind = if matches!(self, Self::ChangeProne) {
					StanceKind::Prone
				} else {
					StanceKind::Squat
				};
				commands.entity(entity).insert(CharacterStance::settled(kind));
			}
		}
	}
}

pub(crate) fn ensure_motor_intents(
	mut commands: Commands,
	bodies: Query<Entity, (With<CharacterController>, Without<MotorIntent>)>,
) {
	for entity in &bodies {
		commands.entity(entity).insert(MotorIntent::default());
	}
}

pub(crate) fn reset_motor_intents(mut intents: Query<&mut MotorIntent>) {
	for mut intent in &mut intents {
		intent.clear();
	}
}

/// One apply path for every capsule that declared a [`MotorIntent`].
pub fn apply_motor(
	mut commands: Commands,
	mut bodies: Query<
		(Entity, &MotorIntent, &mut MoveWish, Option<&mut CharacterStance>, Option<&Jumping>),
		With<CharacterController>,
	>,
) {
	for (entity, intent, mut wish, mut stance, jumping) in &mut bodies {
		intent
			.resolve()
			.apply(entity, &mut wish, stance.as_deref_mut(), jumping, &mut commands);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::identity::{Npc, Player};
	use anyhow::{anyhow, Result};
	use characters::LocomotionCapsule;

	#[test]
	fn resolver_prefers_tanz_over_standard_without_overwrite() {
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample {
			translation: Vec3::X,
			sprint: SprintRequest::Start,
			speed_cap: SpeedCapRequest::Release,
			..MotorSample::default()
		});
		intent.set_tanz(MotorSample {
			translation: Vec3::NEG_X,
			speed_cap: SpeedCapRequest::Set(12.0),
			..MotorSample::default()
		});
		let resolved = intent.resolve();
		assert!((resolved.translation.x + 1.0).abs() < 1e-5);
		assert_eq!(resolved.sprint, SprintRequest::Release);
		assert_eq!(resolved.speed_cap, SpeedCapRequest::Set(12.0));
		assert!(resolved.release_jump);
	}

	#[test]
	fn unused_tanz_slot_releases_the_cap() {
		let mut intent = MotorIntent::default();
		intent.set_standard(MotorSample::translation(Vec3::Z));
		let resolved = intent.resolve();
		assert_eq!(resolved.translation, Vec3::Z);
		assert_eq!(resolved.speed_cap, SpeedCapRequest::Release);
		assert!(!resolved.release_jump);
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
		intent.set_suppress(true);
		assert_eq!(intent.resolve(), ResolvedMotorRequest::released());
	}

	#[test]
	fn recovery_and_separation_compose_on_the_selected_sample() {
		let mut intent = MotorIntent::default();
		intent.set_npc(MotorSample::translation(Vec3::X));
		intent.set_recovery(MotorRecovery { wish_override: Some(Vec3::X), jump: true });
		intent.add_separation(Vec2::X * 0.5);
		let resolved = intent.resolve();
		assert!(resolved.translation.z.abs() > 1e-4, "{:?}", resolved.translation);
		assert!(resolved.jump);
	}

	fn motor_app() -> (App, Entity) {
		let mut app = App::new();
		app.add_systems(Update, apply_motor);
		let player = app
			.world_mut()
			.spawn((
				CharacterController,
				Player,
				MoveWish::default(),
				MotorIntent::default(),
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
	fn tanz_lane_drops_stale_sprint_and_cap() -> Result<()> {
		let (mut app, player) = motor_app();
		app.world_mut()
			.entity_mut(player)
			.insert((Sprinting, MoveSpeedCap(9.0), JumpWish));
		{
			let mut intent = app
				.world_mut()
				.get_mut::<MotorIntent>(player)
				.ok_or_else(|| anyhow!("motor intent"))?;
			intent.set_standard(MotorSample { sprint: SprintRequest::Start, ..default() });
			intent.set_tanz(MotorSample::translation(Vec3::NEG_Z));
		}
		app.update();
		if app.world().get::<Sprinting>(player).is_some() {
			return Err(anyhow!("Tanz lane must release Sprinting"));
		}
		if app.world().get::<MoveSpeedCap>(player).is_some() {
			return Err(anyhow!("idle Tanz must release MoveSpeedCap"));
		}
		if app.world().get::<JumpWish>(player).is_some() {
			return Err(anyhow!("Tanz lane must drop a leftover JumpWish"));
		}
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::NEG_Z));
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
		app.add_systems(Update, (crate::separation::declare_npc_soft_bump, apply_motor).chain());
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
}
