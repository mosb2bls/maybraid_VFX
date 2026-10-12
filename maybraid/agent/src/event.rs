//! Line-delimited JSON events the game writes for the agent / Tracy alignment.

use serde::{Deserialize, Serialize};

use crate::command::ScenarioMark;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
	Acknowledged,
	Started,
	Finished,
	Failed,
}

/// One event. Internally tagged with `"kind"`; `t` and `frame` sit beside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEvent {
	pub t: f32,
	pub frame: u64,
	#[serde(flatten)]
	pub body: AgentEventBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEventBody {
	Ready {
		detail: String,
	},
	Player {
		x: f32,
		y: f32,
		z: f32,
		yaw: f32,
		#[serde(rename = "move")]
		r#move: [f32; 2],
	},
	Command {
		id: u32,
		status: CommandStatus,
		cmd: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		detail: Option<String>,
	},
	Waypoint {
		id: u32,
		xz: [f32; 2],
		reached: bool,
		remaining: f32,
	},
	Scenario {
		name: String,
		mark: ScenarioMark,
	},
	ChunkLoads {
		added: u32,
	},
	/// Live actor census, written when it changes. `bodies` are spawned mob
	/// characters; `brains` run threat management (the NPC mixer).
	Actors {
		hosts: u32,
		bodies: u32,
		brains: u32,
	},
	/// Offscreen present image resize or blit/camera recreate.
	Presentation {
		action: String,
		window: [u32; 2],
		target: [u32; 2],
	},
	Quit,
}

impl AgentEvent {
	pub fn to_line(&self) -> Result<String, String> {
		serde_json::to_string(self).map_err(|error| error.to_string())
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		serde_json::from_str(line.trim()).map_err(|error| error.to_string())
	}

	pub fn tracy_message(&self) -> Option<String> {
		match &self.body {
			AgentEventBody::Scenario { name, mark } => {
				Some(format!("agent.scenario {name} {mark:?}"))
			}
			AgentEventBody::Command { id, status, cmd, detail } => {
				let extra = detail.as_deref().unwrap_or("");
				Some(format!("agent.cmd #{id} {status:?} {cmd} {extra}"))
			}
			AgentEventBody::Waypoint { id, xz, reached, .. } => {
				let status = if *reached { "reached" } else { "timeout" };
				Some(format!("agent.waypoint #{id} {status} {:.1},{:.1}", xz[0], xz[1]))
			}
			AgentEventBody::Ready { detail } => Some(format!("agent.ready {detail}")),
			AgentEventBody::Actors { hosts, bodies, brains } => {
				Some(format!("agent.actors hosts={hosts} bodies={bodies} brains={brains}"))
			}
			AgentEventBody::Presentation { action, window, target } => Some(format!(
				"agent.presentation {action} window {}x{} target {}x{}",
				window[0], window[1], target[0], target[1]
			)),
			_ => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn player_roundtrip() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 1.25,
			frame: 74,
			body: AgentEventBody::Player { x: 3.0, y: 12.5, z: -1.0, yaw: 0.4, r#move: [0.0, 1.0] },
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		let parsed = AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(parsed, event);
		assert!(line.contains(r#""kind":"player""#));
		Ok(())
	}

	#[test]
	fn actors_roundtrip_and_tracy_message() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 30.0,
			frame: 1800,
			body: AgentEventBody::Actors { hosts: 4, bodies: 61, brains: 61 },
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"actors""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		assert_eq!(
			event.tracy_message().as_deref(),
			Some("agent.actors hosts=4 bodies=61 brains=61")
		);
		Ok(())
	}

	#[test]
	fn presentation_roundtrip() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 12.0,
			frame: 700,
			body: AgentEventBody::Presentation {
				action: "resize".into(),
				window: [1920, 1052],
				target: [719, 394],
			},
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"presentation""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		Ok(())
	}

	#[test]
	fn command_failed_includes_detail() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 2.0,
			frame: 10,
			body: AgentEventBody::Command {
				id: 3,
				status: CommandStatus::Failed,
				cmd: "goto 10.0,10.0".into(),
				detail: Some("timeout".into()),
			},
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains("timeout"));
		assert!(event.tracy_message().is_some());
		Ok(())
	}
}
