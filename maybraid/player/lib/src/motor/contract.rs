//! Motor contribution types and the compose / apply contract.

use bevy::prelude::*;

use crate::body::{JumpWish, Jumping, MoveSpeedCap, MoveWish, Sprinting};
use crate::separation::SoftBump;
use crate::stance::{CharacterStance, StanceKind};

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
/// [`ResolvedMotorRequest`]. [`super::apply_motor`] realizes that request and
/// does not inspect who produced it.
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

	/// Drop the Standard contribution. Pad disconnect / capture / removal.
	pub fn clear_standard(&mut self) {
		self.standard = None;
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

	pub fn suppressed_by(&self, token: MotorSuppressionToken) -> bool {
		self.suppressions.iter().any(|slot| *slot == Some(token))
	}

	pub fn suppressed(&self) -> bool {
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
