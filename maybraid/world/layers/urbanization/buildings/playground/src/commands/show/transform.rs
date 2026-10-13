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

/// Closed world polyline `x,y,z;x,y,z;…` — newtype so clap does not treat it as multi-arg `Vec`.
#[derive(Clone, Debug, PartialEq)]
pub struct Vec3Polyline(pub Vec<Vec3>);

/// Parse a closed polyline of world points: `x,y,z;x,y,z;…` (semicolon-separated).
pub fn parse_vec3_polyline(s: &str) -> Result<Vec3Polyline, String> {
	let s = s.trim();
	if s.is_empty() {
		return Err("expected at least one x,y,z point".into());
	}
	let pts = s
		.split(';')
		.map(|part| parse_vec3_csv(part.trim()))
		.collect::<Result<Vec<_>, _>>()?;
	Ok(Vec3Polyline(pts))
}

/// Parse panel-space `x,z` (allows negatives, e.g. `-1,1`).
pub fn parse_vec2_csv(s: &str) -> Result<Vec2, String> {
	let parts: Vec<_> = s.split(',').collect();
	if parts.len() != 2 {
		return Err(format!("expected x,z got {s:?}"));
	}
	let x: f32 = parts[0].trim().parse().map_err(|e| format!("x: {e}"))?;
	let z: f32 = parts[1].trim().parse().map_err(|e| format!("z: {e}"))?;
	Ok(Vec2::new(x, z))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_vec2_csv_accepts_negatives() -> Result<(), String> {
		assert_eq!(parse_vec2_csv("-1,1")?, Vec2::new(-1.0, 1.0));
		assert_eq!(parse_vec2_csv(" -0.5 , 2 ")?, Vec2::new(-0.5, 2.0));
		Ok(())
	}

	#[test]
	fn parse_vec2_csv_rejects_wrong_arity() -> Result<(), String> {
		assert!(parse_vec2_csv("1").is_err());
		assert!(parse_vec2_csv("1,2,3").is_err());
		Ok(())
	}
}
