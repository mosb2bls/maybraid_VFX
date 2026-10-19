//! Default follow + optional [`PlayerCameraAim`] blend.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use characters::LocomotionCapsule;
use characters::{hide_socketed_parts, BoneMap, CharacterMembers, CharacterPartSlot, PartNode};
use player::{
	CameraFollow, PlayerCameraAim, PlayerCameraPose, PlayerLook, PlayerVisual,
	RestLocomotionCapsule,
};

use crate::look::{CameraController, CameraPov};
use crate::FollowCamera;

pub(crate) fn follow_character_camera(
	time: Res<Time>,
	players: Query<
		(
			Entity,
			&PlayerCameraAim,
			&PlayerLook,
			Option<&LocomotionCapsule>,
			Option<&RestLocomotionCapsule>,
		),
		(With<CameraFollow>, Without<Camera3d>),
	>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	maps: Query<&BoneMap>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<(&mut Transform, &mut CameraController, &FollowCamera), With<Camera3d>>,
	)>,
) {
	{
		let mut cameras = transforms.p1();
		let Ok((mut camera_transform, controller, _)) = cameras.single_mut() else {
			return;
		};
		if controller.pov.is_map() {
			*camera_transform =
				map_pose(controller.map_focus, controller.map_ground_y, controller.map_height)
					.transform();
			return;
		}
	}

	let Ok((player, aim, look, live_hull, rest_hull)) = players.single() else {
		return;
	};
	let height_scale = stance_height_scale(rest_hull, live_hull);
	let current = transforms.p0();
	let Ok(player) = current.compute_global_transform(player) else {
		return;
	};
	let head = head_camera_translation(&visuals, &maps, &current);
	drop(current);
	let mut cameras = transforms.p1();
	let Ok((mut camera_transform, mut controller, follow)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let look_rotation = yaw * pitch;

	let focus_target =
		if look.first_person { aim.focus.max(controller.focus).max(controller.ads) } else { 0.0 };
	controller.focus_blend = focus_blend_toward(
		controller.focus_blend,
		focus_target,
		follow.focus_blend_speed,
		time.delta_secs(),
	);

	let mut pose = match controller.pov {
		CameraPov::ThirdPerson => {
			third_person_pose(player.translation(), yaw, look_rotation, follow, height_scale)
		}
		CameraPov::FirstPerson => {
			first_person_pose(player.translation(), head, look_rotation, follow, height_scale)
		}
		CameraPov::Map => {
			map_pose(controller.map_focus, controller.map_ground_y, controller.map_height)
		}
	};
	if controller.pov.is_first_person() {
		if let Some(sight) = aim.pose {
			pose = pose.interpolate(sight, controller.focus_blend);
		}
	}
	*camera_transform = pose.transform();
}

fn stance_height_scale(
	rest: Option<&RestLocomotionCapsule>,
	live: Option<&LocomotionCapsule>,
) -> f32 {
	let Some(rest) = rest else {
		return 1.0;
	};
	let Some(live) = live else {
		return 1.0;
	};
	(live.half_height() / rest.0.half_height().max(1e-4)).clamp(0.15, 1.0)
}

fn third_person_pose(
	player: Vec3,
	yaw: Quat,
	look_rotation: Quat,
	follow: &FollowCamera,
	height_scale: f32,
) -> PlayerCameraPose {
	let offset = look_rotation * Vec3::new(0.0, 0.0, follow.distance)
		+ Vec3::Y * follow.height * height_scale;
	let target = player
		+ Vec3::Y * follow.look_height * height_scale
		+ yaw * Vec3::X * follow.shoulder_offset;
	let mut pose = Transform::from_translation(target + offset);
	pose.look_at(target, Vec3::Y);
	PlayerCameraPose { translation: pose.translation, rotation: pose.rotation }
}

pub fn map_pose(focus: Vec2, ground_y: f32, height: f32) -> PlayerCameraPose {
	let look = Vec3::new(focus.x, ground_y, focus.y);
	let eye = look + Vec3::Y * height.max(1.0);
	let mut pose = Transform::from_translation(eye);
	pose.look_at(look, Vec3::Z);
	PlayerCameraPose { translation: pose.translation, rotation: pose.rotation }
}

fn first_person_pose(
	player: Vec3,
	head: Option<Vec3>,
	look_rotation: Quat,
	follow: &FollowCamera,
	height_scale: f32,
) -> PlayerCameraPose {
	let head_translation =
		head.unwrap_or(player + Vec3::Y * (follow.height + follow.look_height) * height_scale);
	PlayerCameraPose {
		translation: head_translation + look_rotation * -Vec3::Z * follow.eye_forward,
		rotation: look_rotation,
	}
}

pub fn sync_camera_fov(
	followers: Query<&PlayerCameraAim, With<CameraFollow>>,
	mut cameras: Query<(&CameraController, &FollowCamera, &mut Projection), With<FollowCamera>>,
) {
	let Ok((controller, follow, mut projection)) = cameras.single_mut() else {
		return;
	};
	if followers.is_empty() && !controller.pov.is_map() {
		return;
	}
	let Projection::Perspective(perspective) = projection.as_mut() else {
		return;
	};
	let sight_fov = followers.single().ok().and_then(|aim| aim.sight_fov);
	let optic_zoom = controller.focus > 1e-4;
	perspective.fov =
		vertical_fov(controller.pov, controller.focus_blend, follow, sight_fov, optic_zoom);
}

fn vertical_fov(
	pov: CameraPov,
	focus_blend: f32,
	follow: &FollowCamera,
	sight_fov: Option<f32>,
	optic_zoom: bool,
) -> f32 {
	let ads_fov = if optic_zoom { sight_fov.unwrap_or(follow.sight_fov) } else { follow.sight_fov };
	let hip = follow.hip_fov(pov);
	match pov {
		CameraPov::ThirdPerson | CameraPov::Map => hip,
		CameraPov::FirstPerson => hip + (ads_fov - hip) * focus_blend,
	}
}

fn focus_blend_toward(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
	let step = 1.0 - (-speed * dt.max(0.0)).exp();
	current + (target - current) * step
}

pub fn sync_first_person_head_visibility(
	followers: Query<(), With<CameraFollow>>,
	cameras: Query<&CameraController, With<FollowCamera>>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	parts: Query<&PartNode>,
	mut visibilities: Query<&mut Visibility>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	let hidden = !followers.is_empty() && controller.pov.is_first_person();
	for members in &visuals {
		hide_socketed_parts(
			members,
			&parts,
			&mut visibilities,
			CharacterPartSlot::hides_in_first_person,
			hidden,
		);
	}
}

fn head_camera_translation(
	visuals: &Query<&CharacterMembers, With<PlayerVisual>>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
) -> Option<Vec3> {
	let members = visuals.single().ok()?;
	let left = member_landmark_translation(members.iter(), maps, transforms, "eye_socket.L")?;
	let right = member_landmark_translation(members.iter(), maps, transforms, "eye_socket.R")?;
	Some((left + right) * 0.5)
}

fn member_landmark_translation(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
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

	#[test]
	fn first_person_hides_face_not_body() {
		assert!(CharacterPartSlot::HeadMesh.hides_in_first_person());
		assert!(!CharacterPartSlot::BodyMesh.hides_in_first_person());
	}

	#[test]
	fn first_person_hipfire_is_wider_than_orbit() {
		let follow = FollowCamera::default();
		assert!(
			vertical_fov(CameraPov::FirstPerson, 0.0, &follow, None, false)
				> vertical_fov(CameraPov::ThirdPerson, 0.0, &follow, None, false)
		);
		assert!(
			(vertical_fov(CameraPov::FirstPerson, 1.0, &follow, None, false) - follow.sight_fov)
				.abs() < 1e-5
		);
		let midpoint = vertical_fov(CameraPov::FirstPerson, 0.5, &follow, None, true);
		assert!((midpoint - (follow.first_person_fov + follow.sight_fov) * 0.5).abs() < 1e-5);
	}

	#[test]
	fn held_optic_overrides_iron_sight_fov() {
		let follow = FollowCamera::default();
		let optic = 18.0_f32.to_radians();
		assert!(
			(vertical_fov(CameraPov::FirstPerson, 1.0, &follow, Some(optic), true) - optic).abs()
				< 1e-5
		);
	}

	#[test]
	fn iron_ads_keeps_base_fov_when_an_optic_is_held() {
		let follow = FollowCamera::default();
		let optic = 18.0_f32.to_radians();
		assert!(
			(vertical_fov(CameraPov::FirstPerson, 1.0, &follow, Some(optic), false)
				- follow.sight_fov)
				.abs() < 1e-5
		);
	}

	#[test]
	fn map_pose_looks_down_north_up() {
		let pose = map_pose(Vec2::new(12.0, -8.0), 4.0, 420.0);
		assert!((pose.translation - Vec3::new(12.0, 424.0, -8.0)).length() < 1e-4);
		let forward = pose.rotation * -Vec3::Z;
		assert!(forward.y < -0.99, "map looks down, forward={forward}");
		let up = pose.rotation * Vec3::Y;
		assert!(up.z > 0.99, "map up is world +Z, up={up}");
	}

	#[test]
	fn enter_map_restores_first_person() {
		let mut controller = CameraController { pov: CameraPov::FirstPerson, ..default() };
		controller.enter_map(Vec2::ONE, 200.0, 3.0);
		assert_eq!(controller.pov, CameraPov::Map);
		assert_eq!(controller.resume_pov, CameraPov::FirstPerson);
		controller.exit_map();
		assert_eq!(controller.pov, CameraPov::FirstPerson);
	}

	#[test]
	fn begin_life_leaves_the_map_in_third_person() {
		let mut controller = CameraController { pov: CameraPov::FirstPerson, ..default() };
		controller.enter_map(Vec2::ONE, 200.0, 3.0);
		controller.begin_life();
		assert_eq!(controller.pov, CameraPov::ThirdPerson);
		assert_eq!(controller.resume_pov, CameraPov::ThirdPerson);
		assert_eq!(controller.focus, 0.0);
	}

	#[test]
	fn map_pose_applies_without_a_follow_target() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<Time>();
		world.spawn((
			Camera3d::default(),
			FollowCamera::default(),
			CameraController {
				pov: CameraPov::Map,
				map_focus: Vec2::new(4.0, 8.0),
				map_height: 200.0,
				map_ground_y: 2.0,
				..default()
			},
			Transform::default(),
		));
		world
			.run_system_once(follow_character_camera)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let transform = world.query::<&Transform>().single(&world)?;
		assert!((transform.translation - Vec3::new(4.0, 202.0, 8.0)).length() < 1e-3);
		Ok(())
	}

	#[test]
	fn focus_blend_moves_smoothly_to_and_from_sight() {
		let speed = FollowCamera::default().focus_blend_speed;
		let focused = focus_blend_toward(0.0, 1.0, speed, 1.0 / 60.0);
		assert!(focused > 0.0 && focused < 1.0);
		let released = focus_blend_toward(focused, 0.0, speed, 1.0 / 60.0);
		assert!(released >= 0.0 && released < focused);
	}
}
