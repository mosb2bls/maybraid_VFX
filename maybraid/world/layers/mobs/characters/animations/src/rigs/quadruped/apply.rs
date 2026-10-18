use character_rigs::authoring::QuadrupedPose;
use character_rigs::Side;

pub fn apply_front_leg(
	pose: &mut QuadrupedPose,
	side: Side,
	shoulder_swing: f32,
	shoulder_flex: f32,
	thigh_swing: f32,
	shin_flex: f32,
) {
	let leg = pose.front_mut(side);
	leg.proximal_turn = shoulder_swing;
	leg.proximal_lateral = shoulder_flex;
	leg.stride = thigh_swing;
	leg.hinge = shin_flex;
}

pub fn apply_hind_leg(
	pose: &mut QuadrupedPose,
	side: Side,
	hip_swing: f32,
	hip_flex: f32,
	thigh_swing: f32,
	shin_flex: f32,
) {
	let leg = pose.hind_mut(side);
	leg.proximal_turn = hip_swing;
	leg.proximal_lateral = hip_flex;
	leg.stride = thigh_swing;
	leg.hinge = shin_flex;
}

pub fn apply_spine(pose: &mut QuadrupedPose, back_ridge_swing: f32, lumbar_flex: f32) {
	pose.spine_axial = back_ridge_swing;
	pose.spine_lateral = lumbar_flex;
}

/// Turn the neck about +Y. Tilt and nod stay at rest.
pub fn apply_neck(pose: &mut QuadrupedPose, neck_swing: f32) {
	apply_neck_axes(pose, neck_swing, 0.0, 0.0);
}

/// Pose the neck on all three anatomical channels.
///
/// - `swing` — axial turn (`neck_turn`, +Y)
/// - `flex` — lateral tilt (`neck_tilt`, +Z)
/// - `twist` — sagittal nod (`neck_nod`, +X)
pub fn apply_neck_axes(pose: &mut QuadrupedPose, neck_swing: f32, neck_flex: f32, neck_twist: f32) {
	pose.neck_turn = neck_swing;
	pose.neck_tilt = neck_flex;
	pose.neck_nod = neck_twist;
}

/// Turn and tilt only. Prefer [`apply_neck_axes`] when the head should nod.
pub fn apply_neck_posed(pose: &mut QuadrupedPose, neck_swing: f32, neck_flex: f32) {
	apply_neck_axes(pose, neck_swing, neck_flex, 0.0);
}
