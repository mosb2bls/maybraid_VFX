//! Stock-at-shoulder pose and spawn helpers.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use characters::{
	BoneMap, CharacterHeading, CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot,
	RigSkeletonKind,
};
use firearms::{
	firearm_bounds, spawn_firearm_components, FireOnTrigger, FirearmConcept, FirearmMembers,
	FirearmRoot, ProjectileSource, WeaponTrigger,
};
use maybraid_equipment::EquippedItemHandler;
use player::{wrap_pi, yaw_xz, PlayerLook, PlayerUse};

use crate::hold::HoldingArms;
use crate::weapon::{LiveWeapon, RecoilPattern};
use crate::{FirearmUser, FirearmUserSettings};

#[derive(Component, Clone, Copy, Debug)]
pub struct HeldFirearm {
	pub scale: f32,
}

impl HeldFirearm {
	pub fn root_translation_for(&self, anchor: Vec3, rotation: Quat, socket_local: Vec3) -> Vec3 {
		anchor - rotation * (socket_local * self.scale)
	}
}

pub fn authored_length(bounds: Aabb3d) -> f32 {
	let size = bounds.max - bounds.min;
	size.x.max(size.y).max(size.z).max(1e-3)
}

pub fn held_scale_from_bounds(bounds: Aabb3d, held_length: f32) -> f32 {
	(held_length / authored_length(bounds)).clamp(0.15, 1.0)
}

pub fn spawn_held_firearm(commands: &mut Commands, user: Entity) -> Entity {
	spawn_held_firearm_with(commands, user, FirearmUserSettings::default())
}

pub fn spawn_held_firearm_with(
	commands: &mut Commands,
	user: Entity,
	settings: FirearmUserSettings,
) -> Entity {
	spawn_held_kit(commands, user, settings, FirearmConcept::Bullpup.kit(), LiveWeapon::default())
}

pub fn spawn_held_kit<T>(
	commands: &mut Commands,
	user: Entity,
	settings: FirearmUserSettings,
	kit: T,
	live: LiveWeapon,
) -> Entity
where
	T: firearms::FirearmComponents + Clone + Default + Send + Sync + 'static,
{
	let bounds = firearm_bounds(&kit);
	let scale = held_scale_from_bounds(bounds, settings.held_length);
	let entities = spawn_firearm_components(commands, &kit, Transform::IDENTITY, bounds);
	let mut root = Entity::PLACEHOLDER;
	for entity in entities {
		commands.entity(entity).insert((
			Name::new("held-firearm"),
			live.weapon,
			live.payload,
			live.fire,
			live.recoil,
			RecoilPattern::from_seed(live.recoil_seed),
			live,
			FireOnTrigger,
			WeaponTrigger(false),
			ProjectileSource(user),
			HeldFirearm { scale },
		));
		root = entity;
	}
	let mut entity = commands.entity(user);
	entity.insert((FirearmUser { held: root, settings }, PlayerUse { driver: root }));
	EquippedItemHandler::Firearm.install(&mut entity);
	root
}

pub fn stamp_holding_arms(
	mut commands: Commands,
	users: Query<(), With<FirearmUser>>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<(Entity, &CharacterRig), Without<HoldingArms>>,
) {
	for (members, child_of) in &visuals {
		if users.get(child_of.parent()).is_err() {
			continue;
		}
		for member in members.iter() {
			let Ok((entity, rig)) = rigs.get(member) else {
				continue;
			};
			if rig.role == CharacterRigRole::Body && rig.skeleton == RigSkeletonKind::Humanoid {
				commands.entity(entity).insert(HoldingArms);
			}
		}
	}
}

pub(crate) fn clamped_aim_yaw(facing: Vec3, look: Vec3, aim_yaw_limit: f32) -> f32 {
	let face = yaw_xz(facing);
	let delta = wrap_pi(yaw_xz(look) - face).clamp(-aim_yaw_limit, aim_yaw_limit);
	face + delta
}

pub(crate) fn gun_aim_rotation_for(
	facing: Vec3,
	look: Vec3,
	pitch: f32,
	track_look: bool,
	aim_yaw_limit: f32,
) -> Quat {
	let yaw = if track_look { yaw_xz(look) } else { clamped_aim_yaw(facing, look, aim_yaw_limit) };
	// PlayerLook's positive pitch raises the camera's -Z view. The firearm's
	// bore is +Z, so it needs the opposite local X rotation to rise with it.
	Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-pitch)
}

fn right_shoulder_anchor(
	left_origin: Vec3,
	right_origin: Vec3,
	facing: Vec3,
	arm_length: f32,
	settings: &FirearmUserSettings,
) -> Vec3 {
	let mid = (left_origin + right_origin) * 0.5;
	let right = (right_origin - left_origin).normalize_or(Vec3::X);
	let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z);
	let half_width = left_origin.distance(right_origin) * 0.5;
	mid + right * (half_width * settings.stock_along_right_chest)
		+ forward * (arm_length * settings.stock_forward_of_arm_reach)
}

pub fn pose_held_firearm(
	users: Query<(&FirearmUser, &PlayerLook)>,
	visuals: Query<
		(&Transform, &CharacterHeading, &CharacterMembers, &ChildOf),
		(With<CharacterRoot>, Without<HeldFirearm>, Without<characters::AnimBone>),
	>,
	maps: Query<&BoneMap, Without<HeldFirearm>>,
	gun_members: Query<
		(&FirearmMembers, &HeldFirearm),
		(With<FirearmRoot>, With<HeldFirearm>, Without<CharacterRoot>),
	>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<&mut Transform, (With<FirearmRoot>, With<HeldFirearm>, Without<CharacterRoot>)>,
	)>,
) {
	for (visual, heading, members, child_of) in &visuals {
		let Ok((user, look)) = users.get(child_of.parent()) else {
			continue;
		};
		let Ok((gun_members, held)) = gun_members.get(user.held) else {
			continue;
		};
		let current = transforms.p0();
		let Some((right_origin, right_length)) = arm_measure(members, &maps, &current, "R") else {
			continue;
		};
		let Some((left_origin, _left_length)) = arm_measure(members, &maps, &current, "L") else {
			continue;
		};
		let Some(stock_local) =
			firearm_landmark_local(user.held, gun_members, &maps, &current, "stock")
		else {
			continue;
		};
		drop(current);
		let facing = heading.facing(visual);
		let look_dir = Quat::from_axis_angle(Vec3::Y, look.yaw) * -Vec3::Z;
		let rotation = gun_aim_rotation_for(
			facing,
			look_dir,
			look.pitch,
			look.first_person,
			user.settings.aim_yaw_limit,
		);
		let anchor =
			right_shoulder_anchor(left_origin, right_origin, facing, right_length, &user.settings);
		let translation = held.root_translation_for(anchor, rotation, stock_local);
		let mut guns = transforms.p1();
		let Ok(mut transform) = guns.get_mut(user.held) else {
			continue;
		};
		*transform = Transform { translation, rotation, scale: Vec3::splat(held.scale) };
	}
}

fn arm_measure(
	members: &CharacterMembers,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	transforms: &TransformHelper,
	suffix: &str,
) -> Option<(Vec3, f32)> {
	let humerus = named_translation(members, maps, transforms, &format!("humerus.{suffix}"))?;
	let forearm = named_translation(members, maps, transforms, &format!("forearm.{suffix}"))?;
	Some((humerus, humerus.distance(forearm) * 2.0))
}

fn firearm_landmark_local(
	root: Entity,
	members: &FirearmMembers,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<Vec3> {
	let root = transforms.compute_global_transform(root).ok()?;
	let world = named_translation_from(members.iter(), maps, transforms, name)?;
	Some(root.affine().inverse().transform_point3(world))
}

fn named_translation(
	members: &CharacterMembers,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<Vec3> {
	named_translation_from(members.iter(), maps, transforms, name)
}

fn named_translation_from(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
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

#[cfg(test)]
mod tests {
	use super::*;

	fn settings() -> FirearmUserSettings {
		FirearmUserSettings::default()
	}

	#[test]
	fn facing_plus_x_sends_bore_plus_x() {
		let q = gun_aim_rotation_for(Vec3::X, Vec3::X, 0.0, false, settings().aim_yaw_limit);
		assert!((q * Vec3::Z - Vec3::X).length() < 1e-4, "bore {}", q * Vec3::Z);
	}

	#[test]
	fn look_beyond_limit_clamps() {
		let limit = settings().aim_yaw_limit;
		let yaw = clamped_aim_yaw(Vec3::Z, Vec3::X, limit);
		assert!((yaw - limit).abs() < 1e-4, "{yaw}");
	}

	#[test]
	fn first_person_gun_tracks_look_past_body_cone() {
		let look = Vec3::X;
		let q = gun_aim_rotation_for(Vec3::Z, look, 0.0, true, settings().aim_yaw_limit);
		assert!((q * Vec3::Z - look).length() < 1e-4, "bore {}", q * Vec3::Z);
	}

	#[test]
	fn terrain_tilt_does_not_replace_explicit_heading() {
		let visual = Transform::from_rotation(
			Quat::from_rotation_y(0.8) * Quat::from_rotation_x(-0.55) * Quat::from_rotation_z(0.35),
		);
		let mut heading = CharacterHeading(Vec3::X);
		let facing = heading.resolve(&visual);
		let q = gun_aim_rotation_for(facing, Vec3::X, 0.0, false, settings().aim_yaw_limit);
		assert!((facing - Vec3::X).length() < 1e-5);
		assert!((q * Vec3::Z - Vec3::X).length() < 1e-4);
	}

	#[test]
	fn positive_player_pitch_sends_bore_up() {
		let q = gun_aim_rotation_for(Vec3::NEG_Z, Vec3::NEG_Z, 0.25, true, std::f32::consts::PI);
		let bore = q * Vec3::Z;
		assert!(bore.y > 0.2, "{bore}");
		assert!(bore.z < -0.9, "{bore}");
	}

	#[test]
	fn negative_player_pitch_sends_bore_down() {
		let q = gun_aim_rotation_for(Vec3::NEG_Z, Vec3::NEG_Z, -0.25, true, std::f32::consts::PI);
		let bore = q * Vec3::Z;
		assert!(bore.y < -0.2, "{bore}");
		assert!(bore.z < -0.9, "{bore}");
	}

	#[test]
	fn root_translation_pins_stock_socket() {
		let held = HeldFirearm { scale: 0.25 };
		let anchor = Vec3::new(0.3, 1.7, 0.2);
		let rotation = Quat::from_euler(EulerRot::YXZ, 0.4, -0.2, 0.0);
		let socket = Vec3::new(-0.1, 0.0, -0.5);
		let root = held.root_translation_for(anchor, rotation, socket);
		let pinned = root + rotation * (socket * held.scale);
		assert!((pinned - anchor).length() < 1e-5, "{pinned} vs {anchor}");
	}

	#[test]
	fn held_scale_shrinks_meter_kit() {
		let bounds = Aabb3d::from_min_max(Vec3::new(-0.5, -0.5, -2.2), Vec3::new(0.5, 1.4, 0.5));
		let held_length = settings().held_length;
		let scale = held_scale_from_bounds(bounds, held_length);
		assert!(scale < 0.5, "{scale}");
		assert!((scale * 2.7 - held_length).abs() < 0.05, "{scale}");
	}
}
