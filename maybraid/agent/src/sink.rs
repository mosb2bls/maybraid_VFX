//! Line-delimited event writer. Flushes every line so a killed process keeps the log.

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;

use crate::event::AgentEvent;

pub struct EventSink {
	writer: Option<Mutex<BufWriter<Box<dyn Write + Send>>>>,
}

impl EventSink {
	pub fn none() -> Self {
		Self { writer: None }
	}

	pub fn open(path: &Path) -> Result<Self, String> {
		if path == Path::new("-") {
			return Ok(Self { writer: Some(Mutex::new(BufWriter::new(Box::new(io::stdout())))) });
		}
		if let Some(parent) = path.parent() {
			if !parent.as_os_str().is_empty() {
				std::fs::create_dir_all(parent)
					.map_err(|error| format!("{}: {error}", parent.display()))?;
			}
		}
		let file: File = OpenOptions::new()
			.create(true)
			.append(true)
			.open(path)
			.map_err(|error| format!("{}: {error}", path.display()))?;
		Ok(Self { writer: Some(Mutex::new(BufWriter::new(Box::new(file)))) })
	}

	pub fn emit(&self, event: &AgentEvent) -> Result<(), String> {
		let Some(writer) = self.writer.as_ref() else {
			return Ok(());
		};
		let line = event.to_line()?;
		let mut writer = writer.lock().map_err(|error| error.to_string())?;
		writeln!(writer, "{line}").map_err(|error| error.to_string())?;
		writer.flush().map_err(|error| error.to_string())?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::event::AgentEventBody;

	#[test]
	fn writes_jsonl() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let path = dir.path().join("events.jsonl");
		let sink = EventSink::open(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
		sink.emit(&AgentEvent {
			t: 0.5,
			frame: 3,
			body: AgentEventBody::Ready { detail: "surface".into() },
		})
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let text = std::fs::read_to_string(&path)?;
		assert!(text.contains(r#""kind":"ready""#));
		assert!(text.ends_with('\n'));
		Ok(())
	}
}
