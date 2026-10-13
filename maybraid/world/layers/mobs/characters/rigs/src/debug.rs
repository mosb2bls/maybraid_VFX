use std::collections::HashMap;
use std::fmt;

use bevy::prelude::*;
use log::info;

use crate::{BonePose, Name, RigPose, RiggedAxis};

/// Throttled rig-pose logging for diagnosing bind vs animated transforms.
///
/// Enable with `CROZON_ANIMATION_DEBUG=1` and `RUST_LOG=info,character_rigs=info`.
#[derive(Debug, Clone)]
pub struct RigPoseDebug {
	pub enabled: bool,
	pub log_interval_secs: f32,
	last_log_at: f32,
}

impl Default for RigPoseDebug {
	fn default() -> Self {
		Self {
			enabled: std::env::var("CROZON_ANIMATION_DEBUG").is_ok(),
			log_interval_secs: 2.0,
			last_log_at: f32::NEG_INFINITY,
		}
	}
}

impl RigPoseDebug {
	pub fn should_log(&mut self, t: f32) -> bool {
		if !self.enabled || t - self.last_log_at < self.log_interval_secs {
			return false;
		}
		self.last_log_at = t;
		true
	}
}

pub fn log_bind_pose<'a>(
	label: &str,
	bones: impl Iterator<Item = (&'a Name, &'a Transform, impl fmt::Display)>,
) {
	info!("--- bind pose ({label}) ---");
	for (name, transform, axis) in bones {
		info!(
			"[{}] trans={} rot_euler={} | rigged_axis={axis}",
			name.as_str(),
			format_vec3(transform.translation),
			format_euler_deg(transform.rotation),
		);
	}
	info!("--- end bind pose ---");
}

pub fn log_pose_deltas(
	label: &str,
	pose: &RigPose,
	rest: &HashMap<Name, Transform>,
	bone_names: &[&str],
	axis_for: impl Fn(&Name) -> String,
	header_lines: &[String],
) {
	info!("--- {label} ---");
	for line in header_lines {
		info!("{line}");
	}

	for &bone_name in bone_names {
		let name = Name::from(bone_name);
		let Some(bone_pose) = pose.get(&name) else {
			info!("[{bone_name}] missing from rig pose");
			continue;
		};
		let Some(rest_transform) = rest.get(&name) else {
			info!("[{bone_name}] missing bind rest");
			continue;
		};

		log_bone_delta(bone_name, rest_transform, bone_pose, &axis_for(&name));
	}
}

fn log_bone_delta(bone: &str, rest: &Transform, pose: &BonePose, axis: &str) {
	let trans_delta = pose.transform.translation - rest.translation;
	let rot_delta = rest.rotation.inverse() * pose.transform.rotation;

	info!(
		"[{bone}] trans_rest={} trans_pose={} trans_delta={} | rot_rest={} rot_pose={} rot_delta={} | swing={:.3} flex={:.3} twist={:.3} | rigged_axis={axis}",
		format_vec3(rest.translation),
		format_vec3(pose.transform.translation),
		format_vec3(trans_delta),
		format_euler_deg(rest.rotation),
		format_euler_deg(pose.transform.rotation),
		format_euler_deg(rot_delta),
		pose.swing,
		pose.flex,
		pose.twist,
	);
}

pub fn format_vec3(v: Vec3) -> String {
	format!("({:.4}, {:.4}, {:.4})", v.x, v.y, v.z)
}

pub fn format_euler_deg(q: Quat) -> String {
	let (x, y, z) = q.to_euler(EulerRot::XYZ);
	format!("({:.1}, {:.1}, {:.1})°", x.to_degrees(), y.to_degrees(), z.to_degrees())
}

pub fn format_rigged_axis(axis: Option<RiggedAxis>) -> String {
	match axis {
		Some(axis) => format!(
			"swing={} flex={} twist={}",
			format_vec3(axis.swing_axis),
			format_vec3(axis.flex_axis),
			format_vec3(axis.twist_axis),
		),
		None => "none".into(),
	}
}
