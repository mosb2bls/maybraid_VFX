//! Discovery / playground start location (`--start-at` / [`START_AT_ENV`]).

use std::ffi::OsString;

use bevy::prelude::*;
use world_player::PlayerSpawnXz;

/// `MAYBRAID_START_AT=x,z` when argv does not pass `--start-at`.
pub const START_AT_ENV: &str = "MAYBRAID_START_AT";

/// Parse `x,z` or `x,y,z` metres (y is ignored).
pub fn parse_xz_metres(raw: &str) -> Result<Vec2, String> {
	let parts: Vec<&str> = raw.split(',').map(str::trim).filter(|part| !part.is_empty()).collect();
	match parts.as_slice() {
		[x, z] => Ok(Vec2::new(parse_coord(x)?, parse_coord(z)?)),
		[x, _y, z] => Ok(Vec2::new(parse_coord(x)?, parse_coord(z)?)),
		_ => Err(format!("expected x,z or x,y,z metres, got {raw:?}")),
	}
}

fn parse_coord(raw: &str) -> Result<f32, String> {
	raw.parse::<f32>().map_err(|_| format!("invalid metre coordinate {raw:?}"))
}

pub fn start_at_from_env() -> Result<Option<Vec2>, String> {
	match std::env::var(START_AT_ENV) {
		Ok(raw) if !raw.trim().is_empty() => parse_xz_metres(&raw).map(Some),
		Ok(_) | Err(std::env::VarError::NotPresent) => Ok(None),
		Err(error) => Err(error.to_string()),
	}
}

/// Strip `--start-at` / `--start-at=` from argv. Remaining tokens stay for clap.
pub fn take_start_at_from_args(
	args: impl IntoIterator<Item = OsString>,
) -> Result<(Option<Vec2>, Vec<OsString>), String> {
	let mut start = None;
	let mut rest = Vec::new();
	let mut args = args.into_iter();
	while let Some(arg) = args.next() {
		let Some(text) = arg.to_str() else {
			rest.push(arg);
			continue;
		};
		if let Some(value) = text.strip_prefix("--start-at=") {
			start = Some(parse_xz_metres(value)?);
			continue;
		}
		if text == "--start-at" {
			let value = args.next().ok_or_else(|| "--start-at needs x,z metres".to_string())?;
			start = Some(parse_xz_metres(&value.to_string_lossy())?);
			continue;
		}
		rest.push(arg);
	}
	Ok((start, rest))
}

pub fn resolve_start_at(
	args: impl IntoIterator<Item = OsString>,
) -> Result<(Option<Vec2>, Vec<OsString>), String> {
	let (from_args, rest) = take_start_at_from_args(args)?;
	let start = match from_args {
		Some(xz) => Some(xz),
		None => start_at_from_env()?,
	};
	Ok((start, rest))
}

pub fn player_spawn_xz(start: Option<Vec2>) -> PlayerSpawnXz {
	PlayerSpawnXz(start)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_xz_and_xyz() {
		assert_eq!(parse_xz_metres("100,200").unwrap(), Vec2::new(100.0, 200.0));
		assert_eq!(parse_xz_metres("100, 12.5, -50").unwrap(), Vec2::new(100.0, -50.0));
	}

	#[test]
	fn take_start_at_strips_the_flag() {
		let (start, rest) = take_start_at_from_args([
			OsString::from("--start-at"),
			OsString::from("12,34"),
			OsString::from("stats"),
			OsString::from("fps"),
		])
		.unwrap();
		assert_eq!(start, Some(Vec2::new(12.0, 34.0)));
		assert_eq!(rest, vec![OsString::from("stats"), OsString::from("fps")]);
	}

	#[test]
	fn take_start_at_equals_form() {
		let (start, rest) = take_start_at_from_args([OsString::from("--start-at=-10,20")]).unwrap();
		assert_eq!(start, Some(Vec2::new(-10.0, 20.0)));
		assert!(rest.is_empty());
	}

	#[test]
	fn take_start_at_space_form_accepts_negatives() {
		let (start, rest) =
			take_start_at_from_args([OsString::from("--start-at"), OsString::from("-1500,-600")])
				.unwrap();
		assert_eq!(start, Some(Vec2::new(-1500.0, -600.0)));
		assert!(rest.is_empty());
	}
}
