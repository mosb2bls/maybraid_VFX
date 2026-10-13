use std::sync::Arc;

use bevy::prelude::*;

use super::buffer::PoseBuffer;
use crate::humanoid::LegSegmentLengths as HumanoidLengths;
use crate::quadruped::LegSegmentLengths as QuadrupedLengths;

/// Bone index inside one [`RigDefinition`]. Not comparable across definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoneId(pub u16);

impl BoneId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

/// Which sampler and metric set a definition uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonFamily {
	Humanoid,
	Quadruped,
	Forelimbed,
}

/// Shared, immutable skeleton: names, parents, and evaluation order.
///
/// Names are for import and diagnostics. Sampling indexes [`BoneId`].
#[derive(Debug, Clone)]
pub struct RigDefinition {
	pub family: SkeletonFamily,
	pub names: Box<[&'static str]>,
	pub parents: Box<[Option<BoneId>]>,
	pub evaluation_order: Box<[BoneId]>,
}

impl RigDefinition {
	pub fn from_names(
		family: SkeletonFamily,
		names: &[&'static str],
		parent_names: &[(&str, &str)],
	) -> Self {
		let parents = names
			.iter()
			.map(|name| {
				parent_names
					.iter()
					.find(|(child, parent)| child == name && !parent.is_empty())
					.and_then(|(_, parent)| {
						names
							.iter()
							.position(|candidate| candidate == parent)
							.map(|index| BoneId(index as u16))
					})
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let evaluation_order = evaluation_order(&parents);
		Self { family, names: names.to_vec().into_boxed_slice(), parents, evaluation_order }
	}

	pub fn len(&self) -> usize {
		self.names.len()
	}

	pub fn id(&self, name: &str) -> Option<BoneId> {
		self.names
			.iter()
			.position(|candidate| *candidate == name)
			.map(|index| BoneId(index as u16))
	}

	pub fn parent(&self, bone: BoneId) -> Option<BoneId> {
		self.parents.get(bone.index()).copied().flatten()
	}

	/// Character-space rotation of `bone` in `pose`, parents first.
	pub fn rotation_in_character(&self, pose: &PoseBuffer, bone: BoneId) -> Quat {
		let mut chain = [BoneId(0); 24];
		let mut count = 0;
		let mut cursor = Some(bone);
		while let Some(id) = cursor {
			if count >= chain.len() {
				break;
			}
			chain[count] = id;
			count += 1;
			cursor = self.parent(id);
		}
		let mut rotation = Quat::IDENTITY;
		for id in chain[..count].iter().rev() {
			rotation *= pose.rotation(*id);
		}
		rotation
	}

	pub fn parent_rotation(&self, pose: &PoseBuffer, bone: BoneId) -> Quat {
		match self.parent(bone) {
			Some(parent) => self.rotation_in_character(pose, parent),
			None => Quat::IDENTITY,
		}
	}

	/// Character-space origin of `bone` in `pose`, parents first.
	pub fn translation_in_character(&self, pose: &PoseBuffer, bone: BoneId) -> Vec3 {
		let mut chain = [BoneId(0); 24];
		let mut count = 0;
		let mut cursor = Some(bone);
		while let Some(id) = cursor {
			if count >= chain.len() {
				break;
			}
			chain[count] = id;
			count += 1;
			cursor = self.parent(id);
		}
		let mut transform = Transform::IDENTITY;
		for id in chain[..count].iter().rev() {
			if let Some(local) = pose.get(*id) {
				transform = transform * local;
			}
		}
		transform.translation
	}

	fn first_child(&self, bone: BoneId) -> Option<BoneId> {
		self.parents
			.iter()
			.enumerate()
			.find(|(_, parent)| **parent == Some(bone))
			.map(|(index, _)| BoneId(index as u16))
	}
}

fn evaluation_order(parents: &[Option<BoneId>]) -> Box<[BoneId]> {
	let mut seen = vec![false; parents.len()];
	let mut order = Vec::with_capacity(parents.len());
	fn visit(index: usize, parents: &[Option<BoneId>], seen: &mut [bool], order: &mut Vec<BoneId>) {
		if seen.get(index).copied().unwrap_or(true) {
			return;
		}
		if let Some(parent) = parents[index] {
			visit(parent.index(), parents, seen, order);
		}
		seen[index] = true;
		order.push(BoneId(index as u16));
	}
	for index in 0..parents.len() {
		visit(index, parents, &mut seen, &mut order);
	}
	order.into_boxed_slice()
}

/// Per-character rest, entities, and derived segment lengths.
#[derive(Clone, Debug)]
pub struct RigBinding {
	pub definition: Arc<RigDefinition>,
	pub entities: Box<[Entity]>,
	pub effective_rest: PoseBuffer,
	pub metrics: RigMetrics,
}

impl RigBinding {
	pub fn from_rest(
		definition: Arc<RigDefinition>,
		entities: Box<[Entity]>,
		effective_rest: PoseBuffer,
	) -> Self {
		let metrics = RigMetrics::from_rest(&definition, &effective_rest);
		Self { definition, entities, effective_rest, metrics }
	}

	/// Replace rest when a proportion edit or bone-map reload changes bind transforms.
	pub fn refresh_rest(&mut self, effective_rest: PoseBuffer) {
		if effective_rest.local == self.effective_rest.local {
			return;
		}
		self.metrics = RigMetrics::from_rest(&self.definition, &effective_rest);
		self.effective_rest = effective_rest;
	}
}

/// Lengths derived from joint-to-joint rest distances.
///
/// A bone's own translation is the offset to its origin, not its segment length.
/// Femur length is the shin origin's translation. A missing distal joint keeps
/// the family default (0.5 m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigMetrics {
	pub humanoid_leg: HumanoidLengths,
	pub quadruped_leg: QuadrupedLengths,
}

impl Default for RigMetrics {
	fn default() -> Self {
		Self {
			humanoid_leg: HumanoidLengths::default(),
			quadruped_leg: QuadrupedLengths::default(),
		}
	}
}

impl RigMetrics {
	pub fn from_rest(definition: &RigDefinition, rest: &PoseBuffer) -> Self {
		let mut metrics = Self::default();
		let distal = |name: &str| -> f32 {
			definition
				.id(name)
				.and_then(|id| definition.first_child(id))
				.and_then(|child| rest.get(child))
				.map(|transform| transform.translation.length())
				.filter(|length| *length > 1e-3)
				.unwrap_or(0.0)
		};
		match definition.family {
			SkeletonFamily::Humanoid => {
				let femur = distal("femur.L");
				let shin = distal("shin.L");
				if femur > 0.0 {
					metrics.humanoid_leg.femur = femur;
				}
				if shin > 0.0 {
					metrics.humanoid_leg.shin = shin;
				}
			}
			SkeletonFamily::Quadruped => {
				let upper = distal("anterior_thigh.L");
				let lower = distal("anterior_shin.L");
				if upper > 0.0 {
					metrics.quadruped_leg.upper = upper;
				}
				if lower > 0.0 {
					metrics.quadruped_leg.lower = lower;
				}
			}
			SkeletonFamily::Forelimbed => {}
		}
		metrics
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::authoring::{apply_humanoid_glb_rest, humanoid_v0_definition};

	#[test]
	fn humanoid_glb_femur_length_is_the_shin_origin() {
		let definition = humanoid_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		apply_humanoid_glb_rest(&definition, &mut rest);
		let metrics = RigMetrics::from_rest(&definition, &rest);
		assert!(
			(metrics.humanoid_leg.femur - 0.5).abs() < 1e-4,
			"got {}",
			metrics.humanoid_leg.femur
		);
		assert!((metrics.humanoid_leg.shin - 0.5).abs() < 1e-4, "shin keeps the default");
		let femur = definition.id("femur.L").expect("femur");
		assert!((rest.get(femur).expect("t").translation.length() - 0.25).abs() < 1e-4);
	}
}
