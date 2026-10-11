//! Opt-in agent drive / observe. Registered only when [`AgentDrive`] is present.

use avian3d::prelude::{LinearVelocity, Position};
use bevy::app::AppExit;
use bevy::prelude::*;
use durham::{CascadeChunk, DurhamSurface, WorldBaseTerrain};
use maybraid_agent::{
	read_script_commands, take_leading_start, AgentArgs, AgentButton, AgentCommand, AgentEvent,
	AgentEventBody, CommandStatus, EventSink, LineInbox, PadIntent, Playback, TickEffect, TickView,
	SCRIPT_API_VERSION,
};
use maybraid_input::{PadButton, PadOverlay, VirtualPadSystems};
use mob_characters::CharacterSceneRecipe;
use mob_scenes::MobScene;
use player_camera::CameraController;
use threat_management_intelligence::ThreatManagementIntelligence;
use world_player::player::{holding_elevation, player_spawn_point_at, AwaitingTerrainSurface};
use world_player::Player;

use crate::commands::PlaygroundCommand;
use crate::control::{WorldGameplayEnabled, WorldSurfaceReady};
use crate::PlayerSpawnXz;

const DEFAULT_SAMPLE_PERIOD: f32 = 0.5;
const DEFAULT_READY_TIMEOUT: f32 = 90.0;

/// Insert before [`crate::WorldPlugin`] to enable the agent. Missing = no systems.
#[derive(Resource, Clone, Debug)]
pub struct AgentDrive {
	pub args: AgentArgs,
	pub sample_period: f32,
	pub ready_timeout: f32,
}

impl AgentDrive {
	pub fn from_args(args: AgentArgs) -> Self {
		Self { args, sample_period: DEFAULT_SAMPLE_PERIOD, ready_timeout: DEFAULT_READY_TIMEOUT }
	}
}

#[derive(Resource)]
struct AgentRuntime {
	playback: Playback,
	sink: EventSink,
	inbox: Option<LineInbox>,
	ready: bool,
	frame: u64,
	last_sample: f32,
	last_intent: PadIntent,
	pending: Vec<(u32, AgentCommand)>,
	quit: bool,
	quit_when_idle: bool,
	load_error: Option<String>,
	ready_timeout: f32,
	sample_period: f32,
}

pub(crate) struct WorldAgentPlugin;

impl Plugin for WorldAgentPlugin {
	fn build(&self, app: &mut App) {
		let drive = app.world().resource::<AgentDrive>().clone();
		let mut playback = Playback::new();
		let mut load_error = None;
		if let Some(path) = &drive.args.script {
			match read_script_commands(path) {
				Ok(mut commands) => {
					info!(
						target: "maybraid.agent",
						"loaded {} command(s) from {} (script api v{SCRIPT_API_VERSION})",
						commands.len(),
						path.display()
					);
					if let Some(xz) = take_leading_start(&mut commands) {
						info!(
							target: "maybraid.agent",
							"script start {:.1},{:.1} replaces --start-at",
							xz[0],
							xz[1]
						);
						app.insert_resource(PlayerSpawnXz(Some(Vec2::new(xz[0], xz[1]))));
					}
					for command in commands {
						playback.push(command);
					}
				}
				Err(error) => load_error = Some(error),
			}
		}
		let sink = match &drive.args.events {
			Some(path) => EventSink::open(path).unwrap_or_else(|error| {
				eprintln!("maybraid-agent events: {error}");
				EventSink::none()
			}),
			None => EventSink::none(),
		};
		let inbox =
			drive.args.pipe.as_ref().and_then(|path| match LineInbox::spawn(path.clone()) {
				Ok(inbox) => Some(inbox),
				Err(error) => {
					eprintln!("maybraid-agent pipe: {error}");
					None
				}
			});

		app.insert_resource(PadOverlay::default())
			.insert_resource(AgentRuntime {
				playback,
				sink,
				inbox,
				ready: false,
				frame: 0,
				last_sample: -1_000.0,
				last_intent: PadIntent::default(),
				pending: Vec::new(),
				quit: false,
				quit_when_idle: drive.args.quit_when_idle,
				load_error,
				ready_timeout: drive.ready_timeout,
				sample_period: drive.sample_period,
			})
			.add_systems(
				PreUpdate,
				(agent_drain_pipe, agent_sync_ready, agent_tick_playback)
					.chain()
					.before(VirtualPadSystems::Produce),
			)
			.add_systems(
				Update,
				(
					agent_apply_world_ops,
					agent_sample_player,
					agent_chunk_loads,
					agent_actor_census,
					agent_maybe_quit,
				)
					.chain(),
			);
	}
}

fn emit(sink: &EventSink, event: AgentEvent) {
	if let Some(message) = event.tracy_message() {
		info!(target: "maybraid.agent", "{message}");
	}
	if let Err(error) = sink.emit(&event) {
		warn!(target: "maybraid.agent", "event write failed: {error}");
	}
}

fn agent_drain_pipe(mut runtime: ResMut<AgentRuntime>, time: Res<Time>) {
	let Some(inbox) = runtime.inbox.as_ref() else {
		return;
	};
	let lines = inbox.try_recv_all();
	let t = time.elapsed_secs();
	let frame = runtime.frame;
	for line in lines {
		let trimmed = line.trim();
		if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
			continue;
		}
		match AgentCommand::parse_line(trimmed) {
			Ok(command) => {
				let id = runtime.playback.push(command.clone());
				emit(
					&runtime.sink,
					AgentEvent {
						t,
						frame,
						body: AgentEventBody::Command {
							id,
							status: CommandStatus::Acknowledged,
							cmd: command.label(),
							detail: None,
						},
					},
				);
			}
			Err(detail) => {
				emit(
					&runtime.sink,
					AgentEvent {
						t,
						frame,
						body: AgentEventBody::Command {
							id: runtime.playback_next_id(),
							status: CommandStatus::Failed,
							cmd: trimmed.to_string(),
							detail: Some(detail),
						},
					},
				);
			}
		}
	}
}

impl AgentRuntime {
	fn playback_next_id(&self) -> u32 {
		// Playback ids are assigned on push; a failed parse has no id.
		u32::MAX
	}
}

fn agent_sync_ready(
	mut runtime: ResMut<AgentRuntime>,
	gameplay: Res<WorldGameplayEnabled>,
	surface: Res<WorldSurfaceReady>,
	time: Res<Time>,
) {
	if runtime.ready {
		return;
	}
	if let Some(error) = runtime.load_error.take() {
		emit(
			&runtime.sink,
			AgentEvent {
				t: time.elapsed_secs(),
				frame: runtime.frame,
				body: AgentEventBody::Command {
					id: 0,
					status: CommandStatus::Failed,
					cmd: "script".into(),
					detail: Some(error),
				},
			},
		);
		runtime.quit = true;
		return;
	}
	let elapsed = time.elapsed_secs();
	let surface_ready = gameplay.0 && surface.0;
	let timed_out = elapsed >= runtime.ready_timeout;
	if !surface_ready && !timed_out {
		return;
	}
	runtime.ready = true;
	let detail = if surface_ready { "surface" } else { "timeout" };
	emit(
		&runtime.sink,
		AgentEvent {
			t: elapsed,
			frame: runtime.frame,
			body: AgentEventBody::Ready { detail: detail.into() },
		},
	);
}

fn agent_tick_playback(
	mut runtime: ResMut<AgentRuntime>,
	mut overlay: ResMut<PadOverlay>,
	time: Res<Time>,
	players: Query<&Transform, With<Player>>,
	cameras: Query<&CameraController, With<Camera3d>>,
) {
	runtime.frame = runtime.frame.saturating_add(1);
	if !runtime.ready {
		overlay.clear();
		return;
	}
	let view = TickView {
		player_xz: players.single().ok().map(|transform| {
			let xz = transform.translation.xz();
			[xz.x, xz.y]
		}),
		yaw: cameras.single().ok().map(|camera| camera.yaw).unwrap_or(0.0),
	};
	let tick = runtime.playback.tick(time.delta_secs(), &view);
	runtime.last_intent = tick.intent.clone();
	apply_intent(&mut overlay, &tick.intent);
	let t = time.elapsed_secs();
	let frame = runtime.frame;
	for effect in tick.effects {
		match effect {
			TickEffect::Started { id, cmd } => emit(
				&runtime.sink,
				AgentEvent {
					t,
					frame,
					body: AgentEventBody::Command {
						id,
						status: CommandStatus::Started,
						cmd,
						detail: None,
					},
				},
			),
			TickEffect::Finished { id, cmd } => emit(
				&runtime.sink,
				AgentEvent {
					t,
					frame,
					body: AgentEventBody::Command {
						id,
						status: CommandStatus::Finished,
						cmd,
						detail: None,
					},
				},
			),
			TickEffect::Failed { id, cmd, detail } => emit(
				&runtime.sink,
				AgentEvent {
					t,
					frame,
					body: AgentEventBody::Command {
						id,
						status: CommandStatus::Failed,
						cmd,
						detail: Some(detail),
					},
				},
			),
			TickEffect::Instant { id, command } => {
				if matches!(command, AgentCommand::Quit) {
					runtime.quit = true;
				}
				if let AgentCommand::Scenario { name, mark } = &command {
					emit(
						&runtime.sink,
						AgentEvent {
							t,
							frame,
							body: AgentEventBody::Scenario { name: name.clone(), mark: *mark },
						},
					);
				}
				runtime.pending.push((id, command));
			}
			TickEffect::Waypoint { id, xz, reached, remaining } => emit(
				&runtime.sink,
				AgentEvent {
					t,
					frame,
					body: AgentEventBody::Waypoint { id, xz, reached, remaining },
				},
			),
		}
	}
	if tick.idle && runtime.quit_when_idle && runtime.inbox.is_none() {
		runtime.quit = true;
	}
}

fn apply_intent(overlay: &mut PadOverlay, intent: &PadIntent) {
	overlay.clear();
	overlay.move_stick = Vec2::from(intent.move_stick);
	overlay.look_stick = Vec2::from(intent.look_stick);
	overlay.trigger_focus = intent.trigger_focus;
	overlay.trigger_fire = intent.trigger_fire;
	for button in &intent.buttons {
		overlay.hold(pad_button(*button));
	}
}

fn pad_button(button: AgentButton) -> PadButton {
	match button {
		AgentButton::Jump => PadButton::A,
		AgentButton::Sprint => PadButton::StickClickMove,
		AgentButton::Stance => PadButton::B,
		AgentButton::Interact => PadButton::X,
		AgentButton::Swap => PadButton::Y,
		AgentButton::Ads => PadButton::BumperFocus,
		AgentButton::BumperFire => PadButton::BumperFire,
		AgentButton::SwapPov => PadButton::StickClickLook,
		AgentButton::Focus => PadButton::TriggerFocus,
		AgentButton::Fire => PadButton::TriggerFire,
		AgentButton::Menu => PadButton::Start,
		AgentButton::Map => PadButton::Select,
		AgentButton::DpadUp => PadButton::DpadUp,
		AgentButton::DpadDown => PadButton::DpadDown,
		AgentButton::DpadLeft => PadButton::DpadLeft,
		AgentButton::DpadRight => PadButton::DpadRight,
	}
}

fn agent_apply_world_ops(
	mut runtime: ResMut<AgentRuntime>,
	mut commands: Commands,
	surface: DurhamSurface,
	base: Res<WorldBaseTerrain>,
	mut players: Query<
		(
			Entity,
			&mut Transform,
			&mut GlobalTransform,
			Option<&mut Position>,
			Option<&mut LinearVelocity>,
		),
		With<Player>,
	>,
) {
	let pending = std::mem::take(&mut runtime.pending);
	for (_id, command) in pending {
		let command = match command {
			AgentCommand::Start { xz } => AgentCommand::Teleport { xz, y: None },
			other => other,
		};
		match command {
			AgentCommand::Teleport { xz, y } => {
				let target = Vec2::new(xz[0], xz[1]);
				let elevation = match y {
					Some(y) => y,
					None => match surface.height_at(target) {
						Ok(Some(elevation)) => elevation,
						Ok(None) | Err(_) => holding_elevation(&base.0, target.x, target.y),
					},
				};
				let at = player_spawn_point_at(target, elevation);
				if let Ok((entity, mut transform, mut global, mut body, mut velocity)) =
					players.single_mut()
				{
					transform.translation = at;
					*global = GlobalTransform::from(*transform);
					if let Some(position) = body.as_deref_mut() {
						position.0 = at;
					}
					if let Some(velocity) = velocity.as_deref_mut() {
						velocity.0 = Vec3::ZERO;
					}
					commands.entity(entity).insert(AwaitingTerrainSurface);
				}
			}
			AgentCommand::Console { line } => {
				let mut console = String::new();
				match PlaygroundCommand::parse_line(&line) {
					Ok(command) => command.react(&mut commands, &mut console),
					Err(error) => warn!(target: "maybraid.agent", "console {line}: {error}"),
				}
				if !console.is_empty() {
					info!(target: "maybraid.agent", "{console}");
				}
			}
			AgentCommand::Quit => runtime.quit = true,
			_ => {}
		}
	}
}

fn agent_sample_player(
	mut runtime: ResMut<AgentRuntime>,
	time: Res<Time>,
	players: Query<&Transform, With<Player>>,
	cameras: Query<&CameraController, With<Camera3d>>,
) {
	if !runtime.ready {
		return;
	}
	let now = time.elapsed_secs();
	if now - runtime.last_sample < runtime.sample_period {
		return;
	}
	runtime.last_sample = now;
	let Ok(transform) = players.single() else {
		return;
	};
	let p = transform.translation;
	let yaw = cameras.single().ok().map(|camera| camera.yaw).unwrap_or(0.0);
	let r#move = runtime.last_intent.move_stick;
	emit(
		&runtime.sink,
		AgentEvent {
			t: now,
			frame: runtime.frame,
			body: AgentEventBody::Player { x: p.x, y: p.y, z: p.z, yaw, r#move },
		},
	);
}

fn agent_chunk_loads(
	runtime: Res<AgentRuntime>,
	time: Res<Time>,
	added: Query<Entity, Added<CascadeChunk>>,
) {
	if !runtime.ready {
		return;
	}
	let added = added.iter().count() as u32;
	if added == 0 {
		return;
	}
	emit(
		&runtime.sink,
		AgentEvent {
			t: time.elapsed_secs(),
			frame: runtime.frame,
			body: AgentEventBody::ChunkLoads { added },
		},
	);
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
struct ActorCensus {
	hosts: u32,
	bodies: u32,
	brains: u32,
}

fn agent_actor_census(
	runtime: Res<AgentRuntime>,
	time: Res<Time>,
	hosts: Query<(), With<MobScene>>,
	bodies: Query<(), (With<CharacterSceneRecipe>, Without<Player>)>,
	brains: Query<(), With<ThreatManagementIntelligence>>,
	mut last: Local<Option<ActorCensus>>,
	mut last_at: Local<f32>,
) {
	if !runtime.ready {
		return;
	}
	let now = time.elapsed_secs();
	if last.is_some() && now - *last_at < runtime.sample_period {
		return;
	}
	let census = ActorCensus {
		hosts: hosts.iter().len() as u32,
		bodies: bodies.iter().len() as u32,
		brains: brains.iter().len() as u32,
	};
	if *last == Some(census) {
		return;
	}
	*last = Some(census);
	*last_at = now;
	let frame = runtime.frame;
	emit(
		&runtime.sink,
		AgentEvent {
			t: now,
			frame,
			body: AgentEventBody::Actors {
				hosts: census.hosts,
				bodies: census.bodies,
				brains: census.brains,
			},
		},
	);
}

fn agent_maybe_quit(
	mut runtime: ResMut<AgentRuntime>,
	mut exit: MessageWriter<AppExit>,
	time: Res<Time>,
) {
	if !runtime.quit {
		return;
	}
	emit(
		&runtime.sink,
		AgentEvent { t: time.elapsed_secs(), frame: runtime.frame, body: AgentEventBody::Quit },
	);
	runtime.quit = false;
	exit.write(AppExit::Success);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn jump_maps_to_face_a() {
		assert_eq!(pad_button(AgentButton::Jump), PadButton::A);
		assert_eq!(pad_button(AgentButton::Sprint), PadButton::StickClickMove);
	}

	#[test]
	fn overlay_copies_move_stick() {
		let mut overlay = PadOverlay::default();
		apply_intent(&mut overlay, &PadIntent { move_stick: [0.0, 1.0], ..PadIntent::default() });
		assert_eq!(overlay.move_stick, Vec2::Y);
	}
}
