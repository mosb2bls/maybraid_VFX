//! Shared transform flags flattened onto `/show` leaf commands.

use bevy::prelude::*;

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

pub fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	let parts: Vec<_> = s.split(',').collect();
	if parts.len() != 3 {
		return Err(format!("expected x,y,z got {s:?}"));
	}
	let x: f32 = parts[0].trim().parse().map_err(|e| format!("x: {e}"))?;
	let y: f32 = parts[1].trim().parse().map_err(|e| format!("y: {e}"))?;
	let z: f32 = parts[2].trim().parse().map_err(|e| format!("z: {e}"))?;
	Ok(Vec3::new(x, y, z))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_vec3_csv_accepts_negatives() -> Result<(), String> {
		assert_eq!(parse_vec3_csv("-1,0.5,2")?, Vec3::new(-1.0, 0.5, 2.0));
		Ok(())
	}
}
