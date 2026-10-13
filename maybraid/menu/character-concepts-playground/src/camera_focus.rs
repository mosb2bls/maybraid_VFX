//! One-shot camera framing for creator-UI focus requests.
//!
//! # Resolution source: shadow rigs
//!
//! Focus targets always resolve against the hidden focus-reference rigs (see
//! [`crate::focus_reference`]), never the visible preview. The shadow rigs
//! carry the character's proportional pose but no animation, so a framing
//! computed from them is stable across walk/run/gallop cycles.
//!
//! # Readiness is imperative, not approximated
//!
//! A queued focus waits until the state it depends on has provably been
//! written, signalled by markers that flip exactly once per rig spawn:
//!
//! - **Body focus** waits for
//!   [`ResolvedPoseApplied`](crate::skinning::ResolvedPoseApplied) on the
//!   shadow body rig, inserted by `maintain_resolved_pose` the first frame it
//!   writes the proportional pose.
//! - **Head focus** additionally waits for the shadow head rig to be parented
//!   to its socket bone, signalled by [`SocketRefApplied`].
//! - The socket bone itself is a plain existence lookup in the rig's
//!   [`BoneMap`]; `"root"` anchors on the rig entity instead of a named bone.
//!
//! There are no bone-height heuristics: once the markers are present the
//! shadow pose is final (pose updates mutate `ActiveRigPose` in place, and
//! armature changes respawn the rigs, resetting the markers).
//!
//! # Application
//!
//! The target transform is captured once on the first ready frame
//! (`resolved_target`) and the camera lerps to it, so later shadow-rig writes
//! cannot drag the camera mid-flight. Enabling free look (`L`) cancels any
//! pending focus.

use bevy::prelude::*;
use camera_controls::look::CameraLookEnabled;
use character_creation_menus::characters::brenal::BODY_FOCUS as BRENAL_BODY_FOCUS;
use character_creation_menus::characters::caole::BODY_FOCUS as CAOLE_BODY_FOCUS;
use character_creation_menus::characters::claber::BODY_FOCUS as CLABER_BODY_FOCUS;
use character_creation_menus::characters::croconot::BODY_FOCUS as CROCONOT_BODY_FOCUS;
use character_creation_menus::characters::epiphant::BODY_FOCUS as EPIPHANT_BODY_FOCUS;
use character_creation_menus::characters::hars::BODY_FOCUS as HARS_BODY_FOCUS;
use character_creation_menus::characters::sonyak::BODY_FOCUS as SONYAK_BODY_FOCUS;
use character_creation_menus::characters::tuberwaber::BODY_FOCUS as TUBERWABER_BODY_FOCUS;
use character_creation_menus::characters::ylter::BODY_FOCUS as YILTER_BODY_FOCUS;
use character_creation_menus::focus::CHUPRI_BODY_FOCUS;
use character_creation_menus::focus::GRENER_BODY_FOCUS;
use character_creation_menus::focus::MISTLER_BODY_FOCUS;
use character_creation_menus::focus::SMALL_BIRD_BODY_FOCUS;
use character_creation_menus::focus::SPIBMOM_BODY_FOCUS;
use character_creation_menus::focus::THUMPLUS_BODY_FOCUS;
use character_creation_menus::focus::TIPPLE_BODY_FOCUS;
use character_creation_menus::BODY_FOCUS;
use character_ui_menu::{CameraFocus, FocusRig};
use characters::{SocketRefApplied, SocketRefRoot};
use characters_playground::CameraController;

use crate::{
	focus_reference::FocusReferenceRig,
	preview::ConceptPreviewConfig,
	skinning::{BoneMap, CharacterRig, CharacterRigRole, ResolvedPoseApplied},
	ui::CreatorUiState,
};

/// One-shot camera move queued when the user selects an asset in the creator UI.
#[derive(Resource, Default)]
pub struct PendingCameraFocus {
	pub focus: Option<CameraFocus>,
	/// Label for debug logging (`ui-press`, `startup-default`, etc.).
	pub focus_trigger: Option<String>,
	/// Target captured on the first ready frame so animation does not chase the lerp.
	pub resolved_target: Option<Transform>,
}

const SNAP_DISTANCE: f32 = 0.04;
const SNAP_ANGLE: f32 = 0.03;

pub fn focus_debug_enabled() -> bool {
	std::env::var("CROZON_CAMERA_FOCUS_DEBUG").is_ok()
}

pub fn queue_camera_focus(
	pending: &mut PendingCameraFocus,
	focus: CameraFocus,
	trigger: impl Into<String>,
) {
	let trigger = trigger.into();
	if focus_debug_enabled() {
		info!("[camera-focus] queue trigger={trigger} {}", focus_summary(focus));
	}
	pending.focus_trigger = Some(trigger);
	pending.focus = Some(focus);
	pending.resolved_target = None;
}

/// Queue default body framing for the active species.
pub fn queue_species_default_camera_focus(
	pending: &mut PendingCameraFocus,
	ui_state: &mut CreatorUiState,
	config: &ConceptPreviewConfig,
	trigger: impl Into<String>,
) {
	let focus = default_focus_target(config);
	ui_state.last_selected = Some(focus);
	queue_camera_focus(pending, focus, trigger);
}

pub fn default_focus_target(config: &ConceptPreviewConfig) -> CameraFocus {
	match config.species() {
		crate::preview::ConceptSpecies::Brenal => BRENAL_BODY_FOCUS,
		crate::preview::ConceptSpecies::Caole => CAOLE_BODY_FOCUS,
		crate::preview::ConceptSpecies::Epiphant => EPIPHANT_BODY_FOCUS,
		crate::preview::ConceptSpecies::Hars => HARS_BODY_FOCUS,
		crate::preview::ConceptSpecies::Yilter => YILTER_BODY_FOCUS,
		crate::preview::ConceptSpecies::Sonyak => SONYAK_BODY_FOCUS,
		crate::preview::ConceptSpecies::Claber => CLABER_BODY_FOCUS,
		crate::preview::ConceptSpecies::Croconot => CROCONOT_BODY_FOCUS,
		crate::preview::ConceptSpecies::Spibmom => SPIBMOM_BODY_FOCUS,
		crate::preview::ConceptSpecies::Chupri => CHUPRI_BODY_FOCUS,
		crate::preview::ConceptSpecies::Tipple => TIPPLE_BODY_FOCUS,
		crate::preview::ConceptSpecies::Topple
		| crate::preview::ConceptSpecies::Kispar
		| crate::preview::ConceptSpecies::Tapp
		| crate::preview::ConceptSpecies::Kaller
		| crate::preview::ConceptSpecies::Kappler => SMALL_BIRD_BODY_FOCUS,
		crate::preview::ConceptSpecies::Grener => GRENER_BODY_FOCUS,
		crate::preview::ConceptSpecies::Thumplus => THUMPLUS_BODY_FOCUS,
		crate::preview::ConceptSpecies::Mistler => MISTLER_BODY_FOCUS,
		crate::preview::ConceptSpecies::Tuberwaber => TUBERWABER_BODY_FOCUS,
		_ => BODY_FOCUS,
	}
}

/// Shadow rig data needed to resolve a focus target.
type ShadowRigQuery<'w, 's> = Query<
	'w,
	's,
	(
		&'static BoneMap,
		&'static CharacterRig,
		&'static GlobalTransform,
		Has<ResolvedPoseApplied>,
		Has<SocketRefRoot>,
		Has<SocketRefApplied>,
	),
	With<FocusReferenceRig>,
>;

pub fn apply_camera_suggestion(
	time: Res<Time>,
	look_enabled: Option<Res<CameraLookEnabled>>,
	mut pending: ResMut<PendingCameraFocus>,
	shadow_rigs: ShadowRigQuery,
	bone_globals: Query<&GlobalTransform>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
	mut last_wait_log: Local<Option<String>>,
) {
	if look_enabled.is_none_or(|enabled| enabled.0) {
		pending.focus = None;
		pending.focus_trigger = None;
		pending.resolved_target = None;
		return;
	}
	let Some(focus) = pending.focus else {
		return;
	};

	if pending.resolved_target.is_none() {
		let Some(target) = resolve_focus_transform(focus, &shadow_rigs, &bone_globals) else {
			log_focus_waiting(focus, &pending, &shadow_rigs, &mut last_wait_log);
			return;
		};
		last_wait_log.take();
		pending.resolved_target = Some(target);
		if focus_debug_enabled() {
			info!(
				"[camera-focus] resolved trigger={:?} {} target=({:.3},{:.3},{:.3})",
				pending.focus_trigger,
				focus_summary(focus),
				target.translation.x,
				target.translation.y,
				target.translation.z,
			);
		}
	}

	let target = pending.resolved_target.unwrap();
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};

	let t = (time.delta_secs() * 6.0).clamp(0.0, 1.0);
	transform.translation = transform.translation.lerp(target.translation, t);
	transform.rotation = transform.rotation.slerp(target.rotation, t);

	let settled = transform.translation.distance(target.translation) < SNAP_DISTANCE
		&& transform.rotation.angle_between(target.rotation) < SNAP_ANGLE;

	if settled {
		*transform = target;
		if focus_debug_enabled() {
			info!(
				"[camera-focus] settled trigger={:?} {}",
				pending.focus_trigger,
				focus_summary(focus),
			);
		}
		pending.focus = None;
		pending.focus_trigger = None;
		pending.resolved_target = None;
	} else {
		let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);
		controller.yaw = yaw;
		controller.pitch = pitch;
	}
}

/// Resolve the camera transform for `focus` from the shadow rigs, or `None`
/// while their imperative readiness signals are still pending.
fn resolve_focus_transform(
	focus: CameraFocus,
	shadow_rigs: &ShadowRigQuery,
	bone_globals: &Query<&GlobalTransform>,
) -> Option<Transform> {
	// The head rig hangs off a body socket bone, so even head focuses are only
	// meaningful once the body pose has been applied.
	let body_pose_applied = shadow_rigs
		.iter()
		.any(|(_, rig, _, pose_applied, _, _)| rig.role == CharacterRigRole::Body && pose_applied);
	if !body_pose_applied {
		return None;
	}

	let neck_awaiting = shadow_rigs.iter().any(|(_, rig, _, _, has_socket, applied)| {
		rig.role == CharacterRigRole::Neck && has_socket && !applied
	});

	for (bone_map, rig, rig_global, _, has_socket, applied) in shadow_rigs.iter() {
		if !rig_role_matches(focus.rig, rig.role) {
			continue;
		}
		if rig.role == CharacterRigRole::Head && ((has_socket && !applied) || neck_awaiting) {
			continue;
		}
		let Some(socket) = focus_socket_global(focus, bone_map, rig_global, bone_globals) else {
			continue;
		};
		let camera_pos = socket_oriented_point(&socket, focus.camera_offset);
		let look_at = socket_oriented_point(&socket, focus.look_at_offset);
		return Some(Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y));
	}
	None
}

fn rig_role_matches(focus_rig: FocusRig, rig_role: CharacterRigRole) -> bool {
	matches!(
		(focus_rig, rig_role),
		(FocusRig::Body, CharacterRigRole::Body) | (FocusRig::Head, CharacterRigRole::Head)
	)
}

/// `"root"` framing anchors on the rig entity; other sockets use named bones.
fn focus_socket_global(
	focus: CameraFocus,
	bone_map: &BoneMap,
	rig_global: &GlobalTransform,
	bone_globals: &Query<&GlobalTransform>,
) -> Option<GlobalTransform> {
	if focus.socket == "root" {
		return Some(*rig_global);
	}
	let bone_entity = bone_map.by_name.get(focus.socket)?;
	bone_globals.get(*bone_entity).ok().copied()
}

/// Map a meter offset along the socket's local axes into world space (no bone scale).
fn socket_oriented_point(socket: &GlobalTransform, local_offset: Vec3) -> Vec3 {
	socket.translation() + socket.rotation() * local_offset
}

fn log_focus_waiting(
	focus: CameraFocus,
	pending: &PendingCameraFocus,
	shadow_rigs: &ShadowRigQuery,
	last_wait_log: &mut Local<Option<String>>,
) {
	if !focus_debug_enabled() {
		return;
	}
	let trigger = pending.focus_trigger.clone();
	if last_wait_log.as_ref() == trigger.as_ref() {
		return;
	}
	let status = shadow_rigs
		.iter()
		.filter(|(_, rig, _, _, _, _)| rig_role_matches(focus.rig, rig.role))
		.map(|(map, rig, _, pose_applied, has_socket, applied)| {
			format!(
				"role={:?} bones={} pose_applied={pose_applied} awaiting_socket={} socket_mapped={}",
				rig.role,
				map.by_name.len(),
				has_socket && !applied,
				focus.socket == "root" || map.by_name.contains_key(focus.socket),
			)
		})
		.collect::<Vec<_>>()
		.join("; ");
	let status = if status.is_empty() { "shadow rig missing".to_string() } else { status };
	warn!("[camera-focus] waiting trigger={trigger:?} {} {status}", focus_summary(focus),);
	**last_wait_log = trigger;
}

fn focus_summary(focus: CameraFocus) -> String {
	format!(
		"rig={:?} socket={} cam_offset={:?} look_at_offset={:?}",
		focus.rig, focus.socket, focus.camera_offset, focus.look_at_offset
	)
}

fn yaw_pitch_from_rotation(rotation: Quat) -> (f32, f32) {
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	let pitch = sin_pitch.asin();
	(yaw, pitch)
}
