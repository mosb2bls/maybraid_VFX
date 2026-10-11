//! Sequential, game-clock playback. No I/O — the world plugin supplies `dt` and pose.

use std::collections::VecDeque;

use crate::command::AgentCommand;
use crate::pad::{camera_relative_move, xz_distance, PadIntent};

const MAX_INSTANTS_PER_TICK: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct TickView {
	pub player_xz: Option<[f32; 2]>,
	pub yaw: f32,
}

impl Default for TickView {
	fn default() -> Self {
		Self { player_xz: None, yaw: 0.0 }
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum TickEffect {
	Started { id: u32, cmd: String },
	Finished { id: u32, cmd: String },
	Failed { id: u32, cmd: String, detail: String },
	Instant { id: u32, command: AgentCommand },
	Waypoint { id: u32, xz: [f32; 2], reached: bool, remaining: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
	pub intent: PadIntent,
	pub effects: Vec<TickEffect>,
	pub idle: bool,
}

#[derive(Debug, Clone)]
struct Scheduled {
	id: u32,
	command: AgentCommand,
}

#[derive(Debug, Clone)]
enum Active {
	Wait {
		id: u32,
		remaining: f32,
	},
	Hold {
		id: u32,
		remaining: f32,
		intent: PadIntent,
	},
	Goto {
		id: u32,
		xz: [f32; 2],
		timeout: f32,
		radius: f32,
		sprint: bool,
		look: Option<[f32; 2]>,
		elapsed: f32,
	},
}

#[derive(Debug, Clone, Default)]
pub struct Playback {
	queue: VecDeque<Scheduled>,
	active: Option<Active>,
	next_id: u32,
}

impl Playback {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn from_commands(commands: impl IntoIterator<Item = AgentCommand>) -> Self {
		let mut playback = Self::new();
		for command in commands {
			playback.push(command);
		}
		playback
	}

	pub fn push(&mut self, command: AgentCommand) -> u32 {
		let id = self.next_id;
		self.next_id += 1;
		self.queue.push_back(Scheduled { id, command });
		id
	}

	pub fn is_idle(&self) -> bool {
		self.queue.is_empty() && self.active.is_none()
	}

	pub fn tick(&mut self, mut dt: f32, view: &TickView) -> Tick {
		let mut effects = Vec::new();
		let mut intent = PadIntent::default();
		let mut instants = 0;

		loop {
			if self.active.is_none() {
				if instants >= MAX_INSTANTS_PER_TICK {
					break;
				}
				let Some(next) = self.queue.pop_front() else {
					break;
				};
				effects.push(TickEffect::Started { id: next.id, cmd: next.command.label() });
				if start_command(next, &mut self.active, &mut effects, &mut intent) {
					instants += 1;
					continue;
				}
			}

			let Some(active) = self.active.take() else {
				break;
			};
			match step_active(active, dt, view, &mut intent, &mut effects) {
				Step::Continue(next) => {
					self.active = Some(next);
					break;
				}
				Step::Done { leftover } => {
					dt = leftover;
					if dt <= 0.0 && self.queue.is_empty() {
						break;
					}
				}
			}
		}

		Tick { idle: self.is_idle(), intent, effects }
	}
}

enum Step {
	Continue(Active),
	Done { leftover: f32 },
}

/// Returns `true` when the command finished in the same tick (instant / press).
fn start_command(
	scheduled: Scheduled,
	active: &mut Option<Active>,
	effects: &mut Vec<TickEffect>,
	intent: &mut PadIntent,
) -> bool {
	let id = scheduled.id;
	let label = scheduled.command.label();
	match scheduled.command {
		AgentCommand::Wait { secs } if secs > 0.0 => {
			*active = Some(Active::Wait { id, remaining: secs });
			false
		}
		AgentCommand::Wait { .. } => {
			effects.push(TickEffect::Finished { id, cmd: label });
			true
		}
		AgentCommand::Hold { r#move, look, focus, fire, buttons, secs } if secs > 0.0 => {
			*active = Some(Active::Hold {
				id,
				remaining: secs,
				intent: PadIntent::from_hold(r#move, look, focus, fire, &buttons),
			});
			false
		}
		AgentCommand::Hold { r#move, look, focus, fire, buttons, .. } => {
			*intent = PadIntent::from_hold(r#move, look, focus, fire, &buttons);
			effects.push(TickEffect::Finished { id, cmd: label });
			true
		}
		AgentCommand::Press { button } => {
			*intent = PadIntent::press(button);
			effects.push(TickEffect::Finished { id, cmd: label });
			true
		}
		AgentCommand::Goto { xz, timeout, radius, sprint, look } => {
			*active = Some(Active::Goto { id, xz, timeout, radius, sprint, look, elapsed: 0.0 });
			false
		}
		command => {
			effects.push(TickEffect::Instant { id, command });
			effects.push(TickEffect::Finished { id, cmd: label });
			true
		}
	}
}

fn step_active(
	active: Active,
	dt: f32,
	view: &TickView,
	intent: &mut PadIntent,
	effects: &mut Vec<TickEffect>,
) -> Step {
	match active {
		Active::Wait { id, remaining } => {
			if dt >= remaining {
				effects.push(TickEffect::Finished { id, cmd: "wait".into() });
				Step::Done { leftover: dt - remaining }
			} else {
				Step::Continue(Active::Wait { id, remaining: remaining - dt })
			}
		}
		Active::Hold { id, remaining, intent: hold } => {
			*intent = hold.clone();
			if dt >= remaining {
				effects.push(TickEffect::Finished { id, cmd: "hold".into() });
				Step::Done { leftover: dt - remaining }
			} else {
				Step::Continue(Active::Hold { id, remaining: remaining - dt, intent: hold })
			}
		}
		Active::Goto { id, xz, timeout, radius, sprint, look, elapsed } => {
			let elapsed = elapsed + dt;
			if let Some(from) = view.player_xz {
				let remaining = xz_distance(from, xz);
				if remaining <= radius {
					effects.push(TickEffect::Waypoint { id, xz, reached: true, remaining });
					effects.push(TickEffect::Finished {
						id,
						cmd: format!("goto {:.1},{:.1}", xz[0], xz[1]),
					});
					return Step::Done { leftover: 0.0 };
				}
				*intent = steer_goto(from, xz, view.yaw, sprint, look);
			}
			if elapsed >= timeout {
				let remaining = view.player_xz.map(|from| xz_distance(from, xz)).unwrap_or(timeout);
				effects.push(TickEffect::Waypoint { id, xz, reached: false, remaining });
				effects.push(TickEffect::Failed {
					id,
					cmd: format!("goto {:.1},{:.1}", xz[0], xz[1]),
					detail: "timeout".into(),
				});
				return Step::Done { leftover: 0.0 };
			}
			Step::Continue(Active::Goto { id, xz, timeout, radius, sprint, look, elapsed })
		}
	}
}

fn steer_goto(
	from: [f32; 2],
	to: [f32; 2],
	yaw: f32,
	sprint: bool,
	look: Option<[f32; 2]>,
) -> PadIntent {
	let mut intent = PadIntent {
		move_stick: camera_relative_move(from, to, yaw),
		look_stick: look.map(crate::pad::clamp_stick).unwrap_or([0.0, 0.0]),
		..PadIntent::default()
	};
	if sprint {
		intent.hold_button(crate::command::AgentButton::Sprint);
	}
	intent
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::command::{AgentButton, ScenarioMark};

	fn started(effects: &[TickEffect]) -> Vec<u32> {
		effects
			.iter()
			.filter_map(|effect| match effect {
				TickEffect::Started { id, .. } => Some(*id),
				_ => None,
			})
			.collect()
	}

	#[test]
	fn wait_then_hold_uses_leftover_dt() -> anyhow::Result<()> {
		let mut playback = Playback::from_commands([
			AgentCommand::Wait { secs: 1.0 },
			AgentCommand::Hold {
				r#move: Some([0.0, 1.0]),
				look: None,
				focus: None,
				fire: None,
				buttons: vec![],
				secs: 0.5,
			},
		]);
		let first = playback.tick(0.4, &TickView::default());
		assert!(first.intent.is_idle());
		assert_eq!(started(&first.effects), vec![0]);
		assert!(!first.idle);

		let second = playback.tick(0.7, &TickView::default());
		assert!(second
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Finished { id: 0, .. })));
		assert!(second
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Started { id: 1, .. })));
		assert_eq!(second.intent.move_stick, [0.0, 1.0]);
		assert!(!second.idle);

		let third = playback.tick(0.5, &TickView::default());
		assert!(third.idle);
		assert!(third
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Finished { id: 1, .. })));
		Ok(())
	}

	#[test]
	fn instants_chain_on_one_tick() -> anyhow::Result<()> {
		let mut playback = Playback::from_commands([
			AgentCommand::Scenario { name: "route".into(), mark: ScenarioMark::Start },
			AgentCommand::Console { line: "stats vsync".into() },
			AgentCommand::Wait { secs: 1.0 },
		]);
		let tick = playback.tick(0.016, &TickView::default());
		assert_eq!(started(&tick.effects), vec![0, 1, 2]);
		assert_eq!(
			tick.effects
				.iter()
				.filter(|effect| matches!(effect, TickEffect::Instant { .. }))
				.count(),
			2
		);
		assert!(!tick.idle);
		Ok(())
	}

	#[test]
	fn press_is_one_frame() -> anyhow::Result<()> {
		let mut playback =
			Playback::from_commands([AgentCommand::Press { button: AgentButton::Jump }]);
		let tick = playback.tick(0.016, &TickView::default());
		assert_eq!(tick.intent.buttons, vec![AgentButton::Jump]);
		assert!(tick.idle);
		Ok(())
	}

	#[test]
	fn goto_reaches_and_times_out() -> anyhow::Result<()> {
		let mut reached = Playback::from_commands([AgentCommand::Goto {
			xz: [10.0, 0.0],
			timeout: 5.0,
			radius: 3.0,
			sprint: false,
			look: None,
		}]);
		let near = reached.tick(0.1, &TickView { player_xz: Some([8.0, 0.0]), yaw: 0.0 });
		assert!(near
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Waypoint { reached: true, .. })));
		assert!(near.idle);

		let mut timed = Playback::from_commands([AgentCommand::Goto {
			xz: [100.0, 0.0],
			timeout: 0.2,
			radius: 3.0,
			sprint: true,
			look: Some([0.4, 0.0]),
		}]);
		let first = timed.tick(0.1, &TickView { player_xz: Some([0.0, 0.0]), yaw: 0.0 });
		assert!(first.intent.buttons.contains(&AgentButton::Sprint));
		assert!(first.intent.move_stick[0] > 0.5);
		assert!((first.intent.look_stick[0] - 0.4).abs() < 1e-5);
		let second = timed.tick(0.2, &TickView { player_xz: Some([1.0, 0.0]), yaw: 0.0 });
		assert!(second.effects.iter().any(|effect| matches!(
			effect,
			TickEffect::Failed { detail, .. } if detail == "timeout"
		)));
		Ok(())
	}

	#[test]
	fn live_push_appends() {
		let mut playback = Playback::new();
		assert!(playback.tick(0.016, &TickView::default()).idle);
		playback.push(AgentCommand::Wait { secs: 1.0 });
		assert!(!playback.tick(0.016, &TickView::default()).idle);
	}
}
