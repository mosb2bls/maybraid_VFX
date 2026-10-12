pub mod dorsoventral_undulation;
pub mod fall;
pub mod fixed;
pub mod fixed_tuck;
pub mod flapping;
pub mod gallop;
pub mod idle;
pub mod jab;
pub mod land;
pub mod lateral_undulation;
pub mod leap;
pub mod mix;
pub mod prone;
pub mod quadruped_bray;
pub mod quadruped_gallop;
pub mod quadruped_idle;
pub mod quadruped_leap;
pub mod quadruped_run;
pub mod quadruped_sleep;
pub mod run;
pub mod soaring;
pub mod spring;
pub mod squat;
pub mod squat_descent;
pub mod transition;
pub mod tuck;
pub mod tucked_flip;
pub mod two_footed_jump;
pub mod two_footed_tucked_flip;
pub mod upright_leap;
pub mod upright_run;
pub mod upright_walk;
pub mod walk;

pub use dorsoventral_undulation::DorsoventralUndulation;
pub use fall::Fall;
pub use fixed::FixedPosition;
pub use fixed_tuck::FixedTuck;
pub use flapping::Flapping;
pub use gallop::Gallop;
pub use idle::Idle;
pub use jab::{Jab, DEFAULT_BACKSWING, DEFAULT_JAB_TARGET};
pub use land::Land;
pub use lateral_undulation::LateralUndulation;
pub use leap::{Leap, AIR_END, TAKEOFF_END};
pub use mix::{burst_envelope, smoothstep, Mix, Smooth, DEFAULT_BURST_EDGE};
pub use prone::Prone;
pub use quadruped_bray::{
	EpiphantBray, MOUTH_EMIT_NECK_LOCAL, MOUTH_EMIT_SOCKET, SONIC_EMIT_NORM,
};
pub use quadruped_gallop::QuadrupedGallop;
pub use quadruped_idle::QuadrupedIdle;
pub use quadruped_leap::QuadrupedLeap;
pub use quadruped_sleep::{
	EpiphantSleepEnter, EpiphantSleepHold, EpiphantWake, SLEEP_ENTER_IMPACT_NORM, WAKE_IMPACT_NORM,
};
pub use quadruped_run::{QuadrupedRun, QuadrupedRunPose};
pub use run::Run;
pub use soaring::Soaring;
pub use spring::Spring;
pub use squat::{vertical_drop, Squat};
pub use squat_descent::{SquatDescent, DEFAULT_DESCENT_SPEED};
pub use transition::{BlendCurve, Transition, TransitionCurve};
pub use tuck::{Tuck, TuckProfile};
pub use tucked_flip::{FlipDirection, TuckedFlip};
pub use two_footed_jump::{
	BallisticJump, JumpSegment, JumpTiming, TwoFootedJump, DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT,
	DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED, DEFAULT_SPRING_DURATION,
	FALL_BLEND_FRACTION, LAND_BLEND_FRACTION, LAND_POSE_BLEND_MAX_SECS,
};
pub use two_footed_tucked_flip::TwoFootedTuckedFlip;
pub use upright_leap::UprightLeap;
pub use upright_run::UprightRun;
pub use upright_walk::UprightWalk;
pub use walk::Walk;
