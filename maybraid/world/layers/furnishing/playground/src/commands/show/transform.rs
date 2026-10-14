//! Shared transform flags flattened onto `/show` leaf commands.

use bevy::prelude::*;
use procedural_common::parse_vec3_csv;

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTransform {
	/// Translation `x,y,z` in world units.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub translate: Vec3,

	/// Euler rotation in degrees around X, then Y, then Z.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub rotate_euler: Vec3,

	/// Scale factors `x,y,z`.
	#[arg(long, default_value = "1,1,1", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub scale: Vec3,
}

impl ShowTransform {
	pub fn transform(&self) -> Transform {
		let rot = Quat::from_euler(
			EulerRot::XYZ,
			self.rotate_euler.x.to_radians(),
			self.rotate_euler.y.to_radians(),
			self.rotate_euler.z.to_radians(),
		);
		Transform::from_translation(self.translate)
			.with_rotation(rot)
			.with_scale(self.scale)
	}
}
