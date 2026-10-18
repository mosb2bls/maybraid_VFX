//! Forelimbed (axial swimmer) semantic pose.
//!
//! Lateral undulation is a yaw about character +Y. Dorsoventral bend is flexion
//! about +X. Fin sweep is shoulder yaw; fin flap is shoulder lateral bend.

use std::sync::{Arc, OnceLock};

use super::binding::{BoneId, RigBinding, RigDefinition, SkeletonFamily};
use super::buffer::PoseBuffer;
use crate::articulation::compose_parent_rotation;
use crate::RiggedAxis;

/// upper_mid, upper, lower_mid, lower, tailbone.
pub const FORELIMBED_SPINE: usize = 5;

pub const FORELIMBED_V0_BONES: &[&str] = &[
	"upper_mid_spine",
	"upper_spine",
	"lower_mid_spine",
	"lower_spine",
	"tailbone",
	"back_ridge",
	"shoulder.L",
	"upper_arm.L",
	"lower_arm.L",
	"shoulder.R",
	"upper_arm.R",
	"lower_arm.R",
];

pub const FORELIMBED_V0_COUNT: usize = FORELIMBED_V0_BONES.len();

const PARENTS: &[(&str, &str)] = &[
	("upper_spine", "upper_mid_spine"),
	("lower_spine", "lower_mid_spine"),
	("tailbone", "lower_spine"),
	("shoulder.L", "upper_spine"),
	("upper_arm.L", "shoulder.L"),
	("lower_arm.L", "upper_arm.L"),
	("shoulder.R", "upper_spine"),
	("upper_arm.R", "shoulder.R"),
	("lower_arm.R", "upper_arm.R"),
];

const SPINE_NAMES: [&str; FORELIMBED_SPINE] =
	["upper_mid_spine", "upper_spine", "lower_mid_spine", "lower_spine", "tailbone"];

pub fn forelimbed_v0_definition() -> Arc<RigDefinition> {
	static DEFINITION: OnceLock<Arc<RigDefinition>> = OnceLock::new();
	DEFINITION
		.get_or_init(|| {
			Arc::new(RigDefinition::from_names(
				SkeletonFamily::Forelimbed,
				FORELIMBED_V0_BONES,
				PARENTS,
			))
		})
		.clone()
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ForelimbedPose {
	/// Lateral yaw per spine bone, radians about +Y.
	pub lateral: [f32; FORELIMBED_SPINE],
	/// Dorsoventral bend per spine bone, radians about +X.
	pub dorsoventral: [f32; FORELIMBED_SPINE],
	/// Long-axis twist per spine bone.
	pub axial: [f32; FORELIMBED_SPINE],
	/// Pectoral sweep, left = 0, right = 1.
	pub fin_sweep: [f32; 2],
	/// Pectoral flap, left = 0, right = 1.
	pub fin_flap: [f32; 2],
}

pub fn resolve_forelimbed(pose: &ForelimbedPose, binding: &RigBinding, out: &mut PoseBuffer) {
	apply(pose, &binding.definition, &binding.effective_rest, out);
}

pub(crate) fn apply(
	pose: &ForelimbedPose,
	definition: &RigDefinition,
	rest: &PoseBuffer,
	out: &mut PoseBuffer,
) {
	out.copy_from(rest);
	for (index, name) in definition.names.iter().enumerate() {
		let Some((swing, flex, twist)) = channels_for(name, pose) else {
			continue;
		};
		let bone = BoneId(index as u16);
		out.set_rotation(
			bone,
			compose_parent_rotation(rest.rotation(bone), RiggedAxis::DEFAULT, swing, flex, twist),
		);
	}
}

fn channels_for(name: &str, pose: &ForelimbedPose) -> Option<(f32, f32, f32)> {
	if let Some(slot) = SPINE_NAMES.iter().position(|candidate| *candidate == name) {
		// Old DEFAULT: swing = yaw, flex = leftover, twist = dorsoventral pitch.
		return Some((pose.lateral[slot], pose.axial[slot], pose.dorsoventral[slot]));
	}
	let fin = |side: usize, sweep_scale: f32, flap_scale: f32| {
		Some((pose.fin_sweep[side] * sweep_scale, pose.fin_flap[side] * flap_scale, 0.0))
	};
	match name {
		"shoulder.L" => fin(0, 1.0, 1.0),
		"upper_arm.L" => fin(0, 0.6, 0.6),
		"shoulder.R" => fin(1, 1.0, 1.0),
		"upper_arm.R" => fin(1, 0.6, 0.6),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::authoring::HUMANOID_GLB_SHOULDER_L;
	use bevy::prelude::{Entity, Vec3};

	#[test]
	fn t_pose_fin_sweep_uses_parent_y_not_a_length_roll() {
		// forelimbed_rig.glb shoulder.L is the same Rz(−90°) as the humanoid T-pose.
		let definition = forelimbed_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		let shoulder = definition.id("shoulder.L").expect("shoulder");
		rest.local[shoulder.index()].rotation = HUMANOID_GLB_SHOULDER_L;
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; rest.len()].into_boxed_slice(),
			rest,
		);
		let mut pose = ForelimbedPose::default();
		pose.fin_sweep[0] = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_forelimbed(&pose, &binding, &mut out);
		let along = binding.definition.rotation_in_character(&out, shoulder) * Vec3::Y;
		assert!(along.z.abs() > 0.2, "old fin sweep was parent Y, got {along:?}");
		assert!(along.y.abs() < 0.05, "sweep is not a length roll, got {along:?}");
	}
}
