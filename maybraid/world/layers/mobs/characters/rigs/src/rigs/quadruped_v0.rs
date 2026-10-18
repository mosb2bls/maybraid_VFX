use bevy::prelude::*;

use crate::{
	authoring::{
		quadruped_v0_definition, resolve_quadruped, PoseBuffer, PoseScratch, QuadrupedPose,
		RigBinding, QUADRUPED_RIGHT_SHIN_AXIS, QUADRUPED_RIGHT_THIGH_AXIS, QUADRUPED_SHIN_AXIS,
		QUADRUPED_THIGH_AXIS, QUADRUPED_V0_BONES,
	},
	quadruped::LegSegmentLengths,
	BoneDefinition, BoneTable, Name, RiggedAxis,
};

/// Store the bones of the imported quadruped rig in a semantically reasonable hierarchy.
#[derive(Component, Debug, Clone)]
pub struct QuadrupedV0Rig {
	pub bones: BoneTable,
	pub binding: RigBinding,
	/// Last resolved local pose. Sampling always starts from [`Self::binding`] rest.
	pub pose: PoseBuffer,
	pub scratch: PoseScratch,
	pub segment_lengths: LegSegmentLengths,
}

impl QuadrupedV0Rig {
	pub fn for_clip_test() -> Self {
		let mut rig = Self::imported();
		let mut rest = rig.binding.effective_rest.clone();
		crate::authoring::apply_quadruped_glb_rest(&rig.binding.definition, &mut rest);
		rig.binding.refresh_rest(rest);
		rig.pose.copy_from(&rig.binding.effective_rest);
		rig.segment_lengths = rig.binding.metrics.quadruped_leg;
		rig
	}

	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in QUADRUPED_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}
		let definition = quadruped_v0_definition();
		let len = definition.len();
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; len].into_boxed_slice(),
			PoseBuffer::identity(len),
		);
		Self {
			bones,
			pose: PoseBuffer::identity(len),
			scratch: PoseScratch::identity(len),
			binding,
			segment_lengths: LegSegmentLengths::default(),
		}
	}

	pub fn write_pose(&mut self, pose: &QuadrupedPose) {
		resolve_quadruped(pose, &self.binding, &mut self.pose);
		self.segment_lengths = self.binding.metrics.quadruped_leg;
	}

	pub fn posed_angle(&self, name: &str) -> f32 {
		let Some(id) = self.binding.definition.id(name) else {
			return 0.0;
		};
		self.pose.rotation(id).angle_between(self.binding.effective_rest.rotation(id))
	}

	pub fn rotation(&self, name: &str) -> Quat {
		self.binding
			.definition
			.id(name)
			.map(|id| self.pose.rotation(id))
			.unwrap_or(Quat::IDENTITY)
	}

	pub fn animation_bone_names(&self) -> impl Iterator<Item = &'static str> {
		QUADRUPED_V0_BONES.iter().copied()
	}

	pub fn rigged_axis(&self, bone: &Name) -> Option<RiggedAxis> {
		self.bones.get(bone).map(|bone| bone.relative_axis)
	}

	pub fn animation_bones(&self) -> Vec<Name> {
		self.animation_bone_names().map(Name::from).collect()
	}
}

impl Default for QuadrupedV0Rig {
	fn default() -> Self {
		Self::imported()
	}
}

pub const QUADRUPED_V0_BONE_DEFINITIONS: [(&str, RiggedAxis); 24] = [
	("back_ridge", RiggedAxis::DEFAULT),
	("upper_back", RiggedAxis::DEFAULT),
	("lumbar", RiggedAxis::DEFAULT),
	("neck", RiggedAxis::DEFAULT),
	("shoulder.L", RiggedAxis::DEFAULT),
	("anterior_thigh.L", QUADRUPED_THIGH_AXIS),
	("anterior_shin.L", QUADRUPED_SHIN_AXIS),
	("shoulder.R", RiggedAxis::DEFAULT),
	("anterior_thigh.R", QUADRUPED_RIGHT_THIGH_AXIS),
	("anterior_shin.R", QUADRUPED_RIGHT_SHIN_AXIS),
	("hip.L", RiggedAxis::DEFAULT),
	("posterior_thigh.L", QUADRUPED_THIGH_AXIS),
	("posterior_shin.L", QUADRUPED_SHIN_AXIS),
	("hip.R", RiggedAxis::DEFAULT),
	("posterior_thigh.R", QUADRUPED_RIGHT_THIGH_AXIS),
	("posterior_shin.R", QUADRUPED_RIGHT_SHIN_AXIS),
	("tailbone", RiggedAxis::DEFAULT),
	("head_socket", RiggedAxis::DEFAULT),
	("chest_thickness", RiggedAxis::DEFAULT),
	("belly", RiggedAxis::DEFAULT),
	("waist.L", RiggedAxis::DEFAULT),
	("waist.R", RiggedAxis::DEFAULT),
	("shoulder_vertical_thickness", RiggedAxis::DEFAULT),
	("haunch_vertical_thickness", RiggedAxis::DEFAULT),
];

pub fn quadruped_v0_bone_names() -> impl Iterator<Item = &'static str> {
	QUADRUPED_V0_BONE_DEFINITIONS.into_iter().map(|(name, _axis)| name)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Side;

	#[test]
	fn quadruped_v0_animation_bones_exist_in_definition_table() {
		let rig = QuadrupedV0Rig::imported();
		for name in rig.animation_bones() {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
			assert!(rig.binding.definition.id(name.as_str()).is_some(), "missing id {name}");
		}
	}

	#[test]
	fn quadruped_v0_definition_covers_imported_dump() {
		let rig = QuadrupedV0Rig::imported();
		for name in quadruped_v0_bone_names() {
			assert!(rig.bones.get(&Name::from(name)).is_some(), "missing bone {name}");
		}
		assert_eq!(rig.bones.len(), QUADRUPED_V0_BONE_DEFINITIONS.len());
	}

	#[test]
	fn same_positive_stride_and_hinge_use_mirrored_imported_axes() {
		use crate::articulation::compose_parent_rotation;
		use crate::authoring::{
			QUADRUPED_RIGHT_SHIN_AXIS, QUADRUPED_SHIN_AXIS, QUADRUPED_THIGH_AXIS,
		};

		let mut rig = QuadrupedV0Rig::imported();
		let mut pose = QuadrupedPose::default();
		for side in [Side::Left, Side::Right] {
			let front = pose.front_mut(side);
			front.stride = 0.5;
			front.hinge = 0.7;
			let hind = pose.hind_mut(side);
			hind.stride = 0.5;
			hind.hinge = 0.7;
		}
		rig.write_pose(&pose);

		assert!(
			rig.rotation("anterior_thigh.L")
				.dot(compose_parent_rotation(Quat::IDENTITY, QUADRUPED_THIGH_AXIS, 0.5, 0.0, 0.0))
				.abs() > 1.0 - 1e-5
		);
		assert!(
			rig.rotation("anterior_shin.L")
				.dot(compose_parent_rotation(Quat::IDENTITY, QUADRUPED_SHIN_AXIS, 0.0, 0.7, 0.0))
				.abs() > 1.0 - 1e-5
		);
		assert!(
			rig.rotation("anterior_shin.R")
				.dot(compose_parent_rotation(
					Quat::IDENTITY,
					QUADRUPED_RIGHT_SHIN_AXIS,
					0.0,
					0.7,
					0.0
				))
				.abs() > 1.0 - 1e-5
		);
	}
}
