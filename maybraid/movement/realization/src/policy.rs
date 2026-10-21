//! Local motor recovery: strafe, hop, backup. Does not replan until the policy ends.

use bevy::prelude::*;

const WATCH_SECONDS: f32 = 0.55;
const STRAFE_SECONDS: f32 = 0.35;
const JUMP_SECONDS: f32 = 0.45;
const BACKUP_SECONDS: f32 = 0.35;
const COOLDOWN_SECONDS: f32 = 0.9;
const STUCK_SPEED: f32 = 0.18;
const PROGRESS_SPEED: f32 = 0.28;
const PROGRESS_Y_SPEED: f32 = 0.22;
const PROGRESS_MOVE: f32 = 0.18;
const WISH_EPS: f32 = 0.05;

/// Per-capsule realization state. Idle until a live wish is not becoming motion.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct MovementRealization {
	pub phase: RealizationPhase,
	pub attempts: u32,
}

impl Default for MovementRealization {
	fn default() -> Self {
		Self { phase: RealizationPhase::Idle, attempts: 0 }
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RealizationPhase {
	Idle,
	Watching { seconds: f32, at: Vec3 },
	Strafe { seconds: f32, sign: f32, at: Vec3 },
	Jump { seconds: f32, launched: bool, at: Vec3 },
	Backup { seconds: f32, at: Vec3 },
	Cooldown { seconds: f32 },
}

impl Default for RealizationPhase {
	fn default() -> Self {
		Self::Idle
	}
}

/// Snapshot the motor needs for one tick. Planner wish is `wish`; this layer may override it.
#[derive(Clone, Copy, Debug)]
pub struct RealizationSample {
	pub dt: f32,
	pub position: Vec3,
	pub velocity: Vec3,
	pub wish: Vec3,
	pub grounded: bool,
	pub jumping: bool,
	pub max_jump: f32,
}

/// Side effects for the body this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RealizationCommand {
	pub wish_override: Option<Vec3>,
	pub jump: bool,
	pub replan: bool,
}

impl MovementRealization {
	pub fn tick(&mut self, sample: RealizationSample) -> RealizationCommand {
		if sample.dt <= 0.0 {
			return RealizationCommand::default();
		}
		if xz(sample.wish).length() < WISH_EPS {
			self.phase = RealizationPhase::Idle;
			return RealizationCommand::default();
		}
		let in_jump = matches!(self.phase, RealizationPhase::Jump { .. });
		let in_cooldown = matches!(self.phase, RealizationPhase::Cooldown { .. });
		if matches!(self.phase, RealizationPhase::Strafe { .. } | RealizationPhase::Backup { .. })
			&& !is_jammed(&sample)
		{
			self.phase = RealizationPhase::Idle;
			return RealizationCommand::default();
		}
		if !in_jump && !in_cooldown && self.made_progress(&sample) {
			self.phase = RealizationPhase::Idle;
			return RealizationCommand::default();
		}

		match self.phase {
			RealizationPhase::Idle => {
				if is_jammed(&sample) {
					self.phase = RealizationPhase::Watching { seconds: 0.0, at: sample.position };
				}
				RealizationCommand::default()
			}
			RealizationPhase::Watching { seconds, at } => {
				if !is_jammed(&sample) {
					self.phase = RealizationPhase::Idle;
					return RealizationCommand::default();
				}
				let seconds = seconds + sample.dt;
				if seconds >= WATCH_SECONDS {
					let sign = if self.attempts % 2 == 0 { 1.0 } else { -1.0 };
					self.attempts = self.attempts.saturating_add(1);
					self.phase = RealizationPhase::Strafe { seconds: 0.0, sign, at };
				} else {
					self.phase = RealizationPhase::Watching { seconds, at };
				}
				RealizationCommand::default()
			}
			RealizationPhase::Strafe { seconds, sign, at } => {
				let seconds = seconds + sample.dt;
				if seconds >= STRAFE_SECONDS {
					self.phase = if sample.max_jump > 1e-3 {
						RealizationPhase::Jump { seconds: 0.0, launched: false, at }
					} else {
						RealizationPhase::Backup { seconds: 0.0, at }
					};
					return RealizationCommand::default();
				}
				self.phase = RealizationPhase::Strafe { seconds, sign, at };
				RealizationCommand {
					wish_override: Some(strafe_dir(sample.wish, sign)),
					..Default::default()
				}
			}
			RealizationPhase::Jump { seconds, launched, at } => {
				let mut jump = false;
				let mut launched = launched;
				if !launched && sample.grounded && !sample.jumping {
					jump = true;
					launched = true;
				}
				let seconds = seconds + sample.dt;
				let landed = launched && sample.grounded && !sample.jumping && seconds > 0.12;
				if seconds >= JUMP_SECONDS || landed {
					if !is_jammed(&sample) {
						self.phase = RealizationPhase::Idle;
						return RealizationCommand { jump, ..Default::default() };
					}
					self.phase = RealizationPhase::Backup { seconds: 0.0, at: sample.position };
					return RealizationCommand { jump, ..Default::default() };
				}
				self.phase = RealizationPhase::Jump { seconds, launched, at };
				RealizationCommand { jump, ..Default::default() }
			}
			RealizationPhase::Backup { seconds, at } => {
				let seconds = seconds + sample.dt;
				if seconds >= BACKUP_SECONDS {
					let still_jammed = is_jammed(&sample);
					self.phase = RealizationPhase::Cooldown { seconds: 0.0 };
					return RealizationCommand {
						wish_override: Some(backup_dir(sample.wish)),
						replan: still_jammed,
						..Default::default()
					};
				}
				self.phase = RealizationPhase::Backup { seconds, at };
				RealizationCommand {
					wish_override: Some(backup_dir(sample.wish)),
					..Default::default()
				}
			}
			RealizationPhase::Cooldown { seconds } => {
				let seconds = seconds + sample.dt;
				if seconds >= COOLDOWN_SECONDS {
					self.phase = RealizationPhase::Idle;
				} else {
					self.phase = RealizationPhase::Cooldown { seconds };
				}
				RealizationCommand::default()
			}
		}
	}

	fn made_progress(&self, sample: &RealizationSample) -> bool {
		let speed = xz(sample.velocity).length();
		if speed >= PROGRESS_SPEED || sample.velocity.y >= PROGRESS_Y_SPEED {
			return true;
		}
		let Some(at) = self.anchor() else {
			return false;
		};
		sample.position.distance(at) >= PROGRESS_MOVE
	}

	fn anchor(&self) -> Option<Vec3> {
		match self.phase {
			RealizationPhase::Watching { at, .. }
			| RealizationPhase::Strafe { at, .. }
			| RealizationPhase::Jump { at, .. }
			| RealizationPhase::Backup { at, .. } => Some(at),
			_ => None,
		}
	}
}

fn is_jammed(sample: &RealizationSample) -> bool {
	if sample.jumping || !sample.grounded {
		return false;
	}
	if sample.velocity.y > PROGRESS_Y_SPEED {
		return false;
	}
	xz(sample.velocity).length() < STUCK_SPEED
}

fn xz(v: Vec3) -> Vec3 {
	Vec3::new(v.x, 0.0, v.z)
}

/// Horizontal wish rotated 90° about +Y (`sign` +1 is wish × +Y).
pub fn strafe_dir(wish: Vec3, sign: f32) -> Vec3 {
	let w = xz(wish);
	if w.length_squared() < 1e-8 {
		return Vec3::ZERO;
	}
	w.cross(Vec3::Y).normalize_or_zero() * sign.signum()
}

pub fn backup_dir(wish: Vec3) -> Vec3 {
	-xz(wish).normalize_or_zero()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn jammed_at(pos: Vec3) -> RealizationSample {
		RealizationSample {
			dt: 0.05,
			position: pos,
			velocity: Vec3::ZERO,
			wish: Vec3::X,
			grounded: true,
			jumping: false,
			max_jump: 1.0,
		}
	}

	#[test]
	fn strafe_is_perpendicular_to_wish() -> anyhow::Result<()> {
		let left = strafe_dir(Vec3::X, 1.0);
		assert!((left - Vec3::Z).length() < 1e-4, "{left}");
		let back = backup_dir(Vec3::X);
		assert!((back - Vec3::NEG_X).length() < 1e-4, "{back}");
		Ok(())
	}

	#[test]
	fn jammed_wish_strafes_then_jumps_then_replans() -> anyhow::Result<()> {
		let mut motor = MovementRealization::default();
		let mut pos = Vec3::ZERO;
		let mut saw_strafe = false;
		let mut saw_jump = false;
		let mut saw_replan = false;
		for _ in 0..80 {
			let cmd = motor.tick(jammed_at(pos));
			if cmd.wish_override.is_some() {
				if matches!(motor.phase, RealizationPhase::Strafe { .. })
					|| matches!(motor.phase, RealizationPhase::Backup { .. })
				{
					saw_strafe |= matches!(motor.phase, RealizationPhase::Strafe { .. });
				}
			}
			saw_jump |= cmd.jump;
			if cmd.replan {
				saw_replan = true;
				break;
			}
			pos += Vec3::ZERO;
		}
		assert!(saw_strafe, "expected a strafe override");
		assert!(saw_jump, "expected a hop");
		assert!(saw_replan, "expected replan after backup");
		Ok(())
	}

	#[test]
	fn y_progress_cancels_unstick() -> anyhow::Result<()> {
		let mut motor = MovementRealization::default();
		for _ in 0..10 {
			motor.tick(jammed_at(Vec3::ZERO));
		}
		assert!(!matches!(motor.phase, RealizationPhase::Idle));
		let climbing = RealizationSample {
			dt: 0.05,
			position: Vec3::new(0.0, 0.2, 0.0),
			velocity: Vec3::new(0.0, 0.5, 0.0),
			wish: Vec3::X,
			grounded: true,
			jumping: false,
			max_jump: 1.0,
		};
		motor.tick(climbing);
		assert_eq!(motor.phase, RealizationPhase::Idle);
		Ok(())
	}

	#[test]
	fn motion_during_strafe_drops_unstick() -> anyhow::Result<()> {
		let mut motor = MovementRealization::default();
		for _ in 0..40 {
			motor.tick(jammed_at(Vec3::ZERO));
			if matches!(motor.phase, RealizationPhase::Strafe { .. }) {
				break;
			}
		}
		assert!(matches!(motor.phase, RealizationPhase::Strafe { .. }), "{:?}", motor.phase);
		let walking = RealizationSample {
			dt: 0.05,
			position: Vec3::new(0.3, 0.0, 0.0),
			velocity: Vec3::new(1.2, 0.0, 0.0),
			wish: Vec3::X,
			grounded: true,
			jumping: false,
			max_jump: 1.0,
		};
		motor.tick(walking);
		assert_eq!(motor.phase, RealizationPhase::Idle);
		Ok(())
	}

	#[test]
	fn zero_max_jump_skips_hop() -> anyhow::Result<()> {
		let mut motor = MovementRealization::default();
		let sample = RealizationSample { max_jump: 0.0, ..jammed_at(Vec3::ZERO) };
		for _ in 0..40 {
			let cmd = motor.tick(sample);
			assert!(!cmd.jump);
			if matches!(motor.phase, RealizationPhase::Backup { .. }) {
				return Ok(());
			}
		}
		panic!("expected backup without jump, phase={:?}", motor.phase);
	}

	#[test]
	fn backup_skips_replan_once_moving() -> anyhow::Result<()> {
		let mut motor = MovementRealization::default();
		let jammed = RealizationSample { max_jump: 0.0, ..jammed_at(Vec3::ZERO) };
		for _ in 0..40 {
			motor.tick(jammed);
			if matches!(motor.phase, RealizationPhase::Backup { .. }) {
				break;
			}
		}
		assert!(matches!(motor.phase, RealizationPhase::Backup { .. }), "{:?}", motor.phase);
		let walking = RealizationSample {
			dt: 0.05,
			position: Vec3::new(-0.4, 0.0, 0.0),
			velocity: Vec3::new(-1.0, 0.0, 0.0),
			wish: Vec3::X,
			grounded: true,
			jumping: false,
			max_jump: 0.0,
		};
		let cmd = motor.tick(walking);
		assert!(!cmd.replan);
		assert_eq!(motor.phase, RealizationPhase::Idle);
		Ok(())
	}
}
