use bevy::prelude::*;
use character_persist::SaveRoot;
use clap::Parser;
use maybraid::{assets_root, GamePlugin};
use maybraid_agent::AgentArgs;
use maybraid_input::PadHidPlugins;
use maybraid_world::{
	default_window_present_mode, parse_xz_metres, player_spawn_xz, start_at_from_env, AgentDrive,
};

#[derive(Parser, Debug)]
#[command(name = "maybraid", about = "Maybraid")]
struct GameArgs {
	/// Start Discovery at world XZ metres (`x,z` or `x,y,z`; y is ignored).
	#[arg(long = "start-at", value_name = "X,Z", allow_hyphen_values = true)]
	start_at: Option<String>,
	#[command(flatten)]
	agent: AgentArgs,
}

fn main() {
	let args = GameArgs::parse();
	let start_at = match args.start_at {
		Some(raw) => Some(parse_start_at(&raw)),
		None => start_at_from_env().unwrap_or_else(|error| {
			eprintln!("{error}");
			std::process::exit(2);
		}),
	};
	let agent = args.agent.merge_env();

	let assets_path = assets_root();
	let mut app = App::new();
	app.insert_resource(SaveRoot::discover())
		.insert_resource(player_spawn_xz(start_at));
	if agent.enabled() {
		app.insert_resource(AgentDrive::from_args(agent));
	}
	app.add_plugins(
		DefaultPlugins
			.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Maybraid".into(),
					resolution: window_resolution().into(),
					present_mode: default_window_present_mode(),
					..default()
				}),
				..default()
			})
			.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
			.with_pad_hid(),
	)
	.add_plugins(GamePlugin)
	.run();
}

const ENV_RESOLUTION: &str = "MAYBRAID_RESOLUTION";

fn window_resolution() -> (u32, u32) {
	parse_resolution_env(std::env::var(ENV_RESOLUTION).ok().as_deref()).unwrap_or((1280, 720))
}

fn parse_resolution_env(raw: Option<&str>) -> Option<(u32, u32)> {
	let raw = raw?.trim();
	if raw.is_empty() {
		return None;
	}
	let lowered = raw.to_ascii_lowercase();
	let Some((width, height)) = lowered.split_once('x') else {
		eprintln!("[{ENV_RESOLUTION}] unknown {raw:?} (use WIDTHxHEIGHT)");
		return None;
	};
	match (width.trim().parse::<u32>(), height.trim().parse::<u32>()) {
		(Ok(width), Ok(height)) if width > 0 && height > 0 => Some((width, height)),
		_ => {
			eprintln!("[{ENV_RESOLUTION}] unknown {raw:?} (use WIDTHxHEIGHT)");
			None
		}
	}
}

fn parse_start_at(raw: &str) -> bevy::math::Vec2 {
	parse_xz_metres(raw).unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn start_at_accepts_negative_xz() {
		let args = GameArgs::try_parse_from(["maybraid", "--start-at", "-1500,-600"]).unwrap();
		assert_eq!(args.start_at.as_deref(), Some("-1500,-600"));
		assert_eq!(parse_xz_metres("-1500,-600").unwrap(), bevy::math::Vec2::new(-1500.0, -600.0));
	}

	#[test]
	fn start_at_equals_form_accepts_negatives() {
		let args = GameArgs::try_parse_from(["maybraid", "--start-at=-1500,-600"]).unwrap();
		assert_eq!(args.start_at.as_deref(), Some("-1500,-600"));
	}

	#[test]
	fn resolution_env_accepts_1080p() {
		assert_eq!(parse_resolution_env(Some("1920x1080")), Some((1920, 1080)));
		assert_eq!(parse_resolution_env(Some("1920X1080")), Some((1920, 1080)));
		assert_eq!(parse_resolution_env(Some("")), None);
		assert_eq!(parse_resolution_env(None), None);
		assert_eq!(ENV_RESOLUTION, "MAYBRAID_RESOLUTION");
	}

	#[test]
	fn agent_flags_flatten_onto_game_args() {
		let args = GameArgs::try_parse_from([
			"maybraid",
			"--start-at=0,0",
			"--agent-script",
			"walk.jsonl",
			"--agent-events",
			"events.jsonl",
			"--agent-quit",
		])
		.unwrap();
		assert!(args.agent.enabled());
		assert!(args.agent.quit_when_idle);
		assert_eq!(args.agent.script.as_deref(), Some(std::path::Path::new("walk.jsonl")));
	}
}
