//! Comma-separated `x,y,z` parsing for clap (`--foo 1,2,3`).

use bevy_math::Vec3;

/// Three comma-separated floats (optional ASCII whitespace around commas).
pub fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
	if parts.len() != 3 {
		return Err(format!(
			"expected three comma-separated numbers (e.g. 1.0,2.0,3.0), got {s:?}"
		));
	}
	let x = parts[0]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[0]))?;
	let y = parts[1]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[1]))?;
	let z = parts[2]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[2]))?;
	Ok(Vec3::new(x, y, z))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_vec3_csv_accepts_spaces() -> anyhow::Result<()> {
		let v = parse_vec3_csv("1.0, 2.0 ,3.0").map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(v, Vec3::new(1.0, 2.0, 3.0));
		Ok(())
	}
}
