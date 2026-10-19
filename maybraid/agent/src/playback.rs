//! Sequential, game-clock playback. No I/O — the world plugin supplies `dt` and pose.

use std::collections::VecDeque;

use crate::command::AgentCommand;
use crate::inventory::{AgentMatcher, InventoryObservation};
use crate::menu::{mash_press, MenuObservation};
use crate::pad::{camera_relative_move, xz_distance, PadIntent};

const MAX_INSTANTS_PER_TICK: usize = 64;

/// Metres of remaining-distance improvement that counts as making progress.
const STUCK_PROGRESS_M: f32 = 0.35;
/// Stall seconds before the first jump pulse.
const STUCK_JUMP_AFTER: f32 = 1.4;
const STUCK_JUMP_PERIOD: f32 = 1.2;
/// Stall seconds before adding a lateral stick offset (alternates sides).
const STUCK_STRAFE_AFTER: f32 = 2.2;
const STUCK_STRAFE_PERIOD: f32 = 1.1;
/// Give up instead of burning the remaining `goto` timeout against terrain.
const STUCK_GIVE_UP: f32 = 5.5;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TickView {
	pub player_xz: Option<[f32; 2]>,
	pub yaw: f32,
	pub menu: MenuObservation,
	pub inventory: InventoryObservation,
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
		last_remaining: f32,
		stall_secs: f32,
	},
	WaitFor {
		id: u32,
		matcher: AgentMatcher,
		remaining: f32,
	},
	Mash {
		id: u32,
		buttons: Vec<crate::command::AgentButton>,
		interval: f32,
		remaining: f32,
		cooldown: f32,
		rng: u64,
	},
}

#[derive(Debug, Clone, Default)]
pub struct Playback {
	queue: VecDeque<Scheduled>,
	active: Option<Active>,
	next_id: u32,
	/// After a digital press, the next tick must release so `just_pressed` can fire again.
	need_release: bool,
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
		let releasing = self.need_release;
		self.need_release = false;

		loop {
			if self.active.is_none() {
				if releasing {
					// Keep the pad up for one frame so the next press edges.
					break;
				}
				if instants >= MAX_INSTANTS_PER_TICK {
					break;
				}
				let Some(next) = self.queue.pop_front() else {
					break;
				};
				effects.push(TickEffect::Started { id: next.id, cmd: next.command.label() });
				if start_command(next, view, &mut self.active, &mut effects, &mut intent) {
					if failed(&effects) {
						self.queue.clear();
						self.active = None;
						break;
					}
					instants += 1;
					continue;
				}
			}

			let Some(active) = self.active.take() else {
				break;
			};
			match step_active(active, dt, view, &mut intent, &mut effects, releasing) {
				Step::Continue(next) => {
					self.active = Some(next);
					break;
				}
				Step::Done { leftover } => {
					if failed(&effects) {
						self.queue.clear();
						break;
					}
					dt = leftover;
					if dt <= 0.0 && self.queue.is_empty() {
						break;
					}
				}
			}
		}

		if !intent.buttons.is_empty() {
			self.need_release = true;
		}

		Tick { idle: self.is_idle(), intent, effects }
	}
}

enum Step {
	Continue(Active),
	Done { leftover: f32 },
}

fn failed(effects: &[TickEffect]) -> bool {
	effects.iter().any(|effect| matches!(effect, TickEffect::Failed { .. }))
}

/// Returns `true` when the command finished in the same tick (instant / press).
fn start_command(
	scheduled: Scheduled,
	view: &TickView,
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
		command @ AgentCommand::Press { .. } => {
			*intent = PadIntent::press_many(&command.press_buttons());
			effects.push(TickEffect::Finished { id, cmd: label });
			// Consume the frame. Leftover `dt` must not start the next `press`
			// (or eat a short `wait` and then press again) on a fat tick.
			false
		}
		AgentCommand::WaitFor { matcher, timeout } => {
			*active = Some(Active::WaitFor { id, matcher, remaining: timeout.max(0.0) });
			false
		}
		AgentCommand::Expect { matcher } => {
			if matcher.matches(&view.menu, &view.inventory) {
				effects.push(TickEffect::Finished { id, cmd: label });
			} else {
				effects.push(TickEffect::Failed {
					id,
					cmd: label,
					detail: matcher.mismatch_detail(&view.menu, &view.inventory),
				});
			}
			true
		}
		AgentCommand::Mash { buttons, rate_hz, secs, seed } if secs > 0.0 => {
			let rate = rate_hz.max(0.1);
			*active = Some(Active::Mash {
				id,
				buttons,
				interval: 1.0 / rate,
				remaining: secs,
				cooldown: 0.0,
				rng: seed,
			});
			false
		}
		AgentCommand::Mash { .. } => {
			effects.push(TickEffect::Finished { id, cmd: label });
			true
		}
		AgentCommand::Goto { xz, timeout, radius, sprint, look } => {
			*active = Some(Active::Goto {
				id,
				xz,
				timeout,
				radius,
				sprint,
				look,
				elapsed: 0.0,
				last_remaining: f32::MAX,
				stall_secs: 0.0,
			});
			false
		}
		command @ AgentCommand::VisualStyle { .. } => {
			effects.push(TickEffect::Instant { id, command });
			true
		}
		command @ AgentCommand::Screenshot { .. } => {
			effects.push(TickEffect::Instant { id, command });
			true
		}
		command @ AgentCommand::TanzMode { .. } => {
			effects.push(TickEffect::Instant { id, command });
			true
		}
		command @ AgentCommand::Tanz { .. } => {
			effects.push(TickEffect::Instant { id, command });
			true
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
	releasing: bool,
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
		Active::Goto {
			id,
			xz,
			timeout,
			radius,
			sprint,
			look,
			elapsed,
			last_remaining,
			stall_secs,
		} => {
			let elapsed = elapsed + dt;
			let mut last_remaining = last_remaining;
			let mut stall_secs = stall_secs;
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
				if remaining + STUCK_PROGRESS_M < last_remaining {
					last_remaining = remaining;
					stall_secs = 0.0;
				} else {
					stall_secs += dt;
				}
				if stall_secs >= STUCK_GIVE_UP {
					return fail_goto(id, xz, remaining, "stuck", effects);
				}
				*intent = steer_goto(from, xz, view.yaw, sprint, look);
				apply_unstuck(intent, stall_secs, dt);
			}
			if elapsed >= timeout {
				let remaining = view.player_xz.map(|from| xz_distance(from, xz)).unwrap_or(timeout);
				return fail_goto(id, xz, remaining, "timeout", effects);
			}
			Step::Continue(Active::Goto {
				id,
				xz,
				timeout,
				radius,
				sprint,
				look,
				elapsed,
				last_remaining,
				stall_secs,
			})
		}
		Active::WaitFor { id, matcher, remaining } => {
			if matcher.matches(&view.menu, &view.inventory) {
				effects.push(TickEffect::Finished {
					id,
					cmd: format!("wait_for {}", matcher.describe()),
				});
				return Step::Done { leftover: dt };
			}
			if dt >= remaining {
				effects.push(TickEffect::Failed {
					id,
					cmd: format!("wait_for {}", matcher.describe()),
					detail: format!(
						"timeout {}",
						matcher.mismatch_detail(&view.menu, &view.inventory)
					),
				});
				Step::Done { leftover: dt - remaining }
			} else {
				Step::Continue(Active::WaitFor { id, matcher, remaining: remaining - dt })
			}
		}
		Active::Mash { id, buttons, interval, remaining, cooldown, mut rng } => {
			let fire = cooldown <= 0.0 && !releasing;
			if fire {
				*intent = PadIntent::press_many(&mash_press(&mut rng, &buttons));
			}
			if dt >= remaining {
				effects.push(TickEffect::Finished { id, cmd: "mash".into() });
				Step::Done { leftover: dt - remaining }
			} else {
				let next_cooldown = if fire { interval } else { cooldown };
				Step::Continue(Active::Mash {
					id,
					buttons,
					interval,
					remaining: remaining - dt,
					cooldown: (next_cooldown - dt).max(0.0),
					rng,
				})
			}
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

fn fail_goto(
	id: u32,
	xz: [f32; 2],
	remaining: f32,
	detail: &str,
	effects: &mut Vec<TickEffect>,
) -> Step {
	effects.push(TickEffect::Waypoint { id, xz, reached: false, remaining });
	effects.push(TickEffect::Failed {
		id,
		cmd: format!("goto {:.1},{:.1}", xz[0], xz[1]),
		detail: detail.into(),
	});
	Step::Done { leftover: 0.0 }
}

fn apply_unstuck(intent: &mut PadIntent, stall_secs: f32, dt: f32) {
	if should_pulse_jump(stall_secs, dt) {
		intent.hold_button(crate::command::AgentButton::Jump);
	}
	if stall_secs >= STUCK_STRAFE_AFTER {
		let phase = ((stall_secs - STUCK_STRAFE_AFTER) / STUCK_STRAFE_PERIOD).floor() as i32;
		let sign = if phase.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
		let fwd = intent.move_stick;
		let lateral = [fwd[1] * sign, -fwd[0] * sign];
		intent.move_stick =
			crate::pad::clamp_stick([fwd[0] + 0.75 * lateral[0], fwd[1] + 0.75 * lateral[1]]);
	}
}

fn should_pulse_jump(stall_secs: f32, dt: f32) -> bool {
	if stall_secs < STUCK_JUMP_AFTER {
		return false;
	}
	let since = stall_secs - STUCK_JUMP_AFTER;
	let prev = since - dt;
	if prev < 0.0 {
		return true;
	}
	(since / STUCK_JUMP_PERIOD).floor() > (prev / STUCK_JUMP_PERIOD).floor()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::command::{AgentButton, ScenarioMark};
	use crate::inventory::{AgentMatcher, InventoryMatcher, InventoryObservation};
	use crate::menu::MenuMatcher;

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
	fn visual_style_is_receipt_then_instant() -> anyhow::Result<()> {
		let mut playback =
			Playback::from_commands([AgentCommand::VisualStyle { style: "retro".into() }]);
		let tick = playback.tick(0.016, &TickView::default());
		assert_eq!(started(&tick.effects), vec![0]);
		assert!(tick.effects.iter().any(|effect| matches!(
			effect,
			TickEffect::Instant { id: 0, command: AgentCommand::VisualStyle { style } }
				if style == "retro"
		)));
		assert!(!tick.effects.iter().any(|effect| matches!(effect, TickEffect::Finished { .. })));
		assert!(tick.idle);
		Ok(())
	}

	#[test]
	fn screenshot_is_receipt_then_instant() -> anyhow::Result<()> {
		let mut playback =
			Playback::from_commands([AgentCommand::Screenshot { path: "/tmp/tanz.png".into() }]);
		let tick = playback.tick(0.016, &TickView::default());
		assert_eq!(started(&tick.effects), vec![0]);
		assert!(tick.effects.iter().any(|effect| matches!(
			effect,
			TickEffect::Instant { id: 0, command: AgentCommand::Screenshot { path } }
				if path == "/tmp/tanz.png"
		)));
		assert!(!tick.effects.iter().any(|effect| matches!(effect, TickEffect::Finished { .. })));
		assert!(tick.idle);
		Ok(())
	}

	#[test]
	fn tanz_commands_are_receipt_then_instant() -> anyhow::Result<()> {
		let mut playback = Playback::from_commands([
			AgentCommand::TanzMode { mode: "tanzer".into() },
			AgentCommand::Tanz { id: "left_strafe".into() },
		]);
		let tick = playback.tick(0.016, &TickView::default());
		assert_eq!(started(&tick.effects), vec![0, 1]);
		assert_eq!(
			tick.effects
				.iter()
				.filter(|effect| matches!(effect, TickEffect::Instant { .. }))
				.count(),
			2
		);
		assert!(!tick.effects.iter().any(|effect| matches!(effect, TickEffect::Finished { .. })));
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
		let mut playback = Playback::from_commands([AgentCommand::press(AgentButton::Jump)]);
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
		let near = reached
			.tick(0.1, &TickView { player_xz: Some([8.0, 0.0]), yaw: 0.0, ..Default::default() });
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
		let first = timed
			.tick(0.1, &TickView { player_xz: Some([0.0, 0.0]), yaw: 0.0, ..Default::default() });
		assert!(first.intent.buttons.contains(&AgentButton::Sprint));
		assert!(first.intent.move_stick[0] > 0.5);
		assert!((first.intent.look_stick[0] - 0.4).abs() < 1e-5);
		let second = timed
			.tick(0.2, &TickView { player_xz: Some([1.0, 0.0]), yaw: 0.0, ..Default::default() });
		assert!(second.effects.iter().any(|effect| matches!(
			effect,
			TickEffect::Failed { detail, .. } if detail == "timeout"
		)));
		Ok(())
	}

	fn stuck_goto(timeout: f32) -> Playback {
		Playback::from_commands([AgentCommand::Goto {
			xz: [100.0, 0.0],
			timeout,
			radius: 3.0,
			sprint: false,
			look: None,
		}])
	}

	fn origin_view() -> TickView {
		TickView { player_xz: Some([0.0, 0.0]), yaw: 0.0, ..Default::default() }
	}

	#[test]
	fn goto_unstuck_jumps_strafes_then_fails_early() -> anyhow::Result<()> {
		let mut playback = stuck_goto(30.0);
		let armed = playback.tick(0.5, &origin_view());
		assert!(armed.intent.move_stick[0] > 0.5);
		assert!(!armed.intent.buttons.contains(&AgentButton::Jump));
		assert!(armed.intent.move_stick[1].abs() < 1e-5);

		let jump = playback.tick(STUCK_JUMP_AFTER, &origin_view());
		assert!(jump.intent.buttons.contains(&AgentButton::Jump));
		assert!(!jump.idle);

		let strafe = playback.tick(STUCK_STRAFE_AFTER - STUCK_JUMP_AFTER + 0.2, &origin_view());
		assert!(strafe.intent.move_stick[1].abs() > 0.3, "{:?}", strafe.intent.move_stick);
		assert!(!strafe.idle);

		let gave_up = playback.tick(STUCK_GIVE_UP, &origin_view());
		assert!(gave_up.effects.iter().any(
			|effect| matches!(effect, TickEffect::Failed { detail, .. } if detail == "stuck")
		));
		assert!(gave_up.idle);
		Ok(())
	}

	#[test]
	fn goto_progress_clears_stall() -> anyhow::Result<()> {
		let mut playback = stuck_goto(30.0);
		let view = |x: f32| TickView { player_xz: Some([x, 0.0]), yaw: 0.0, ..Default::default() };
		playback.tick(0.5, &view(0.0));
		for step in 1..=8 {
			let tick = playback.tick(1.0, &view(step as f32 * 2.0));
			assert!(!tick.intent.buttons.contains(&AgentButton::Jump), "jumped at step {step}");
			assert!(
				!tick.effects.iter().any(|effect| matches!(effect, TickEffect::Failed { .. })),
				"failed at step {step}"
			);
		}
		assert!(!playback.is_idle());
		Ok(())
	}

	#[test]
	fn live_push_appends() {
		let mut playback = Playback::new();
		assert!(playback.tick(0.016, &TickView::default()).idle);
		playback.push(AgentCommand::Wait { secs: 1.0 });
		assert!(!playback.tick(0.016, &TickView::default()).idle);
	}

	fn in_game_view() -> TickView {
		TickView {
			menu: MenuObservation { screen: "in_game".into(), ..MenuObservation::default() },
			..Default::default()
		}
	}

	#[test]
	fn press_chord_is_one_frame() -> anyhow::Result<()> {
		let mut playback = Playback::from_commands([AgentCommand::press_many(vec![
			AgentButton::Jump,
			AgentButton::Menu,
		])]);
		let tick = playback.tick(0.016, &TickView::default());
		assert_eq!(tick.intent.buttons, vec![AgentButton::Jump, AgentButton::Menu]);
		assert!(tick.idle);
		Ok(())
	}

	#[test]
	fn press_consumes_leftover_dt() -> anyhow::Result<()> {
		let mut playback = Playback::from_commands([
			AgentCommand::Wait { secs: 0.1 },
			AgentCommand::press(AgentButton::DpadDown),
			AgentCommand::Wait { secs: 0.1 },
			AgentCommand::press(AgentButton::DpadDown),
		]);
		let first = playback.tick(1.0, &TickView::default());
		assert_eq!(first.intent.buttons, vec![AgentButton::DpadDown]);
		assert!(!first.idle);
		let release = playback.tick(1.0, &TickView::default());
		assert!(release.intent.buttons.is_empty());
		assert!(!release.idle);
		let second = playback.tick(1.0, &TickView::default());
		assert_eq!(second.intent.buttons, vec![AgentButton::DpadDown]);
		Ok(())
	}

	#[test]
	fn wait_for_matches_then_expect_fails() -> anyhow::Result<()> {
		let matcher = AgentMatcher {
			menu: MenuMatcher { screen: Some("in_game".into()), ..MenuMatcher::default() },
			..AgentMatcher::default()
		};
		let mut playback = Playback::from_commands([
			AgentCommand::WaitFor { matcher: matcher.clone(), timeout: 1.0 },
			AgentCommand::Expect {
				matcher: AgentMatcher {
					menu: MenuMatcher { pause: Some("playing".into()), ..MenuMatcher::default() },
					..AgentMatcher::default()
				},
			},
		]);
		let waiting = playback.tick(0.1, &TickView::default());
		assert!(!waiting.idle);
		assert!(waiting
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Started { id: 0, .. })));

		let matched = playback.tick(0.1, &in_game_view());
		assert!(matched
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Finished { id: 0, .. })));
		assert!(matched.effects.iter().any(|effect| matches!(
			effect,
			TickEffect::Failed { id: 1, detail, .. } if detail.contains("want pause=playing")
		)));
		assert!(matched.idle);
		Ok(())
	}

	#[test]
	fn wait_for_times_out_with_detail() -> anyhow::Result<()> {
		let mut playback = Playback::from_commands([AgentCommand::WaitFor {
			matcher: AgentMatcher {
				menu: MenuMatcher { screen: Some("gallery".into()), ..MenuMatcher::default() },
				..AgentMatcher::default()
			},
			timeout: 0.2,
		}]);
		playback.tick(0.1, &in_game_view());
		let timed = playback.tick(0.2, &in_game_view());
		assert!(timed.effects.iter().any(|effect| matches!(
			effect,
			TickEffect::Failed { detail, .. } if detail.starts_with("timeout") && detail.contains("gallery")
		)));
		assert!(timed.idle);
		Ok(())
	}

	#[test]
	fn wait_for_inventory_cycle_category() -> anyhow::Result<()> {
		let mut playback = Playback::from_commands([AgentCommand::WaitFor {
			matcher: AgentMatcher {
				inventory: InventoryMatcher {
					cycle_category: Some("lubikans".into()),
					weapon: Some("bullpup".into()),
					..InventoryMatcher::default()
				},
				..AgentMatcher::default()
			},
			timeout: 1.0,
		}]);
		let waiting = playback.tick(
			0.1,
			&TickView {
				inventory: InventoryObservation {
					cycle_category: "weapons".into(),
					weapon: Some("bullpup".into()),
					..InventoryObservation::default()
				},
				..TickView::default()
			},
		);
		assert!(!waiting.idle);
		let matched = playback.tick(
			0.1,
			&TickView {
				inventory: InventoryObservation {
					cycle_category: "lubikans".into(),
					weapon: Some("bullpup".into()),
					..InventoryObservation::default()
				},
				..TickView::default()
			},
		);
		assert!(matched
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Finished { id: 0, .. })));
		assert!(matched.idle);
		Ok(())
	}

	#[test]
	fn mash_same_seed_same_presses() -> anyhow::Result<()> {
		let mash = AgentCommand::Mash {
			buttons: vec![AgentButton::Jump, AgentButton::Stance, AgentButton::Menu],
			rate_hz: 10.0,
			secs: 0.35,
			seed: 42,
		};
		let mut a = Playback::from_commands([mash.clone()]);
		let mut b = Playback::from_commands([mash]);
		let mut seq_a = Vec::new();
		let mut seq_b = Vec::new();
		for _ in 0..8 {
			seq_a.push(a.tick(0.05, &TickView::default()).intent.buttons);
			seq_b.push(b.tick(0.05, &TickView::default()).intent.buttons);
		}
		assert_eq!(seq_a, seq_b);
		assert!(seq_a.iter().any(|buttons| !buttons.is_empty()));
		assert!(a.is_idle());
		Ok(())
	}
}
