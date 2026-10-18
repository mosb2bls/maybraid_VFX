//! Layered bind-pose composition for rig proportions.
//!
//! Effects compose as `bind * layer1 * layer2 * …`. Absolute clip poses
//! (character locomotion) stay in domain crates; this module is the shared
//! proportion stack applied through a [`crate::BoneMap`].
//!
//! Prefer [`BoneTranslation::length`] over [`BoneScale::length`] when a bone
//! also needs a pose rotation and has descendants that must stay unsheared
//! (non-uniform scale + child rotation shears `GlobalTransform`).

use std::collections::HashMap;

use bevy::prelude::*;

use crate::bone_map::{BoneMap, RigRoot};

/// Scale multiplier for one named bone.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneScale {
	pub bone: &'static str,
	pub scale: Vec3,
}

impl BoneScale {
	pub const fn new(bone: &'static str, scale: Vec3) -> Self {
		Self { bone, scale }
	}

	pub fn uniform(bone: &'static str, scale: f32) -> Self {
		Self { bone, scale: Vec3::splat(scale) }
	}

	pub fn length(bone: &'static str, scale: f32) -> Self {
		Self { bone, scale: Vec3::new(1.0, scale, 1.0) }
	}

	pub fn lateral(bone: &'static str, scale: f32) -> Self {
		Self { bone, scale: Vec3::new(scale, 1.0, 1.0) }
	}

	pub fn thickness(bone: &'static str, scale: f32) -> Self {
		Self { bone, scale: Vec3::new(scale, 1.0, scale) }
	}
}

/// Bind-pose translation multiplier for one named bone.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneTranslation {
	pub bone: &'static str,
	pub multiplier: Vec3,
}

impl BoneTranslation {
	pub const fn new(bone: &'static str, multiplier: Vec3) -> Self {
		Self { bone, multiplier }
	}

	pub fn length(bone: &'static str, scale: f32) -> Self {
		Self { bone, multiplier: Vec3::new(1.0, scale, 1.0) }
	}
}

/// Rotation offset for one named bone, composed onto the bind rotation.
///
/// Pose apply uses parent-space composition (`delta * bind`) so a counter-pitch
/// on a non-identity rest bone (e.g. `head_socket`) can cancel a parent pitch.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneRotation {
	pub bone: &'static str,
	pub rotation: Quat,
}

impl BoneRotation {
	pub const fn new(bone: &'static str, rotation: Quat) -> Self {
		Self { bone, rotation }
	}

	pub fn pitch_x(bone: &'static str, radians: f32) -> Self {
		Self { bone, rotation: Quat::from_rotation_x(radians) }
	}

	pub fn pitch_z(bone: &'static str, radians: f32) -> Self {
		Self { bone, rotation: Quat::from_rotation_z(radians) }
	}
}

/// A named proportional layer in the bind-pose composition stack.
#[derive(Debug, Clone, PartialEq)]
pub struct RigPoseLayer {
	pub label: &'static str,
	pub scales: Vec<BoneScale>,
	pub translations: Vec<BoneTranslation>,
	pub rotations: Vec<BoneRotation>,
}

impl RigPoseLayer {
	pub fn new(label: &'static str) -> Self {
		Self { label, scales: Vec::new(), translations: Vec::new(), rotations: Vec::new() }
	}

	pub fn with_scale(mut self, scale: BoneScale) -> Self {
		self.scales.push(scale);
		self
	}

	pub fn with_translation(mut self, translation: BoneTranslation) -> Self {
		self.translations.push(translation);
		self
	}

	pub fn with_rotation(mut self, rotation: BoneRotation) -> Self {
		self.rotations.push(rotation);
		self
	}

	pub fn scales(&self) -> impl Iterator<Item = &BoneScale> {
		self.scales.iter()
	}

	pub fn translations(&self) -> impl Iterator<Item = &BoneTranslation> {
		self.translations.iter()
	}

	pub fn rotations(&self) -> impl Iterator<Item = &BoneRotation> {
		self.rotations.iter()
	}
}

/// Resolved layers to apply to a rig, in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedRigPose {
	pub layers: Vec<RigPoseLayer>,
}

impl ResolvedRigPose {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_layer(mut self, layer: RigPoseLayer) -> Self {
		self.layers.push(layer);
		self
	}

	pub fn layers(&self) -> impl Iterator<Item = &RigPoseLayer> {
		self.layers.iter()
	}

	pub fn scale_for_bone(&self, bone: &str) -> Vec3 {
		self.layers
			.iter()
			.flat_map(|layer| layer.scales())
			.filter(|scale| scale.bone == bone)
			.fold(Vec3::ONE, |acc, scale| acc * scale.scale)
	}

	pub fn translation_for_bone(&self, bone: &str) -> Vec3 {
		self.layers
			.iter()
			.flat_map(|layer| layer.translations())
			.filter(|translation| translation.bone == bone)
			.fold(Vec3::ONE, |acc, translation| acc * translation.multiplier)
	}

	pub fn rotation_for_bone(&self, bone: &str) -> Quat {
		self.layers
			.iter()
			.flat_map(|layer| layer.rotations())
			.filter(|rotation| rotation.bone == bone)
			.fold(Quat::IDENTITY, |acc, rotation| acc * rotation.rotation)
	}
}

/// Resolved proportional layers to maintain on this rig across frames.
#[derive(Component, Clone, Default, PartialEq)]
pub struct ActiveRigPose {
	pub pose: ResolvedRigPose,
}

/// Bind-pose bone TRS captured once each named bone appears in the rig map.
#[derive(Component, Default, Clone)]
pub struct BindPose {
	pub scales: HashMap<String, Vec3>,
	pub translations: HashMap<String, Vec3>,
	pub rotations: HashMap<String, Quat>,
}

/// Inserted the first frame [`ActiveRigPose`] is applied to a ready rig.
#[derive(Component)]
pub struct PoseApplied;

/// Force the next [`maintain_bind_pose`] pass (spawn reset / explicit dirty).
#[derive(Component, Clone, Copy, Default)]
pub struct BindPoseDirty;

/// Skip rotation writes on this bone (clip mailbox owns the joint).
#[derive(Component, Clone, Copy, Default)]
pub struct PoseSkipRotation;

pub fn bind_pose_ready(bind: &BindPose, map: &BoneMap, landmarks: &[&str]) -> bool {
	if landmarks.is_empty() {
		return true;
	}
	landmarks.iter().all(|bone| {
		bind.scales.contains_key(*bone)
			&& bind.translations.contains_key(*bone)
			&& bind.rotations.contains_key(*bone)
			&& map.by_name.contains_key(*bone)
	})
}

/// Capture bind TRS, then apply [`ActiveRigPose`] layers onto named bones.
///
/// Skips rotation on [`PoseSkipRotation`] so a clip mailbox can own those joints.
/// Settled rigs leave this query until the pose, map, or [`BindPoseDirty`] changes.
/// PostUpdate uses the same filter so a quiet herd is not walked twice.
pub fn maintain_bind_pose(
	mut commands: Commands,
	mut rig_roots: Query<
		(Entity, &BoneMap, &ActiveRigPose, &mut BindPose, &RigRoot, Has<PoseApplied>),
		(
			With<RigRoot>,
			Or<(
				Changed<ActiveRigPose>,
				Changed<BoneMap>,
				Without<PoseApplied>,
				With<BindPoseDirty>,
			)>,
		),
	>,
	mut transforms: Query<&mut Transform>,
	skip_rotation: Query<(), With<PoseSkipRotation>>,
) {
	for (entity, bone_map, active_pose, mut bind, rig, pose_applied) in &mut rig_roots {
		if !bone_map.landmarks_ready(rig.landmarks) {
			continue;
		}

		for (bone_name, bone_entity) in &bone_map.by_name {
			if bind.scales.contains_key(bone_name)
				&& bind.translations.contains_key(bone_name)
				&& bind.rotations.contains_key(bone_name)
			{
				continue;
			}
			let Ok(transform) = transforms.get(*bone_entity) else {
				continue;
			};
			bind.scales.entry(bone_name.clone()).or_insert(transform.scale);
			bind.translations.entry(bone_name.clone()).or_insert(transform.translation);
			bind.rotations.entry(bone_name.clone()).or_insert(transform.rotation);
		}

		for (bone_name, bone_entity) in &bone_map.by_name {
			let scale_mult = active_pose.pose.scale_for_bone(bone_name);
			let trans_mult = active_pose.pose.translation_for_bone(bone_name);
			let rot_offset = active_pose.pose.rotation_for_bone(bone_name);
			if scale_mult == Vec3::ONE && trans_mult == Vec3::ONE && rot_offset == Quat::IDENTITY {
				continue;
			}
			let Ok(mut transform) = transforms.get_mut(*bone_entity) else {
				continue;
			};
			if scale_mult != Vec3::ONE {
				if let Some(bind_scale) = bind.scales.get(bone_name) {
					transform.scale = *bind_scale * scale_mult;
				}
			}
			if trans_mult != Vec3::ONE {
				if let Some(bind_trans) = bind.translations.get(bone_name) {
					transform.translation = *bind_trans * trans_mult;
				}
			}
			if rot_offset != Quat::IDENTITY && !skip_rotation.contains(*bone_entity) {
				if let Some(bind_rot) = bind.rotations.get(bone_name) {
					transform.rotation = rot_offset * *bind_rot;
				}
			}
		}

		if !pose_applied && bind_pose_ready(&bind, bone_map, rig.landmarks) {
			commands.entity(entity).try_insert(PoseApplied);
		}
		commands.entity(entity).try_remove::<BindPoseDirty>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::bone_map::RigKey;

	#[test]
	fn scale_for_bone_multiplies_layers() {
		let pose = ResolvedRigPose::new()
			.with_layer(RigPoseLayer::new("a").with_scale(BoneScale::uniform("root", 2.0)))
			.with_layer(RigPoseLayer::new("b").with_scale(BoneScale::uniform("root", 0.5)));
		assert_eq!(pose.scale_for_bone("root"), Vec3::ONE);
		assert_eq!(pose.scale_for_bone("other"), Vec3::ONE);
	}

	fn pose_app() -> App {
		let mut app = App::new();
		app.add_systems(Update, maintain_bind_pose);
		app
	}

	fn spawn_scaled_rig(world: &mut World, scale: f32) -> (Entity, Entity) {
		let bone = world.spawn((Name::new("root"), Transform::from_scale(Vec3::ONE))).id();
		let rig = world
			.spawn((
				RigRoot::new(RigKey::named("body")).with_landmarks(&["root"]),
				BoneMap { by_name: HashMap::from([("root".into(), bone)]) },
				ActiveRigPose {
					pose: ResolvedRigPose::new().with_layer(
						RigPoseLayer::new("size").with_scale(BoneScale::uniform("root", scale)),
					),
				},
				BindPose::default(),
			))
			.id();
		(rig, bone)
	}

	#[test]
	fn first_apply_writes_layers_and_settled_frames_do_not_rewrite() {
		let mut app = pose_app();
		let (rig, bone) = spawn_scaled_rig(app.world_mut(), 2.0);
		app.update();
		assert!(app.world().get::<PoseApplied>(rig).is_some());
		assert_eq!(app.world().get::<Transform>(bone).unwrap().scale, Vec3::splat(2.0));

		app.world_mut().get_mut::<Transform>(bone).unwrap().scale = Vec3::splat(9.0);
		app.update();
		assert_eq!(app.world().get::<Transform>(bone).unwrap().scale, Vec3::splat(9.0));
	}

	#[test]
	fn changing_active_pose_reapplies() {
		let mut app = pose_app();
		let (rig, bone) = spawn_scaled_rig(app.world_mut(), 2.0);
		app.update();
		app.world_mut().get_mut::<ActiveRigPose>(rig).unwrap().pose = ResolvedRigPose::new()
			.with_layer(RigPoseLayer::new("size").with_scale(BoneScale::uniform("root", 3.0)));
		app.update();
		assert_eq!(app.world().get::<Transform>(bone).unwrap().scale, Vec3::splat(3.0));
	}
}
