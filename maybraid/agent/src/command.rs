//! Line-delimited JSON commands the agent (or a script file) sends.

use serde::{Deserialize, Serialize};

const DEFAULT_GOTO_TIMEOUT: f32 = 30.0;
const DEFAULT_GOTO_RADIUS: f32 = 3.0;

fn default_goto_timeout() -> f32 {
	DEFAULT_GOTO_TIMEOUT
}

fn default_goto_radius() -> f32 {
	DEFAULT_GOTO_RADIUS
}

/// Virtual-pad buttons the agent can press or hold.
///
/// Names match gameplay, not Xbox letters. Serde aliases accept the pad names
/// the controllers already use (`a`, `l3`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentButton {
	/// Face **A** / Space — jump.
	#[serde(alias = "a", alias = "south")]
	Jump,
	/// L3 / Shift — sprint while held.
	#[serde(alias = "l3", alias = "stick_click_move")]
	Sprint,
	/// Face **B** / Escape — squat tap, prone hold.
	#[serde(alias = "b", alias = "east", alias = "crouch")]
	Stance,
	/// Face **X** / E — interact.
	#[serde(alias = "x", alias = "west", alias = "interact")]
	Interact,
	/// Face **Y** — swap active weapon.
	#[serde(alias = "y", alias = "north", alias = "swap")]
	Swap,
	/// Left bumper / C — iron ADS.
	#[serde(alias = "lb", alias = "bumper_focus", alias = "ads")]
	Ads,
	/// Right bumper.
	#[serde(alias = "rb", alias = "bumper_fire")]
	BumperFire,
	/// Right stick click — swap POV.
	#[serde(alias = "r3", alias = "stick_click_look")]
	SwapPov,
	/// Left trigger analog (also digital past the press threshold).
	#[serde(alias = "lt", alias = "trigger_focus")]
	Focus,
	/// Right trigger analog — fire / use.
	#[serde(alias = "rt", alias = "trigger_fire", alias = "fire")]
	Fire,
	#[serde(alias = "start")]
	Menu,
	#[serde(alias = "select", alias = "view")]
	Map,
	DpadUp,
	DpadDown,
	DpadLeft,
	DpadRight,
}

impl AgentButton {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Jump => "jump",
			Self::Sprint => "sprint",
			Self::Stance => "stance",
			Self::Interact => "interact",
			Self::Swap => "swap",
			Self::Ads => "ads",
			Self::BumperFire => "bumper_fire",
			Self::SwapPov => "swap_pov",
			Self::Focus => "focus",
			Self::Fire => "fire",
			Self::Menu => "menu",
			Self::Map => "map",
			Self::DpadUp => "dpad_up",
			Self::DpadDown => "dpad_down",
			Self::DpadLeft => "dpad_left",
			Self::DpadRight => "dpad_right",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioMark {
	Start,
	End,
}

/// One agent command. Internally tagged with `"cmd"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum AgentCommand {
	/// Hold sticks / buttons for `secs` of game time.
	Hold {
		#[serde(default, skip_serializing_if = "Option::is_none")]
		r#move: Option<[f32; 2]>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		look: Option<[f32; 2]>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		focus: Option<f32>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		fire: Option<f32>,
		#[serde(default, skip_serializing_if = "Vec::is_empty")]
		buttons: Vec<AgentButton>,
		secs: f32,
	},
	/// One-frame digital press (just_pressed on the virtual pad).
	Press { button: AgentButton },
	/// Idle for `secs` of game time. Overlay is cleared.
	Wait { secs: f32 },
	/// Camera-relative walk toward world XZ until reached or `timeout`.
	Goto {
		xz: [f32; 2],
		#[serde(default = "default_goto_timeout")]
		timeout: f32,
		#[serde(default = "default_goto_radius")]
		radius: f32,
		#[serde(default)]
		sprint: bool,
		/// Optional look-stick while steering (same gamepad −1..1 as `hold`).
		#[serde(default, skip_serializing_if = "Option::is_none")]
		look: Option<[f32; 2]>,
	},
	/// Utility: snap the player to XZ (Y from terrain when omitted).
	Teleport {
		xz: [f32; 2],
		#[serde(default, skip_serializing_if = "Option::is_none")]
		y: Option<f32>,
	},
	/// Existing `/` console line (`stats vsync`, `sky noon`, …).
	Console { line: String },
	/// Scenario marker (also a Tracy message when Tracy is enabled).
	Scenario { name: String, mark: ScenarioMark },
	/// Request process exit after the current tick's events flush.
	Quit,
}

impl AgentCommand {
	pub fn parse_line(line: &str) -> Result<Self, String> {
		let line = line.trim();
		if line.is_empty() {
			return Err("empty command".into());
		}
		serde_json::from_str(line).map_err(|error| error.to_string())
	}

	pub fn label(&self) -> String {
		match self {
			Self::Hold { .. } => "hold".into(),
			Self::Press { button } => format!("press {}", button.as_str()),
			Self::Wait { .. } => "wait".into(),
			Self::Goto { xz, .. } => format!("goto {:.1},{:.1}", xz[0], xz[1]),
			Self::Teleport { xz, .. } => format!("teleport {:.1},{:.1}", xz[0], xz[1]),
			Self::Console { line } => format!("console {line}"),
			Self::Scenario { name, mark } => format!("scenario {name} {mark:?}"),
			Self::Quit => "quit".into(),
		}
	}

	pub fn is_duration(&self) -> bool {
		matches!(
			self,
			Self::Hold { .. } | Self::Wait { .. } | Self::Goto { .. } | Self::Press { .. }
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_hold_move() -> anyhow::Result<()> {
		let cmd = AgentCommand::parse_line(r#"{"cmd":"hold","move":[0,1],"secs":2.5}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(
			cmd,
			AgentCommand::Hold {
				r#move: Some([0.0, 1.0]),
				look: None,
				focus: None,
				fire: None,
				buttons: vec![],
				secs: 2.5,
			}
		);
		Ok(())
	}

	#[test]
	fn parse_press_aliases() -> anyhow::Result<()> {
		let jump = AgentCommand::parse_line(r#"{"cmd":"press","button":"jump"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let a = AgentCommand::parse_line(r#"{"cmd":"press","button":"a"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(jump, a);
		assert!(matches!(jump, AgentCommand::Press { button: AgentButton::Jump }));
		Ok(())
	}

	#[test]
	fn parse_goto_defaults() -> anyhow::Result<()> {
		let cmd = AgentCommand::parse_line(r#"{"cmd":"goto","xz":[80,-20]}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(
			cmd,
			AgentCommand::Goto {
				xz: [80.0, -20.0],
				timeout: DEFAULT_GOTO_TIMEOUT,
				radius: DEFAULT_GOTO_RADIUS,
				sprint: false,
				look: None,
			}
		);
		Ok(())
	}

	#[test]
	fn parse_goto_look() -> anyhow::Result<()> {
		let cmd = AgentCommand::parse_line(
			r#"{"cmd":"goto","xz":[40,10],"sprint":true,"look":[0.25,0]}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(
			cmd,
			AgentCommand::Goto {
				xz: [40.0, 10.0],
				timeout: DEFAULT_GOTO_TIMEOUT,
				radius: DEFAULT_GOTO_RADIUS,
				sprint: true,
				look: Some([0.25, 0.0]),
			}
		);
		Ok(())
	}

	#[test]
	fn parse_scenario_and_console() -> anyhow::Result<()> {
		let start = AgentCommand::parse_line(
			r#"{"cmd":"scenario","name":"biome-traverse","mark":"start"}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(start, AgentCommand::Scenario { mark: ScenarioMark::Start, .. }));
		let console = AgentCommand::parse_line(r#"{"cmd":"console","line":"stats vsync"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(console, AgentCommand::Console { line: "stats vsync".into() });
		Ok(())
	}

	#[test]
	fn reject_unknown_cmd() {
		assert!(AgentCommand::parse_line(r#"{"cmd":"fly"}"#).is_err());
		assert!(AgentCommand::parse_line("not json").is_err());
	}
}
