//! Write [`PlayerCameraAim`] from the held firearm's sight socket.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use characters::BoneMap;
use firearms::{FirearmMembers, FirearmRoot};
use player::{PlayerCameraAim, PlayerCameraPose, PlayerLook};

use crate::pose::HeldFirearm;
use crate::weapon::LiveWeapon;
use crate::FirearmUser;

pub(crate) fn write_sight_aim(
	mut users: Query<(&FirearmUser, &PlayerLook, &mut PlayerCameraAim)>,
	guns: Query<(&FirearmMembers, Option<&LiveWeapon>), (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: Query<&BoneMap>,
	transforms: TransformHelper,
) {
	for (user, look, mut aim) in &mut users {
		if !look.first_person {
			aim.pose = None;
			aim.focus = 0.0;
			aim.sight_fov = None;
			continue;
		}
		aim.focus = look.focus;
		aim.sight_fov =
			guns.get(user.held).ok().and_then(|(_, live)| live.map(|live| live.sight_fov));
		aim.pose = sight_camera_pose(
			user.held,
			user.settings.sight_camera_back,
			&guns,
			&maps,
			&transforms,
		);
	}
}

fn sight_camera_pose(
	held: Entity,
	sight_camera_back: f32,
	guns: &Query<(&FirearmMembers, Option<&LiveWeapon>), (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
) -> Option<PlayerCameraPose> {
	let (members, _) = guns.get(held).ok()?;
	let socket = member_landmark_global(members.iter(), maps, transforms, "sight_camera_socket")?;
	Some(sight_camera_pose_from_socket(socket, sight_camera_back))
}

fn sight_camera_pose_from_socket(
	socket: GlobalTransform,
	sight_camera_back: f32,
) -> PlayerCameraPose {
	let look = (socket.rotation() * Vec3::Z).normalize_or(Vec3::Z);
	let mut aimed = Transform::from_translation(socket.translation() - look * sight_camera_back);
	aimed.look_to(look, Vec3::Y);
	PlayerCameraPose { translation: aimed.translation, rotation: aimed.rotation }
}

fn member_landmark_global(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<GlobalTransform> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = transforms.compute_global_transform(entity) {
			return Some(global);
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::FirearmUserSettings;

	#[test]
	fn sight_camera_sits_behind_socket() {
		let back = FirearmUserSettings::default().sight_camera_back;
		let socket = GlobalTransform::from(
			Transform::from_translation(Vec3::new(1.0, 1.5, 0.0))
				.with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
		);
		let camera = sight_camera_pose_from_socket(socket, back);
		assert!((camera.translation - Vec3::new(1.0 - back, 1.5, 0.0)).length() < 1e-4);
		assert!((camera.rotation * -Vec3::Z - Vec3::X).length() < 1e-4);
	}
}
