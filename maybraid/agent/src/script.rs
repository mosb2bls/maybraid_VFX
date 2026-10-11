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

	#[test]
	fn example_scripts_parse() -> anyhow::Result<()> {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
		for name in [
			"origin-walk.jsonl",
			"origin-idle.jsonl",
			"biome-traverse.jsonl",
			"locomotion-mix.jsonl",
		] {
			let commands =
				read_script_commands(&root.join(name)).map_err(|e| anyhow::anyhow!("{e}"))?;
			assert!(!commands.is_empty(), "{name}");
		}
		Ok(())
	}
}
