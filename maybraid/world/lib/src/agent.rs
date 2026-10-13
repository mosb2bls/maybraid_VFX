//! Opt-in agent drive / observe. Registered only when [`AgentDrive`] is present.

use avian3d::prelude::{LinearVelocity, Position};
use bevy::app::AppExit;
use bevy::prelude::*;
use damage::{Health, Hit};
use durham::{CascadeChunk, DurhamSurface, WorldBaseTerrain};
use maybraid_agent::{
	AgentArgs, AgentButton, AgentCommand, AgentEvent, AgentEventBody, CommandStatus, EventSink,
	LineInbox, PadIntent, Playback, SCRIPT_API_VERSION, TickEffect, TickView, read_script_commands,
	take_leading_start,
};
use maybraid_input::{PadButton, PadOverlay, VirtualPadSystems};
use mob_characters::CharacterSceneRecipe;
use mob_scenes::MobScene;
use player::Player as MotorPlayer;
use player_camera::CameraController;
use threat_management_intelligence::ThreatManagementIntelligence;
use world_player::Player as VegetationPlayer;
use world_player::player::{AwaitingTerrainSurface, holding_elevation, player_spawn_point_at};

use crate::PlayerSpawnXz;
use crate::commands::PlaygroundCommand;
use crate::control::{WorldGameplayEnabled, WorldSurfaceReady};
use crate::map_view::WorldMapView;
use crate::render_scale::{PresentationChurn, RequestVisualStyle, VisualStyle};

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
					agent_emit_presentation_churn,
					agent_sample_player,
					agent_sample_picker,
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
	motors: Query<&Transform, With<MotorPlayer>>,
	vegetation: Query<&Transform, (With<VegetationPlayer>, Without<MotorPlayer>)>,
	cameras: Query<&CameraController, With<Camera3d>>,
) {
	runtime.frame = runtime.frame.saturating_add(1);
	if !runtime.ready {
		overlay.clear();
		return;
	}
	let pose = motors.single().ok().or_else(|| vegetation.single().ok());
	let view = TickView {
		player_xz: pose.map(|transform| {
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

fn visual_style_applied_detail(
	style: VisualStyle,
	window_physical: UVec2,
	window_scale_factor: f32,
) -> String {
	let plan = style.plan(window_physical, window_scale_factor);
	format!("applied {} {}x{}", style.script_id(), plan.target_size.x, plan.target_size.y)
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
	mut visual_style_req: MessageWriter<RequestVisualStyle>,
	surface: DurhamSurface,
	base: Res<WorldBaseTerrain>,
	windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
	time: Res<Time>,
	mut hits: MessageWriter<Hit>,
	mut players: Query<
		(
			Entity,
			&mut Transform,
			&mut GlobalTransform,
			Option<&mut Position>,
			Option<&mut LinearVelocity>,
			Option<&Health>,
		),
		With<MotorPlayer>,
	>,
) {
	let pending = std::mem::take(&mut runtime.pending);
	let t = time.elapsed_secs();
	let frame = runtime.frame;
	for (id, command) in pending {
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
				if let Ok((entity, mut transform, mut global, mut body, mut velocity, _)) =
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
			AgentCommand::Console { line } if line.trim() == "player down" => {
				// Agent-only: not a PlaygroundCommand, so production `/` never
				// registers a kill. Lethal self-hit so death-picker scripts can
				// confirm a POI without debug-commands in the game plugin.
				if let Ok((entity, transform, _, _, _, health)) = players.single() {
					let amount = health.map(|pool| pool.current + 1.0).unwrap_or(200.0);
					hits.write(Hit {
						target: entity,
						source: None,
						amount,
						point: transform.translation,
						allow_self: true,
					});
					info!(target: "maybraid.agent", "player down: hit {amount:.0}");
				} else {
					warn!(target: "maybraid.agent", "player down: no live player");
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
			AgentCommand::VisualStyle { style } => {
				if let Some(visual_style) = VisualStyle::from_script_id(&style) {
					visual_style_req.write(RequestVisualStyle(visual_style));
					let (physical, factor) = match windows.single() {
						Ok(window) => (
							UVec2::new(window.physical_width(), window.physical_height()),
							window.scale_factor(),
						),
						Err(_) => (UVec2::ZERO, 1.0),
					};
					emit(
						&runtime.sink,
						AgentEvent {
							t,
							frame,
							body: AgentEventBody::Command {
								id,
								status: CommandStatus::Finished,
								cmd: format!("visual_style {style}"),
								detail: Some(visual_style_applied_detail(
									visual_style,
									physical,
									factor,
								)),
							},
						},
					);
				} else {
					emit(
						&runtime.sink,
						AgentEvent {
							t,
							frame,
							body: AgentEventBody::Command {
								id,
								status: CommandStatus::Failed,
								cmd: format!("visual_style {style}"),
								detail: Some(format!("unknown visual style: {style}")),
							},
						},
					);
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
	surface: DurhamSurface,
	map: Option<Res<WorldMapView>>,
	motors: Query<&Transform, With<MotorPlayer>>,
	vegetation: Query<&Transform, (With<VegetationPlayer>, Without<MotorPlayer>)>,
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
	let Some(transform) = motors.single().ok().or_else(|| vegetation.single().ok()) else {
		return;
	};
	let p = transform.translation;
	let yaw = cameras.single().ok().map(|camera| camera.yaw).unwrap_or(0.0);
	let r#move = runtime.last_intent.move_stick;
	let ground = surface.composed_height_at(p.xz());
	let map_open = map.as_deref().is_some_and(|map| map.open);
	let picker = map.as_deref().is_some_and(|map| map.open && map.close_locked);
	emit(
		&runtime.sink,
		AgentEvent {
			t: now,
			frame: runtime.frame,
			body: AgentEventBody::Player {
				x: p.x,
				y: p.y,
				z: p.z,
				yaw,
				r#move,
				ground,
				map_open,
				picker,
			},
		},
	);
}

fn agent_sample_picker(
	runtime: Res<AgentRuntime>,
	time: Res<Time>,
	map: Option<Res<WorldMapView>>,
	mut last: Local<Option<bool>>,
) {
	if !runtime.ready {
		return;
	}
	let open = map.as_deref().is_some_and(|map| map.open && map.close_locked);
	if *last == Some(open) {
		return;
	}
	*last = Some(open);
	emit(
		&runtime.sink,
		AgentEvent {
			t: time.elapsed_secs(),
			frame: runtime.frame,
			body: AgentEventBody::Picker { open },
		},
	);
}

fn agent_emit_presentation_churn(
	runtime: Res<AgentRuntime>,
	time: Res<Time>,
	mut churn: MessageReader<PresentationChurn>,
) {
	for event in churn.read() {
		emit(
			&runtime.sink,
			AgentEvent {
				t: time.elapsed_secs(),
				frame: runtime.frame,
				body: AgentEventBody::Presentation {
					action: event.action.as_str().into(),
					window: [event.window.x, event.window.y],
					target: [event.target.x, event.target.y],
				},
			},
		);
	}
}

fn agent_chunk_loads(
	runtime: Res<AgentRuntime>,
	time: Res<Time>,
	added: Query<Entity, Added<CascadeChunk>>,
) {
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
	/// 0.5 m bins so a post-spawn sink is a census change.
	min_y_q: Option<i32>,
	max_y_q: Option<i32>,
}

fn quantize_metres(y: f32) -> i32 {
	(y * 2.0).round() as i32
}

fn agent_actor_census(
	runtime: Res<AgentRuntime>,
	time: Res<Time>,
	surface: DurhamSurface,
	hosts: Query<(), With<MobScene>>,
	bodies: Query<&Transform, (With<CharacterSceneRecipe>, Without<MotorPlayer>)>,
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
	let mut min_y = None;
	let mut max_y = None;
	let mut ground_at_min_y = None;
	let mut body_count = 0u32;
	for transform in &bodies {
		let y = transform.translation.y;
		body_count += 1;
		match min_y {
			Some(current) if y >= current => {}
			_ => {
				min_y = Some(y);
				ground_at_min_y = surface.composed_height_at(transform.translation.xz());
			}
		}
		max_y = Some(max_y.map_or(y, |current: f32| current.max(y)));
	}
	let census = ActorCensus {
		hosts: hosts.iter().len() as u32,
		bodies: body_count,
		brains: brains.iter().len() as u32,
		min_y_q: min_y.map(quantize_metres),
		max_y_q: max_y.map(quantize_metres),
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
				min_y,
				max_y,
				ground_at_min_y,
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
	fn visual_style_applied_detail_includes_resolved_size() {
		assert_eq!(
			visual_style_applied_detail(VisualStyle::Impressionist, UVec2::new(1920, 1080), 1.0),
			"applied impressionist 700x394"
		);
		assert_eq!(
			visual_style_applied_detail(VisualStyle::Watercolor, UVec2::new(1920, 1080), 1.0),
			"applied watercolor 1280x720"
		);
		assert_eq!(
			visual_style_applied_detail(VisualStyle::Sharp, UVec2::new(1920, 1080), 1.0),
			"applied sharp 1920x1080"
		);
	}

	#[test]
	fn overlay_copies_move_stick() {
		let mut overlay = PadOverlay::default();
		apply_intent(&mut overlay, &PadIntent { move_stick: [0.0, 1.0], ..PadIntent::default() });
		assert_eq!(overlay.move_stick, Vec2::Y);
	}
}
