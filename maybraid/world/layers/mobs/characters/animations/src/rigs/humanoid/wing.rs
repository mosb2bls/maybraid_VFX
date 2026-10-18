//! Shared biped wing-spread pose used by soaring and flapping.
//!
//! The humanoid rest pose is a T-pose (arms already out along ±X). Held lift
//! stays near zero. Wing beats still use the imported shoulder **swing** axis
//! (parent Y on DEFAULT), the stroke the previous clips used.

use character_rigs::authoring::HumanoidPose;
use character_rigs::Side;

/// Held shoulder forward flexion (sagittal): angle the wing root away from the spine.
pub(crate) const SOAR_SHOULDER_SWING: f32 = 0.35;
/// T-pose already spreads the arms; keep held lift near zero.
pub(crate) const SOAR_SHOULDER_FLEX: f32 = 0.0;
pub(crate) const SOAR_HUMERUS_SWING: f32 = 0.08;
pub(crate) const FOREARM_EXTEND: f32 = -0.05;
pub(crate) const LEG_TRAIL: f32 = -0.22;
pub(crate) const KNEE_SOFT: f32 = 0.28;
pub(crate) const ROOT_LEAN: f32 = -0.12;

/// Flap stroke about the imported shoulder swing axis (parent Y), not a new lift.
pub(crate) const FLAP_SHOULDER_SWING_AMP: f32 = 0.4;
pub(crate) const FLAP_HUMERUS_AMP: f32 = 0.22;
pub(crate) const FLAP_ELBOW_AMP: f32 = 0.12;

/// Opposite signs angle each shoulder away from the spine. This is pose bias.
fn lateral_sign(side: Side) -> f32 {
	match side {
		Side::Left => 1.0,
		Side::Right => -1.0,
	}
}

/// Apply trailing legs + slight sagittal lean for a flight silhouette.
pub(crate) fn apply_flight_body(pose: &mut HumanoidPose) {
	pose.apply_root(ROOT_LEAN);
	pose.apply_leg(Side::Left, LEG_TRAIL, KNEE_SOFT);
	pose.apply_leg(Side::Right, LEG_TRAIL, KNEE_SOFT);
}

/// Hold a T-pose-relative wing spread with optional sagittal flap modulation.
///
/// `flap_amount` is typically in `[-range, range]`; negative is the rearward stroke.
pub(crate) fn apply_flight_wings(pose: &mut HumanoidPose, flap_amount: f32) {
	let shoulder_swing = SOAR_SHOULDER_SWING + FLAP_SHOULDER_SWING_AMP * flap_amount;
	let humerus_swing = SOAR_HUMERUS_SWING + FLAP_HUMERUS_AMP * flap_amount;
	let forearm_flex = FOREARM_EXTEND + FLAP_ELBOW_AMP * flap_amount.max(0.0);

	for side in [Side::Left, Side::Right] {
		let lateral = lateral_sign(side);
		pose.apply_arm(
			side,
			// Sagittal: held spine angle + front/back wing beat.
			shoulder_swing * lateral,
			// Lateral lift stays near rest — T-pose already holds the spread.
			SOAR_SHOULDER_FLEX,
			humerus_swing * -lateral,
			0.0,
			forearm_flex,
		);
	}
}
