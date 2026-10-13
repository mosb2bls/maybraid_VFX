//! Quadruped semantic pose.
//!
//! Semantic fields map onto the imported V0 swing / flex / twist compose.
//! Shoulder and hip keep DEFAULT swing/flex. Thigh stride is swing. Shin hinge
//! is flex, with the right-side −Z mirror from the pre-rewrite table.

use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use super::binding::{BoneId, RigBinding, RigDefinition, SkeletonFamily};
use super::buffer::PoseBuffer;
use crate::articulation::compose_parent_rotation;
use crate::{RiggedAxis, Side};

pub const QUADRUPED_THIGH_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::X, twist_axis: Vec3::Z };

pub const QUADRUPED_SHIN_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::Z, twist_axis: Vec3::X };

pub const QUADRUPED_RIGHT_THIGH_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::NEG_Y, flex_axis: Vec3::NEG_X, twist_axis: Vec3::Z };

pub const QUADRUPED_RIGHT_SHIN_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::NEG_Z, twist_axis: Vec3::X };

/// Inspected `quadruped_rig.glb` node rotations (xyzw). Production quadruped bodies share them.
pub const QUADRUPED_GLB_ANTERIOR_MID_BACK: Quat = Quat::from_xyzw(0.0, 0.70710677, 0.70710677, 0.0);
pub const QUADRUPED_GLB_SHOULDER_L: Quat = Quat::from_xyzw(0.0, 0.0, 0.70710677, 0.70710677);
pub const QUADRUPED_GLB_THIGH: Quat = Quat::from_xyzw(-0.70710677, 0.0, 0.0, 0.70710677);

/// Imported V0 hinge axes. Forelimb elbows use shin flex, including the right −Z mirror.
pub fn bone_axis(name: &str) -> RiggedAxis {
	match name {
		"anterior_thigh.L" | "posterior_thigh.L" => QUADRUPED_THIGH_AXIS,
		"anterior_thigh.R" | "posterior_thigh.R" => QUADRUPED_RIGHT_THIGH_AXIS,
		"anterior_shin.L" | "posterior_shin.L" => QUADRUPED_SHIN_AXIS,
		"anterior_shin.R" | "posterior_shin.R" => QUADRUPED_RIGHT_SHIN_AXIS,
		_ => RiggedAxis::DEFAULT,
	}
}

pub const QUADRUPED_V0_BONES: &[&str] = &[
	"back_ridge",
	"upper_back",
	"lumbar",
	"neck",
	"shoulder.L",
	"shoulder.R",
	"anterior_thigh.L",
	"anterior_shin.L",
	"anterior_thigh.R",
	"anterior_shin.R",
	"hip.L",
	"hip.R",
	"posterior_thigh.L",
	"posterior_shin.L",
	"posterior_thigh.R",
	"posterior_shin.R",
];

pub const QUADRUPED_V0_COUNT: usize = QUADRUPED_V0_BONES.len();

const PARENTS: &[(&str, &str)] = &[
	("upper_back", "back_ridge"),
	("lumbar", "back_ridge"),
	("neck", "upper_back"),
	("shoulder.L", "upper_back"),
	("shoulder.R", "upper_back"),
	("anterior_thigh.L", "shoulder.L"),
	("anterior_thigh.R", "shoulder.R"),
	("anterior_shin.L", "anterior_thigh.L"),
	("anterior_shin.R", "anterior_thigh.R"),
	("hip.L", "lumbar"),
	("hip.R", "lumbar"),
	("posterior_thigh.L", "hip.L"),
	("posterior_thigh.R", "hip.R"),
	("posterior_shin.L", "posterior_thigh.L"),
	("posterior_shin.R", "posterior_thigh.R"),
];

pub fn quadruped_v0_definition() -> Arc<RigDefinition> {
	static DEFINITION: OnceLock<Arc<RigDefinition>> = OnceLock::new();
	DEFINITION
		.get_or_init(|| {
			Arc::new(RigDefinition::from_names(
				SkeletonFamily::Quadruped,
				QUADRUPED_V0_BONES,
				PARENTS,
			))
		})
		.clone()
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuadrupedLimbPose {
	/// Shoulder or hip yaw, radians about +Y.
	pub proximal_turn: f32,
	/// Shoulder or hip lateral, radians about +Z.
	pub proximal_lateral: f32,
	/// Thigh sagittal stride.
	pub stride: f32,
	/// Shin sagittal hinge.
	pub hinge: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuadrupedPose {
	/// Back-ridge axial rotation (old swing).
	pub spine_axial: f32,
	/// Lumbar lateral bend (old flex). Gather clips still write this channel.
	pub spine_lateral: f32,
	/// Lumbar sagittal bend.
	pub spine_sagittal: f32,
	pub neck_nod: f32,
	pub neck_tilt: f32,
	pub neck_turn: f32,
	pub front: [QuadrupedLimbPose; 2],
	pub hind: [QuadrupedLimbPose; 2],
}

impl QuadrupedPose {
	pub fn front_mut(&mut self, side: Side) -> &mut QuadrupedLimbPose {
		&mut self.front[side.index()]
	}

	pub fn hind_mut(&mut self, side: Side) -> &mut QuadrupedLimbPose {
		&mut self.hind[side.index()]
	}
}

pub fn resolve_quadruped(pose: &QuadrupedPose, binding: &RigBinding, out: &mut PoseBuffer) {
	apply(pose, &binding.definition, &binding.effective_rest, out);
}

pub(crate) fn apply(
	pose: &QuadrupedPose,
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
			compose_parent_rotation(rest.rotation(bone), bone_axis(name), swing, flex, twist),
		);
	}
}

fn channels_for(name: &str, pose: &QuadrupedPose) -> Option<(f32, f32, f32)> {
	match name {
		"back_ridge" => Some((pose.spine_axial, 0.0, 0.0)),
		"lumbar" => Some((0.0, pose.spine_lateral, pose.spine_sagittal)),
		"neck" => Some((pose.neck_turn, pose.neck_tilt, pose.neck_nod)),
		"shoulder.L" => Some((pose.front[0].proximal_turn, pose.front[0].proximal_lateral, 0.0)),
		"shoulder.R" => Some((pose.front[1].proximal_turn, pose.front[1].proximal_lateral, 0.0)),
		"hip.L" => Some((pose.hind[0].proximal_turn, pose.hind[0].proximal_lateral, 0.0)),
		"hip.R" => Some((pose.hind[1].proximal_turn, pose.hind[1].proximal_lateral, 0.0)),
		"anterior_thigh.L" => Some((pose.front[0].stride, 0.0, 0.0)),
		"anterior_thigh.R" => Some((pose.front[1].stride, 0.0, 0.0)),
		"posterior_thigh.L" => Some((pose.hind[0].stride, 0.0, 0.0)),
		"posterior_thigh.R" => Some((pose.hind[1].stride, 0.0, 0.0)),
		"anterior_shin.L" => Some((0.0, pose.front[0].hinge, 0.0)),
		"anterior_shin.R" => Some((0.0, pose.front[1].hinge, 0.0)),
		"posterior_shin.L" => Some((0.0, pose.hind[0].hinge, 0.0)),
		"posterior_shin.R" => Some((0.0, pose.hind[1].hinge, 0.0)),
		_ => None,
	}
}

/// Inspected `quadruped_rig.glb` local transforms for the V0 animation bones.
pub fn apply_quadruped_glb_rest(definition: &RigDefinition, rest: &mut PoseBuffer) {
	let set = |rest: &mut PoseBuffer, name: &str, translation: Vec3, rotation: Quat| {
		if let Some(id) = definition.id(name) {
			if let Some(slot) = rest.local.get_mut(id.index()) {
				slot.translation = translation;
				slot.rotation = rotation;
			}
		}
	};
	set(rest, "back_ridge", Vec3::ZERO, Quat::IDENTITY);
	set(rest, "upper_back", Vec3::new(0.0, 0.25, 0.0), Quat::IDENTITY);
	set(rest, "lumbar", Vec3::new(0.0, 0.25, 0.0), Quat::IDENTITY);
	set(rest, "neck", Vec3::new(0.0, 0.50, 0.0), Quat::IDENTITY);
	set(rest, "shoulder.L", Vec3::new(0.0, 0.50, 0.0), QUADRUPED_GLB_SHOULDER_L);
	set(
		rest,
		"shoulder.R",
		Vec3::new(0.0, 0.50, 0.0),
		Quat::from_xyzw(0.0, 0.0, -0.70710677, 0.70710677),
	);
	set(rest, "anterior_thigh.L", Vec3::new(0.0, 0.50, 0.0), QUADRUPED_GLB_THIGH);
	set(rest, "anterior_thigh.R", Vec3::new(0.0, 0.50, 0.0), QUADRUPED_GLB_THIGH);
	set(rest, "anterior_shin.L", Vec3::new(0.0, 0.50, 0.0), Quat::IDENTITY);
	set(rest, "anterior_shin.R", Vec3::new(0.0, 0.50, 0.0), Quat::IDENTITY);
	set(
		rest,
		"hip.L",
		Vec3::new(0.0, 0.50, 0.0),
		Quat::from_xyzw(0.0, 0.0, -0.70710677, 0.70710677),
	);
	set(
		rest,
		"hip.R",
		Vec3::new(0.0, 0.50, 0.0),
		Quat::from_xyzw(0.0, 0.0, 0.70710677, 0.70710677),
	);
	set(rest, "posterior_thigh.L", Vec3::new(0.0, 0.40, 0.0), QUADRUPED_GLB_THIGH);
	set(rest, "posterior_thigh.R", Vec3::new(0.0, 0.40, 0.0), QUADRUPED_GLB_THIGH);
	set(rest, "posterior_shin.L", Vec3::new(0.0, 0.50, 0.0), Quat::IDENTITY);
	set(rest, "posterior_shin.R", Vec3::new(0.0, 0.50, 0.0), Quat::IDENTITY);
}

pub fn identity_binding() -> RigBinding {
	let definition = quadruped_v0_definition();
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
	use crate::articulation::compose_parent_rotation;

	#[test]
	fn hinge_uses_imported_shin_flex() {
		let binding = identity_binding();
		let mut pose = QuadrupedPose::default();
		pose.front_mut(Side::Left).hinge = 0.4;
		pose.front_mut(Side::Right).hinge = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_quadruped(&pose, &binding, &mut out);
		let left = binding.definition.id("anterior_shin.L").expect("shin.L");
		let right = binding.definition.id("anterior_shin.R").expect("shin.R");
		assert!(
			out.rotation(left)
				.dot(compose_parent_rotation(Quat::IDENTITY, QUADRUPED_SHIN_AXIS, 0.0, 0.4, 0.0))
				.abs() > 1.0 - 1e-5
		);
		assert!(
			out.rotation(right)
				.dot(compose_parent_rotation(
					Quat::IDENTITY,
					QUADRUPED_RIGHT_SHIN_AXIS,
					0.0,
					0.4,
					0.0
				))
				.abs() > 1.0 - 1e-5
		);
	}

	#[test]
	fn imported_forelimb_chain_folds_the_elbow_backward() {
		// quadruped_rig.glb: anterior_mid_back parents upper_back. Old shin flex is parent Z.
		let mid = QUADRUPED_GLB_ANTERIOR_MID_BACK;
		let shoulder = QUADRUPED_GLB_SHOULDER_L;
		let thigh = QUADRUPED_GLB_THIGH;
		let binding = identity_binding();
		let mut pose = QuadrupedPose::default();
		pose.front_mut(Side::Left).hinge = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_quadruped(&pose, &binding, &mut out);
		let shin = binding.definition.id("anterior_shin.L").expect("shin");
		let visual = mid * shoulder * thigh * out.rotation(shin) * Vec3::Y;
		let rest = mid * shoulder * thigh * Vec3::Y;
		assert!(rest.y < -0.9, "bind hangs down, got {rest:?}");
		assert!(visual.z < -0.2, "old flex folds the shin backward, got {visual:?}");
		assert!(visual.x.abs() < 0.05, "hinge stays sagittal, got {visual:?}");
	}
}
