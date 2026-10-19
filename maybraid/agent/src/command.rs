//! Line-delimited JSON commands the agent (or a script file) sends.

use serde::{Deserialize, Serialize};

use crate::inventory::AgentMatcher;

/// Script API version. Bump when a command is added or its meaning changes;
/// record each bump in `maybraid/agent/README.md`.
pub const SCRIPT_API_VERSION: u32 = 7;

fn default_wait_for_timeout() -> f32 {
	10.0
}

fn default_mash_rate_hz() -> f32 {
	15.0
}

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
	/// Face **A** / Space — jump. Menu confirm.
	#[serde(alias = "a", alias = "south", alias = "confirm")]
	Jump,
	/// L3 / Shift — sprint while held.
	#[serde(alias = "l3", alias = "stick_click_move")]
	Sprint,
	/// Face **B** / Escape — squat tap, prone hold. Menu back.
	#[serde(alias = "b", alias = "east", alias = "crouch", alias = "back")]
	Stance,
	/// Face **X** / E — interact.
	#[serde(alias = "x", alias = "west", alias = "interact")]
	Interact,
	/// Face **Y** — cycle the equipped queue in the D-pad-selected category.
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
	/// Spawn at world XZ. Before the first duration command it replaces
	/// `--start-at` so the player and streaming start there; later it acts
	/// like `teleport`.
	Start { xz: [f32; 2] },
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
	///
	/// `button` is the original single-press form. `buttons` presses several
	/// in the same frame (chords). Both may be set; they are unioned.
	/// Playback consumes the frame so leftover `dt` cannot start the next
	/// command on the same tick.
	Press {
		#[serde(default, skip_serializing_if = "Option::is_none")]
		button: Option<AgentButton>,
		#[serde(default, skip_serializing_if = "Vec::is_empty")]
		buttons: Vec<AgentButton>,
	},
	/// Idle until the published snapshot matches, or `timeout` (fail).
	WaitFor {
		#[serde(flatten)]
		matcher: AgentMatcher,
		#[serde(default = "default_wait_for_timeout")]
		timeout: f32,
	},
	/// Instant check of the published snapshot. Mismatch is `failed`.
	Expect {
		#[serde(flatten)]
		matcher: AgentMatcher,
	},
	/// Seeded random presses (and chords) for `secs`. Same seed, same sequence.
	Mash {
		buttons: Vec<AgentButton>,
		#[serde(default = "default_mash_rate_hz")]
		rate_hz: f32,
		secs: f32,
		seed: u64,
	},
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
	/// Set visual style presentation mode. Accepts `native`, `sharp` (1080p
	/// offscreen), `impressionist`, `watercolor`, `retro`, `antediluvian`, or
	/// `pre_post_modern`.
	VisualStyle { style: String },
	/// Set Tanzer / Standard, or `toggle`. Does not go through the LB+RB chord.
	TanzMode { mode: String },
	/// Submit an equipped Tanz by script id (`left_strafe`, `right_strafe`,
	/// `forward_dash`, `healing_salute`). Enters Tanzer if needed.
	Tanz { id: String },
	/// Capture the presented 3D surface after render (GPU readback). `finished`
	/// waits until the PNG is written; `detail` includes mean luma so a black
	/// frame is visible in events.
	Screenshot { path: String },
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
			Self::Start { xz } => format!("start {:.1},{:.1}", xz[0], xz[1]),
			Self::Hold { .. } => "hold".into(),
			Self::Press { .. } => {
				let names = self.press_buttons();
				if names.is_empty() {
					"press".into()
				} else {
					format!(
						"press {}",
						names.iter().map(|button| button.as_str()).collect::<Vec<_>>().join("+")
					)
				}
			}
			Self::Wait { .. } => "wait".into(),
			Self::WaitFor { matcher, .. } => format!("wait_for {}", matcher.describe()),
			Self::Expect { matcher } => format!("expect {}", matcher.describe()),
			Self::Mash { seed, rate_hz, secs, .. } => {
				format!("mash seed={seed} {rate_hz}hz {secs}s")
			}
			Self::Goto { xz, .. } => format!("goto {:.1},{:.1}", xz[0], xz[1]),
			Self::Teleport { xz, .. } => format!("teleport {:.1},{:.1}", xz[0], xz[1]),
			Self::Console { line } => format!("console {line}"),
			Self::VisualStyle { style } => format!("visual_style {style}"),
			Self::TanzMode { mode } => format!("tanz_mode {mode}"),
			Self::Tanz { id } => format!("tanz {id}"),
			Self::Screenshot { path } => format!("screenshot {path}"),
			Self::Scenario { name, mark } => format!("scenario {name} {mark:?}"),
			Self::Quit => "quit".into(),
		}
	}

	pub fn is_duration(&self) -> bool {
		matches!(
			self,
			Self::Hold { .. }
				| Self::Wait { .. }
				| Self::Goto { .. }
				| Self::Press { .. }
				| Self::WaitFor { .. }
				| Self::Mash { .. }
		)
	}

	pub fn press(button: AgentButton) -> Self {
		Self::Press { button: Some(button), buttons: Vec::new() }
	}

	pub fn press_many(buttons: Vec<AgentButton>) -> Self {
		Self::Press { button: None, buttons }
	}

	pub fn press_buttons(&self) -> Vec<AgentButton> {
		let Self::Press { button, buttons } = self else {
			return Vec::new();
		};
		let mut out = Vec::new();
		if let Some(button) = *button {
			out.push(button);
		}
		for button in buttons {
			if !out.contains(button) {
				out.push(*button);
			}
		}
		out
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
		assert_eq!(jump.press_buttons(), vec![AgentButton::Jump]);
		Ok(())
	}

	#[test]
	fn parse_menu_aliases_and_press_chord() -> anyhow::Result<()> {
		let confirm = AgentCommand::parse_line(r#"{"cmd":"press","button":"confirm"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let back = AgentCommand::parse_line(r#"{"cmd":"press","button":"back"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let start = AgentCommand::parse_line(r#"{"cmd":"press","button":"start"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(confirm.press_buttons(), vec![AgentButton::Jump]);
		assert_eq!(back.press_buttons(), vec![AgentButton::Stance]);
		assert_eq!(start.press_buttons(), vec![AgentButton::Menu]);
		let chord = AgentCommand::parse_line(r#"{"cmd":"press","buttons":["confirm","start"]}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(chord.press_buttons(), vec![AgentButton::Jump, AgentButton::Menu]);
		assert_eq!(chord.label(), "press jump+menu");
		Ok(())
	}

	#[test]
	fn parse_wait_for_expect_mash() -> anyhow::Result<()> {
		let wait_for = AgentCommand::parse_line(
			r#"{"cmd":"wait_for","screen":"in_game","modal":null,"timeout":5}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			&wait_for,
			AgentCommand::WaitFor { timeout, matcher }
				if (timeout - 5.0).abs() < 1e-5 && matcher.menu.screen.as_deref() == Some("in_game") && matcher.menu.modal == Some(None)
		));
		assert!(wait_for.is_duration());
		let expect = AgentCommand::parse_line(r#"{"cmd":"expect","pause":"playing"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			&expect,
			AgentCommand::Expect { matcher } if matcher.menu.pause.as_deref() == Some("playing")
		));
		let focus = AgentCommand::parse_line(
			r#"{"cmd":"wait_for","screen":"home","focus_label":"Characters","timeout":3}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			&focus,
			AgentCommand::WaitFor { matcher, .. }
				if matcher.menu.screen.as_deref() == Some("home")
					&& matcher.menu.focus_label.as_deref() == Some("Characters")
		));
		let cycle = AgentCommand::parse_line(
			r#"{"cmd":"expect","cycle_category":"weapons","weapon":"bullpup"}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			&cycle,
			AgentCommand::Expect { matcher }
				if matcher.inventory.cycle_category.as_deref() == Some("weapons")
					&& matcher.inventory.weapon.as_deref() == Some("bullpup")
		));
		assert!(!expect.is_duration());
		let mash = AgentCommand::parse_line(
			r#"{"cmd":"mash","buttons":["confirm","back","start"],"rate_hz":15,"secs":60,"seed":42}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			mash,
			AgentCommand::Mash { seed: 42, secs, .. } if (secs - 60.0).abs() < 1e-5
		));
		assert_eq!(mash.label(), "mash seed=42 15hz 60s");
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
		let visual_style =
			AgentCommand::parse_line(r#"{"cmd":"visual_style","style":"impressionist"}"#)
				.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(visual_style, AgentCommand::VisualStyle { style: "impressionist".into() });
		let native = AgentCommand::parse_line(r#"{"cmd":"visual_style","style":"native"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(native, AgentCommand::VisualStyle { style: "native".into() });
		Ok(())
	}

	#[test]
	fn parse_tanz_and_screenshot() -> anyhow::Result<()> {
		let mode = AgentCommand::parse_line(r#"{"cmd":"tanz_mode","mode":"tanzer"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(mode, AgentCommand::TanzMode { mode: "tanzer".into() });
		assert_eq!(mode.label(), "tanz_mode tanzer");
		let tanz = AgentCommand::parse_line(r#"{"cmd":"tanz","id":"left_strafe"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(tanz, AgentCommand::Tanz { id: "left_strafe".into() });
		assert_eq!(tanz.label(), "tanz left_strafe");
		let shot = AgentCommand::parse_line(r#"{"cmd":"screenshot","path":"/tmp/tanz.png"}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(shot, AgentCommand::Screenshot { path: "/tmp/tanz.png".into() });
		assert_eq!(shot.label(), "screenshot /tmp/tanz.png");
		assert!(!shot.is_duration());
		Ok(())
	}

	#[test]
	fn parse_start() -> anyhow::Result<()> {
		let cmd = AgentCommand::parse_line(r#"{"cmd":"start","xz":[-1500,-600]}"#)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(cmd, AgentCommand::Start { xz: [-1500.0, -600.0] });
		assert!(!cmd.is_duration());
		Ok(())
	}

	#[test]
	fn reject_unknown_cmd() {
		assert!(AgentCommand::parse_line(r#"{"cmd":"fly"}"#).is_err());
		assert!(AgentCommand::parse_line("not json").is_err());
	}
}
