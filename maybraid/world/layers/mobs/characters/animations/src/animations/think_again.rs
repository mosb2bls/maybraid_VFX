//! "Think Again" meme gesture: raised upper arm, forearm ticks beside the head.
//!
//! Character space matches [`Jab`](super::Jab): +X right, +Y up, +Z fight-forward.
//! The humerus abducts laterally and drops ~5° as the elbow folds. The head
//! cocks toward the raised hand on the move into Think and eases back on Again.
//! Cadence is four beats at one second each:
//!
//! 1. T-pose → Think (head cocks)
//! 2. Hold Think
//! 3. Think → Again (head un-cocks)
//! 4. Hold Again, then recover so a loop can restart from T-pose
//!
//! Forearm tilt in tests is derived from posed elbow→tip positions: `atan2(Δx, Δy)`.
//! Think ≈ +45° inboard; Again ≈ +15° inboard.

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;

/// One second per beat at unit speed.
pub const BEAT: f32 = 1.0;
/// Whole one-shot: raise | hold Think | move | hold Again.
pub const DURATION: f32 = 4.0 * BEAT;
/// Default playback rate. `2.0` is a half-second beat.
pub const DEFAULT_SPEED: f32 = 2.0;
/// End of beat 2. Think is held from [`BEAT`] until this instant.
const THINK_HOLD_END: f32 = 2.0 * BEAT;
/// End of beat 3. Again is reached here and held into beat 4.
const AGAIN_ARRIVE: f32 = 3.0 * BEAT;
/// Beat 4 holds Again, then this recover returns to T-pose.
const RECOVER_START: f32 = 3.75;

/// Elbow flex at Think. Extra fold so the hand stays inboard after the −5° drop.
const THINK_ELBOW: f32 = std::f32::consts::FRAC_PI_2 + 50_f32.to_radians();
/// Elbow flex at Again. Shallower than Think by 30°.
const AGAIN_ELBOW: f32 = std::f32::consts::FRAC_PI_2 + 20_f32.to_radians();
/// Humerus depression from the T-pose horizontal, in radians, at Think.
const HUMERUS_ELEVATION: f32 = -5_f32.to_radians();
/// Roll about the lateral humerus so the forearm hinge opens beside the head.
///
/// Not multiplied by [`Side::sign`]: bilateral mirroring is already carried by
/// [`Self::humerus_along`] and the rig's mirrored shoulder rests. See rig tests.
const THINK_AGAIN_ROLL: f32 = 0.0;
/// Ear-toward-shoulder cock on the move into Think. Follows [`Side::sign`].
const HEAD_COCK_TILT: f32 = 22_f32.to_radians();
/// Slight chin tuck so the cock reads as a thought, not a shrug.
const HEAD_COCK_NOD: f32 = 8_f32.to_radians();

/// One-shot Think Again knobs. The rig resolver maps these onto anatomical frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThinkAgain {
	pub side: Side,
}

impl Default for ThinkAgain {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl ThinkAgain {
	pub fn new(side: Side) -> Self {
		Self { side }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn opposite_side(&self) -> Side {
		self.side.opposite()
	}

	/// How much of the raised pose is still Think. 1 on beat 2, 0 once Again arrives.
	pub fn think_amount(&self, progress: f32) -> f32 {
		self.gesture_amount(progress) * (1.0 - self.again_mix(progress))
	}

	/// How much of the raised pose is Again. 0 through Think, 1 on beat 4.
	pub fn again_amount(&self, progress: f32) -> f32 {
		self.gesture_amount(progress) * self.again_mix(progress)
	}

	/// Arm is up from the end of beat 1 through the Again hold.
	pub fn gesture_amount(&self, progress: f32) -> f32 {
		let t = clip_time(progress);
		if t < BEAT {
			smoothstep(t / BEAT)
		} else if t < RECOVER_START {
			1.0
		} else {
			1.0 - smoothstep((t - RECOVER_START) / (DURATION - RECOVER_START))
		}
	}

	/// 0 = Think, 1 = Again. Sweeps only on beat 3.
	fn again_mix(&self, progress: f32) -> f32 {
		let t = clip_time(progress);
		if t < THINK_HOLD_END {
			0.0
		} else if t < AGAIN_ARRIVE {
			smoothstep((t - THINK_HOLD_END) / (AGAIN_ARRIVE - THINK_HOLD_END))
		} else {
			1.0
		}
	}

	/// Forearm flex in radians. Think holds, then blends to Again on beat 3.
	pub fn elbow_flexion(&self, progress: f32) -> f32 {
		THINK_ELBOW * self.think_amount(progress) + AGAIN_ELBOW * self.again_amount(progress)
	}

	/// Character-space humerus length direction: lateral abduction plus a small lift.
	///
	/// [`Side::sign`] is `+1` on left and `−1` on right (yaw/roll mirroring). Lateral
	/// outboard matches rest geometry: right → `−X`, left → `+X`. Elevation tracks
	/// the elbow envelope so the upper arm drops below the T-pose as the forearm folds.
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.gesture_amount(progress);
		let elev = HUMERUS_ELEVATION * amount;
		Vec3::new(self.side.sign() * elev.cos(), elev.sin(), 0.0).normalize_or_zero()
	}

	pub fn humerus_roll(&self, _progress: f32) -> f32 {
		THINK_AGAIN_ROLL
	}

	/// Lateral neck tilt toward the gesture arm. Peaks with Think, eases on Again.
	///
	/// Neck flex is parent Z: positive cocks toward −X. Gesture outboard is
	/// [`Side::sign`] (left +X, right −X), so the tilt negates that sign.
	pub fn head_cock_tilt(&self, progress: f32) -> f32 {
		-HEAD_COCK_TILT * self.side.sign() * self.think_amount(progress)
	}

	/// Small forward nod that rides with [`Self::head_cock_tilt`].
	pub fn head_cock_nod(&self, progress: f32) -> f32 {
		HEAD_COCK_NOD * self.think_amount(progress)
	}
}

fn clip_time(progress: f32) -> f32 {
	progress.clamp(0.0, DURATION)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn think_hold() -> f32 {
		1.50
	}

	fn again_hold() -> f32 {
		3.40
	}

	#[test]
	fn think_again_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!(clip.gesture_amount(0.0) < 1e-4);
		assert!(clip.gesture_amount(DURATION) < 1e-4);
		assert!(clip.elbow_flexion(0.0) < 1e-4);
		assert!(clip.elbow_flexion(DURATION) < 0.05);
		Ok(())
	}

	#[test]
	fn think_again_reaches_think_hold() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!(clip.think_amount(think_hold()) > 0.99);
		assert!(clip.again_amount(think_hold()) < 1e-4);
		assert!((clip.elbow_flexion(think_hold()) - THINK_ELBOW).abs() < 0.02);
		Ok(())
	}

	#[test]
	fn think_again_second_beat_holds_think() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!((clip.elbow_flexion(1.2) - THINK_ELBOW).abs() < 0.02);
		assert!((clip.elbow_flexion(1.8) - THINK_ELBOW).abs() < 0.02);
		assert!(clip.gesture_amount(1.5) > 0.99);
		Ok(())
	}

	#[test]
	fn think_again_reaches_again_hold() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!(clip.again_amount(again_hold()) > 0.99);
		assert!(clip.think_amount(again_hold()) < 1e-4);
		assert!((clip.elbow_flexion(again_hold()) - AGAIN_ELBOW).abs() < 0.02);
		Ok(())
	}

	#[test]
	fn think_again_humerus_drops_with_the_elbow() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let along = clip.humerus_along(think_hold());
		let elev = along.y.atan2(along.x.abs()).to_degrees();
		assert!(along.x.abs() > 0.98, "stays mostly lateral, got {along:?}");
		assert!(
			(elev + 5.0).abs() < 0.5,
			"humerus should drop ~5° at Think, got {elev:.1}° {along:?}"
		);
		Ok(())
	}

	#[test]
	fn think_again_lateral_sign_follows_side() -> anyhow::Result<()> {
		let right = ThinkAgain::default().with_side(Side::Right);
		let left = ThinkAgain::default().with_side(Side::Left);
		let r = right.humerus_along(think_hold());
		let l = left.humerus_along(think_hold());
		assert!(r.x < 0.0, "right arm lateral −X, got {r:?}");
		assert!(l.x > 0.0, "left arm lateral +X, got {l:?}");
		assert!((r.x + l.x).abs() < 0.05, "mirrored lateral aim {r:?} {l:?}");
		Ok(())
	}

	#[test]
	fn think_again_again_is_shallower_than_think() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let delta = clip.elbow_flexion(think_hold()) - clip.elbow_flexion(again_hold());
		assert!(
			(delta - 30_f32.to_radians()).abs() < 0.05,
			"expected 30° Think→Again, got {delta}"
		);
		Ok(())
	}

	#[test]
	fn think_again_head_cocks_on_think() -> anyhow::Result<()> {
		let right = ThinkAgain::default().with_side(Side::Right);
		let left = ThinkAgain::default().with_side(Side::Left);
		assert!(
			(right.head_cock_tilt(think_hold()) + HEAD_COCK_TILT * Side::Right.sign()).abs() < 1e-4
		);
		assert!(
			(left.head_cock_tilt(think_hold()) + HEAD_COCK_TILT * Side::Left.sign()).abs() < 1e-4
		);
		assert!(right.head_cock_nod(think_hold()) > 0.1);
		assert!(right.head_cock_tilt(again_hold()).abs() < 1e-4);
		assert!(right.head_cock_nod(again_hold()).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn think_again_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!((clip.gesture_amount(DURATION + 0.5) - clip.gesture_amount(DURATION)).abs() < 1e-4);
		Ok(())
	}
}
