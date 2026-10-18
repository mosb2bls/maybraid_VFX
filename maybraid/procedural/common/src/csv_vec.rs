//! Comma-separated `Vec2` / `Vec3` parsing for clap `value_parser`s.

use bevy_math::{Vec2, Vec3};

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

/// Parse world-space `x,y,z` (allows negatives, e.g. `-1,0.5,2`).
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

	#[test]
	fn parse_vec3_csv_accepts_negatives() -> Result<(), String> {
		assert_eq!(parse_vec3_csv("-1,0.5,2")?, Vec3::new(-1.0, 0.5, 2.0));
		Ok(())
	}
}
