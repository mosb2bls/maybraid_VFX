//! After walk/run, aim both arms at the held firearm's hand landmarks.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use character_rigs::articulation::{
	compose_parent_rotation, rotation_along_with_roll, TwoBoneAim, BONE_LENGTH_AXIS,
};
use character_rigs::authoring::humanoid_bone_axis;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;
use characters::{
	AnimBone, AnimMailbox, AnimateBones, BoneMap, CharacterMembers, CharacterRoot, SuspendAnimation,
};
use firearms::{FirearmMembers, FirearmRoot};

use crate::pose::HeldFirearm;
use crate::FirearmUser;

/// Body-rig marker: after locomotion, apply the firearm hold.
#[derive(Component, Clone, Copy, Default)]
pub struct HoldingArms;

/// Point the trigger hand at `trigger_point` and the support hand at the grip socket.
pub fn sync_hands_to_firearm(
	users: Query<&FirearmUser>,
	visuals: Query<(Entity, &CharacterMembers, &ChildOf), (With<CharacterRoot>, Without<AnimBone>)>,
	guns: Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>, Without<AnimBone>)>,
	gun_maps: Query<&BoneMap, Without<HoldingArms>>,
	mut rigs: Query<
		(&mut HumanoidV0Rig, &BoneMap, &AnimMailbox),
		(With<HoldingArms>, With<AnimateBones>, Without<SuspendAnimation>),
	>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
	)>,
) {
	for (visual, members, child_of) in &visuals {
		let Ok(user) = users.get(child_of.parent()) else {
			continue;
		};
		let settings = user.settings;
		let current = transforms.p0();
		let body_rot = current.compute_global_transform(visual).ok().map(|t| t.rotation());
		let trigger = gun_landmark(user.held, &guns, &gun_maps, &current, "trigger_point");
		let grip = gun_landmark(user.held, &guns, &gun_maps, &current, settings.grip_socket);
		drop(current);
		let Some(body_rot) = body_rot else {
			continue;
		};
		if trigger.is_none() && grip.is_none() {
			continue;
		}

		for member in members.iter() {
			let Ok((mut rig, map, mailbox)) = rigs.get_mut(member) else {
				continue;
			};
			if !mailbox.posed {
				continue;
			}
			let current = transforms.p0();
			let right_target = trigger.and_then(|trigger| {
				target_from(body_rot, bone_world(map, &current, "humerus.R"), trigger)
			});
			let left_target = grip.and_then(|grip| {
				target_from(body_rot, bone_world(map, &current, "humerus.L"), grip)
			});
			drop(current);
			let mut bones = transforms.p1();
			rig.pose.copy_from(&mailbox.output);
			pose_firing_torso(&mut rig, settings.firing_torso_yaw);
			let right = arm_reach(&rig, Side::Right, right_target, settings.right_pole, 1.0);
			let left = arm_reach(
				&rig,
				Side::Left,
				left_target,
				settings.left_pole,
				settings.left_reach_stretch,
			);
			if let Some(right) = right {
				reset_arm_to_rest(&mut rig, map, &bones, Side::Right);
				pose_arm(&mut rig, Side::Right, right, 1.0, settings.humerus_roll);
			}
			if let Some(left) = left {
				reset_arm_to_rest(&mut rig, map, &bones, Side::Left);
				pose_arm(
					&mut rig,
					Side::Left,
					left,
					settings.left_reach_stretch,
					settings.humerus_roll,
				);
			}
			write_hold_bones(&rig, map, &mut bones);
		}
	}
}

fn gun_landmark(
	held: Entity,
	guns: &Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>, Without<AnimBone>)>,
	maps: &Query<&BoneMap, Without<HoldingArms>>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<Vec3> {
	let members = guns.get(held).ok()?;
	named_translation(members.iter(), maps, transforms, name)
}

fn bone_world(map: &BoneMap, transforms: &TransformHelper, name: &str) -> Option<Vec3> {
	let entity = *map.by_name.get(name)?;
	transforms
		.compute_global_transform(entity)
		.ok()
		.map(|global| global.translation())
}

fn named_translation(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap, Without<HoldingArms>>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<Vec3> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = transforms.compute_global_transform(entity) {
			return Some(global.translation());
		}
	}
	None
}

fn target_from(body_rot: Quat, from: Option<Vec3>, to: Vec3) -> Option<Vec3> {
	let dir = to - from?;
	if dir.length_squared() < 1e-6 {
		return None;
	}
	Some(body_rot.inverse() * dir)
}

fn arm_reach(
	rig: &HumanoidV0Rig,
	side: Side,
	target: Option<Vec3>,
	pole: Vec3,
	stretch: f32,
) -> Option<TwoBoneAim> {
	let name = match side {
		Side::Left => "forearm.L",
		Side::Right => "forearm.R",
	};
	let id = rig.binding.definition.id(name)?;
	let upper_length = rig.binding.effective_rest.get(id)?.translation.length() * stretch;
	let target = target?;
	TwoBoneAim::reach(target, pole, upper_length, upper_length).or_else(|| {
		TwoBoneAim::reach(target, Vec3::X * side.sign() + Vec3::Z, upper_length, upper_length)
	})
}

fn pose_arm(
	rig: &mut HumanoidV0Rig,
	side: Side,
	reach: TwoBoneAim,
	stretch: f32,
	humerus_roll: f32,
) {
	let (humerus_name, forearm_name) = match side {
		Side::Left => ("humerus.L", "forearm.L"),
		Side::Right => ("humerus.R", "forearm.R"),
	};
	let Some(humerus) = rig.binding.definition.id(humerus_name) else {
		return;
	};
	let Some(forearm) = rig.binding.definition.id(forearm_name) else {
		return;
	};
	let roll = humerus_roll_for_reach(rig, side, reach, humerus_roll * side.sign());
	let rest_humerus = rig.binding.effective_rest.rotation(humerus);
	let parent = rig.binding.definition.parent_rotation(&rig.pose, humerus);
	let aimed = rotation_along_with_roll(
		rest_humerus,
		parent.inverse() * reach.upper_along,
		roll,
		BONE_LENGTH_AXIS,
	);
	rig.pose.set_rotation(humerus, aimed);
	scale_translation(rig, humerus, stretch);
	let flexed = compose_parent_rotation(
		rig.binding.effective_rest.rotation(forearm),
		humanoid_bone_axis(forearm_name),
		0.0,
		reach.flex,
		0.0,
	);
	rig.pose.set_rotation(forearm, flexed);
	scale_translation(rig, forearm, stretch);
}

fn scale_translation(
	rig: &mut HumanoidV0Rig,
	bone: character_rigs::authoring::BoneId,
	stretch: f32,
) {
	let rest = rig
		.binding
		.effective_rest
		.get(bone)
		.map(|t| t.translation)
		.unwrap_or(Vec3::ZERO);
	if let Some(slot) = rig.pose.local.get_mut(bone.index()) {
		slot.translation = rest * stretch;
	}
}

fn pose_firing_torso(rig: &mut HumanoidV0Rig, firing_torso_yaw: f32) {
	for (name, weight) in [("lumbar", 0.05), ("midback", 0.20), ("upper_back", 0.75)] {
		let Some(bone) = rig.binding.definition.id(name) else {
			continue;
		};
		let turned = Quat::from_rotation_y(firing_torso_yaw * weight) * rig.pose.rotation(bone);
		rig.pose.set_rotation(bone, turned);
	}
}

/// Solve the long-axis roll that rotates the authored forearm hinge into the
/// elbow plane selected by [`TwoBoneAim`].
fn humerus_roll_for_reach(
	rig: &HumanoidV0Rig,
	side: Side,
	reach: TwoBoneAim,
	fallback: f32,
) -> f32 {
	let Some((humerus, forearm)) = arm_ids(rig, side) else {
		return fallback;
	};
	let parent = rig.binding.definition.parent_rotation(&rig.pose, humerus);
	let aimed = rotation_along_with_roll(
		rig.binding.effective_rest.rotation(humerus),
		parent.inverse() * reach.upper_along,
		0.0,
		BONE_LENGTH_AXIS,
	);
	let humerus_world = parent * aimed;
	let forearm_rotation = compose_parent_rotation(
		rig.binding.effective_rest.rotation(forearm),
		humanoid_bone_axis(match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		}),
		0.0,
		reach.flex,
		0.0,
	);
	let zero_roll_lower = forearm_rotation * BONE_LENGTH_AXIS;
	let desired_lower = humerus_world.inverse() * reach.lower_along;
	signed_angle_about_axis(zero_roll_lower, desired_lower, BONE_LENGTH_AXIS).unwrap_or(fallback)
}

fn arm_ids(
	rig: &HumanoidV0Rig,
	side: Side,
) -> Option<(character_rigs::authoring::BoneId, character_rigs::authoring::BoneId)> {
	let (humerus_name, forearm_name) = match side {
		Side::Left => ("humerus.L", "forearm.L"),
		Side::Right => ("humerus.R", "forearm.R"),
	};
	Some((rig.binding.definition.id(humerus_name)?, rig.binding.definition.id(forearm_name)?))
}

#[cfg(test)]
fn lower_arm_direction(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneAim, roll: f32) -> Vec3 {
	let Some((humerus, forearm)) = arm_ids(rig, side) else {
		return Vec3::Z;
	};
	let parent = rig.binding.definition.parent_rotation(&rig.pose, humerus);
	let aimed = rotation_along_with_roll(
		rig.binding.effective_rest.rotation(humerus),
		parent.inverse() * reach.upper_along,
		roll,
		BONE_LENGTH_AXIS,
	);
	let forearm_rotation = compose_parent_rotation(
		rig.binding.effective_rest.rotation(forearm),
		humanoid_bone_axis(match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		}),
		0.0,
		reach.flex,
		0.0,
	);
	(parent * aimed * forearm_rotation * BONE_LENGTH_AXIS).normalize_or(Vec3::Z)
}

fn signed_angle_about_axis(from: Vec3, to: Vec3, axis: Vec3) -> Option<f32> {
	let axis = axis.try_normalize()?;
	let from = (from - axis * from.dot(axis)).try_normalize()?;
	let to = (to - axis * to.dot(axis)).try_normalize()?;
	Some(axis.dot(from.cross(to)).atan2(from.dot(to)))
}

fn reset_arm_to_rest(
	rig: &mut HumanoidV0Rig,
	map: &BoneMap,
	bones: &Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
	side: Side,
) {
	let suffix = side.suffix();
	for name in
		[format!("shoulder.{suffix}"), format!("humerus.{suffix}"), format!("forearm.{suffix}")]
	{
		let Some(&entity) = map.by_name.get(name.as_str()) else {
			continue;
		};
		let Ok((bone, _)) = bones.get(entity) else {
			continue;
		};
		let Some(id) = rig.binding.definition.id(&name) else {
			continue;
		};
		if let Some(slot) = rig.pose.local.get_mut(id.index()) {
			*slot = bone.rest;
		}
	}
}

fn write_hold_bones(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
) {
	for name in [
		"lumbar",
		"midback",
		"upper_back",
		"shoulder.L",
		"humerus.L",
		"forearm.L",
		"shoulder.R",
		"humerus.R",
		"forearm.R",
	] {
		write_bone(rig, map, bones, name);
	}
}

fn write_bone(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
	name: &str,
) {
	let Some(&entity) = map.by_name.get(name) else {
		return;
	};
	let Some(id) = rig.binding.definition.id(name) else {
		return;
	};
	let Some(pose) = rig.pose.get(id) else {
		return;
	};
	let Ok((_, mut transform)) = bones.get_mut(entity) else {
		return;
	};
	if *transform != pose {
		*transform = pose;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::FirearmUserSettings;
	use bevy::ecs::system::RunSystemOnce;

	fn settings() -> FirearmUserSettings {
		FirearmUserSettings::default()
	}

	#[test]
	fn sync_hands_queries_are_disjoint() -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		world.run_system_once(sync_hands_to_firearm)?;
		Ok(())
	}

	#[test]
	fn analytical_roll_points_forearm_toward_target() -> Result<(), &'static str> {
		let s = settings();
		let rig = HumanoidV0Rig::imported();
		let reach = TwoBoneAim::reach(Vec3::new(0.15, 0.0, 0.8), s.right_pole, 0.5, 0.5)
			.ok_or("missing reach")?;
		let roll = humerus_roll_for_reach(&rig, Side::Right, reach, -s.humerus_roll);
		let lower = lower_arm_direction(&rig, Side::Right, reach, roll);
		assert!(lower.dot(reach.lower_along) > 0.999, "{lower:?} vs {reach:?}");
		Ok(())
	}

	#[test]
	fn analytical_roll_handles_both_mirrored_arms() -> Result<(), &'static str> {
		let s = settings();
		let rig = HumanoidV0Rig::imported();
		for (side, target, pole, fallback) in [
			(Side::Left, Vec3::new(0.2, -0.15, 0.55), s.left_pole, s.humerus_roll),
			(Side::Right, Vec3::new(-0.2, -0.1, 0.7), s.right_pole, -s.humerus_roll),
		] {
			let reach = TwoBoneAim::reach(target, pole, 0.5, 0.5).ok_or("missing reach")?;
			let roll = humerus_roll_for_reach(&rig, side, reach, fallback);
			let lower = lower_arm_direction(&rig, side, reach, roll);
			assert!(lower.dot(reach.lower_along) > 0.999, "{side:?}: {lower:?} vs {reach:?}");
		}
		Ok(())
	}

	#[test]
	fn signed_roll_uses_fallback_for_a_straight_hinge() {
		assert!(signed_angle_about_axis(Vec3::Y, Vec3::Y, Vec3::Y).is_none());
	}

	#[test]
	fn right_elbow_pole_wings_out_and_down() -> Result<(), &'static str> {
		let reach = TwoBoneAim::reach(Vec3::Z * 0.7, settings().right_pole, 0.5, 0.5)
			.ok_or("missing reach")?;
		assert!(reach.upper_along.x < 0.0, "{reach:?}");
		assert!(reach.upper_along.y < 0.0, "{reach:?}");
		assert!((reach.upper_along.x.abs() - reach.upper_along.y.abs()).abs() < 1e-4, "{reach:?}");
		Ok(())
	}

	#[test]
	fn support_hand_targets_grip_socket() {
		let socket = settings().grip_socket;
		assert_eq!(socket, "grip");
		assert_ne!(socket, "grip_point");
	}

	#[test]
	fn support_arm_stretch_reaches_past_equal_segments() -> Result<(), &'static str> {
		let s = settings();
		let far = Vec3::Z * 1.1;
		let short = TwoBoneAim::reach(far, s.left_pole, 0.5, 0.5).ok_or("missing short reach")?;
		let stretched = TwoBoneAim::reach(
			far,
			s.left_pole,
			0.5 * s.left_reach_stretch,
			0.5 * s.left_reach_stretch,
		)
		.ok_or("missing stretched reach")?;
		assert!(stretched.flex > short.flex, "short {short:?} stretched {stretched:?}");
		Ok(())
	}

	#[test]
	fn left_humerus_swings_forward_to_a_close_grip() -> Result<(), &'static str> {
		// Grip sits in front of the left shoulder, inside rest length.
		let target = Vec3::new(0.2, -0.15, 0.55);
		let reach =
			TwoBoneAim::reach(target, settings().left_pole, 0.5, 0.5).ok_or("missing reach")?;
		assert!(
			reach.upper_along.z > 0.12,
			"humerus should still come forward, got {:?}",
			reach.upper_along
		);
		assert!(
			reach.upper_along.x > 0.15,
			"left elbow should wing out a bit (+X), got {:?}",
			reach.upper_along
		);
		assert!(reach.upper_along.y < -0.25, "elbow should tuck down, got {:?}", reach.upper_along);
		assert!(
			reach.upper_along.y > -0.7,
			"tuck should not hang the humerus on the ribs, got {:?}",
			reach.upper_along
		);
		Ok(())
	}

	#[test]
	fn pose_arm_flexes_the_t_pose_forearm_off_length_roll() -> Result<(), &'static str> {
		let s = settings();
		let mut rig = HumanoidV0Rig::for_clip_test();
		let reach = TwoBoneAim::reach(Vec3::new(0.2, -0.15, 0.55), s.left_pole, 0.5, 0.5)
			.ok_or("missing reach")?;
		pose_arm(&mut rig, Side::Left, reach, 1.0, s.humerus_roll);
		let along = rig.character_length("forearm.L");
		assert!(rig.posed_angle("forearm.L") > 0.1, "hold writes an elbow flex");
		assert!(along.y.abs() > 0.1, "flex changes the hinge, not only a length roll, {along:?}");
		Ok(())
	}

	#[test]
	fn pose_arm_aims_humerus_length_along_reach() -> Result<(), &'static str> {
		let s = settings();
		let mut rig = HumanoidV0Rig::imported();
		let reach = TwoBoneAim::reach(Vec3::new(0.2, -0.15, 0.55), s.left_pole, 0.5, 0.5)
			.ok_or("missing reach")?;
		pose_arm(&mut rig, Side::Left, reach, 1.0, s.humerus_roll);
		let along = (rig.rotation("humerus.L") * BONE_LENGTH_AXIS).normalize_or(Vec3::Y);
		assert!(
			along.dot(reach.upper_along) > 0.97,
			"expected aim along {:?}, got {along:?}",
			reach.upper_along
		);
		Ok(())
	}

	#[test]
	fn downward_pole_would_hang_the_left_humerus() -> Result<(), &'static str> {
		let target = Vec3::new(0.2, -0.15, 0.55);
		let hung = TwoBoneAim::reach(target, Vec3::new(-0.4, -1.0, 0.05), 0.5, 0.5)
			.ok_or("missing hung reach")?;
		assert!(
			hung.upper_along.y < -0.5,
			"old pole is the hang we are leaving, got {:?}",
			hung.upper_along
		);
		Ok(())
	}

	#[test]
	fn firing_torso_turns_right_shoulder_back() -> Result<(), &'static str> {
		let mut rig = HumanoidV0Rig::imported();
		pose_firing_torso(&mut rig, settings().firing_torso_yaw);
		let yaw = |name: &str| {
			let forward = rig.rotation(name) * Vec3::Z;
			forward.x.abs()
		};
		assert!(yaw("lumbar") > 0.0);
		assert!(yaw("midback") > yaw("lumbar"));
		assert!(yaw("upper_back") > yaw("midback"));
		Ok(())
	}

	#[test]
	fn failed_reach_keeps_idle_hang() -> anyhow::Result<()> {
		let mut world = hold_world()?;
		let right = world.resource::<HoldBones>().humerus_r;
		let hang = world.get::<Transform>(right).copied().expect("hang");
		let rest = world.get::<AnimBone>(right).expect("rest").rest;
		assert!(
			(hang.rotation.xyz() - rest.rotation.xyz()).length() > 0.2,
			"fixture should start hung, not rest"
		);

		world
			.run_system_once(sync_hands_to_firearm)
			.map_err(|err| anyhow::anyhow!("{err}"))?;

		let after = world.get::<Transform>(right).copied().expect("after");
		anyhow::ensure!(
			(after.rotation.xyz() - hang.rotation.xyz()).length() < 0.05,
			"failed reach must leave Idle hang, got {after:?} vs hang {hang:?}"
		);
		anyhow::ensure!(
			(after.rotation.xyz() - rest.rotation.xyz()).length() > 0.2,
			"failed reach must not reset to T-pose rest"
		);
		Ok(())
	}

	#[test]
	fn armed_ignore_holds_off_rest() -> anyhow::Result<()> {
		let mut world = hold_world_reachable()?;
		let right = world.resource::<HoldBones>().humerus_r;
		let rest = world.get::<AnimBone>(right).expect("rest").rest;

		world
			.run_system_once(crate::pose::pose_held_firearm)
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		sync_gun_global(&mut world);
		world
			.run_system_once(sync_hands_to_firearm)
			.map_err(|err| anyhow::anyhow!("{err}"))?;

		let after = world.get::<Transform>(right).copied().expect("after");
		anyhow::ensure!(
			(after.rotation.xyz() - rest.rotation.xyz()).length() > 0.15,
			"trigger hand should leave T-pose rest, got {after:?} vs rest {rest:?}"
		);
		let gun = world.resource::<HoldBones>().gun;
		let stock = world.resource::<HoldBones>().stock;
		let gun_tf = world.get::<Transform>(gun).copied().expect("gun");
		let stock_local = world.get::<Transform>(stock).expect("stock").translation;
		let stock_world = gun_tf.translation + gun_tf.rotation * stock_local;
		let shoulder = Vec3::new(0.2, 1.5, 0.0);
		anyhow::ensure!(
			(stock_world - shoulder).length() < 0.45,
			"stock should sit at the shoulder, {stock_world:?} vs {shoulder:?}"
		);
		Ok(())
	}

	#[derive(Resource)]
	struct HoldBones {
		humerus_r: Entity,
		gun: Entity,
		stock: Entity,
	}

	fn hang_transform(translation: Vec3) -> Transform {
		Transform { translation, rotation: Quat::from_rotation_x(1.2), ..default() }
	}

	fn rest_transform(translation: Vec3) -> Transform {
		Transform::from_translation(translation)
	}

	fn insert_arm_pose(rig: &mut HumanoidV0Rig, name: &str, transform: Transform, _flex: f32) {
		let Some(id) = rig.binding.definition.id(name) else {
			return;
		};
		if let Some(slot) = rig.pose.local.get_mut(id.index()) {
			*slot = transform;
		}
	}

	fn hold_world() -> anyhow::Result<World> {
		hold_scene(Vec3::new(0.2, 1.5, 0.0), Vec3::new(-0.2, 1.5, 0.0))
	}

	fn hold_world_reachable() -> anyhow::Result<World> {
		hold_scene(Vec3::new(0.2, 1.45, -0.4), Vec3::new(-0.15, 1.4, -0.35))
	}

	fn hold_scene(trigger_at: Vec3, grip_at: Vec3) -> anyhow::Result<World> {
		use std::collections::HashMap;

		use crate::{FirearmUser, FirearmUserSettings};
		use character_rigs::Name as RigName;
		use characters::{AnimMailbox, AnimateBones, CharacterHeading, CharacterRoot, MemberOf};
		use firearms::FirearmRoot;
		use player::PlayerLook;

		use crate::pose::HeldFirearm;

		let mut world = World::new();
		let mut rig = HumanoidV0Rig::imported();
		let hang_r = hang_transform(Vec3::new(0.2, 1.5, 0.0));
		let hang_l = hang_transform(Vec3::new(-0.2, 1.5, 0.0));
		let forearm = Transform::from_translation(Vec3::NEG_Y * 0.5);
		for (name, tf, flex) in [
			("shoulder.L", hang_transform(Vec3::new(-0.2, 1.55, 0.0)), 0.0),
			("shoulder.R", hang_transform(Vec3::new(0.2, 1.55, 0.0)), 0.0),
			("humerus.L", hang_l, 1.2),
			("humerus.R", hang_r, 1.2),
			("forearm.L", forearm, 0.32),
			("forearm.R", forearm, 0.32),
			("lumbar", Transform::IDENTITY, 0.0),
			("midback", Transform::IDENTITY, 0.0),
			("upper_back", Transform::IDENTITY, 0.0),
		] {
			insert_arm_pose(&mut rig, name, tf, flex);
		}
		let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
		mailbox.output = rig.pose.clone();
		mailbox.posed = true;

		let gun = world
			.spawn((
				FirearmRoot,
				HeldFirearm { scale: 1.0 },
				Transform::IDENTITY,
				GlobalTransform::IDENTITY,
			))
			.id();
		let user = world
			.spawn((
				FirearmUser { held: gun, settings: FirearmUserSettings::default() },
				PlayerLook::default(),
			))
			.id();
		let visual = world
			.spawn((
				CharacterRoot,
				CharacterHeading(Vec3::NEG_Z),
				Transform::IDENTITY,
				GlobalTransform::IDENTITY,
				ChildOf(user),
			))
			.id();

		let mut body_bones = HashMap::new();
		let mut humerus_r = Entity::PLACEHOLDER;
		for (name, translation) in [
			("shoulder.L", Vec3::new(-0.2, 1.55, 0.0)),
			("shoulder.R", Vec3::new(0.2, 1.55, 0.0)),
			("humerus.L", Vec3::new(-0.2, 1.5, 0.0)),
			("humerus.R", Vec3::new(0.2, 1.5, 0.0)),
			("forearm.L", Vec3::new(-0.2, 1.0, 0.0)),
			("forearm.R", Vec3::new(0.2, 1.0, 0.0)),
			("lumbar", Vec3::ZERO),
			("midback", Vec3::ZERO),
			("upper_back", Vec3::ZERO),
		] {
			let hang = if name.starts_with("humerus") || name.starts_with("shoulder") {
				hang_transform(translation)
			} else if name.starts_with("forearm") {
				Transform { translation, rotation: Quat::from_rotation_x(0.32), ..default() }
			} else {
				Transform::from_translation(translation)
			};
			let entity = world
				.spawn((
					AnimBone { name: RigName::from(name), rest: rest_transform(translation) },
					hang,
					GlobalTransform::from(hang),
				))
				.id();
			body_bones.insert(name.to_string(), entity);
			if name == "humerus.R" {
				humerus_r = entity;
			}
		}

		world.spawn((
			HoldingArms,
			AnimateBones,
			rig,
			mailbox,
			BoneMap { by_name: body_bones },
			ChildOf(visual),
			MemberOf(visual),
		));

		let stock = world
			.spawn((
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.2)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -0.2)),
			))
			.id();
		let trigger = world
			.spawn((
				Transform::from_translation(trigger_at),
				GlobalTransform::from_translation(trigger_at),
			))
			.id();
		let grip = world
			.spawn((
				Transform::from_translation(grip_at),
				GlobalTransform::from_translation(grip_at),
			))
			.id();
		let mut gun_bones = HashMap::new();
		gun_bones.insert("stock".to_string(), stock);
		gun_bones.insert("trigger_point".to_string(), trigger);
		gun_bones.insert("grip".to_string(), grip);
		world.spawn((BoneMap { by_name: gun_bones }, ChildOf(gun), MemberOf(gun)));
		world.flush();

		world.insert_resource(HoldBones { humerus_r, gun, stock });
		Ok(world)
	}

	fn sync_gun_global(world: &mut World) {
		let gun = world.resource::<HoldBones>().gun;
		let tf = world.get::<Transform>(gun).copied().expect("gun tf");
		if let Some(mut global) = world.get_mut::<GlobalTransform>(gun) {
			*global = GlobalTransform::from(tf);
		}
	}
}
