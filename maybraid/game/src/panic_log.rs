//! File-backed panic hook for packaged builds (DMG / zip / tarball).
//!
//! The unsigned app has no console. Without this hook a Rust panic is silent:
//! `panic = "unwind"` is the workspace default, but nothing captures stderr
//! when the player launches Maybraid.app. Write under the platform log dir
//! (`~/Library/Logs/Maybraid` on macOS) and then call the previous hook.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic::{self, PanicHookInfo};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Env override for the directory that receives `panic.log`.
pub const ENV_LOGS: &str = "MAYBRAID_LOGS";

const PANIC_LOG: &str = "panic.log";
const ERROR_LOG: &str = "errors.log";

/// Install a panic hook that appends to [`panic_log_path`] then delegates.
pub fn install_panic_log_hook() {
	let previous = panic::take_hook();
	panic::set_hook(Box::new(move |info| {
		let _ = write_panic_log(info);
		previous(info);
	}));
}

/// Platform log directory. `MAYBRAID_LOGS` wins.
pub fn log_dir() -> PathBuf {
	if let Ok(path) = env::var(ENV_LOGS) {
		let trimmed = path.trim();
		if !trimmed.is_empty() {
			return PathBuf::from(trimmed);
		}
	}
	platform_log_dir()
}

pub fn panic_log_path() -> PathBuf {
	log_dir().join(PANIC_LOG)
}

pub fn error_log_path() -> PathBuf {
	log_dir().join(ERROR_LOG)
}

/// Append a recoverable ECS error (sibling of [`panic_log_path`]).
pub fn append_error_log(message: &str) -> std::io::Result<()> {
	append_log(ERROR_LOG, "error", message)
}

fn platform_log_dir() -> PathBuf {
	#[cfg(target_os = "macos")]
	{
		home_dir().join("Library/Logs/Maybraid")
	}
	#[cfg(target_os = "windows")]
	{
		env::var_os("APPDATA")
			.map(PathBuf::from)
			.unwrap_or_else(home_dir)
			.join("Maybraid")
			.join("logs")
	}
	#[cfg(not(any(target_os = "macos", target_os = "windows")))]
	{
		env::var_os("XDG_STATE_HOME")
			.map(PathBuf::from)
			.unwrap_or_else(|| home_dir().join(".local/state"))
			.join("maybraid")
			.join("logs")
	}
}

fn home_dir() -> PathBuf {
	env::var_os("HOME")
		.or_else(|| env::var_os("USERPROFILE"))
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("."))
}

fn write_panic_log(info: &PanicHookInfo<'_>) -> std::io::Result<()> {
	append_panic_log(&format!("{info}"))
}

fn append_panic_log(message: &str) -> std::io::Result<()> {
	append_log(PANIC_LOG, "panic", message)
}

fn append_log(file_name: &str, kind: &str, message: &str) -> std::io::Result<()> {
	let dir = log_dir();
	fs::create_dir_all(&dir)?;
	let mut file = OpenOptions::new().create(true).append(true).open(dir.join(file_name))?;
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|elapsed| elapsed.as_secs())
		.unwrap_or(0);
	writeln!(file, "--- {kind} at unix={now} ---")?;
	writeln!(file, "{message}")?;
	if kind == "panic" {
		writeln!(file, "backtrace:\n{}", std::backtrace::Backtrace::force_capture())?;
	}
	file.flush()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;
	use std::sync::Mutex;

	static ENV_LOCK: Mutex<()> = Mutex::new(());

	#[test]
	fn logs_env_overrides_the_platform_dir() {
		let _guard = ENV_LOCK.lock().expect("env lock");
		let dir = tempfile::tempdir().expect("tempdir");
		env::set_var(ENV_LOGS, dir.path());
		assert_eq!(log_dir(), dir.path());
		assert_eq!(panic_log_path(), dir.path().join(PANIC_LOG));
		assert_eq!(error_log_path(), dir.path().join(ERROR_LOG));
		assert_eq!(ENV_LOGS, "MAYBRAID_LOGS");
		env::remove_var(ENV_LOGS);
	}

	#[test]
	fn platform_dir_is_the_conventional_log_root() {
		let _guard = ENV_LOCK.lock().expect("env lock");
		env::remove_var(ENV_LOGS);
		let dir = platform_log_dir();
		#[cfg(target_os = "macos")]
		assert!(dir.ends_with("Library/Logs/Maybraid"), "{}", dir.display());
		#[cfg(target_os = "windows")]
		assert!(
			dir.ends_with("Maybraid/logs") || dir.ends_with("Maybraid\\logs"),
			"{}",
			dir.display()
		);
		#[cfg(not(any(target_os = "macos", target_os = "windows")))]
		assert!(dir.ends_with("maybraid/logs"), "{}", dir.display());
	}

	#[test]
	fn append_panic_log_creates_the_file() {
		let _guard = ENV_LOCK.lock().expect("env lock");
		let dir = tempfile::tempdir().expect("tempdir");
		env::set_var(ENV_LOGS, dir.path());
		append_panic_log("guard-kill probe").expect("write");
		let text = fs::read_to_string(dir.path().join(PANIC_LOG)).expect("panic.log");
		assert!(text.contains("guard-kill probe"), "{text}");
		env::remove_var(ENV_LOGS);
	}

	#[test]
	fn append_error_log_creates_the_file() {
		let _guard = ENV_LOCK.lock().expect("env lock");
		let dir = tempfile::tempdir().expect("tempdir");
		env::set_var(ENV_LOGS, dir.path());
		append_error_log("despawned insert").expect("write");
		let text = fs::read_to_string(dir.path().join(ERROR_LOG)).expect("errors.log");
		assert!(text.contains("despawned insert"), "{text}");
		assert!(!text.contains("backtrace:"), "{text}");
		env::remove_var(ENV_LOGS);
	}
}
