use bevy::prelude::*;

use crate::{BonePose, Name, RigPose, RiggedAxis, Side};

/// Rest-pose thigh and shin segment lengths used for analytic IK-style drop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSegmentLengths {
	pub femur: f32,
	pub shin: f32,
}

impl Default for LegSegmentLengths {
	fn default() -> Self {
		Self { femur: 0.5, shin: 0.5 }
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidLeg {
	pub pelvis: BonePose,
	pub femur: BonePose,
	pub shin: BonePose,
}

impl HumanoidLeg {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.pelvis);
		hydrate_bone(pose, &mut self.femur);
		hydrate_bone(pose, &mut self.shin);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.pelvis.clone());
		pose.insert(self.femur.clone());
		pose.insert(self.shin.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidArm {
	pub shoulder: BonePose,
	pub humerus: BonePose,
	pub forearm: BonePose,
}

impl HumanoidArm {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.shoulder);
		hydrate_bone(pose, &mut self.humerus);
		hydrate_bone(pose, &mut self.forearm);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.shoulder.clone());
		pose.insert(self.humerus.clone());
		pose.insert(self.forearm.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidSpine {
	pub root: BonePose,
	pub lumbar: BonePose,
	pub midback: BonePose,
	pub upper_back: BonePose,
}

impl HumanoidSpine {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.root);
		hydrate_bone(pose, &mut self.lumbar);
		hydrate_bone(pose, &mut self.midback);
		hydrate_bone(pose, &mut self.upper_back);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.root.clone());
		pose.insert(self.lumbar.clone());
		pose.insert(self.midback.clone());
		pose.insert(self.upper_back.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidNeck {
	pub lower_neck: BonePose,
	pub upper_neck: BonePose,
}

impl HumanoidNeck {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.lower_neck);
		hydrate_bone(pose, &mut self.upper_neck);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.lower_neck.clone());
		pose.insert(self.upper_neck.clone());
	}
}

fn hydrate_bone(pose: &RigPose, bone: &mut BonePose) {
	if let Some(stored) = pose.get(&bone.name) {
		bone.transform = stored.transform;
		bone.swing = stored.swing;
		bone.flex = stored.flex;
		bone.twist = stored.twist;
	}
}

pub trait HumanoidRig {
	fn leg(&self, side: Side) -> HumanoidLeg;
	fn arm(&self, side: Side) -> HumanoidArm;
	fn spine(&self) -> HumanoidSpine;
	fn neck(&self) -> HumanoidNeck;
	fn pose(&self) -> &RigPose;
	fn pose_mut(&mut self) -> &mut RigPose;

	/// Bones driven by procedural animation in the playground.
	fn animation_bones(&self) -> Vec<Name>;

	fn segment_lengths(&self) -> LegSegmentLengths {
		LegSegmentLengths::default()
	}

	/// +1 or −1 so mirrored forearms flex forward consistently in run animation.
	fn forearm_flex_sign(&self, _side: Side) -> f32 {
		1.0
	}

	/// Static per-bone swing/flex axes for the rig.
	fn rigged_axis(&self, _bone: &Name) -> Option<RiggedAxis> {
		None
	}

	/// Apply swing/flex about the bone's rig-defined local axes.
	fn articulate_on_rig(&self, bone: BonePose, swing: f32, flex: f32) -> BonePose {
		self.articulate_on_rig_twisted(bone, swing, flex, 0.0)
	}

	/// Apply swing/flex/twist about the bone's rig-defined local axes.
	fn articulate_on_rig_twisted(
		&self,
		mut bone: BonePose,
		swing: f32,
		flex: f32,
		twist: f32,
	) -> BonePose {
		if let Some(axis) = self.rigged_axis(&bone.name) {
			bone = bone.articulate(axis, swing, flex, twist);
		} else {
			bone.swing = swing;
			bone.flex = flex;
			bone.twist = twist;
		}
		bone
	}

	/// Aim the humerus length axis along a **world-space** direction, then apply long-axis roll.
	///
	/// Prefer this over stacking swing/flex/twist when an animation needs a stable aim
	/// (punches, reaches). Aim is a shortest-arc from the bind length direction; `roll`
	/// is about local length (Y) and cannot re-aim the arm. Callers should pass directions
	/// in a documented body/world convention (jab uses body +Z = fight-forward).
	fn humerus_along_with_roll(&self, side: Side, along_world: Vec3, roll: f32) -> BonePose {
		let arm = self.arm_pose(side);
		let mut humerus = arm.humerus;
		let parent_world = self.parent_world_rotation(&humerus.name);
		let along_parent = parent_world.inverse() * along_world;
		let rest = humerus.transform.rotation;
		humerus.transform.rotation = crate::articulation::rotation_along_with_roll(
			rest,
			along_parent,
			roll,
			crate::articulation::BONE_LENGTH_AXIS,
		);
		// Semantic channels: only roll is meaningful after an aim solve.
		humerus.swing = 0.0;
		humerus.flex = 0.0;
		humerus.twist = roll;
		humerus
	}

	fn leg_pose(&self, side: Side) -> HumanoidLeg {
		let mut leg = self.leg(side);
		leg.hydrate_from(self.pose());
		leg
	}

	fn arm_pose(&self, side: Side) -> HumanoidArm {
		let mut arm = self.arm(side);
		arm.hydrate_from(self.pose());
		arm
	}

	fn spine_pose(&self) -> HumanoidSpine {
		let mut spine = self.spine();
		spine.hydrate_from(self.pose());
		spine
	}

	fn neck_pose(&self) -> HumanoidNeck {
		let mut neck = self.neck();
		neck.hydrate_from(self.pose());
		neck
	}

	fn pose_leg(&mut self, leg: HumanoidLeg) {
		leg.apply_to(self.pose_mut());
	}

	fn pose_arm(&mut self, arm: HumanoidArm) {
		arm.apply_to(self.pose_mut());
	}

	fn pose_spine(&mut self, spine: HumanoidSpine) {
		spine.apply_to(self.pose_mut());
	}

	fn pose_neck(&mut self, neck: HumanoidNeck) {
		neck.apply_to(self.pose_mut());
	}

	/// World rotation of the bone's parent at the current pose (identity when unknown).
	fn parent_world_rotation(&self, _bone: &Name) -> Quat {
		Quat::IDENTITY
	}

	/// Shift every animation bone by a world-space displacement without shortening bind segments.
	fn move_all(&mut self, world_displacement: Vec3) {
		let bones = self.animation_bones();
		for bone in bones {
			let parent_world = self.parent_world_rotation(&bone);
			let axis = self.rigged_axis(&bone).unwrap_or(RiggedAxis::DEFAULT);
			let Some(pose) = self.pose_mut().get_mut(&bone) else {
				continue;
			};
			let segment = pose.transform.translation;
			let delta = crate::articulation::axis_aware_translation_delta(
				segment,
				axis,
				world_displacement,
				parent_world,
			);
			pose.transform.translation += delta;
		}
	}
}
