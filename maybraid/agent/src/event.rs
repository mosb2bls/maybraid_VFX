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
		/// Composed Durham height under the capsule, when that cell is published.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		ground: Option<f32>,
		/// Overhead map is visible (player-toggled or locked picker).
		#[serde(default, skip_serializing_if = "std::ops::Not::not")]
		map_open: bool,
		/// Locked first-life / death spawn picker (`map.open && close_locked`).
		#[serde(default, skip_serializing_if = "std::ops::Not::not")]
		picker: bool,
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
		/// Lowest / highest mob-body Y when `bodies > 0`.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_y: Option<f32>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		max_y: Option<f32>,
		/// Composed height under the lowest body, when that cell is published.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		ground_at_min_y: Option<f32>,
	},
	/// Locked first-life / death spawn picker opened or closed.
	Picker {
		open: bool,
	},
	/// Offscreen present image resize or blit/camera recreate.
	Presentation {
		action: String,
		window: [u32; 2],
		target: [u32; 2],
	},
	/// Live menu screen changed (`home`, `gallery`, …, `none`).
	Screen {
		name: String,
	},
	/// A leave prompt, name modal, or catalog overlay opened or closed.
	Modal {
		name: Option<String>,
	},
	/// Focused menu row changed.
	Focus {
		screen: String,
		row: u32,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		label: Option<String>,
	},
	/// `GameFlow` or `WorldPause` transition.
	Flow {
		flow: String,
		pause: String,
	},
	/// A menu or cycling invariant broke. Always a playtest failure.
	Invariant {
		detail: String,
	},
	/// D-pad-selected inventory cycle category changed.
	Cycle {
		category: String,
	},
	/// Equipped / active item in a cycle category changed.
	Equipped {
		category: String,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		item: Option<String>,
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
			AgentEventBody::Picker { open } => Some(format!("agent.picker open={open}")),
			AgentEventBody::Actors { hosts, bodies, brains, min_y, max_y, ground_at_min_y } => {
				let mut message =
					format!("agent.actors hosts={hosts} bodies={bodies} brains={brains}");
				if let Some(min_y) = min_y {
					message.push_str(&format!(" min_y={min_y:.1}"));
				}
				if let Some(max_y) = max_y {
					message.push_str(&format!(" max_y={max_y:.1}"));
				}
				if let Some(ground) = ground_at_min_y {
					message.push_str(&format!(" ground={ground:.1}"));
				}
				Some(message)
			}
			AgentEventBody::Presentation { action, window, target } => Some(format!(
				"agent.presentation {action} window {}x{} target {}x{}",
				window[0], window[1], target[0], target[1]
			)),
			AgentEventBody::Screen { name } => Some(format!("agent.screen {name}")),
			AgentEventBody::Modal { name } => {
				Some(format!("agent.modal {}", name.as_deref().unwrap_or("none")))
			}
			AgentEventBody::Focus { screen, row, label } => {
				let extra = label.as_deref().unwrap_or("");
				Some(format!("agent.focus {screen} row={row} {extra}"))
			}
			AgentEventBody::Flow { flow, pause } => {
				Some(format!("agent.flow {flow} pause={pause}"))
			}
			AgentEventBody::Invariant { detail } => Some(format!("agent.invariant {detail}")),
			AgentEventBody::Cycle { category } => Some(format!("agent.cycle {category}")),
			AgentEventBody::Equipped { category, item } => {
				Some(format!("agent.equipped {category} {}", item.as_deref().unwrap_or("none")))
			}
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
			body: AgentEventBody::Player {
				x: 3.0,
				y: 12.5,
				z: -1.0,
				yaw: 0.4,
				r#move: [0.0, 1.0],
				ground: Some(11.4),
				map_open: false,
				picker: false,
			},
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
			body: AgentEventBody::Actors {
				hosts: 4,
				bodies: 61,
				brains: 61,
				min_y: Some(-2.0),
				max_y: Some(14.0),
				ground_at_min_y: Some(0.5),
			},
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"actors""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		assert_eq!(
			event.tracy_message().as_deref(),
			Some("agent.actors hosts=4 bodies=61 brains=61 min_y=-2.0 max_y=14.0 ground=0.5")
		);
		Ok(())
	}

	#[test]
	fn older_player_and_actors_lines_still_parse() -> anyhow::Result<()> {
		let player = AgentEvent::parse_line(
			r#"{"t":1.0,"frame":10,"kind":"player","x":0.0,"y":1.4,"z":0.0,"yaw":0.0,"move":[0,0]}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			player.body,
			AgentEventBody::Player { ground: None, map_open: false, picker: false, .. }
		));
		let actors = AgentEvent::parse_line(
			r#"{"t":2.0,"frame":20,"kind":"actors","hosts":1,"bodies":3,"brains":3}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			actors.body,
			AgentEventBody::Actors { min_y: None, max_y: None, ground_at_min_y: None, .. }
		));
		Ok(())
	}

	#[test]
	fn picker_roundtrip_and_tracy_message() -> anyhow::Result<()> {
		let event = AgentEvent { t: 8.0, frame: 480, body: AgentEventBody::Picker { open: true } };
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"picker""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		assert_eq!(event.tracy_message().as_deref(), Some("agent.picker open=true"));
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
	fn menu_events_roundtrip() -> anyhow::Result<()> {
		for body in [
			AgentEventBody::Screen { name: "in_game".into() },
			AgentEventBody::Modal { name: Some("discard".into()) },
			AgentEventBody::Modal { name: None },
			AgentEventBody::Focus {
				screen: "home".into(),
				row: 2,
				label: Some("Characters".into()),
			},
			AgentEventBody::Flow { flow: "world".into(), pause: "menu".into() },
			AgentEventBody::Invariant { detail: "multiple MenuScreen (2)".into() },
			AgentEventBody::Cycle { category: "skill_maps".into() },
			AgentEventBody::Equipped { category: "weapons".into(), item: Some("reltor".into()) },
		] {
			let event = AgentEvent { t: 1.0, frame: 12, body };
			let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
			assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
			assert!(event.tracy_message().is_some(), "{line}");
		}
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
