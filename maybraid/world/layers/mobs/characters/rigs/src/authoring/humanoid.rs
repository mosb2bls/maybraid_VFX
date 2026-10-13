//! Humanoid semantic pose and V0 parent-space compose.
//!
//! Semantic fields map onto imported swing / flex / twist. They are not a
//! character-space anatomical triad. Spine stack weights sum to 1 so an authored
//! bend is the total fold, split across root, lumbar, mid-back, and upper back.

use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use super::binding::{BoneId, RigBinding, RigDefinition, SkeletonFamily};
use super::buffer::PoseBuffer;
use crate::articulation::{compose_parent_rotation, rotation_along_with_roll, BONE_LENGTH_AXIS};
use crate::{RiggedAxis, Side};

/// Left femur: imported stride on parent Y, medial/lateral on X.
pub const HUMANOID_FEMUR_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::X, twist_axis: Vec3::Z };

pub const HUMANOID_SHIN_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::Z, twist_axis: Vec3::X };

pub const HUMANOID_RIGHT_FEMUR_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::NEG_Y, flex_axis: Vec3::NEG_X, twist_axis: Vec3::Z };

pub const HUMANOID_RIGHT_SHIN_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::NEG_Z, twist_axis: Vec3::X };

pub const HUMANOID_RIGHT_FLEX_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::NEG_Z, twist_axis: Vec3::X };

/// Inspected `humanoid_rig.glb` node rotations (xyzw). Production biped bodies share them.
pub const HUMANOID_GLB_SHOULDER_L: Quat = Quat::from_xyzw(0.0, 0.0, -0.70710677, 0.70710677);
pub const HUMANOID_GLB_SHOULDER_R: Quat = Quat::from_xyzw(0.0, 0.0, 0.70710677, 0.70710677);
pub const HUMANOID_GLB_PELVIS_L: Quat = Quat::from_xyzw(-0.5, -0.5, -0.5, 0.5);
pub const HUMANOID_GLB_PELVIS_R: Quat = Quat::from_xyzw(-0.5, 0.5, 0.5, 0.5);
pub const HUMANOID_GLB_FEMUR: Quat = Quat::from_xyzw(-0.70710677, 0.0, 0.0, 0.70710677);

/// Imported V0 hinge axes. Clips still evaluate through these, not character X/Z/Y.
pub fn bone_axis(name: &str) -> RiggedAxis {
	match name {
		"femur.L" => HUMANOID_FEMUR_AXIS,
		"femur.R" => HUMANOID_RIGHT_FEMUR_AXIS,
		"shin.L" => HUMANOID_SHIN_AXIS,
		"shin.R" => HUMANOID_RIGHT_SHIN_AXIS,
		"forearm.R" => HUMANOID_RIGHT_FLEX_AXIS,
		_ => RiggedAxis::DEFAULT,
	}
}

pub const HUMANOID_V0_BONES: &[&str] = &[
	"root",
	"lumbar",
	"midback",
	"upper_back",
	"lower_neck",
	"upper_neck",
	"shoulder.L",
	"humerus.L",
	"forearm.L",
	"shoulder.R",
	"humerus.R",
	"forearm.R",
	"pelvis.L",
	"femur.L",
	"shin.L",
	"pelvis.R",
	"femur.R",
	"shin.R",
];

pub const HUMANOID_V0_COUNT: usize = HUMANOID_V0_BONES.len();

const PARENTS: &[(&str, &str)] = &[
	("lumbar", "root"),
	("midback", "lumbar"),
	("upper_back", "midback"),
	("lower_neck", "upper_back"),
	("upper_neck", "lower_neck"),
	("shoulder.L", "upper_back"),
	("shoulder.R", "upper_back"),
	("humerus.L", "shoulder.L"),
	("humerus.R", "shoulder.R"),
	("forearm.L", "humerus.L"),
	("forearm.R", "humerus.R"),
	("femur.L", "pelvis.L"),
	("femur.R", "pelvis.R"),
	("shin.L", "femur.L"),
	("shin.R", "femur.R"),
];

const STACK_FORWARD: [f32; 4] = [0.28, 0.26, 0.24, 0.22];
const WAIST_FORWARD: [f32; 4] = [0.40, 0.35, 0.15, 0.10];
const TURN_WEIGHTS: [f32; 4] = [0.0, 0.35, 0.40, 0.25];

pub fn humanoid_v0_definition() -> Arc<RigDefinition> {
	static DEFINITION: OnceLock<Arc<RigDefinition>> = OnceLock::new();
	DEFINITION
		.get_or_init(|| {
			Arc::new(RigDefinition::from_names(
				SkeletonFamily::Humanoid,
				HUMANOID_V0_BONES,
				PARENTS,
			))
		})
		.clone()
}

/// Sagittal, lateral, and axial bend already split across the spine stack.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpinePose {
	/// Index 0 root, 1 lumbar, 2 mid-back, 3 upper back. Radians.
	pub forward_bend: [f32; 4],
	pub side_bend: [f32; 4],
	pub turn: [f32; 4],
}

impl SpinePose {
	/// Full sagittal lean on the root. Walk uses this so the authored angle is not diluted.
	pub fn add_root_forward(&mut self, radians: f32) {
		self.forward_bend[0] += radians;
	}

	/// Held and looping squat share this spread. Weights sum to 1.
	pub fn add_stack_forward(&mut self, radians: f32) {
		for (slot, weight) in self.forward_bend.iter_mut().zip(STACK_FORWARD) {
			*slot += radians * weight;
		}
	}

	/// Jab waist fold, heavier at the root and lumbar.
	pub fn add_waist_forward(&mut self, radians: f32) {
		for (slot, weight) in self.forward_bend.iter_mut().zip(WAIST_FORWARD) {
			*slot += radians * weight;
		}
	}

	pub fn add_turn(&mut self, radians: f32) {
		for (slot, weight) in self.turn.iter_mut().zip(TURN_WEIGHTS) {
			*slot += radians * weight;
		}
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NeckBone {
	pub nod: f32,
	pub side_tilt: f32,
	pub turn: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NeckPose {
	pub lower: NeckBone,
	pub upper: NeckBone,
}

/// One leg. Pelvis fields are the pelvis bone; hip fields are the femur.
///
/// The same positive [`Self::knee_flexion`] closes both knees. Phase offsets are not stored here.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LegPose {
	pub hip_flexion: f32,
	pub hip_abduction: f32,
	pub hip_rotation: f32,
	pub knee_flexion: f32,
	pub pelvis_flexion: f32,
	pub pelvis_lateral: f32,
	pub pelvis_turn: f32,
}

/// Character-space humerus aim. `along` is a direction, not a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmAim {
	pub along: Vec3,
	pub roll: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmPose {
	pub forward_elevation: f32,
	pub lateral_elevation: f32,
	pub axial_rotation: f32,
	pub elbow_flexion: f32,
	pub shoulder_forward: f32,
	pub shoulder_lift: f32,
	pub shoulder_twist: f32,
	pub aim: Option<ArmAim>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HumanoidPose {
	pub spine: SpinePose,
	pub neck: NeckPose,
	pub legs: [LegPose; 2],
	pub arms: [ArmPose; 2],
}

impl HumanoidPose {
	pub fn leg_mut(&mut self, side: Side) -> &mut LegPose {
		&mut self.legs[side.index()]
	}

	pub fn arm_mut(&mut self, side: Side) -> &mut ArmPose {
		&mut self.arms[side.index()]
	}

	/// Symmetric leg flexion. `femur_swing` is hip flexion; `shin_flex` is knee flexion.
	pub fn apply_leg(&mut self, side: Side, femur_swing: f32, shin_flex: f32) {
		let leg = self.leg_mut(side);
		leg.hip_flexion += femur_swing;
		leg.knee_flexion += shin_flex;
	}

	/// Sagittal lean on the root only. Walk and other “lean” knobs use this so the
	/// authored angle is a forward bend, not a yaw.
	pub fn apply_root(&mut self, root_swing: f32) {
		self.spine.add_root_forward(root_swing);
	}

	/// Sagittal fold spread across the spine stack. Held and looping squat share it.
	pub fn apply_spine_pitch(&mut self, pitch: f32) {
		self.spine.add_stack_forward(pitch);
	}

	/// Hip crease as pelvis flexion.
	pub fn apply_hip_fold(&mut self, side: Side, fold: f32) {
		self.leg_mut(side).pelvis_flexion += fold;
	}

	pub fn apply_neck(
		&mut self,
		lower_swing: f32,
		lower_flex: f32,
		upper_swing: f32,
		upper_flex: f32,
	) {
		self.apply_neck_twisted(lower_swing, lower_flex, 0.0, upper_swing, upper_flex, 0.0);
	}

	/// Neck channels: swing is turn, flex is side tilt, twist is nod.
	pub fn apply_neck_twisted(
		&mut self,
		lower_swing: f32,
		lower_flex: f32,
		lower_twist: f32,
		upper_swing: f32,
		upper_flex: f32,
		upper_twist: f32,
	) {
		self.neck.lower.turn += lower_swing;
		self.neck.lower.side_tilt += lower_flex;
		self.neck.lower.nod += lower_twist;
		self.neck.upper.turn += upper_swing;
		self.neck.upper.side_tilt += upper_flex;
		self.neck.upper.nod += upper_twist;
	}

	pub fn apply_arm(
		&mut self,
		side: Side,
		shoulder_swing: f32,
		shoulder_flex: f32,
		humerus_swing: f32,
		humerus_flex: f32,
		forearm_flex: f32,
	) {
		self.apply_arm_twisted(
			side,
			shoulder_swing,
			shoulder_flex,
			0.0,
			humerus_swing,
			humerus_flex,
			forearm_flex,
		);
	}

	/// Shoulder and humerus `swing` / `flex` / `twist` keep those imported axes.
	/// Elbow `flex` is the same number on both arms; the right forearm axis is −Z.
	pub fn apply_arm_twisted(
		&mut self,
		side: Side,
		shoulder_swing: f32,
		shoulder_flex: f32,
		shoulder_twist: f32,
		humerus_swing: f32,
		humerus_flex: f32,
		forearm_flex: f32,
	) {
		let arm = self.arm_mut(side);
		arm.shoulder_forward += shoulder_swing;
		arm.shoulder_lift += shoulder_flex;
		arm.shoulder_twist += shoulder_twist;
		arm.forward_elevation += humerus_swing;
		arm.lateral_elevation += humerus_flex;
		arm.elbow_flexion += forearm_flex;
	}
}

pub fn resolve_humanoid(pose: &HumanoidPose, binding: &RigBinding, out: &mut PoseBuffer) {
	apply(pose, &binding.definition, &binding.effective_rest, out);
}

/// Apply only `mask` bones from `pose` onto rest. Unmasked bones stay at the baseline.
///
/// The baseline is [`RigBinding::effective_rest`], not an arbitrary incoming whole pose.
pub fn resolve_humanoid_masked(
	pose: &HumanoidPose,
	mask: u32,
	binding: &RigBinding,
	out: &mut PoseBuffer,
) {
	apply_masked(pose, mask, &binding.definition, &binding.effective_rest, out);
}

/// Bit in a humanoid bone mask for `name`, if it is a V0 animation bone.
pub fn humanoid_bone_bit(name: &str) -> Option<u32> {
	HUMANOID_V0_BONES
		.iter()
		.position(|candidate| *candidate == name)
		.map(|index| 1u32 << index)
}

/// OR of [`humanoid_bone_bit`] for each name.
pub fn humanoid_write_mask(names: &[&str]) -> u32 {
	names
		.iter()
		.filter_map(|name| humanoid_bone_bit(name))
		.fold(0, |mask, bit| mask | bit)
}

/// Bones with any non-zero authored channel in `pose`.
///
/// Used to verify clip write masks stay aligned with sampled output.
pub fn humanoid_pose_write_mask(pose: &HumanoidPose) -> u32 {
	let mut mask = 0u32;
	for (index, name) in HUMANOID_V0_BONES.iter().enumerate() {
		if bone_has_authored_channels(name, pose) {
			mask |= 1u32 << index;
		}
	}
	mask
}

fn bone_has_authored_channels(name: &str, pose: &HumanoidPose) -> bool {
	match name {
		"humerus.L" if pose.arms[0].aim.is_some() => true,
		"humerus.R" if pose.arms[1].aim.is_some() => true,
		_ => channels_for(name, pose)
			.is_some_and(|(swing, flex, twist)| swing != 0.0 || flex != 0.0 || twist != 0.0),
	}
}

/// Parent-space authored rotation `Q` such that posed = `Q * rest`.
///
/// Built with an identity rest so a clip cache can share deltas across compatible
/// bindings.
pub fn humanoid_parent_rotation_delta(name: &str, pose: &HumanoidPose) -> Option<Quat> {
	let (swing, flex, twist) = channels_for(name, pose)?;
	Some(compose_parent_rotation(Quat::IDENTITY, bone_axis(name), swing, flex, twist))
}

pub(crate) fn apply(
	pose: &HumanoidPose,
	definition: &RigDefinition,
	rest: &PoseBuffer,
	out: &mut PoseBuffer,
) {
	out.copy_from(rest);
	for (index, name) in definition.names.iter().enumerate() {
		let bone = BoneId(index as u16);
		let Some((swing, flex, twist)) = channels_for(name, pose) else {
			continue;
		};
		out.set_rotation(
			bone,
			compose_parent_rotation(rest.rotation(bone), bone_axis(name), swing, flex, twist),
		);
	}
	for side in [Side::Left, Side::Right] {
		let Some(aim) = pose.arms[side.index()].aim else {
			continue;
		};
		aim_humerus(definition, rest, out, side, aim);
	}
}

fn apply_masked(
	pose: &HumanoidPose,
	mask: u32,
	definition: &RigDefinition,
	rest: &PoseBuffer,
	out: &mut PoseBuffer,
) {
	let n = rest.local.len().min(out.local.len());
	for index in 0..n {
		if mask & (1u32 << index) == 0 {
			out.local[index] = rest.local[index];
		}
	}
	for (index, name) in definition.names.iter().enumerate() {
		if mask & (1u32 << index) == 0 {
			continue;
		}
		let bone = BoneId(index as u16);
		let Some((swing, flex, twist)) = channels_for(name, pose) else {
			continue;
		};
		out.set_rotation(
			bone,
			compose_parent_rotation(rest.rotation(bone), bone_axis(name), swing, flex, twist),
		);
	}
	for side in [Side::Left, Side::Right] {
		let Some(aim) = pose.arms[side.index()].aim else {
			continue;
		};
		let humerus = match side {
			Side::Left => "humerus.L",
			Side::Right => "humerus.R",
		};
		let Some(bit) = humanoid_bone_bit(humerus) else {
			continue;
		};
		if mask & bit == 0 {
			continue;
		}
		aim_humerus(definition, rest, out, side, aim);
	}
}

/// Map semantic fields back onto the pre-rewrite swing / flex / twist knobs.
///
/// Spine forward bend is the old DEFAULT **twist** (parent X). That is the
/// squat pitch path; walk lean uses the same channel so it does not yaw.
fn channels_for(name: &str, pose: &HumanoidPose) -> Option<(f32, f32, f32)> {
	let spine = |slot: usize| {
		Some((pose.spine.turn[slot], pose.spine.side_bend[slot], pose.spine.forward_bend[slot]))
	};
	let neck = |bone: NeckBone| Some((bone.turn, bone.side_tilt, bone.nod));
	match name {
		"root" => spine(0),
		"lumbar" => spine(1),
		"midback" => spine(2),
		"upper_back" => spine(3),
		"lower_neck" => neck(pose.neck.lower),
		"upper_neck" => neck(pose.neck.upper),
		"pelvis.L" => {
			let leg = pose.legs[0];
			Some((leg.pelvis_turn, leg.pelvis_lateral, leg.pelvis_flexion))
		}
		"pelvis.R" => {
			let leg = pose.legs[1];
			Some((leg.pelvis_turn, leg.pelvis_lateral, leg.pelvis_flexion))
		}
		"femur.L" => {
			let leg = pose.legs[0];
			Some((leg.hip_flexion, leg.hip_abduction, leg.hip_rotation))
		}
		"femur.R" => {
			let leg = pose.legs[1];
			Some((leg.hip_flexion, leg.hip_abduction, leg.hip_rotation))
		}
		"shin.L" => Some((0.0, pose.legs[0].knee_flexion, 0.0)),
		"shin.R" => Some((0.0, pose.legs[1].knee_flexion, 0.0)),
		"shoulder.L" => {
			let arm = pose.arms[0];
			Some((arm.shoulder_forward, arm.shoulder_lift, arm.shoulder_twist))
		}
		"shoulder.R" => {
			let arm = pose.arms[1];
			Some((arm.shoulder_forward, arm.shoulder_lift, arm.shoulder_twist))
		}
		"humerus.L" => {
			let arm = pose.arms[0];
			(!arm.aim.is_some()).then_some((
				arm.forward_elevation,
				arm.lateral_elevation,
				arm.axial_rotation,
			))
		}
		"humerus.R" => {
			let arm = pose.arms[1];
			(!arm.aim.is_some()).then_some((
				arm.forward_elevation,
				arm.lateral_elevation,
				arm.axial_rotation,
			))
		}
		"forearm.L" => Some((0.0, pose.arms[0].elbow_flexion, 0.0)),
		"forearm.R" => Some((0.0, pose.arms[1].elbow_flexion, 0.0)),
		_ => None,
	}
}

fn aim_humerus(
	definition: &RigDefinition,
	rest: &PoseBuffer,
	out: &mut PoseBuffer,
	side: Side,
	aim: ArmAim,
) {
	let name = match side {
		Side::Left => "humerus.L",
		Side::Right => "humerus.R",
	};
	let Some(bone) = definition.id(name) else {
		return;
	};
	let parent = definition.parent_rotation(out, bone);
	let along_parent = parent.inverse() * aim.along;
	let posed =
		rotation_along_with_roll(rest.rotation(bone), along_parent, aim.roll, BONE_LENGTH_AXIS);
	out.set_rotation(bone, posed);
}

/// Inspected `humanoid_rig.glb` local transforms for the V0 animation bones.
pub fn apply_humanoid_glb_rest(definition: &RigDefinition, rest: &mut PoseBuffer) {
	let set = |rest: &mut PoseBuffer, name: &str, translation: Vec3, rotation: Quat| {
		if let Some(id) = definition.id(name) {
			if let Some(slot) = rest.local.get_mut(id.index()) {
				slot.translation = translation;
				slot.rotation = rotation;
			}
		}
	};
	set(rest, "root", Vec3::ZERO, Quat::IDENTITY);
	set(rest, "lumbar", Vec3::new(0.0, 0.25, 0.0), Quat::IDENTITY);
	set(rest, "midback", Vec3::new(0.0, 0.25, 0.0), Quat::IDENTITY);
	set(rest, "upper_back", Vec3::new(0.0, 0.15, 0.0), Quat::IDENTITY);
	set(rest, "lower_neck", Vec3::new(0.0, 0.10, 0.0), Quat::IDENTITY);
	set(rest, "upper_neck", Vec3::new(0.0, 0.15, 0.0), Quat::IDENTITY);
	set(rest, "shoulder.L", Vec3::new(0.0, 0.10, 0.0), HUMANOID_GLB_SHOULDER_L);
	set(rest, "humerus.L", Vec3::new(0.0, 0.35, 0.0), Quat::IDENTITY);
	set(rest, "forearm.L", Vec3::new(0.0, 0.48, 0.0), Quat::IDENTITY);
	set(rest, "shoulder.R", Vec3::new(0.0, 0.10, 0.0), HUMANOID_GLB_SHOULDER_R);
	set(rest, "humerus.R", Vec3::new(0.0, 0.35, 0.0), Quat::IDENTITY);
	set(rest, "forearm.R", Vec3::new(0.0, 0.48, 0.0), Quat::IDENTITY);
	set(rest, "pelvis.L", Vec3::ZERO, HUMANOID_GLB_PELVIS_L);
	set(rest, "femur.L", Vec3::new(0.0, 0.25, 0.0), HUMANOID_GLB_FEMUR);
	set(rest, "shin.L", Vec3::new(0.0, 0.50, 0.0), Quat::IDENTITY);
	set(rest, "pelvis.R", Vec3::ZERO, HUMANOID_GLB_PELVIS_R);
	set(rest, "femur.R", Vec3::new(0.0, 0.25, 0.0), HUMANOID_GLB_FEMUR);
	set(rest, "shin.R", Vec3::new(0.0, 0.50, 0.0), Quat::IDENTITY);
}

/// Identity-rest binding used by clip tests. Production bindings come from imported bones.
pub fn identity_binding() -> RigBinding {
	let definition = humanoid_v0_definition();
	let len = definition.len();
	RigBinding::from_rest(
		definition,
		vec![Entity::PLACEHOLDER; len].into_boxed_slice(),
		PoseBuffer::identity(len),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn bone(binding: &RigBinding, name: &str) -> BoneId {
		binding.definition.id(name).expect(name)
	}

	#[test]
	fn walk_style_root_lean_is_sagittal() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.spine.add_root_forward(0.4);
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let tipped = out.rotation(bone(&binding, "root")) * Vec3::Y;
		assert!(tipped.z > 0.2, "forward lean goes to +Z, got {tipped:?}");
		assert!(tipped.x.abs() < 1e-3, "lean must not yaw, got {tipped:?}");
	}

	#[test]
	fn stack_and_root_helpers_share_the_sagittal_plane() {
		let binding = identity_binding();
		let mut spread = HumanoidPose::default();
		spread.spine.add_stack_forward(0.8);
		let mut root_only = HumanoidPose::default();
		root_only.spine.add_root_forward(0.8);
		let mut spread_out = PoseBuffer::identity(binding.definition.len());
		let mut root_out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&spread, &binding, &mut spread_out);
		resolve_humanoid(&root_only, &binding, &mut root_out);
		for name in ["root", "lumbar"] {
			let spread_tip = spread_out.rotation(bone(&binding, name)) * Vec3::Y;
			let root_tip = root_out.rotation(bone(&binding, name)) * Vec3::Y;
			assert!(spread_tip.x.abs() < 1e-3, "{name} spread yaw {spread_tip:?}");
			assert!(root_tip.x.abs() < 1e-3, "{name} root yaw {root_tip:?}");
		}
		assert!(
			(spread_out.rotation(bone(&binding, "root")) * Vec3::Y).z > 0.05,
			"spread still pitches the root"
		);
	}

	#[test]
	fn hip_and_knee_use_imported_swing_and_flex() {
		use crate::articulation::compose_parent_rotation;
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).hip_flexion = 0.5;
		pose.leg_mut(Side::Left).knee_flexion = 0.7;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let femur = bone(&binding, "femur.L");
		let shin = bone(&binding, "shin.L");
		assert!(
			out.rotation(femur)
				.dot(compose_parent_rotation(Quat::IDENTITY, HUMANOID_FEMUR_AXIS, 0.5, 0.0, 0.0))
				.abs() > 1.0 - 1e-5
		);
		assert!(
			out.rotation(shin)
				.dot(compose_parent_rotation(Quat::IDENTITY, HUMANOID_SHIN_AXIS, 0.0, 0.7, 0.0))
				.abs() > 1.0 - 1e-5
		);
	}

	#[test]
	fn left_and_right_knees_share_positive_flex_on_mirrored_axes() {
		use crate::articulation::compose_parent_rotation;
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).knee_flexion = 0.6;
		pose.leg_mut(Side::Right).knee_flexion = 0.6;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		assert!(
			out.rotation(bone(&binding, "shin.L"))
				.dot(compose_parent_rotation(Quat::IDENTITY, HUMANOID_SHIN_AXIS, 0.0, 0.6, 0.0))
				.abs() > 1.0 - 1e-5
		);
		assert!(
			out.rotation(bone(&binding, "shin.R"))
				.dot(compose_parent_rotation(
					Quat::IDENTITY,
					HUMANOID_RIGHT_SHIN_AXIS,
					0.0,
					0.6,
					0.0
				))
				.abs() > 1.0 - 1e-5
		);
	}

	#[test]
	fn imported_pelvis_bind_keeps_hip_flexion_sagittal() {
		let definition = humanoid_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		apply_humanoid_glb_rest(&definition, &mut rest);
		let femur = definition.id("femur.L").expect("femur");
		let shin = definition.id("shin.L").expect("shin");
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; rest.len()].into_boxed_slice(),
			rest,
		);
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).hip_flexion = 0.5;
		pose.leg_mut(Side::Left).knee_flexion = 0.6;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let thigh = binding.definition.rotation_in_character(&out, femur) * Vec3::Y;
		let shin_dir = binding.definition.rotation_in_character(&out, shin) * Vec3::Y;
		assert!(thigh.z.abs() > 0.3, "stride must move back/front, got {thigh:?}");
		assert!(thigh.x.abs() < 0.08, "stride must not swing sideways, got {thigh:?}");
		assert!(shin_dir.z.abs() > shin_dir.x.abs(), "knee stays sagittal, got {shin_dir:?}");
	}

	#[test]
	fn t_pose_elbow_flex_is_not_a_length_roll() {
		let definition = humanoid_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		apply_humanoid_glb_rest(&definition, &mut rest);
		let forearm = definition.id("forearm.L").expect("forearm");
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; rest.len()].into_boxed_slice(),
			rest,
		);
		let mut pose = HumanoidPose::default();
		pose.arm_mut(Side::Left).elbow_flexion = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let along = binding.definition.rotation_in_character(&out, forearm) * Vec3::Y;
		assert!(along.y.abs() > 0.2, "old flex lifts the T-pose forearm, got {along:?}");
		assert!(along.x.abs() > 0.8, "length stays near ±X, got {along:?}");
		assert!(along.z.abs() < 0.05, "elbow is not a forward twist, got {along:?}");
		let origin = binding.definition.translation_in_character(&out, forearm);
		let hand = origin + along * 0.48;
		let rest_origin =
			binding.definition.translation_in_character(&binding.effective_rest, forearm);
		let rest_hand = rest_origin + Vec3::X * 0.48;
		assert!(
			hand.y > rest_hand.y + 0.05,
			"previous flex lifts the hand, {hand:?} vs {rest_hand:?}"
		);
		assert!((hand.z - rest_hand.z).abs() < 0.05, "hand stays off the fight axis, {hand:?}");
	}

	#[test]
	fn t_pose_right_elbow_flex_mirrors_the_left_hinge() {
		let definition = humanoid_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		apply_humanoid_glb_rest(&definition, &mut rest);
		let forearm = definition.id("forearm.R").expect("forearm");
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; rest.len()].into_boxed_slice(),
			rest,
		);
		let mut pose = HumanoidPose::default();
		pose.arm_mut(Side::Right).elbow_flexion = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let along = binding.definition.rotation_in_character(&out, forearm) * Vec3::Y;
		assert!(along.y.abs() > 0.2, "right flex lifts the T-pose forearm, got {along:?}");
		assert!(along.x < -0.8, "right length stays near −X, got {along:?}");
		assert!(along.z.abs() < 0.05, "right elbow is not a forward twist, got {along:?}");
	}

	#[test]
	fn t_pose_shoulder_swing_matches_previous_flap_stroke() {
		use crate::articulation::compose_parent_rotation;
		let definition = humanoid_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		apply_humanoid_glb_rest(&definition, &mut rest);
		let shoulder = definition.id("shoulder.L").expect("shoulder");
		let bind = rest.rotation(shoulder);
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; rest.len()].into_boxed_slice(),
			rest,
		);
		let mut pose = HumanoidPose::default();
		pose.arm_mut(Side::Left).shoulder_forward = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let expected = compose_parent_rotation(bind, RiggedAxis::DEFAULT, 0.4, 0.0, 0.0);
		assert!(out.rotation(shoulder).dot(expected).abs() > 1.0 - 1e-5);
		let along = binding.definition.rotation_in_character(&out, shoulder) * Vec3::Y;
		assert!(along.z.abs() > 0.2, "previous flap stroke was parent Y, got {along:?}");
		assert!(along.y.abs() < 0.05, "stroke is not a length roll, got {along:?}");
	}

	#[test]
	fn neck_nod_turn_and_tilt_are_independent() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.neck.lower.nod = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let nodded = out.rotation(bone(&binding, "lower_neck")) * Vec3::Y;
		assert!(nodded.z > 0.1 && nodded.x.abs() < 1e-3, "{nodded:?}");

		let mut pose = HumanoidPose::default();
		pose.neck.lower.turn = 0.4;
		resolve_humanoid(&pose, &binding, &mut out);
		let turned = out.rotation(bone(&binding, "lower_neck")) * Vec3::Z;
		assert!(turned.x.abs() > 0.1 && turned.y.abs() < 1e-3, "{turned:?}");

		let mut pose = HumanoidPose::default();
		pose.neck.lower.side_tilt = 0.4;
		resolve_humanoid(&pose, &binding, &mut out);
		let tilted = out.rotation(bone(&binding, "lower_neck")) * Vec3::Y;
		assert!(tilted.x.abs() > 0.1 && tilted.z.abs() < 1e-3, "{tilted:?}");
	}

	#[test]
	fn repeated_resolve_matches_a_fresh_buffer() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.spine.add_root_forward(0.2);
		pose.leg_mut(Side::Right).hip_flexion = -0.4;
		let mut first = PoseBuffer::identity(binding.definition.len());
		let mut second = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut first);
		resolve_humanoid(&pose, &binding, &mut first);
		resolve_humanoid(&pose, &binding, &mut second);
		assert_eq!(first, second);
	}

	#[test]
	fn aim_sees_the_shoulder_written_in_this_sample() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.arm_mut(Side::Right).shoulder_forward = 0.0;
		pose.arm_mut(Side::Right).aim =
			Some(ArmAim { along: Vec3::new(0.0, -0.2, 1.0).normalize(), roll: 0.0 });
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let humerus = bone(&binding, "humerus.R");
		let parent = binding.definition.parent_rotation(&out, humerus);
		let aimed = (parent * out.rotation(humerus) * BONE_LENGTH_AXIS).normalize();
		assert!(aimed.dot(Vec3::new(0.0, -0.2, 1.0).normalize()) > 0.99, "aim {aimed:?}");
	}
}
