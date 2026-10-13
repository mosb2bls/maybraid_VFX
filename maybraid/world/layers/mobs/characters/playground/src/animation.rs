use bevy::prelude::*;
use character_animations::animations::{DEFAULT_BACKSWING, DEFAULT_JAB_TARGET};
use character_rigs::{articulation::compose_parent_rotation, authoring::humanoid_bone_axis, Side};
use characters::{AnimBone, AnimClip, AnimRef, JabParams};
use clap::ValueEnum;

use crate::character::CharacterConfig;

/// Humanoid clips the playground can sample. Mirrors the mid-August concepts
/// `--animation` catalog plus playground-only squat / tuck variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum AnimationMode {
	Still,
	Walk,
	#[default]
	Run,
	Squat,
	SquatDescent,
	Jump,
	Leap,
	Tuck,
	FixedTuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
	Soaring,
	Flapping,
	Jab,
	Prone,
}

impl AnimationMode {
	pub const fn uses_side(self) -> bool {
		matches!(self, Self::Jab)
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Still => "still",
			Self::Walk => "walk",
			Self::Run => "run",
			Self::Squat => "squat",
			Self::SquatDescent => "squat-descent",
			Self::Jump => "jump",
			Self::Leap => "leap",
			Self::Tuck => "tuck",
			Self::FixedTuck => "fixed-tuck",
			Self::TuckedFlip => "tucked-flip",
			Self::TwoFootedTuckedFlip => "two-footed-tucked-flip",
			Self::Soaring => "soaring",
			Self::Flapping => "flapping",
			Self::Jab => "jab",
			Self::Prone => "prone",
		}
	}

	pub fn anim_clip(self, side: Side) -> AnimClip {
		match self {
			Self::Still => AnimClip::still(),
			Self::Walk => AnimClip::walk(),
			Self::Run => AnimClip::run(),
			Self::Squat => AnimClip::squat(),
			Self::SquatDescent => AnimClip::squat_descent(),
			Self::Jump => AnimClip::jump(),
			Self::Leap => AnimClip::leap(),
			Self::Tuck | Self::FixedTuck => AnimClip::tuck(),
			Self::TuckedFlip => AnimClip::tucked_flip(),
			Self::TwoFootedTuckedFlip => AnimClip::two_footed_tucked_flip(),
			Self::Soaring => AnimClip::soaring(),
			Self::Flapping => AnimClip::flapping(),
			Self::Jab => AnimClip::Jab(JabParams {
				side,
				backswing: DEFAULT_BACKSWING,
				target: DEFAULT_JAB_TARGET,
			}),
			Self::Prone => AnimClip::prone(),
		}
	}

	pub fn anim_ref(self, side: Side) -> AnimRef {
		AnimRef::new(self.anim_clip(side))
	}

	/// Mailbox sample coordinate. Held tuck stays at full fold.
	pub fn mailbox_progress(self, elapsed: f32) -> f32 {
		match self {
			Self::FixedTuck => 1.0,
			_ => elapsed,
		}
	}
}

#[derive(Resource, Debug, Clone)]
pub struct AnimationPlayback {
	pub paused: bool,
	pub speed: f32,
	/// When false, the sample clock stops at one second.
	pub looping: bool,
	pub elapsed: f32,
	/// Set by `/character playback --progress`. Consumed on the next sample.
	pub scrub: Option<f32>,
	/// Added to the sample clock. `0.5` is the opposite gait phase.
	pub phase: f32,
	/// Scales femur and shin rest translations before sampling.
	pub leg_scale: f32,
	/// Bone name for the axis gizmo. Empty hides it.
	pub joint: String,
	/// Draw the effective-rest direction next to the posed bone.
	pub show_rest: bool,
	/// When set, replace that bone's clip rotation with these anatomical degrees.
	pub joint_degrees: Option<JointDegrees>,
}

#[derive(Debug, Clone, Copy)]
pub struct JointDegrees {
	pub flexion: f32,
	pub lateral: f32,
	pub axial: f32,
}

impl Default for AnimationPlayback {
	fn default() -> Self {
		Self {
			paused: false,
			speed: 1.0,
			looping: true,
			elapsed: 0.0,
			scrub: None,
			phase: 0.0,
			leg_scale: 1.0,
			joint: String::new(),
			show_rest: true,
			joint_degrees: None,
		}
	}
}

impl AnimationPlayback {
	pub fn advance(&mut self, delta_seconds: f32) {
		if let Some(progress) = self.scrub.take() {
			self.elapsed = progress.max(0.0);
			return;
		}
		if self.paused {
			return;
		}
		self.elapsed += delta_seconds * self.speed;
		if !self.looping {
			self.elapsed = self.elapsed.min(1.0);
		}
	}
}

pub fn apply_joint_preview(
	playback: Res<AnimationPlayback>,
	mut bones: Query<(&mut Transform, &AnimBone)>,
) {
	let Some(degrees) = playback.joint_degrees else {
		return;
	};
	if playback.joint.is_empty() {
		return;
	}
	for (mut transform, bone) in &mut bones {
		if bone.name.as_str() != playback.joint {
			continue;
		}
		transform.rotation = compose_parent_rotation(
			bone.rest.rotation,
			humanoid_bone_axis(&playback.joint),
			degrees.lateral.to_radians(),
			degrees.flexion.to_radians(),
			degrees.axial.to_radians(),
		);
	}
}

/// Bone-local axes are bright RGB. Parent-local axes are shorter and dimmer.
/// Character-space axes sit on the rig: +X right, +Y up, +Z fight-forward.
pub fn draw_authoring_gizmos(
	playback: Res<AnimationPlayback>,
	mut gizmos: Gizmos,
	bones: Query<(&GlobalTransform, &Transform, &AnimBone)>,
	config: Res<CharacterConfig>,
) {
	if playback.joint.is_empty() {
		return;
	}
	let origin = config.transform.translation;
	let character = config.transform.rotation;
	draw_axes(&mut gizmos, origin, character, 0.4, 1.0);
	let opposite = opposite_bone(&playback.joint);
	for (global, local, bone) in &bones {
		let selected = bone.name.as_str() == playback.joint;
		let compared = opposite.as_deref() == Some(bone.name.as_str());
		if !selected && !compared {
			continue;
		}
		let origin = global.translation();
		let gain = if selected { 1.0 } else { 0.55 };
		let length = if selected { 0.22 } else { 0.16 };
		draw_axes(&mut gizmos, origin, global.rotation(), length, gain);
		let parent = global.rotation() * local.rotation.inverse();
		draw_axes(&mut gizmos, origin, parent, length * 0.65, gain * 0.45);
		if playback.show_rest {
			let rest_dir = parent * bone.rest.rotation * Vec3::Y * 0.28;
			let color =
				if selected { Color::srgb(1.0, 0.85, 0.2) } else { Color::srgb(0.75, 0.6, 0.15) };
			gizmos.line(origin, origin + rest_dir, color);
		}
		let segment = bone.rest.translation.length().max(0.05);
		let end = origin + global.rotation() * Vec3::Y * segment;
		gizmos.line(end - Vec3::X * 0.03, end + Vec3::X * 0.03, Color::srgb(1.0, 1.0, 1.0));
		gizmos.line(end - Vec3::Z * 0.03, end + Vec3::Z * 0.03, Color::srgb(1.0, 1.0, 1.0));
	}
}

fn opposite_bone(name: &str) -> Option<String> {
	if let Some(base) = name.strip_suffix(".L") {
		Some(format!("{base}.R"))
	} else {
		name.strip_suffix(".R").map(|base| format!("{base}.L"))
	}
}

fn draw_axes(gizmos: &mut Gizmos, origin: Vec3, rotation: Quat, length: f32, gain: f32) {
	gizmos.line(
		origin,
		origin + rotation * Vec3::X * length,
		Color::srgb(gain, 0.15 * gain, 0.15 * gain),
	);
	gizmos.line(
		origin,
		origin + rotation * Vec3::Y * length,
		Color::srgb(0.15 * gain, gain, 0.15 * gain),
	);
	gizmos.line(
		origin,
		origin + rotation * Vec3::Z * length,
		Color::srgb(0.2 * gain, 0.35 * gain, gain),
	);
}
