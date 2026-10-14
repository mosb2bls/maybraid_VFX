//! Clip identity on a rig member. Mailbox transitions key on [`AnimId`], not knobs.
//!
//! Parallel to [`material_ref::MaterialRefRoot`]: insert [`AnimRefRoot`] on the
//! body-rig host. `From<ConceptAnimation>` lives in `characters`.

use bevy::prelude::*;
use character_animations::animations::{
	BallisticJump, DorsoventralUndulation, FixedTuck, Flapping, FlipDirection, Gallop,
	LateralUndulation, Leap, QuadrupedRun, Run, Soaring, TuckProfile, TuckedFlip, TwoFootedJump,
	Walk, AIR_END, DEFAULT_BACKSWING, DEFAULT_DESCENT_SPEED, DEFAULT_GRAVITY, DEFAULT_JAB_TARGET,
	DEFAULT_JUMP_HEIGHT, DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED,
	DEFAULT_SPRING_DURATION, TAKEOFF_END,
};
use character_animations::{ClipTimePolicy, SampleAddress};
use character_rigs::Side;

const RUN_CYCLE_SPEED: f32 = 1.68;
const WALK_CYCLE_SPEED: f32 = 1.08;
/// Slow rest cycle so a crowd idle is a sway, not a march.
pub const IDLE_CYCLE_SPEED: f32 = 0.2;
const GALLOP_CYCLE_SPEED: f32 = 0.35;
const QUADRUPED_RUN_CYCLE_SPEED: f32 = 0.5;
const TUCK_CYCLE_SPEED: f32 = 0.6;
const FRONT_FLIP_CYCLE_SPEED: f32 = 0.85;
const JAB_CYCLE_SPEED: f32 = 0.9;
const JUMP_PRE_SQUAT_SPEED: f32 = DEFAULT_PRE_SQUAT_SPEED * 1.2;
const JUMP_LANDING_SQUAT_SPEED: f32 = DEFAULT_LANDING_SQUAT_SPEED * 1.3;
/// One-shot leap lasts ~1.25 s so it covers the physics hang time.
pub const LEAP_CYCLE_SPEED: f32 = 0.8;

/// Clip discriminant. Mailbox transitions key on this, not knob values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AnimId {
	#[default]
	Still,
	Walk,
	Run,
	QuadrupedRun,
	Gallop,
	Jump,
	Leap,
	Tuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
	Soaring,
	Flapping,
	Jab,
	PowerStrafe,
	Squat,
	SquatDescent,
	Prone,
	LateralUndulation,
	DorsoventralUndulation,
}

impl AnimId {
	pub const fn default_speed(self) -> f32 {
		match self {
			Self::Still => IDLE_CYCLE_SPEED,
			Self::Walk => WALK_CYCLE_SPEED,
			Self::Run => RUN_CYCLE_SPEED,
			Self::QuadrupedRun => QUADRUPED_RUN_CYCLE_SPEED,
			Self::Gallop => GALLOP_CYCLE_SPEED,
			Self::Jump => 1.0,
			Self::Leap => LEAP_CYCLE_SPEED,
			Self::Tuck => TUCK_CYCLE_SPEED,
			Self::TuckedFlip => FRONT_FLIP_CYCLE_SPEED,
			Self::TwoFootedTuckedFlip => 1.0,
			Self::Soaring => 1.0,
			Self::Flapping => 1.0,
			Self::Jab => JAB_CYCLE_SPEED,
			Self::PowerStrafe => 1.0,
			Self::Squat => 1.0,
			Self::SquatDescent => DEFAULT_DESCENT_SPEED,
			Self::Prone => 1.0,
			Self::LateralUndulation => 1.0,
			Self::DorsoventralUndulation => 1.0,
		}
	}
}

/// Untyped two-footed jump knobs ([`TwoFootedJump`] is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpParams {
	pub gravity: f32,
	pub jump_height: f32,
	pub pre_squat_speed: f32,
	pub landing_squat_speed: f32,
}

impl Default for JumpParams {
	fn default() -> Self {
		Self {
			gravity: DEFAULT_GRAVITY,
			jump_height: DEFAULT_JUMP_HEIGHT,
			pre_squat_speed: JUMP_PRE_SQUAT_SPEED,
			landing_squat_speed: JUMP_LANDING_SQUAT_SPEED,
		}
	}
}

impl JumpParams {
	pub(crate) fn apply_humanoid(self) -> TwoFootedJump {
		TwoFootedJump::default()
			.with_gravity(self.gravity)
			.with_jump_height(self.jump_height)
			.with_pre_squat_speed(self.pre_squat_speed)
			.with_landing_squat_speed(self.landing_squat_speed)
	}

	/// Seconds into the two-footed jump sampler for a 0..1 takeoff/air/land phase.
	pub fn elapsed_from_phase(self, progress: f32) -> f32 {
		let progress = progress.clamp(0.0, 1.0);
		let squat = 1.0 / self.pre_squat_speed.max(1e-3);
		let takeoff = squat + DEFAULT_SPRING_DURATION;
		let air = (BallisticJump::new(self.gravity, self.jump_height).air_duration()
			- DEFAULT_SPRING_DURATION)
			.max(0.05);
		let land = 2.0 / self.landing_squat_speed.max(1e-3);
		if progress < TAKEOFF_END {
			(progress / TAKEOFF_END) * takeoff
		} else if progress < AIR_END {
			let u = (progress - TAKEOFF_END) / (AIR_END - TAKEOFF_END);
			takeoff + u * air
		} else {
			let u = (progress - AIR_END) / (1.0 - AIR_END).max(1e-4);
			takeoff + air + u * land
		}
	}
}

/// Untyped tuck knobs ([`Tuck`](character_animations::animations::Tuck) is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuckParams {
	pub tightness: f32,
}

impl Default for TuckParams {
	fn default() -> Self {
		Self { tightness: TuckProfile::DEFAULT_TIGHTNESS }
	}
}

/// Untyped tucked-flip knobs ([`TuckedFlip`] is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuckedFlipParams {
	pub turns: f32,
	pub direction: FlipDirection,
	pub tightness: f32,
}

impl Default for TuckedFlipParams {
	fn default() -> Self {
		Self {
			turns: 1.0,
			direction: FlipDirection::Forward,
			tightness: TuckProfile::DEFAULT_TIGHTNESS,
		}
	}
}

impl TuckedFlipParams {
	pub(crate) fn apply_humanoid(self) -> TuckedFlip {
		let mut flip = TuckedFlip::default();
		flip.turns = self.turns;
		flip.direction = self.direction;
		flip.tuck = FixedTuck::new(self.tightness);
		flip
	}
}

/// Jump + flip bags for [`character_animations::animations::TwoFootedTuckedFlip`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TwoFootedTuckedFlipParams {
	pub jump: JumpParams,
	pub flip: TuckedFlipParams,
}

/// Untyped jab knobs ([`Jab`] is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JabParams {
	pub side: Side,
	pub backswing: f32,
	pub target: bevy::prelude::Vec3,
}

/// Untyped power-strafe knobs ([`PowerStrafe`](character_animations::animations::PowerStrafe) is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerStrafeParams {
	pub side: Side,
}

impl Default for PowerStrafeParams {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl Default for JabParams {
	fn default() -> Self {
		Self { side: Side::Right, backswing: DEFAULT_BACKSWING, target: DEFAULT_JAB_TARGET }
	}
}

/// Clip identity: variant + sampler knobs. Mailbox transitions use [`Self::id`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AnimClip {
	#[default]
	Still,
	Walk(Walk),
	Run(Run),
	QuadrupedRun(QuadrupedRun),
	Gallop(Gallop),
	Jump(JumpParams),
	Leap(Leap),
	Tuck(TuckParams),
	TuckedFlip(TuckedFlipParams),
	TwoFootedTuckedFlip(TwoFootedTuckedFlipParams),
	Soaring(Soaring),
	Flapping(Flapping),
	Jab(JabParams),
	PowerStrafe(PowerStrafeParams),
	Squat,
	SquatDescent,
	Prone,
	LateralUndulation(LateralUndulation),
	DorsoventralUndulation(DorsoventralUndulation),
}

impl AnimClip {
	pub const fn id(self) -> AnimId {
		match self {
			Self::Still => AnimId::Still,
			Self::Walk(_) => AnimId::Walk,
			Self::Run(_) => AnimId::Run,
			Self::QuadrupedRun(_) => AnimId::QuadrupedRun,
			Self::Gallop(_) => AnimId::Gallop,
			Self::Jump(_) => AnimId::Jump,
			Self::Leap(_) => AnimId::Leap,
			Self::Tuck(_) => AnimId::Tuck,
			Self::TuckedFlip(_) => AnimId::TuckedFlip,
			Self::TwoFootedTuckedFlip(_) => AnimId::TwoFootedTuckedFlip,
			Self::Soaring(_) => AnimId::Soaring,
			Self::Flapping(_) => AnimId::Flapping,
			Self::Jab(_) => AnimId::Jab,
			Self::PowerStrafe(_) => AnimId::PowerStrafe,
			Self::Squat => AnimId::Squat,
			Self::SquatDescent => AnimId::SquatDescent,
			Self::Prone => AnimId::Prone,
			Self::LateralUndulation(_) => AnimId::LateralUndulation,
			Self::DorsoventralUndulation(_) => AnimId::DorsoventralUndulation,
		}
	}

	pub const fn default_speed(self) -> f32 {
		self.id().default_speed()
	}

	pub fn still() -> Self {
		Self::Still
	}

	pub fn walk() -> Self {
		Self::Walk(Walk::default())
	}

	pub fn run() -> Self {
		Self::Run(Run::default())
	}

	pub fn quadruped_run() -> Self {
		Self::QuadrupedRun(QuadrupedRun::default())
	}

	pub fn gallop() -> Self {
		Self::Gallop(Gallop::default())
	}

	pub fn jump() -> Self {
		Self::Jump(JumpParams::default())
	}

	pub fn leap() -> Self {
		Self::Leap(Leap::default())
	}

	pub fn tuck() -> Self {
		Self::Tuck(TuckParams::default())
	}

	pub fn tucked_flip() -> Self {
		Self::TuckedFlip(TuckedFlipParams::default())
	}

	pub fn two_footed_tucked_flip() -> Self {
		Self::TwoFootedTuckedFlip(TwoFootedTuckedFlipParams::default())
	}

	pub fn soaring() -> Self {
		Self::Soaring(Soaring::default())
	}

	pub fn flapping() -> Self {
		Self::Flapping(Flapping::default())
	}

	pub fn jab() -> Self {
		Self::Jab(JabParams::default())
	}

	pub fn power_strafe(side: Side) -> Self {
		Self::PowerStrafe(PowerStrafeParams { side })
	}

	pub fn power_strafe_left() -> Self {
		Self::power_strafe(Side::Left)
	}

	pub fn power_strafe_right() -> Self {
		Self::power_strafe(Side::Right)
	}

	pub fn squat() -> Self {
		Self::Squat
	}

	pub fn squat_descent() -> Self {
		Self::SquatDescent
	}

	pub fn prone() -> Self {
		Self::Prone
	}

	pub fn lateral_undulation() -> Self {
		Self::LateralUndulation(LateralUndulation::default())
	}

	pub fn dorsoventral_undulation() -> Self {
		Self::DorsoventralUndulation(DorsoventralUndulation::default())
	}

	/// Content generation included in prepared-clip identity.
	pub const fn revision(self) -> u64 {
		0
	}

	/// Idle, walk, and run share quantized humanoid samples. Other clips stay uncached.
	pub const fn is_sample_cacheable(self) -> bool {
		matches!(self, Self::Still | Self::Walk(_) | Self::Run(_))
	}

	pub const fn time_policy(self) -> ClipTimePolicy {
		match self {
			Self::Still => ClipTimePolicy::Unbounded,
			Self::Walk(_) | Self::Run(_) | Self::QuadrupedRun(_) | Self::Gallop(_) => {
				ClipTimePolicy::Cycle { duration: 1.0 }
			}
			Self::Tuck(_) | Self::TuckedFlip(_) | Self::Jab(_) => {
				ClipTimePolicy::Cycle { duration: 1.0 }
			}
			Self::Jump(_)
			| Self::Leap(_)
			| Self::TwoFootedTuckedFlip(_)
			| Self::PowerStrafe(_)
			| Self::Squat
			| Self::SquatDescent
			| Self::Prone => ClipTimePolicy::Clamp { duration: 1.0 },
			Self::Soaring(_) | Self::Flapping(_) => ClipTimePolicy::Unbounded,
			Self::LateralUndulation(_) | Self::DorsoventralUndulation(_) => {
				ClipTimePolicy::Cycle { duration: 1.0 }
			}
		}
	}

	/// Nearest bin for continuous `clip_time`. Evaluation uses [`SampleAddress::canonical_time`].
	pub fn sample_address(self, clip_time: f32, sample_interval_us: u32) -> SampleAddress {
		SampleAddress::from_clip_time(clip_time, sample_interval_us, self.time_policy())
	}
}

/// Clip + playback speed on a rig member.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AnimRef {
	pub clip: AnimClip,
	pub speed: f32,
}

impl AnimRef {
	pub fn new(clip: AnimClip) -> Self {
		Self { clip, speed: clip.default_speed() }
	}

	pub fn still() -> Self {
		Self::new(AnimClip::still())
	}
}

impl Default for AnimRef {
	fn default() -> Self {
		Self::still()
	}
}

/// BSN / ECS identity: play this clip on this rig member.
#[derive(Component, Clone, Copy, Debug, PartialEq, Default)]
pub struct AnimRefRoot(pub AnimRef);

#[cfg(test)]
mod tests {
	use super::*;
	use character_animations::animations::Walk;

	#[test]
	fn knob_changes_keep_clip_id() {
		let a = AnimClip::Walk(Walk { stride: 0.2, bounce: 1.0, rotation: 1.0 });
		let b = AnimClip::Walk(Walk { stride: 0.8, bounce: 2.0, rotation: 0.5 });
		assert_eq!(a.id(), b.id());
		assert_ne!(a, b);
		assert_ne!(AnimClip::walk().id(), AnimClip::run().id());
	}

	#[test]
	fn leap_is_a_distinct_clip() {
		assert_eq!(AnimClip::leap().id(), AnimId::Leap);
		assert_eq!(AnimClip::leap().default_speed(), LEAP_CYCLE_SPEED);
		assert_ne!(AnimClip::leap().id(), AnimClip::jump().id());
	}

	#[test]
	fn still_keeps_the_rest_identity() {
		assert_eq!(AnimClip::still().id(), AnimId::Still);
		assert_eq!(AnimClip::still().default_speed(), IDLE_CYCLE_SPEED);
		assert!(IDLE_CYCLE_SPEED < WALK_CYCLE_SPEED);
		assert_ne!(AnimClip::still().id(), AnimClip::walk().id());
	}

	#[test]
	fn idle_walk_run_are_the_first_cached_clips() {
		assert!(AnimClip::still().is_sample_cacheable());
		assert!(AnimClip::walk().is_sample_cacheable());
		assert!(AnimClip::run().is_sample_cacheable());
		assert!(!AnimClip::jab().is_sample_cacheable());
		assert!(!AnimClip::leap().is_sample_cacheable());
		assert_eq!(AnimClip::walk().revision(), 0);
		assert_eq!(AnimClip::walk().sample_address(0.123, 10_000).canonical_time, 0.12);
	}

	#[test]
	fn jump_elapsed_follows_leap_phase_windows() {
		let params = JumpParams::default();
		let takeoff = params.elapsed_from_phase(TAKEOFF_END);
		let air = params.elapsed_from_phase(AIR_END);
		let end = params.elapsed_from_phase(1.0);
		assert!(takeoff > 0.0);
		assert!(air > takeoff);
		assert!(end > air);
	}
}
