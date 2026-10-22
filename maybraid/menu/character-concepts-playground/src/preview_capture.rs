//! Headless / CI capture hooks (env-driven). Does not affect normal playground use.
//!
//! See `maybraid/world/layers/mobs/characters/animations/previews/epiphant-1130/README.md`.

use bevy::camera::primitives::Aabb;
use bevy::math::bounding::Aabb3d;
use bevy::mesh::{Mesh, VertexAttributeValues};
use bevy::prelude::*;
use character_motion::{sample_bray_mouth_emit, BrayMouthEmit};
use characters::{
	BoneMap, CharacterMembers, CharacterPart, CharacterPartSlot, CharacterRig, CharacterRigRole,
	LocomotionCapsule,
};
use characters::species::epiphant::{pose::EpiphantPose, EpiphantConfig};
use game_commands::GameCommandDrawerVisible;

use crate::focus_reference::FocusReferenceRig;

use crate::animation::ConceptAnimation;
use crate::preview::{ConceptPreviewConfig, ConceptPreviewRoot};

#[derive(Resource, Clone, Copy, Debug)]
pub struct CaptureCameraLocked;

pub fn capture_hides_creator_ui() -> bool {
	let settings = PreviewCaptureSettings::from_env();
	settings.active() && settings.hide_ui
}

#[derive(Resource, Debug, Clone)]
pub struct PreviewCaptureSettings {
	pub encounter_scale: f32,
	pub sample_progress: Option<f32>,
	pub quit_after_secs: Option<f32>,
	pub interrupt_after_secs: Option<f32>,
	pub interrupt_animation: Option<ConceptAnimation>,
	pub hide_ui: bool,
	pub draw_mouth_emit: bool,
}

impl PreviewCaptureSettings {
	pub fn from_env() -> Self {
		Self {
			encounter_scale: env_parse_f32("CROZON_PREVIEW_ENCOUNTER_SCALE").unwrap_or(1.0),
			sample_progress: env_parse_f32("CROZON_PREVIEW_SAMPLE_PROGRESS"),
			quit_after_secs: env_parse_f32("CROZON_PREVIEW_CAPTURE_QUIT_SECS"),
			interrupt_after_secs: env_parse_f32("CROZON_PREVIEW_INTERRUPT_AFTER_SECS"),
			interrupt_animation: env_parse_animation("CROZON_PREVIEW_INTERRUPT_ANIMATION"),
			hide_ui: env_flag("CROZON_PREVIEW_HIDE_UI").unwrap_or(true),
			draw_mouth_emit: env_flag("CROZON_PREVIEW_DRAW_MOUTH_EMIT").unwrap_or(false),
		}
	}

	pub fn active(&self) -> bool {
		self.encounter_scale != 1.0
			|| self.sample_progress.is_some()
			|| self.quit_after_secs.is_some()
			|| self.interrupt_after_secs.is_some()
			|| self.draw_mouth_emit
	}
}

/// Read once when spawning the preview LodScene root ([`preview::spawn_clothed_character`]).
pub fn encounter_scale_from_env() -> f32 {
	PreviewCaptureSettings::from_env().encounter_scale.max(f32::EPSILON)
}

/// Read when stamping the body clip ([`preview::stamp_lod_character_preview`]).
pub fn sample_progress_from_env() -> Option<f32> {
	PreviewCaptureSettings::from_env().sample_progress
}

#[derive(Resource, Default)]
pub(crate) struct PreviewCaptureInterruptState {
	done: bool,
}

#[derive(Resource, Default)]
pub(crate) struct PendingCaptureInterrupt {
	animation: Option<ConceptAnimation>,
}

pub fn load_preview_capture_settings(mut commands: Commands) {
	let settings = PreviewCaptureSettings::from_env();
	if settings.active() && settings.hide_ui {
		commands.insert_resource(GameCommandDrawerVisible(false));
	}
	if settings.active() {
		commands.insert_resource(settings);
		commands.insert_resource(CaptureCameraLocked);
		commands.init_resource::<CaptureGroundContact>();
		commands.init_resource::<PreviewCaptureInterruptState>();
		commands.init_resource::<PendingCaptureInterrupt>();
	}
}

/// Marks the capture ground plane (excluded from mesh bounds).
#[derive(Component)]
pub struct CaptureGroundPlane;

/// World-space Y of the ground plane during capture (foot contact).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct CaptureGroundContact {
	pub y: Option<f32>,
}

/// Fallback when mesh bounds are not ready yet.
pub fn capture_camera_frame_fallback(encounter_scale: f32) -> (Vec3, Vec3) {
	let scale = encounter_scale.max(1.0);
	let ground_y = capture_ground_anchor_y(scale);
	let center = Vec3::new(0.0, ground_y + 0.9 * scale, 0.0);
	let distance = 5.5 * scale;
	let dir = Vec3::new(0.42, 0.28, 0.86).normalize();
	(center + dir * distance, center)
}

/// Frame the full character AABB (feet + ground) in a 3/4 elevated view.
pub fn capture_camera_frame_for_bounds(
	bounds: Aabb3d,
	aspect: f32,
	fov_y: f32,
	ground_y: f32,
) -> (Vec3, Vec3) {
	const MARGIN: f32 = 1.14;
	let mut min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	// Keep ground contact in frame.
	min.y = min.y.min(ground_y);
	let center = (min + max) * 0.5;
	let size = (max - min) * MARGIN;
	let height = size.y.max(0.5);
	let horizontal = (size.x * size.x + size.z * size.z).sqrt().max(0.5);
	let half_fov_y = fov_y * 0.5;
	let dist_for_height = height / (2.0 * half_fov_y.tan());
	let half_fov_x = (fov_y * 0.5).tan() * aspect;
	let dist_for_width = horizontal / (2.0 * half_fov_x);
	let distance = dist_for_height.max(dist_for_width);
	let dir = Vec3::new(0.42, 0.28, 0.86).normalize();
	(center + dir * distance, center)
}

fn foot_bearing_part(slot: CharacterPartSlot) -> bool {
	matches!(slot, CharacterPartSlot::BodyMesh | CharacterPartSlot::Clothing)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PreviewMeshScan {
	pub bounds: Option<Aabb3d>,
	pub foot_contact_y: Option<f32>,
}

pub fn scan_preview_meshes<'a>(
	mesh_assets: &Assets<Mesh>,
	meshes: impl Iterator<Item = (&'a Mesh3d, &'a GlobalTransform, Option<&'a Visibility>, Option<&'a CharacterPart>)>,
) -> PreviewMeshScan {
	let mut union: Option<Aabb3d> = None;
	let mut foot_min: Option<f32> = None;
	let mut all_min: Option<f32> = None;
	for (mesh3d, global, visibility, part) in meshes {
		if matches!(visibility, Some(Visibility::Hidden)) {
			continue;
		}
		let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
			continue;
		};
		let Some(world) = mesh_world_aabb(mesh, global) else {
			continue;
		};
		let y = world.min.y;
		all_min = Some(all_min.map_or(y, |current| current.min(y)));
		if part.is_none_or(|part| foot_bearing_part(part.slot)) {
			foot_min = Some(foot_min.map_or(y, |current| current.min(y)));
		}
		union = Some(match union {
			None => world,
			Some(acc) => Aabb3d::from_min_max(acc.min.min(world.min), acc.max.max(world.max)),
		});
	}
	PreviewMeshScan {
		bounds: union,
		foot_contact_y: foot_min.or(all_min),
	}
}

/// Rest-pose foot depth for the default Epiphant preview recipe, scaled to encounter size.
pub fn capture_ground_anchor_y(encounter_scale: f32) -> f32 {
	let pose = EpiphantPose::from_config(&EpiphantConfig::default_preview());
	let hull = LocomotionCapsule::quadruped_for_limb_length(pose.rest_limb_scale());
	let scale = encounter_scale.max(1.0);
	-hull.half_height() * scale
}

pub fn union_preview_mesh_bounds<'a>(
	mesh_assets: &Assets<Mesh>,
	meshes: impl Iterator<Item = (&'a Mesh3d, &'a GlobalTransform, Option<&'a Visibility>, Option<&'a CharacterPart>)>,
) -> Option<Aabb3d> {
	scan_preview_meshes(mesh_assets, meshes).bounds
}

fn mesh_world_aabb(mesh: &Mesh, global: &GlobalTransform) -> Option<Aabb3d> {
	let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
	else {
		return None;
	};
	let local = Aabb::enclosing(positions.iter().map(|p| Vec3::from(*p)))?;
	let min = local.min();
	let max = local.max();
	let corners = [
		Vec3::new(min.x, min.y, min.z),
		Vec3::new(min.x, min.y, max.z),
		Vec3::new(min.x, max.y, min.z),
		Vec3::new(min.x, max.y, max.z),
		Vec3::new(max.x, min.y, min.z),
		Vec3::new(max.x, min.y, max.z),
		Vec3::new(max.x, max.y, min.z),
		Vec3::new(max.x, max.y, max.z),
	];
	let mut world_min = Vec3::splat(f32::INFINITY);
	let mut world_max = Vec3::splat(f32::NEG_INFINITY);
	for corner in corners {
		let world = global.transform_point(corner);
		world_min = world_min.min(world);
		world_max = world_max.max(world);
	}
	Some(Aabb3d::from_min_max(world_min, world_max))
}

pub fn setup_capture_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	settings: Option<Res<PreviewCaptureSettings>>,
) {
	let Some(settings) = settings else {
		return;
	};
	let scale = settings.encounter_scale.max(1.0);
	let extent = 120.0 * scale;
	let mesh = meshes.add(Plane3d::default().mesh().size(extent, extent));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.78, 0.82, 0.76),
		perceptual_roughness: 0.95,
		..default()
	});
	let anchor_y = capture_ground_anchor_y(scale);
	commands.spawn((
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_translation(Vec3::new(0.0, anchor_y, 0.0)),
		CaptureGroundPlane,
	));
}

pub fn align_capture_ground_to_contact(
	settings: Option<Res<PreviewCaptureSettings>>,
	mut contact: ResMut<CaptureGroundContact>,
	meshes: Query<
		(
			&Mesh3d,
			&GlobalTransform,
			Option<&Visibility>,
			Option<&CharacterPart>,
		),
		(
			With<CharacterPart>,
			Without<FocusReferenceRig>,
			Without<CaptureGroundPlane>,
		),
	>,
	mesh_assets: Res<Assets<Mesh>>,
	mut ground: Query<&mut Transform, With<CaptureGroundPlane>>,
) {
	let Some(settings) = settings else {
		return;
	};
	let scan = scan_preview_meshes(mesh_assets.as_ref(), meshes.iter());
	let ground_y = scan
		.foot_contact_y
		.unwrap_or_else(|| capture_ground_anchor_y(settings.encounter_scale));
	contact.y = Some(ground_y);
	for mut transform in &mut ground {
		transform.translation.y = ground_y;
	}
}

pub fn mouth_emit_gizmo_enabled(settings: Option<Res<PreviewCaptureSettings>>) -> bool {
	settings.is_some_and(|s| s.draw_mouth_emit)
}

pub fn preview_capture_interrupt(
	time: Res<Time>,
	mut state: ResMut<PreviewCaptureInterruptState>,
	mut pending: ResMut<PendingCaptureInterrupt>,
	settings: Option<Res<PreviewCaptureSettings>>,
) {
	let Some(settings) = settings else {
		return;
	};
	if state.done {
		return;
	}
	let Some(after) = settings.interrupt_after_secs else {
		return;
	};
	let Some(animation) = settings.interrupt_animation else {
		return;
	};
	if time.elapsed_secs() < after {
		return;
	}
	pending.animation = Some(animation);
	state.done = true;
}

pub fn apply_pending_capture_interrupt(
	mut commands: Commands,
	mut config: ResMut<ConceptPreviewConfig>,
	mut pending: ResMut<PendingCaptureInterrupt>,
	settings: Option<Res<PreviewCaptureSettings>>,
	roots: Query<&CharacterMembers, With<ConceptPreviewRoot>>,
	rigs: Query<&CharacterRig>,
) {
	if settings.is_none() {
		return;
	}
	let Some(animation) = pending.animation.take() else {
		return;
	};
	*config = config.clone().with_animation(animation);
	for members in &roots {
		for member in members.iter() {
			if rigs.get(member).is_ok_and(|rig| rig.role == CharacterRigRole::Body) {
				commands.entity(member).remove::<characters::AnimProgress>();
			}
		}
	}
}

pub fn preview_capture_auto_quit(
	time: Res<Time>,
	settings: Option<Res<PreviewCaptureSettings>>,
	visible: Query<(), With<ConceptPreviewRoot>>,
) {
	let Some(settings) = settings else {
		return;
	};
	let Some(quit) = settings.quit_after_secs else {
		return;
	};
	if visible.is_empty() || time.elapsed_secs() < quit {
		return;
	}
	std::process::exit(0);
}

pub fn draw_preview_mouth_emit_gizmo(
	settings: Option<Res<PreviewCaptureSettings>>,
	config: Res<ConceptPreviewConfig>,
	roots: Query<&CharacterMembers, With<ConceptPreviewRoot>>,
	rigs: Query<(&CharacterRig, &BoneMap)>,
	globals: Query<&GlobalTransform>,
	mut gizmos: Gizmos,
) {
	let Some(settings) = settings else {
		return;
	};
	if !settings.draw_mouth_emit || !matches!(config.animation(), ConceptAnimation::EpiphantBray) {
		return;
	}
	let scale = settings.encounter_scale.max(1.0);
	let arrow_len = 1.25 * scale;
	let sphere_r = 0.12 * scale.sqrt();
	for members in &roots {
		let maps: Vec<&BoneMap> = members
			.iter()
			.filter_map(|member| rigs.get(member).ok().map(|(_, map)| map))
			.collect();
		let Some(emit) = sample_bray_mouth_emit(&maps, &globals) else {
			continue;
		};
		draw_bray_mouth_emit_gizmo(&mut gizmos, emit, arrow_len, sphere_r);
	}
}

fn draw_bray_mouth_emit_gizmo(
	gizmos: &mut Gizmos,
	emit: BrayMouthEmit,
	arrow_len: f32,
	sphere_r: f32,
) {
	let origin = emit.origin;
	let tip = origin + emit.forward * arrow_len;
	gizmos.sphere(Isometry3d::from_translation(origin), sphere_r, Color::srgb(0.2, 0.95, 1.0));
	gizmos.line(origin, tip, Color::srgb(1.0, 0.92, 0.2));
	let side = Vec3::Y.cross(emit.forward).normalize_or_zero() * (sphere_r * 2.5);
	if side.length_squared() > f32::EPSILON {
		gizmos.line(tip - side, tip + side, Color::srgb(1.0, 0.92, 0.2));
		gizmos.line(tip, tip + Vec3::Y * sphere_r * 1.5, Color::srgb(1.0, 0.92, 0.2));
	}
}

fn env_parse_f32(name: &str) -> Option<f32> {
	std::env::var(name).ok()?.parse().ok()
}

fn env_flag(name: &str) -> Option<bool> {
	let value = std::env::var(name).ok()?.to_ascii_lowercase();
	if matches!(value.as_str(), "1" | "true" | "yes") {
		Some(true)
	} else if matches!(value.as_str(), "0" | "false" | "no") {
		Some(false)
	} else {
		None
	}
}

fn env_parse_animation(name: &str) -> Option<ConceptAnimation> {
	let raw = std::env::var(name).ok()?;
	ConceptAnimation::VALUES.iter().copied().find(|value| value.label() == raw)
}
