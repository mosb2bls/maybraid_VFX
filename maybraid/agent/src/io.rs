//! Background line reader for a live pipe (fifo, file, or stdin).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

/// Non-blocking inbox. A helper thread blocks on `read_line`.
pub struct LineInbox {
	rx: Receiver<String>,
}

impl LineInbox {
	pub fn spawn(path: PathBuf) -> Result<Self, String> {
		let (tx, rx) = mpsc::channel();
		let label = path.display().to_string();
		thread::Builder::new()
			.name("maybraid-agent-pipe".into())
			.spawn(move || read_loop(path, tx))
			.map_err(|error| format!("agent pipe {label}: {error}"))?;
		Ok(Self { rx })
	}

	pub fn try_recv_all(&self) -> Vec<String> {
		let mut lines = Vec::new();
		loop {
			match self.rx.try_recv() {
				Ok(line) => lines.push(line),
				Err(TryRecvError::Empty) => break,
				Err(TryRecvError::Disconnected) => break,
			}
		}
		lines
	}
}

fn read_loop(path: PathBuf, tx: mpsc::Sender<String>) {
	let reader: Box<dyn BufRead> = if path == Path::new("-") {
		Box::new(BufReader::new(std::io::stdin()))
	} else {
		match std::fs::File::open(&path) {
			Ok(file) => Box::new(BufReader::new(file)),
			Err(error) => {
				eprintln!("maybraid-agent: pipe {}: {error}", path.display());
				return;
			}
		}
	};
	for line in reader.lines() {
		match line {
			Ok(line) => {
				if tx.send(line).is_err() {
					break;
				}
			}
			Err(_) => break,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;

	#[test]
	fn reads_lines_from_a_file() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let path = dir.path().join("pipe.jsonl");
		{
			let mut file = std::fs::File::create(&path)?;
			writeln!(file, r#"{{"cmd":"wait","secs":1}}"#)?;
			writeln!(file, r#"{{"cmd":"quit"}}"#)?;
		}
		let inbox = LineInbox::spawn(path).map_err(|e| anyhow::anyhow!("{e}"))?;
		let mut lines = Vec::new();
		for _ in 0..50 {
			lines.extend(inbox.try_recv_all());
			if lines.len() >= 2 {
				break;
			}
			std::thread::sleep(std::time::Duration::from_millis(10));
		}
		assert_eq!(lines.len(), 2);
		assert!(lines[0].contains("wait"));
		Ok(())
	}
}
