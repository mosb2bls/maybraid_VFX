//! CLI / env opt-in. The plugin is only added when a script or pipe is set.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::Args;

pub const AGENT_SCRIPT_ENV: &str = "MAYBRAID_AGENT_SCRIPT";
pub const AGENT_EVENTS_ENV: &str = "MAYBRAID_AGENT_EVENTS";
pub const AGENT_PIPE_ENV: &str = "MAYBRAID_AGENT_PIPE";
pub const AGENT_QUIT_ENV: &str = "MAYBRAID_AGENT_QUIT";

/// Flags the game / world playground accept. Empty means the agent is off.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
#[command(rename_all = "kebab-case")]
pub struct AgentArgs {
	/// Line-delimited JSON script played on the game clock (no round trip).
	#[arg(long = "agent-script", value_name = "PATH")]
	pub script: Option<PathBuf>,
	/// Append line-delimited JSON events (`-` = stdout).
	#[arg(long = "agent-events", value_name = "PATH")]
	pub events: Option<PathBuf>,
	/// Live command pipe (`-` = stdin). Lines enqueue behind the script.
	#[arg(long = "agent-pipe", value_name = "PATH")]
	pub pipe: Option<PathBuf>,
	/// Exit when the script and pipe are idle.
	#[arg(long = "agent-quit")]
	pub quit_when_idle: bool,
}

impl AgentArgs {
	pub fn enabled(&self) -> bool {
		self.script.is_some() || self.pipe.is_some()
	}

	pub fn merge_env(mut self) -> Self {
		if self.script.is_none() {
			self.script = env_path(AGENT_SCRIPT_ENV);
		}
		if self.events.is_none() {
			self.events = env_path(AGENT_EVENTS_ENV);
		}
		if self.pipe.is_none() {
			self.pipe = env_path(AGENT_PIPE_ENV);
		}
		if !self.quit_when_idle {
			self.quit_when_idle = env_flag(AGENT_QUIT_ENV);
		}
		self
	}
}

fn env_path(key: &str) -> Option<PathBuf> {
	match std::env::var(key) {
		Ok(value) if !value.trim().is_empty() => Some(PathBuf::from(value)),
		_ => None,
	}
}

fn env_flag(key: &str) -> bool {
	match std::env::var(key) {
		Ok(value) => {
			matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
		}
		_ => false,
	}
}

/// Strip `--agent-*` flags from argv. Remaining tokens stay for clap / playground commands.
pub fn take_agent_args(
	args: impl IntoIterator<Item = OsString>,
) -> Result<(AgentArgs, Vec<OsString>), String> {
	let mut agent = AgentArgs::default();
	let mut rest = Vec::new();
	let mut args = args.into_iter();
	while let Some(arg) = args.next() {
		let Some(text) = arg.to_str() else {
			rest.push(arg);
			continue;
		};
		if let Some(value) = text.strip_prefix("--agent-script=") {
			agent.script = Some(PathBuf::from(value));
			continue;
		}
		if text == "--agent-script" {
			agent.script = Some(take_value(&mut args, "--agent-script")?);
			continue;
		}
		if let Some(value) = text.strip_prefix("--agent-events=") {
			agent.events = Some(PathBuf::from(value));
			continue;
		}
		if text == "--agent-events" {
			agent.events = Some(take_value(&mut args, "--agent-events")?);
			continue;
		}
		if let Some(value) = text.strip_prefix("--agent-pipe=") {
			agent.pipe = Some(PathBuf::from(value));
			continue;
		}
		if text == "--agent-pipe" {
			agent.pipe = Some(take_value(&mut args, "--agent-pipe")?);
			continue;
		}
		if text == "--agent-quit" {
			agent.quit_when_idle = true;
			continue;
		}
		rest.push(arg);
	}
	Ok((agent, rest))
}

fn take_value(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<PathBuf, String> {
	let value = args.next().ok_or_else(|| format!("{flag} needs a path"))?;
	Ok(PathBuf::from(value))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn strips_agent_flags_and_keeps_the_rest() -> anyhow::Result<()> {
		let (agent, rest) = take_agent_args([
			OsString::from("--start-at"),
			OsString::from("0,0"),
			OsString::from("--agent-script"),
			OsString::from("scripts/walk.jsonl"),
			OsString::from("--agent-events=events.jsonl"),
			OsString::from("--agent-quit"),
			OsString::from("stats"),
			OsString::from("vsync"),
		])
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(agent.script.as_deref(), Some(std::path::Path::new("scripts/walk.jsonl")));
		assert_eq!(agent.events.as_deref(), Some(std::path::Path::new("events.jsonl")));
		assert!(agent.quit_when_idle);
		assert!(agent.enabled());
		assert_eq!(
			rest,
			vec![
				OsString::from("--start-at"),
				OsString::from("0,0"),
				OsString::from("stats"),
				OsString::from("vsync"),
			]
		);
		Ok(())
	}

	#[test]
	fn disabled_when_empty() {
		assert!(!AgentArgs::default().enabled());
	}
}
