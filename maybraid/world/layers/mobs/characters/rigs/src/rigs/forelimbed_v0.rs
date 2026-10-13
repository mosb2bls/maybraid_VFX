use bevy::prelude::*;

use crate::{
	authoring::{
		forelimbed_v0_definition, resolve_forelimbed, ForelimbedPose, PoseBuffer, PoseScratch,
		RigBinding, FORELIMBED_V0_BONES,
	},
	BoneDefinition, BoneTable, Name, RiggedAxis,
};

/// Imported forelimbed (aquatic) rig: axial spine + paired pectoral fins.
#[derive(Component, Debug, Clone)]
pub struct ForelimbedV0Rig {
	pub bones: BoneTable,
	pub binding: RigBinding,
	/// Last resolved local pose. Sampling always starts from [`Self::binding`] rest.
	pub pose: PoseBuffer,
	pub scratch: PoseScratch,
}

impl ForelimbedV0Rig {
	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in FORELIMBED_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}
		let definition = forelimbed_v0_definition();
		let len = definition.len();
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; len].into_boxed_slice(),
			PoseBuffer::identity(len),
		);
		Self {
			bones,
			binding,
			pose: PoseBuffer::identity(len),
			scratch: PoseScratch::identity(len),
		}
	}

	pub fn write_pose(&mut self, pose: &ForelimbedPose) {
		resolve_forelimbed(pose, &self.binding, &mut self.pose);
	}

	pub fn rotation(&self, name: &str) -> Quat {
		self.binding
			.definition
			.id(name)
			.map(|id| self.pose.rotation(id))
			.unwrap_or(Quat::IDENTITY)
	}

	pub fn animation_bone_names(&self) -> impl Iterator<Item = &'static str> {
		FORELIMBED_V0_BONES.iter().copied()
	}

	pub fn rigged_axis(&self, bone: &Name) -> Option<RiggedAxis> {
		self.bones.get(bone).map(|bone| bone.relative_axis)
	}

	pub fn animation_bones(&self) -> Vec<Name> {
		self.animation_bone_names().map(Name::from).collect()
	}
}

impl Default for ForelimbedV0Rig {
	fn default() -> Self {
		Self::imported()
	}
}

pub const FORELIMBED_V0_BONE_DEFINITIONS: [(&str, RiggedAxis); 18] = [
	("lower_mid_spine", RiggedAxis::DEFAULT),
	("lower_spine", RiggedAxis::DEFAULT),
	("tailbone", RiggedAxis::DEFAULT),
	("tail_socket", RiggedAxis::DEFAULT),
	("upper_mid_spine", RiggedAxis::DEFAULT),
	("upper_spine", RiggedAxis::DEFAULT),
	("head_socket", RiggedAxis::DEFAULT),
	("shoulder.L", RiggedAxis::DEFAULT),
	("upper_arm.L", RiggedAxis::DEFAULT),
	("lower_arm.L", RiggedAxis::DEFAULT),
	("shoulder.R", RiggedAxis::DEFAULT),
	("upper_arm.R", RiggedAxis::DEFAULT),
	("lower_arm.R", RiggedAxis::DEFAULT),
	("torso_thickness.L", RiggedAxis::DEFAULT),
	("belly", RiggedAxis::DEFAULT),
	("back_ridge", RiggedAxis::DEFAULT),
	("dorsal_socket", RiggedAxis::DEFAULT),
	("torso_thickness.R", RiggedAxis::DEFAULT),
];

pub fn forelimbed_v0_bone_names() -> impl Iterator<Item = &'static str> {
	FORELIMBED_V0_BONE_DEFINITIONS.into_iter().map(|(name, _axis)| name)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forelimbed_v0_animation_bones_exist_in_definition_table() {
		let rig = ForelimbedV0Rig::imported();
		for name in rig.animation_bones() {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
			assert!(rig.binding.definition.id(name.as_str()).is_some(), "missing id {name}");
		}
	}

	#[test]
	fn forelimbed_v0_definition_covers_imported_dump() {
		let rig = ForelimbedV0Rig::imported();
		for name in forelimbed_v0_bone_names() {
			assert!(rig.bones.get(&Name::from(name)).is_some(), "missing bone {name}");
		}
		assert_eq!(rig.bones.len(), FORELIMBED_V0_BONE_DEFINITIONS.len());
	}

	#[test]
	fn dorsoventral_bend_and_lateral_yaw_use_distinct_axes() {
		let mut rig = ForelimbedV0Rig::imported();
		let mut pose = ForelimbedPose::default();
		pose.dorsoventral[4] = 0.5;
		rig.write_pose(&pose);
		let bend = rig.rotation("tailbone") * Vec3::Y;
		assert!(bend.z > 0.2 && bend.x.abs() < 1e-3, "sagittal bend, got {bend:?}");

		let mut pose = ForelimbedPose::default();
		pose.lateral[4] = 0.5;
		rig.write_pose(&pose);
		let yaw = rig.rotation("tailbone") * Vec3::Z;
		assert!(yaw.x.abs() > 0.2 && yaw.y.abs() < 1e-3, "axial yaw, got {yaw:?}");
	}
}
