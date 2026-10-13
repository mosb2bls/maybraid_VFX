//! Boxing jab: chamber, snap toward a body-space target, recover to guard.
//!
//! # Body-space aim (stylized, not IK)
//!
//! [`Jab::target`] is relative to the body COM (root on humanoid v0). Axes match the
//! target vector — **not** Bevy camera forward:
//! - **+X** = right
//! - **+Y** = up
//! - **+Z** = ahead (fight forward)
//!
//! [`Jab::humerus_along`] builds a length direction in character space. The resolver
//! aims the humerus with that direction and a roll. The elbow stays a bone-local
//! flexion.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

/// Default jab aim: sternum height, straight ahead.
pub const DEFAULT_JAB_TARGET: Vec3 = Vec3::new(0.0, 0.35, 0.7);
/// Nominal chamber depth (`1.0` matches the tuned preparatory motion).
pub const DEFAULT_BACKSWING: f32 = 1.0;

const BACKSWING_END: f32 = 0.18;
const EXTEND_END: f32 = 0.42;
const HOLD_END: f32 = 0.52;

// --- Elbow ---
const GUARD_ELBOW: f32 = 1.5;
const CHAMBER_ELBOW: f32 = 1.65;
const EXTEND_ELBOW: f32 = 0.05;

// --- Humerus aim / whip (body-space weights before normalize) ---
/// ~90° ventral roll so forearm flex bends front ↔ back once aimed.
const PUNCH_ROLL: f32 = FRAC_PI_2;
const ARM_DROP: f32 = 0.42;
const ARM_DROP_MIN: f32 = 0.22;
const ARM_DROP_MAX: f32 = 0.9;
const HUMERUS_FORWARD: f32 = 0.7;
const HUMERUS_WHIP: f32 = 0.85;
const HUMERUS_CHAMBER_RESERVE: f32 = 0.22;
/// Guard-frame inboard |X|; left → −X, right → +X.
const HUMERUS_ADDUCT: f32 = 0.2;
/// Blend of jab-arm X toward [`Jab::target`].x by full extension.
const HUMERUS_LATERAL_BLEND: f32 = 0.55;
const SHOULDER_CARRY: f32 = 0.12;

// --- Trunk / stance ---
const TORSO_TURN: f32 = 1.0;
/// Sagittal waist fold into the punch (root + lumbar), radians at full extend.
const WAIST_BEND: f32 = 0.35;
const ROOT_LEAN: f32 = 0.05;
const LEAD_FEMUR: f32 = 0.16;
const REAR_FEMUR: f32 = -0.1;
const STANCE_SHIN: f32 = 0.14;
const HIP_TURN: f32 = 0.35;

// --- Target offset gains ---
const AIM_DROP_Y: f32 = 0.85;
const AIM_CARRY_Y: f32 = 0.3;
const AIM_ROLL_Y: f32 = 0.45;
const AIM_ROLL_X: f32 = 0.35;
const AIM_YAW_X: f32 = 0.55;
const AIM_ROLL_DELTA_MAX: f32 = 0.3;

/// Boxing jab knobs. The rig resolver maps these onto anatomical frames.
#[derive(Debug, Clone)]
pub struct Jab {
	pub side: Side,
	/// Preparatory chamber depth scale.
	pub backswing: f32,
	/// Aim point relative to body COM ([`DEFAULT_JAB_TARGET`] axes).
	pub target: Vec3,
}

impl Default for Jab {
	fn default() -> Self {
		Self { side: Side::Right, backswing: DEFAULT_BACKSWING, target: DEFAULT_JAB_TARGET }
	}
}

impl Jab {
	pub fn new(side: Side, backswing: f32, target: Vec3) -> Self {
		Self { side, backswing, target }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn with_backswing(mut self, backswing: f32) -> Self {
		self.backswing = backswing;
		self
	}

	pub fn with_target(mut self, target: Vec3) -> Self {
		self.target = target;
		self
	}

	pub fn opposite_side(&self) -> Side {
		self.side.opposite()
	}

	/// Reach scale from [`Self::target`].z vs [`DEFAULT_JAB_TARGET`].z.
	pub fn reach_scale(&self) -> f32 {
		let reference = DEFAULT_JAB_TARGET.z.max(f32::EPSILON);
		(self.target.z / reference).clamp(0.4, 1.5)
	}

	/// Height offset from default (positive = higher / toward the chin).
	pub fn aim_height(&self) -> f32 {
		self.target.y - DEFAULT_JAB_TARGET.y
	}

	/// Lateral offset from default (body +X = right).
	pub fn aim_lateral(&self) -> f32 {
		self.target.x - DEFAULT_JAB_TARGET.x
	}

	/// Chamber envelope: peaks in prep, clears for the snap.
	pub fn chamber_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).cycle();
		let raw = if t < BACKSWING_END {
			smoothstep(t / BACKSWING_END)
		} else if t < EXTEND_END {
			1.0 - smoothstep((t - BACKSWING_END) / (EXTEND_END - BACKSWING_END))
		} else {
			0.0
		};
		raw * self.backswing.clamp(0.0, 2.0)
	}

	/// Extension envelope: snap, brief hold, recover.
	pub fn extension_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).cycle();
		if t < BACKSWING_END {
			0.0
		} else if t < EXTEND_END {
			let u = (t - BACKSWING_END) / (EXTEND_END - BACKSWING_END);
			1.0 - (1.0 - u).powi(2)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	/// Punch-frame humerus roll (~π/2), plus a clamped aim tilt. Held for the clip.
	pub fn punch_roll(&self, _progress: f32) -> f32 {
		let delta = (AIM_ROLL_Y * self.aim_height()
			+ AIM_ROLL_X * self.side.sign() * self.aim_lateral())
		.clamp(-AIM_ROLL_DELTA_MAX, AIM_ROLL_DELTA_MAX);
		PUNCH_ROLL + delta
	}

	/// Down weight for [`Self::humerus_along`] (−Y). Jab arm raises slightly with the whip.
	pub fn arm_drop(&self, side: Side, progress: f32) -> f32 {
		let aimed = (ARM_DROP - AIM_DROP_Y * self.aim_height()).clamp(ARM_DROP_MIN, ARM_DROP_MAX);
		if side == self.side {
			aimed * (1.0 - 0.35 * self.extension_amount(progress))
		} else {
			aimed
		}
	}

	/// Forward weight for [`Self::humerus_along`] (+Z). Jab arm whips; cover holds guard.
	pub fn humerus_forward(&self, side: Side, progress: f32) -> f32 {
		let base = HUMERUS_FORWARD * self.reach_scale();
		if side != self.side {
			return base;
		}
		let extend = self.extension_amount(progress);
		let reserved = HUMERUS_CHAMBER_RESERVE * self.chamber_amount(progress).min(1.0);
		let whip = HUMERUS_WHIP * extend * self.reach_scale();
		(base - reserved + whip).max(0.2)
	}

	/// Lateral weight for [`Self::humerus_along`] (+X). Jab arm eases toward [`Self::target`].x.
	pub fn humerus_lateral(&self, side: Side, progress: f32) -> f32 {
		// Inboard: left −X, right +X.
		let base = -side.sign() * HUMERUS_ADDUCT;
		if side != self.side {
			return base;
		}
		base + (self.target.x - base) * HUMERUS_LATERAL_BLEND * self.extension_amount(progress)
	}

	/// Character-space humerus length direction. The resolver aims with this vector.
	pub fn humerus_along(&self, side: Side, progress: f32) -> Vec3 {
		Vec3::new(
			self.humerus_lateral(side, progress),
			-self.arm_drop(side, progress),
			self.humerus_forward(side, progress),
		)
		.normalize()
	}

	/// Tiny shoulder aim/height assist.
	pub fn shoulder_carry(&self, progress: f32) -> f32 {
		let aimed = (SHOULDER_CARRY + AIM_CARRY_Y * self.aim_height()).clamp(0.0, 0.45);
		aimed * (0.85 + 0.15 * self.extension_amount(progress))
	}

	/// Jab-arm elbow (larger = more flexed). Uncoils with the humerus whip.
	pub fn jab_elbow(&self, progress: f32) -> f32 {
		let chamber = self.chamber_amount(progress);
		let extend = self.extension_amount(progress);
		GUARD_ELBOW * (1.0 - extend) * (1.0 - chamber)
			+ CHAMBER_ELBOW * chamber
			+ EXTEND_ELBOW * extend
	}

	/// Cover elbow stays tucked.
	pub fn guard_elbow(&self, progress: f32) -> f32 {
		GUARD_ELBOW * (0.9 + 0.1 * self.extension_amount(progress))
	}

	pub fn root_lean(&self, progress: f32) -> f32 {
		self.extension_amount(progress) * ROOT_LEAN * self.reach_scale()
	}

	/// Sagittal bend at the waist into the punch (peaks with extension).
	///
	/// Humanoid maps this to spine **twist** (sagittal pitch), weighted toward root/lumbar.
	pub fn waist_bend(&self, progress: f32) -> f32 {
		let chamber = self.chamber_amount(progress);
		let extend = self.extension_amount(progress);
		// Slight upright wind-up, then fold forward with the snap.
		WAIST_BEND * (extend - 0.25 * chamber).max(0.0) * self.reach_scale()
	}

	/// Trunk turn into the jab (peaks with extension; + lateral aim bias).
	pub fn torso_turn(&self, progress: f32) -> f32 {
		let chamber = self.chamber_amount(progress);
		let extend = self.extension_amount(progress);
		let base = TORSO_TURN * (extend - 0.35 * chamber) * self.reach_scale();
		let lateral = AIM_YAW_X * self.side.sign() * self.aim_lateral() * (0.35 + 0.65 * extend);
		base + lateral
	}

	pub fn hip_turn(&self, progress: f32) -> f32 {
		self.torso_turn(progress) * HIP_TURN
	}

	pub fn lead_femur_swing(&self, progress: f32) -> f32 {
		LEAD_FEMUR * (0.55 + 0.45 * self.extension_amount(progress))
	}

	pub fn rear_femur_swing(&self, progress: f32) -> f32 {
		REAR_FEMUR * (0.55 + 0.45 * self.extension_amount(progress))
	}

	pub fn stance_shin_flex(&self, progress: f32) -> f32 {
		STANCE_SHIN * (0.7 + 0.3 * self.extension_amount(progress))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn peak() -> f32 {
		(EXTEND_END + HOLD_END) * 0.5
	}

	#[test]
	fn jab_chambers_before_extension() -> anyhow::Result<()> {
		let jab = Jab::default();
		let mid_chamber = BACKSWING_END * 0.5;
		assert!(jab.chamber_amount(mid_chamber) > 0.4);
		assert!(jab.extension_amount(mid_chamber) < 0.05);
		Ok(())
	}

	#[test]
	fn jab_reaches_near_full_extension() -> anyhow::Result<()> {
		let jab = Jab::default();
		assert!(jab.extension_amount(peak()) > 0.95);
		assert!(jab.chamber_amount(peak()) < 0.05);
		assert!(jab.jab_elbow(peak()) < GUARD_ELBOW * 0.25);
		Ok(())
	}

	#[test]
	fn jab_whips_humerus_and_uncoils_elbow() -> anyhow::Result<()> {
		let jab = Jab::default().with_side(Side::Right);
		let guard = 0.0;
		let p = peak();
		assert!((jab.jab_elbow(guard) - jab.jab_elbow(p)).abs() > 1.0);
		assert!(
			jab.humerus_forward(Side::Right, p) > jab.humerus_forward(Side::Right, guard) + 0.4
		);
		assert!(jab.humerus_along(Side::Right, p).z > jab.humerus_along(Side::Right, guard).z);
		assert!(
			(jab.humerus_forward(Side::Left, p) - jab.humerus_forward(Side::Left, guard)).abs()
				< 1e-4
		);
		assert!(jab.torso_turn(p).abs() > jab.torso_turn(guard).abs());
		Ok(())
	}

	#[test]
	fn jab_humerus_swings_toward_target_x_on_extension() -> anyhow::Result<()> {
		let across = Jab::default().with_side(Side::Right).with_target(Vec3::new(-0.25, 0.35, 0.7));
		let guard_x = across.humerus_lateral(Side::Right, 0.0);
		let peak_x = across.humerus_lateral(Side::Right, peak());
		assert!(peak_x < guard_x);
		assert!((peak_x - guard_x).abs() > 0.05);
		Ok(())
	}

	#[test]
	fn punch_roll_is_held_near_ninety_degrees() -> anyhow::Result<()> {
		let jab = Jab::default();
		assert!((jab.punch_roll(0.0) - FRAC_PI_2).abs() < 1e-4);
		assert!((jab.punch_roll(0.47) - jab.punch_roll(0.0)).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn higher_target_reduces_arm_drop() -> anyhow::Result<()> {
		let sternum = Jab::default().with_side(Side::Right);
		let chin = Jab::default().with_side(Side::Right).with_target(Vec3::new(0.0, 0.55, 0.7));
		assert!(chin.arm_drop(Side::Right, 0.0) < sternum.arm_drop(Side::Right, 0.0));
		assert!(chin.shoulder_carry(0.0) > sternum.shoulder_carry(0.0));
		Ok(())
	}

	#[test]
	fn lower_target_increases_arm_drop() -> anyhow::Result<()> {
		let sternum = Jab::default().with_side(Side::Right);
		let gut = Jab::default().with_side(Side::Right).with_target(Vec3::new(0.0, 0.15, 0.7));
		assert!(gut.arm_drop(Side::Right, 0.0) > sternum.arm_drop(Side::Right, 0.0));
		Ok(())
	}

	#[test]
	fn across_body_target_increases_torso_turn_for_right_jab() -> anyhow::Result<()> {
		let center = Jab::default().with_side(Side::Right);
		let across = Jab::default().with_side(Side::Right).with_target(Vec3::new(-0.25, 0.35, 0.7));
		assert!(across.torso_turn(peak()) > center.torso_turn(peak()));
		Ok(())
	}

	#[test]
	fn aim_keeps_punch_roll_near_sagittal_band() -> anyhow::Result<()> {
		let high_across =
			Jab::default().with_side(Side::Right).with_target(Vec3::new(-0.4, 0.7, 0.7));
		assert!((high_across.punch_roll(0.0) - FRAC_PI_2).abs() <= AIM_ROLL_DELTA_MAX + 1e-4);
		Ok(())
	}

	#[test]
	fn jab_recovers_to_guard_by_cycle_end() -> anyhow::Result<()> {
		let jab = Jab::default();
		assert!(jab.extension_amount(0.99) < 0.08);
		assert!((jab.jab_elbow(0.99) - GUARD_ELBOW).abs() < 0.2);
		Ok(())
	}

	#[test]
	fn backswing_scales_chamber_depth() -> anyhow::Result<()> {
		let deep = Jab::default().with_backswing(1.5);
		let shallow = Jab::default().with_backswing(0.25);
		let t = BACKSWING_END * 0.5;
		assert!(deep.chamber_amount(t) > shallow.chamber_amount(t));
		Ok(())
	}

	#[test]
	fn farther_target_increases_reach_scale() -> anyhow::Result<()> {
		let near = Jab::default().with_target(Vec3::new(0.0, 0.35, 0.4));
		let far = Jab::default().with_target(Vec3::new(0.0, 0.35, 1.0));
		assert!(far.reach_scale() > near.reach_scale());
		Ok(())
	}

	#[test]
	fn chamber_deepens_elbow_bend() -> anyhow::Result<()> {
		let jab = Jab::default();
		let mid_chamber = BACKSWING_END * 0.5;
		assert!(jab.jab_elbow(mid_chamber) > jab.jab_elbow(0.0));
		Ok(())
	}

	#[test]
	fn torso_turns_into_the_extension() -> anyhow::Result<()> {
		let jab = Jab::default();
		assert!(jab.torso_turn(peak()) > jab.torso_turn(0.0));
		assert!(jab.hip_turn(peak()).abs() > 0.0);
		Ok(())
	}

	#[test]
	fn waist_bends_into_the_extension() -> anyhow::Result<()> {
		let jab = Jab::default();
		assert!(jab.waist_bend(0.0) < 0.02);
		assert!(jab.waist_bend(peak()) > 0.1);
		Ok(())
	}

	#[test]
	fn guard_lateral_is_inboard_for_both_sides() -> anyhow::Result<()> {
		let jab = Jab::default();
		assert!(jab.humerus_lateral(Side::Left, 0.0) < 0.0);
		assert!(jab.humerus_lateral(Side::Right, 0.0) > 0.0);
		Ok(())
	}
}
