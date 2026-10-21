//! Hand-held pose and throw-arm overlay.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use character_rigs::arm_reach::{ArmReachPole, BodyReachSpace, OverhandThrow};
use character_rigs::articulation::{
	compose_parent_rotation, rotation_along_with_roll, TwoBoneAim, BONE_LENGTH_AXIS,
};
use character_rigs::authoring::{humanoid_bone_axis, humanoid_write_mask};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;
use characters::{
	mailbox_apply_allows, AnimBone, AnimMailbox, AnimateBones, BoneMap, CharacterHeading,
	CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot, MailboxApplySet,
	RigSkeletonKind, SuspendAnimation,
};
use firearm_user::{HoldingArms, WeaponSwap};
use grenades::{grenade_material, grenade_mesh, GrenadeMaterial};
use maybraid_equipment::EquippedItemHandler;
use player::{look_forward, yaw_xz, PlayerLook, PlayerUse};
use std::f32::consts::FRAC_PI_2;

use crate::throw::{GrenadePhase, GrenadeThrow, GrenadeUser, GrenadeUserSettings};

/// Preferred grip points. A later `hand_socket.R` on the body rig is picked first.
pub const RIGHT_HAND_SOCKETS: &[&str] = &["hand_socket.R", "hand.R", "palm.R"];

const HELD_SCALE: f32 = 0.2;

#[derive(Component, Clone, Copy, Debug)]
pub struct HeldGrenade {
	pub scale: f32,
}

#[derive(Component, Debug)]
pub struct HoldingGrenade;

impl HoldingGrenade {
	/// Right-arm throw overlay. Subset of the firearm hold mask; grenade runs
	/// later in [`characters::CharacterPoseOverlay`] so it wins those bones.
	pub const AFFECTED_BONES: &'static [&'static str] = &["shoulder.R", "humerus.R", "forearm.R"];

	pub fn write_mask() -> u32 {
		humanoid_write_mask(Self::AFFECTED_BONES)
	}
}

pub fn spawn_held_grenade(
	commands: &mut Commands,
	user: Entity,
	settings: GrenadeUserSettings,
) -> Entity {
	let held = commands
		.spawn((
			bevy::prelude::Name::new("held-grenade"),
			Transform::from_scale(Vec3::splat(HELD_SCALE)),
			Visibility::Visible,
			HeldGrenade { scale: HELD_SCALE },
		))
		.id();
	let mut entity = commands.entity(user);
	entity.insert((
		GrenadeUser { held, settings },
		GrenadeThrow::default(),
		PlayerUse { driver: held },
	));
	EquippedItemHandler::Grenade.install(&mut entity);
	held
}

pub fn realize_held_grenade_visuals(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<GrenadeMaterial>>,
	held: Query<Entity, (With<HeldGrenade>, Without<Mesh3d>)>,
) {
	for entity in &held {
		commands.entity(entity).try_insert((
			Mesh3d(meshes.add(grenade_mesh())),
			MeshMaterial3d(materials.add(grenade_material())),
		));
	}
}

pub fn stamp_holding_grenade(
	mut commands: Commands,
	users: Query<&GrenadeThrow, With<GrenadeUser>>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<(Entity, &CharacterRig, Has<HoldingGrenade>, Has<HoldingArms>)>,
) {
	for (members, child_of) in &visuals {
		let overlay = users.get(child_of.parent()).is_ok();
		for member in members.iter() {
			let Ok((entity, rig, holding, armed)) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body || rig.skeleton != RigSkeletonKind::Humanoid {
				continue;
			}
			if overlay && !holding {
				commands.entity(entity).try_insert(HoldingGrenade);
			}
			if overlay && armed {
				commands.entity(entity).remove::<HoldingArms>();
			}
			if !overlay && holding {
				commands.entity(entity).remove::<HoldingGrenade>();
			}
		}
	}
}

pub fn pose_held_grenade(
	users: Query<(&GrenadeUser, &GrenadeThrow, &PlayerLook)>,
	visuals: Query<
		(&Transform, &CharacterHeading, &ChildOf),
		(With<CharacterRoot>, Without<HeldGrenade>, Without<AnimBone>),
	>,
	mut grenades: Query<&mut Transform, (With<HeldGrenade>, Without<CharacterRoot>)>,
) {
	for (visual, heading, child_of) in &visuals {
		let Ok((user, throw, look)) = users.get(child_of.parent()) else {
			continue;
		};
		if throw.busy() {
			continue;
		}
		let forward = primed_forward(heading.0, look);
		let right = Vec3::Y.cross(forward).normalize_or(Vec3::X);
		let Ok(mut transform) = grenades.get_mut(user.held) else {
			continue;
		};
		*transform = Transform {
			translation: visual.translation
				+ Vec3::Y * (1.28 + user.settings.hold_up)
				+ right * (0.08 + user.settings.hold_right)
				+ forward * (0.44 + user.settings.hold_forward),
			rotation: Quat::from_rotation_y(yaw_xz(forward)),
			scale: Vec3::splat(HELD_SCALE),
		};
	}
}

pub fn apply_grenade_swap_pose(
	users: Query<(&GrenadeUser, &WeaponSwap)>,
	mut grenades: Query<&mut Transform, With<HeldGrenade>>,
) {
	for (user, swap) in &users {
		if let Ok(mut transform) = grenades.get_mut(user.held) {
			swap.apply_to(&mut transform);
		}
	}
}

pub fn sync_throw_arm(
	set: Option<Res<MailboxApplySet>>,
	users: Query<(&GrenadeUser, &GrenadeThrow, Has<WeaponSwap>)>,
	carriers: Query<&InventoryUser>,
	bags: Query<&Inventory>,
	visuals: Query<
		(Entity, &CharacterHeading, &CharacterMembers, &ChildOf),
		(With<CharacterRoot>, Without<AnimBone>),
	>,
	mut rigs: Query<
		(Entity, &mut HumanoidV0Rig, &BoneMap, &AnimMailbox),
		(With<HoldingGrenade>, With<AnimateBones>, Without<SuspendAnimation>),
	>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<
			(&AnimBone, &mut Transform),
			(Without<AnimMailbox>, Without<CharacterRoot>, Without<HeldGrenade>),
		>,
		Query<&mut Transform, (With<HeldGrenade>, Without<CharacterRoot>)>,
	)>,
) {
	for (visual, heading, members, child_of) in &visuals {
		let Ok((user, throw, swapping)) = users.get(child_of.parent()) else {
			continue;
		};
		if swapping {
			continue;
		}
		let throw_secs = carriers
			.get(child_of.parent())
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
			.map(|stats| stats.throw_secs)
			.unwrap_or_else(|| GrenadeStats::standard().throw_secs);
		let t = throw_phase_t(throw.phase, throw_secs);
		let _ = visual;
		let target = OverhandThrow::reach_at(t);
		let pole = OverhandThrow::pole();
		for member in members.iter() {
			if !mailbox_apply_allows(set.as_deref(), member) {
				continue;
			}
			let Ok((_entity, mut rig, map, mailbox)) = rigs.get_mut(member) else {
				continue;
			};
			if !mailbox.posed {
				continue;
			}
			let mut bones = transforms.p1();
			if let Some(reach) = throw_arm_reach(&rig, Side::Right, target, pole) {
				reset_arm_to_rest(&mut rig, map, &bones, Side::Right);
				pose_throw_arm(&mut rig, Side::Right, reach);
			}
			write_throw_bones(&rig, map, &mut bones);
			drop(bones);
			let helper = transforms.p0();
			let hand = named_hand(map, &helper).or_else(|| distal_forearm(map, &helper));
			drop(helper);
			if let Some(hand) = hand {
				let forward = Vec3::new(heading.0.x, 0.0, heading.0.z).normalize_or(Vec3::Z);
				let mut grenades = transforms.p2();
				if let Ok(mut transform) = grenades.get_mut(user.held) {
					*transform = Transform {
						translation: hand + forward * 0.06 + Vec3::Y * 0.02,
						rotation: Quat::from_rotation_y(yaw_xz(heading.0)),
						scale: Vec3::splat(HELD_SCALE),
					};
				}
			}
		}
	}
}

pub fn sync_held_visibility(
	carriers: Query<&InventoryUser>,
	bags: Query<&Inventory>,
	users: Query<(Entity, &GrenadeUser, &GrenadeThrow)>,
	mut held: Query<&mut Visibility, With<HeldGrenade>>,
) {
	for (entity, user, throw) in &users {
		let Ok(mut visibility) = held.get_mut(user.held) else {
			continue;
		};
		let ready = carriers
			.get(entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(Inventory::primary_weapon)
			.is_some_and(|item| match item {
				InventoryItem::Grenade { recharge, .. } => recharge.ready(),
				_ => false,
			});
		*visibility = if ready && !matches!(throw.phase, GrenadePhase::Recovery { .. }) {
			Visibility::Visible
		} else {
			Visibility::Hidden
		};
	}
}

fn throw_phase_t(phase: GrenadePhase, throw_secs: f32) -> f32 {
	match phase {
		GrenadePhase::Ready => 0.0,
		GrenadePhase::Cooking { age } => {
			// Hold the authored cock keyframe (0.45) while the pin is pulled.
			(age / throw_secs.max(1e-3)).min(0.45).clamp(0.0, 1.0)
		}
		GrenadePhase::Windup { age } | GrenadePhase::Recovery { age } => {
			(age / throw_secs.max(1e-3)).clamp(0.0, 1.0)
		}
	}
}

fn primed_forward(facing: Vec3, look: &PlayerLook) -> Vec3 {
	if look.first_person {
		let aim = look_forward(look);
		return Vec3::new(aim.x, 0.0, aim.z).normalize_or(-Vec3::Z);
	}
	Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z)
}

fn throw_arm_reach(
	rig: &HumanoidV0Rig,
	side: Side,
	target: Vec3,
	pole: ArmReachPole,
) -> Option<TwoBoneAim> {
	let target = BodyReachSpace::to_shoulder(side, target);
	let primary = BodyReachSpace::to_shoulder(side, pole.primary);
	let fallback = BodyReachSpace::to_shoulder(side, pole.fallback);
	let forearm_name = match side {
		Side::Left => "forearm.L",
		Side::Right => "forearm.R",
	};
	let id = rig.binding.definition.id(forearm_name)?;
	let length = rig.binding.effective_rest.get(id)?.translation.length();
	TwoBoneAim::reach(target, primary, length, length)
		.or_else(|| TwoBoneAim::reach(target, fallback, length, length))
}

fn pose_throw_arm(rig: &mut HumanoidV0Rig, side: Side, reach: TwoBoneAim) {
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
	let roll = humerus_roll_for_throw(rig, side, reach);
	let rest_humerus = rig.binding.effective_rest.rotation(humerus);
	let parent = rig.binding.definition.parent_rotation(&rig.pose, humerus);
	let aimed = rotation_along_with_roll(
		rest_humerus,
		parent.inverse() * reach.upper_along,
		roll,
		BONE_LENGTH_AXIS,
	);
	rig.pose.set_rotation(humerus, aimed);
	let flexed = compose_parent_rotation(
		rig.binding.effective_rest.rotation(forearm),
		humanoid_bone_axis(forearm_name),
		0.0,
		reach.flex,
		0.0,
	);
	rig.pose.set_rotation(forearm, flexed);
}

fn humerus_roll_for_throw(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneAim) -> f32 {
	let (humerus_name, forearm_name) = match side {
		Side::Left => ("humerus.L", "forearm.L"),
		Side::Right => ("humerus.R", "forearm.R"),
	};
	let Some(humerus) = rig.binding.definition.id(humerus_name) else {
		return 0.0;
	};
	let Some(forearm) = rig.binding.definition.id(forearm_name) else {
		return 0.0;
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
		humanoid_bone_axis(forearm_name),
		0.0,
		reach.flex,
		0.0,
	);
	let zero_roll_lower = forearm_rotation * BONE_LENGTH_AXIS;
	let desired_lower = humerus_world.inverse() * reach.lower_along;
	signed_angle_about_axis(zero_roll_lower, desired_lower, BONE_LENGTH_AXIS)
		.unwrap_or(-FRAC_PI_2 * side.sign())
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
	bones: &Query<
		(&AnimBone, &mut Transform),
		(Without<AnimMailbox>, Without<CharacterRoot>, Without<HeldGrenade>),
	>,
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

fn named_hand(map: &BoneMap, helper: &TransformHelper) -> Option<Vec3> {
	for name in RIGHT_HAND_SOCKETS {
		if let Some(&entity) = map.by_name.get(*name) {
			if let Ok(global) = helper.compute_global_transform(entity) {
				return Some(global.translation());
			}
		}
	}
	None
}

fn distal_forearm(map: &BoneMap, helper: &TransformHelper) -> Option<Vec3> {
	let &forearm = map.by_name.get("forearm.R")?;
	let global = helper.compute_global_transform(forearm).ok()?;
	let length = map
		.by_name
		.get("humerus.R")
		.and_then(|&humerus| helper.compute_global_transform(humerus).ok())
		.map(|humerus| humerus.translation().distance(global.translation()))
		.unwrap_or(0.28);
	Some(global.translation() + global.rotation() * (BONE_LENGTH_AXIS * length))
}

fn write_throw_bones(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<
		(&AnimBone, &mut Transform),
		(Without<AnimMailbox>, Without<CharacterRoot>, Without<HeldGrenade>),
	>,
) {
	for &name in HoldingGrenade::AFFECTED_BONES {
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		let Some(id) = rig.binding.definition.id(name) else {
			continue;
		};
		let Some(pose) = rig.pose.get(id) else {
			continue;
		};
		let Ok((_, mut transform)) = bones.get_mut(entity) else {
			continue;
		};
		*transform = pose;
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use bevy::ecs::system::RunSystemOnce;

	use super::*;

	#[test]
	fn pose_queries_do_not_alias_transform() -> anyhow::Result<()> {
		let mut world = World::new();
		world
			.run_system_once(pose_held_grenade)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn throw_arm_queries_do_not_alias_transform() -> anyhow::Result<()> {
		let mut world = World::new();
		world
			.run_system_once(sync_throw_arm)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn sync_throw_arm_honors_mailbox_apply_set() -> anyhow::Result<()> {
		use std::collections::HashSet;

		let mut world = World::new();
		world.init_resource::<MailboxApplySet>();
		let mut rig = HumanoidV0Rig::imported();
		let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
		mailbox.posed = true;
		mailbox.output = rig.pose.clone();
		let body = world
			.spawn((
				HoldingGrenade,
				AnimateBones,
				rig,
				mailbox,
				BoneMap { by_name: HashMap::new() },
			))
			.id();
		world.resource_mut::<MailboxApplySet>().only = Some(HashSet::new());
		anyhow::ensure!(
			!mailbox_apply_allows(world.get_resource::<MailboxApplySet>(), body),
			"grenade overlay must skip when host is not in MailboxApplySet"
		);
		let mut selected = HashSet::new();
		selected.insert(body);
		world.resource_mut::<MailboxApplySet>().only = Some(selected);
		anyhow::ensure!(
			mailbox_apply_allows(world.get_resource::<MailboxApplySet>(), body),
			"grenade overlay must run when host is selected"
		);
		Ok(())
	}

	#[test]
	fn throw_mask_is_right_arm_subset_of_firearm_hold() {
		use firearm_user::HoldingArms;

		let throw = HoldingGrenade::write_mask();
		let hold = HoldingArms::write_mask();
		assert_ne!(throw, 0);
		assert_eq!(throw, humanoid_write_mask(HoldingGrenade::AFFECTED_BONES));
		assert_eq!(throw & !hold, 0);
	}

	#[test]
	fn throw_from_working_pose_matches_legacy_copy_from_base() -> anyhow::Result<()> {
		let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
		let mut rig = HumanoidV0Rig::imported();
		mailbox.output = rig.pose.clone();
		mailbox.posed = true;
		let mut legacy = rig.clone();
		legacy.pose.copy_from(mailbox.base_pose());
		if let Some(id) = legacy.binding.definition.id("humerus.R") {
			if let Some(slot) = legacy.pose.local.get_mut(id.index()) {
				slot.rotation = Quat::from_rotation_x(0.4);
			}
		}
		mailbox.init_working_pose(&mut rig.pose);
		if let Some(id) = rig.binding.definition.id("humerus.R") {
			if let Some(slot) = rig.pose.local.get_mut(id.index()) {
				slot.rotation = Quat::from_rotation_x(0.4);
			}
		}
		anyhow::ensure!(legacy.pose.local == rig.pose.local);
		anyhow::ensure!(mailbox.output.local != rig.pose.local);
		Ok(())
	}

	#[test]
	fn throw_phase_uses_full_throw_window() -> anyhow::Result<()> {
		let stats = GrenadeStats::standard();
		assert!(
			(throw_phase_t(GrenadePhase::Windup { age: stats.release_at }, stats.throw_secs)
				- stats.release_at / stats.throw_secs)
				.abs() < 1e-4
		);
		assert_eq!(
			throw_phase_t(GrenadePhase::Recovery { age: stats.throw_secs }, stats.throw_secs),
			1.0
		);
		assert_eq!(
			throw_phase_t(GrenadePhase::Cooking { age: stats.fuse }, stats.throw_secs),
			0.45
		);
		Ok(())
	}
}
