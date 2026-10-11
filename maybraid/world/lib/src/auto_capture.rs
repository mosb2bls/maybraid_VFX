//! Automated screenshot capture with frame-based scripting (debug-commands feature only).
//!
//! Script format (one command per line, # comments ignored):
//! - `wait <frames>` - wait N frames before next command
//! - `debug <command>` - run a debug command (e.g., `debug grant-points 15`)
//! - `screenshot <path>` - capture screenshot to path
//! - `exit` - quit the app

use bevy::prelude::*;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::commands::{DebugCommand, PlaygroundCommand};

/// Resource that drives automated capture from a script file.
#[derive(Resource)]
pub struct AutoCaptureScript {
	commands: Vec<CaptureCommand>,
	current_index: usize,
	wait_frames: u32,
	frame_count: u32,
}

#[derive(Debug, Clone)]
enum CaptureCommand {
	Wait(u32),
	Debug(String),
	Screenshot(PathBuf),
	Exit,
}

impl AutoCaptureScript {
	pub fn from_file(path: &Path) -> Result<Self, String> {
		let text =
			std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
		let mut commands = Vec::new();
		
		for (idx, line) in text.lines().enumerate() {
			let line = line.trim();
			if line.is_empty() || line.starts_with('#') {
				continue;
			}
			
			let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
			let cmd = match parts[0] {
				"wait" => {
					let frames = parts
						.get(1)
						.and_then(|s| s.trim().parse::<u32>().ok())
						.ok_or_else(|| {
							format!("{} line {}: wait requires frame count", path.display(), idx + 1)
						})?;
					CaptureCommand::Wait(frames)
				}
				"debug" => {
					let debug_cmd = parts
						.get(1)
						.ok_or_else(|| {
							format!("{} line {}: debug requires command", path.display(), idx + 1)
						})?
						.trim()
						.to_string();
					CaptureCommand::Debug(debug_cmd)
				}
				"screenshot" => {
					let path_str = parts
						.get(1)
						.ok_or_else(|| {
							format!("{} line {}: screenshot requires path", path.display(), idx + 1)
						})?
						.trim();
					CaptureCommand::Screenshot(PathBuf::from(path_str))
				}
				"exit" => CaptureCommand::Exit,
				_ => {
					return Err(format!(
						"{} line {}: unknown command '{}'",
						path.display(),
						idx + 1,
						parts[0]
					));
				}
			};
			commands.push(cmd);
		}
		
		Ok(Self { commands, current_index: 0, wait_frames: 0, frame_count: 0 })
	}
}

pub fn run_auto_capture(
	mut commands: Commands,
	mut script: Option<ResMut<AutoCaptureScript>>,
	players: Query<(Entity, &damage::Health), With<player::Player>>,
	lubikan_ledgers: Query<&lubikans::LubikanLedger>,
	lubikan_heal: Res<lubikans::LubikanSelfHeal>,
	map_view: Res<crate::map_view::WorldMapView>,
	mut activation_finished: MessageReader<lubikans::LubikanActivationFinished>,
	mut damage_applied: MessageReader<damage::DamageApplied>,
) {
	// Log activation finished events
	for finished in activation_finished.read() {
		eprintln!(
			"[EVENT] LubikanActivationFinished: account={:?} outcome={:?}",
			finished.account, finished.outcome
		);
	}
	
	// Log damage events with HP state
	for applied in damage_applied.read() {
		if let Ok((_, health)) = players.single() {
			eprintln!(
				"[DAMAGE] Applied {:.1} damage, HP before={:.1} after={:.1} (max={:.1}), heal_active={} damaged={}",
				applied.applied,
				applied.remaining + applied.applied,
				applied.remaining,
				health.max,
				lubikan_heal.active,
				lubikan_heal.damaged
			);
		}
	}
	
	let Some(ref mut script) = script else {
		return;
	};
	
	script.frame_count += 1;
	
	if script.wait_frames > 0 {
		script.wait_frames -= 1;
		return;
	}
	
	if script.current_index >= script.commands.len() {
		return;
	}
	
	let cmd = script.commands[script.current_index].clone();
	script.current_index += 1;
	
	// Log player state before command execution
	match players.single() {
		Ok((player_entity, health)) => {
			let charge = lubikan_ledgers.get(player_entity)
				.map(|ledger| ledger.equipped().charge().current())
				.unwrap_or(0.0);
			
			let heal_status = if lubikan_heal.active {
				format!("healing {:.1} HP/s (rem {:.1})", lubikan_heal.rate, lubikan_heal.remaining)
			} else {
				"idle".to_string()
			};
			
			let camera_view = if map_view.open { "MAP" } else { "3P" };
			
			eprintln!(
				"[STATE] frame={} hp={:.1}/{:.1} charge={:.1} heal={} camera={}",
				script.frame_count, health.current, health.max, charge, heal_status, camera_view
			);
		}
		Err(e) => {
			eprintln!(
				"[STATE] frame={} no player found: {:?}",
				script.frame_count, e
			);
		}
	}
	
	match cmd {
		CaptureCommand::Wait(frames) => {
			script.wait_frames = frames;
		}
		CaptureCommand::Debug(debug_cmd_str) => {
			// Parse and execute debug command
			match parse_debug_command(&debug_cmd_str) {
				Ok(debug_cmd) => {
					let playground_cmd = PlaygroundCommand::Debug(debug_cmd);
					let mut console = String::new();
					playground_cmd.react(&mut commands, &mut console);
					if !console.is_empty() {
						eprintln!("[AUTO_CAPTURE] {}", console);
					}
				}
				Err(e) => {
					eprintln!("[AUTO_CAPTURE] Failed to parse debug command: {e}");
				}
			}
		}
		CaptureCommand::Screenshot(path) => {
			if let Some(parent) = path.parent() {
				let _ = std::fs::create_dir_all(parent);
			}
			// Use scrot for headless screenshot capture
			match ProcessCommand::new("scrot").arg(&path).output() {
				Ok(_) => eprintln!("[AUTO_CAPTURE] Screenshot: {}", path.display()),
				Err(e) => eprintln!("[AUTO_CAPTURE] Screenshot failed: {e}"),
			}
		}
		CaptureCommand::Exit => {
			eprintln!("[AUTO_CAPTURE] Exit - script complete");
			std::process::exit(0);
		}
	}
}

fn parse_debug_command(cmd_str: &str) -> Result<DebugCommand, String> {
	// Parse as a subcommand: "grant-points 15" becomes ["maybraid-world", "debug", "grant-points", "15"]
	let mut args = vec!["maybraid-world".to_string(), "debug".to_string()];
	args.extend(cmd_str.split_whitespace().map(|s| s.to_string()));
	
	use crate::commands::PlaygroundCommand;
	match PlaygroundCommand::try_parse_from(args) {
		Ok(PlaygroundCommand::Debug(debug_cmd)) => Ok(debug_cmd),
		Ok(_) => Err("expected debug command".to_string()),
		Err(e) => Err(e.to_string()),
	}
}
