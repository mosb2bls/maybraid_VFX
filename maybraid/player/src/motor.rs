//! Actor-local motor contract. Composition selects contributions; apply realizes.

use bevy::prelude::*;
use maybraid_controllers_core::ControllerPipeline;

use crate::body::{CharacterController, JumpWish, Jumping, MoveSpeedCap, MoveWish, Sprinting};
use crate::separation::SoftBump;
use crate::stance::{CharacterStance, StanceKind};

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

/// Who contributed the selected base movement this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotorContributor {
	Standard,
	Tanz,
	Npc,
}

/// Last composed base contributor. Survives the per-frame [`MotorIntent`] reset.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MotorSelection {
	base: Option<MotorContributor>,
}

impl MotorSelection {
	pub fn base(self) -> Option<MotorContributor> {
		self.base
	}

	pub fn remember(&mut self, contributor: Option<MotorContributor>) {
		self.base = contributor;
	}
}

/// Token that owns a movement suppression. Releasing A cannot release B.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MotorSuppressionToken(u64);

impl MotorSuppressionToken {
	/// World pause / text / map / non-character mode. Players only.
	pub const WORLD_GAMEPLAY: Self = Self(1);
	/// World-wide motor gate. Written when session Simulation is not installed.
	pub const SIMULATION: Self = Self(2);

	pub const fn from_raw(raw: u64) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u64 {
		self.0
	}
}

/// Movement-specific modifier with an explicit policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotorModifier {
	/// Replace the selected base translation when `wish_override` is set; OR jump.
	Recovery { wish_override: Option<Vec3>, jump: bool },
	/// Add a kinematic XZ split onto the composed translation.
	Separation { push: Vec2 },
}

impl MotorModifier {
	pub fn apply(self, request: &mut ResolvedMotorRequest) {
		match self {
			Self::Recovery { wish_override, jump } => {
				if let Some(wish) = wish_override {
					request.translation = wish;
				}
				request.jump |= jump;
			}
			Self::Separation { push } => {
				SoftBump::apply_to_wish(&mut request.translation, push);
			}
		}
	}
}

/// Unstick override declared by movement-realization.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorRecovery {
	pub wish_override: Option<Vec3>,
	pub jump: bool,
}

impl MotorRecovery {
	pub fn as_modifier(self) -> MotorModifier {
		MotorModifier::Recovery { wish_override: self.wish_override, jump: self.jump }
	}
}

/// Declared motor contributions for one actor this frame.
///
/// Writers fill slots. [`Self::compose`] selects and produces
/// [`ResolvedMotorRequest`]. [`apply_motor`] realizes that request and does
/// not inspect who produced it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorIntent {
	standard: Option<MotorSample>,
	tanz: Option<MotorSample>,
	npc: Option<MotorSample>,
	recovery: Option<MotorModifier>,
	separation: Option<MotorModifier>,
	suppressions: [Option<MotorSuppressionToken>; 2],
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
	/// Drop sprint (focus, unused lane, pause, contributor loss).
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

/// Composed request after contribution selection and modifiers.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedMotorRequest {
	pub translation: Vec3,
	pub jump: bool,
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
		self.recovery = Some(recovery.as_modifier());
	}

	pub fn add_modifier(&mut self, modifier: MotorModifier) {
		match modifier {
			MotorModifier::Recovery { .. } => self.recovery = Some(modifier),
			MotorModifier::Separation { push } => match &mut self.separation {
				Some(MotorModifier::Separation { push: existing }) => *existing += push,
				_ => self.separation = Some(modifier),
			},
		}
	}

	pub fn add_separation(&mut self, push: Vec2) {
		self.add_modifier(MotorModifier::Separation { push });
	}

	pub fn suppress(&mut self, token: MotorSuppressionToken) {
		if self.suppressed_by(token) {
			return;
		}
		if let Some(slot) = self.suppressions.iter_mut().find(|slot| slot.is_none()) {
			*slot = Some(token);
		}
	}

	pub fn release_suppression(&mut self, token: MotorSuppressionToken) {
		for slot in &mut self.suppressions {
			if *slot == Some(token) {
				*slot = None;
			}
		}
	}

	pub fn suppressed_by(self, token: MotorSuppressionToken) -> bool {
		self.suppressions.iter().any(|slot| *slot == Some(token))
	}

	pub fn suppressed(self) -> bool {
		self.suppressions.iter().any(Option::is_some)
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
		match self.recovery {
			Some(MotorModifier::Recovery { wish_override, jump }) => {
				MotorRecovery { wish_override, jump }
			}
			_ => MotorRecovery::default(),
		}
	}

	pub fn separation(&self) -> Vec2 {
		match self.separation {
			Some(MotorModifier::Separation { push }) => push,
			_ => Vec2::ZERO,
		}
	}

	/// Tanz, else NPC, else Standard. Same order as [`Self::compose`].
	pub fn selected_base(&self) -> Option<(MotorContributor, MotorSample)> {
		if let Some(sample) = self.tanz {
			Some((MotorContributor::Tanz, sample))
		} else if let Some(sample) = self.npc {
			Some((MotorContributor::Npc, sample))
		} else {
			self.standard.map(|sample| (MotorContributor::Standard, sample))
		}
	}

	pub fn planned_translation(&self) -> Vec3 {
		self.selected_base().map(|(_, sample)| sample.translation).unwrap_or(Vec3::ZERO)
	}

	/// Select contributions and produce the request apply will realize.
	///
	/// Apply never sees who produced the sample. When the previous base
	/// contributor is gone, translation is the new base (or zero) and that
	/// contributor's unclaimed sprint / cap / jump holds are released.
	pub fn compose(&self, previous: Option<MotorContributor>) -> ResolvedMotorRequest {
		if self.suppressed() {
			return ResolvedMotorRequest::released();
		}
		let Some((contributor, sample)) = self.selected_base() else {
			return ResolvedMotorRequest::released();
		};
		let mut request = ResolvedMotorRequest::from_sample(sample);
		if previous.is_some_and(|was| was != contributor) {
			request.release_unclaimed_holds();
		}
		if let Some(modifier) = self.recovery {
			modifier.apply(&mut request);
		}
		if let Some(modifier) = self.separation {
			modifier.apply(&mut request);
		}
		request
	}
}

impl ResolvedMotorRequest {
	pub fn from_sample(sample: MotorSample) -> Self {
		Self {
			translation: sample.translation,
			jump: sample.jump,
			sprint: sample.sprint,
			stance: sample.stance,
			speed_cap: sample.speed_cap,
		}
	}

	pub fn released() -> Self {
		Self {
			translation: Vec3::ZERO,
			jump: false,
			sprint: SprintRequest::Release,
			stance: StanceRequest::Neutral,
			speed_cap: SpeedCapRequest::Release,
		}
	}

	fn release_unclaimed_holds(&mut self) {
		if matches!(self.sprint, SprintRequest::Neutral) {
			self.sprint = SprintRequest::Release;
		}
		if !self.jump {
			// Jump is an edge. A vanished contributor must not leave JumpWish.
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
		} else {
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

/// Select this frame's contributions into [`ResolvedMotorRequest`].
pub fn compose_motor(
	mut bodies: Query<(&MotorIntent, &mut MotorSelection, &mut ResolvedMotorRequest)>,
) {
	for (intent, mut selection, mut resolved) in &mut bodies {
		let next = intent.selected_base().map(|(contributor, _)| contributor);
		*resolved = intent.compose(selection.base());
		selection.remember(if intent.suppressed() { None } else { next });
	}
}

/// One apply path. Realizes the composed request; does not inspect producers.
pub fn apply_motor(
	mut commands: Commands,
	mut bodies: Query<
		(
			Entity,
			&ResolvedMotorRequest,
			&mut MoveWish,
			Option<&mut CharacterStance>,
			Option<&Jumping>,
		),
		With<CharacterController>,
	>,
) {
	for (entity, resolved, mut wish, mut stance, jumping) in &mut bodies {
		resolved.apply(entity, &mut wish, stance.as_deref_mut(), jumping, &mut commands);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::identity::{Npc, Player};
	use anyhow::{anyhow, Result};
	use characters::LocomotionCapsule;

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
}
