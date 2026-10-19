//! Out-of-process relaunch after a Rust panic.
//!
//! Bevy re-raises system panics, so the World is not safe to keep. The parent
//! waits on the child and, on an abnormal exit, starts it again with
//! `--recover`. Agent / playtest runs never enter this loop.

use crate::panic_log::panic_log_path;
use crate::recovery::env_flag_on;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant, SystemTime};

pub const ENV_SUPERVISED: &str = "MAYBRAID_SUPERVISED";
pub const ENV_SUPERVISE: &str = "MAYBRAID_SUPERVISE";

pub const RELAUNCH_MAX: usize = 3;
pub const RELAUNCH_WINDOW: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOutcome {
	pub abnormal: bool,
	pub code: i32,
}

pub fn should_run_supervisor(
	agent: bool,
	already_supervised: bool,
	no_supervise: bool,
	debug: bool,
	supervise_env: Option<&str>,
) -> bool {
	if agent || already_supervised || no_supervise {
		return false;
	}
	if let Some(raw) = supervise_env {
		if !raw.trim().is_empty() {
			return env_flag_on(Some(raw));
		}
	}
	!debug
}

pub fn already_supervised(raw: Option<&str>) -> bool {
	env_flag_on(raw)
}

pub fn child_is_abnormal(status: Option<i32>, panic_log_fresh: bool) -> bool {
	status != Some(0) || panic_log_fresh
}

pub fn can_relaunch(history: &[Instant], now: Instant) -> bool {
	history.iter().filter(|at| now.duration_since(**at) <= RELAUNCH_WINDOW).count() < RELAUNCH_MAX
}

pub fn child_command_args(original: &[OsString], recover: bool) -> Vec<OsString> {
	let mut args: Vec<OsString> = original
		.iter()
		.filter(|arg| {
			arg.to_str()
				.is_none_or(|text| text != "--recover" && !text.starts_with("--recover="))
		})
		.cloned()
		.collect();
	if recover {
		args.push(OsString::from("--recover"));
	}
	args
}

pub fn outcome_from_status(status: ExitStatus, panic_log_fresh: bool) -> ChildOutcome {
	let code = status.code().unwrap_or(1);
	ChildOutcome { abnormal: child_is_abnormal(status.code(), panic_log_fresh), code }
}

pub fn panic_log_is_fresh(started: SystemTime) -> bool {
	let path = panic_log_path();
	let Ok(meta) = std::fs::metadata(&path) else {
		return false;
	};
	let Ok(modified) = meta.modified() else {
		return false;
	};
	modified >= started
}

/// Run the current executable as a supervised child. Returns the last exit code.
pub fn run_supervisor(exe: PathBuf, original_args: Vec<OsString>) -> i32 {
	let mut history = Vec::new();
	let mut recover = false;
	loop {
		let child_args = child_command_args(&original_args, recover);
		let started = SystemTime::now();
		let status = match Command::new(&exe).args(&child_args).env(ENV_SUPERVISED, "1").status() {
			Ok(status) => status,
			Err(error) => {
				eprintln!("maybraid: failed to launch {}: {error}", exe.display());
				return 1;
			}
		};
		let outcome = outcome_from_status(status, panic_log_is_fresh(started));
		if !outcome.abnormal {
			return outcome.code;
		}
		let now = Instant::now();
		history.retain(|at| now.duration_since(*at) <= RELAUNCH_WINDOW);
		if !can_relaunch(&history, now) {
			eprintln!(
				"maybraid: stopped after {RELAUNCH_MAX} crashes in {}s. See {}",
				RELAUNCH_WINDOW.as_secs(),
				panic_log_path().display()
			);
			return outcome.code;
		}
		history.push(now);
		recover = true;
		eprintln!(
			"maybraid: restarting after a crash ({}/{}). See {}",
			history.len(),
			RELAUNCH_MAX,
			panic_log_path().display()
		);
	}
}

pub fn maybe_supervise(agent: bool, no_supervise: bool) -> Option<i32> {
	if !should_run_supervisor(
		agent,
		already_supervised(std::env::var(ENV_SUPERVISED).ok().as_deref()),
		no_supervise,
		cfg!(debug_assertions),
		std::env::var(ENV_SUPERVISE).ok().as_deref(),
	) {
		return None;
	}
	let exe = match std::env::current_exe() {
		Ok(exe) => exe,
		Err(error) => {
			eprintln!("maybraid: current_exe: {error}");
			return Some(1);
		}
	};
	let args: Vec<OsString> = std::env::args_os().skip(1).collect();
	Some(run_supervisor(exe, args))
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Instant;

	#[test]
	fn agent_and_child_never_supervise() {
		assert!(!should_run_supervisor(true, false, false, false, None));
		assert!(!should_run_supervisor(false, true, false, false, None));
		assert!(!should_run_supervisor(false, false, true, false, None));
	}

	#[test]
	fn release_supervises_and_debug_does_not() {
		assert!(should_run_supervisor(false, false, false, false, None));
		assert!(!should_run_supervisor(false, false, false, true, None));
		assert!(should_run_supervisor(false, false, false, true, Some("1")));
		assert!(!should_run_supervisor(false, false, false, false, Some("0")));
	}

	#[test]
	fn agent_runs_never_relaunch() {
		assert!(!child_is_abnormal(Some(0), false));
		assert!(child_is_abnormal(Some(1), false));
		assert!(child_is_abnormal(None, false));
		assert!(child_is_abnormal(Some(0), true));
	}

	#[test]
	fn relaunch_caps_at_three_in_the_window() {
		let now = Instant::now();
		let history = [now, now, now];
		assert!(!can_relaunch(&history, now));
		assert!(can_relaunch(&history[..2], now));
	}

	#[test]
	fn recover_flag_is_added_once() {
		let original = vec![OsString::from("--start-at=0,0"), OsString::from("--recover")];
		assert_eq!(
			child_command_args(&original, true),
			vec![OsString::from("--start-at=0,0"), OsString::from("--recover")]
		);
		assert_eq!(child_command_args(&original, false), vec![OsString::from("--start-at=0,0")]);
	}
}
