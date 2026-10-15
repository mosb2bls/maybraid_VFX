//! Load a line-delimited JSON script. `#` / `//` comments and blanks are skipped.

use std::path::Path;

use crate::command::AgentCommand;

pub fn read_script_text(text: &str) -> Result<Vec<AgentCommand>, String> {
	let mut commands = Vec::new();
	for (idx, raw) in text.lines().enumerate() {
		let line = strip_comment(raw);
		if line.is_empty() {
			continue;
		}
		match AgentCommand::parse_line(line) {
			Ok(command) => commands.push(command),
			Err(error) => return Err(format!("line {}: {error}", idx + 1)),
		}
	}
	Ok(commands)
}

pub fn read_script_commands(path: &Path) -> Result<Vec<AgentCommand>, String> {
	let text =
		std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
	read_script_text(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// Remove `start` commands that precede the first duration command and return
/// the last one's XZ. The caller applies it as the spawn point before load.
pub fn take_leading_start(commands: &mut Vec<AgentCommand>) -> Option<[f32; 2]> {
	let leading = commands.iter().position(AgentCommand::is_duration).unwrap_or(commands.len());
	let mut start = None;
	let mut idx = 0;
	commands.retain(|command| {
		let keep = match command {
			AgentCommand::Start { xz } if idx < leading => {
				start = Some(*xz);
				false
			}
			_ => true,
		};
		idx += 1;
		keep
	});
	start
}

fn strip_comment(line: &str) -> &str {
	let trimmed = line.trim();
	if trimmed.starts_with('#') || trimmed.starts_with("//") {
		return "";
	}
	trimmed
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::command::ScenarioMark;

	#[test]
	fn skips_comments_and_parses_jsonl() -> anyhow::Result<()> {
		let commands = read_script_text(
			r#"
# warmup
// also a comment

{"cmd":"scenario","name":"demo","mark":"start"}
{"cmd":"wait","secs":1}
{"cmd":"hold","move":[0,1],"secs":2}
{"cmd":"quit"}
"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(commands.len(), 4);
		assert!(matches!(commands[0], AgentCommand::Scenario { mark: ScenarioMark::Start, .. }));
		assert!(matches!(commands[3], AgentCommand::Quit));
		Ok(())
	}

	#[test]
	fn reports_line_number() {
		let error = read_script_text("{\"cmd\":\"wait\",\"secs\":1}\nnope\n").unwrap_err();
		assert!(error.contains("line 2"), "{error}");
	}

	const EXAMPLE_SCRIPTS: &[&str] = &[
		"origin-walk.jsonl",
		"origin-idle.jsonl",
		"biome-traverse.jsonl",
		"locomotion-mix.jsonl",
		"remote-start.jsonl",
		"origin-first-spawn.jsonl",
		"origin-picker.jsonl",
		"dusk-spawn.jsonl",
		"death-picker.jsonl",
		"character-switch-spawn.jsonl",
		"poi-picker-cycle.jsonl",
		"mob-spawns.jsonl",
		"visual-style-idle.jsonl",
		"visual-style-idle-watercolor.jsonl",
		"visual-style-idle-native.jsonl",
		"visual-style-idle-sharp.jsonl",
		"visual-style-idle-offscreen.jsonl",
		"visual-style-idle-retro.jsonl",
		"visual-style-idle-antediluvian.jsonl",
		"visual-style-idle-pre-post-modern.jsonl",
		"visual-style-traverse.jsonl",
		"visual-style-traverse-native.jsonl",
		"visual-style-traverse-sharp.jsonl",
		"visual-style-traverse-antediluvian.jsonl",
		"menu-editor-discard-start.jsonl",
		"menu-editor-save-start.jsonl",
		"menu-create-back-mid-spin.jsonl",
		"menu-settings-cycle.jsonl",
		"menu-leave-and-return.jsonl",
		"menu-mash-home-42.jsonl",
		"menu-mash-pause-42.jsonl",
		"smoke-origin-walk.jsonl",
		"smoke-player-down.jsonl",
		"smoke-menu-home.jsonl",
		"smoke-mobs-guard.jsonl",
		"menu-home-focus.jsonl",
		"menu-home-characters.jsonl",
		"menu-home-settings.jsonl",
		"menu-pause-back.jsonl",
	];

	#[test]
	fn leading_start_is_taken_and_later_start_kept() -> anyhow::Result<()> {
		let mut commands = read_script_text(
			r#"
{"cmd":"console","line":"stats vsync"}
{"cmd":"start","xz":[100,-40]}
{"cmd":"wait","secs":1}
{"cmd":"start","xz":[5,5]}
"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(take_leading_start(&mut commands), Some([100.0, -40.0]));
		assert_eq!(commands.len(), 3);
		assert!(matches!(commands[2], AgentCommand::Start { xz } if xz == [5.0, 5.0]));
		Ok(())
	}

	#[test]
	fn no_start_leaves_commands() -> anyhow::Result<()> {
		let mut commands = read_script_text("{\"cmd\":\"wait\",\"secs\":1}\n")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(take_leading_start(&mut commands), None);
		assert_eq!(commands.len(), 1);
		Ok(())
	}

	#[test]
	fn example_scripts_parse() -> anyhow::Result<()> {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
		for name in EXAMPLE_SCRIPTS {
			let commands =
				read_script_commands(&root.join(name)).map_err(|e| anyhow::anyhow!("{e}"))?;
			assert!(!commands.is_empty(), "{name}");
		}
		Ok(())
	}

	#[test]
	fn profile_scripts_do_not_toggle_vsync() -> anyhow::Result<()> {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
		for name in EXAMPLE_SCRIPTS {
			let text = std::fs::read_to_string(root.join(name))?;
			for (idx, raw) in text.lines().enumerate() {
				let line = strip_comment(raw);
				if line.is_empty() {
					continue;
				}
				assert!(
					!line.contains("stats vsync"),
					"{name}:{} must not toggle vsync over MAYBRAID_VSYNC=off",
					idx + 1
				);
			}
		}
		Ok(())
	}

	#[test]
	fn home_first_scripts_do_not_take_a_leading_start() -> anyhow::Result<()> {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
		for name in [
			"character-switch-spawn.jsonl",
			"poi-picker-cycle.jsonl",
			"menu-home-focus.jsonl",
			"menu-create-back-mid-spin.jsonl",
			"menu-mash-home-42.jsonl",
		] {
			let mut commands =
				read_script_commands(&root.join(name)).map_err(|e| anyhow::anyhow!("{e}"))?;
			assert_eq!(take_leading_start(&mut commands), None, "{name}");
		}
		Ok(())
	}
}
