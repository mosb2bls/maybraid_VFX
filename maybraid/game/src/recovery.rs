//! In-process recovery for Bevy ECS errors.
//!
//! The fallback handler has no World access. It logs, appends `errors.log`,
//! and pushes a [`RecoveryRequest`] into a static inbox. [`RecoverySystems`]
//! in [`First`] drains that inbox and routes with the same helpers live
//! navigation uses ([`MenuScreens::clear`], [`end_character_editor`]).

use crate::flow::{GameFlow, PlaySession, WorldPause};
use crate::panic_log::{append_error_log, error_log_path};
use crate::shell::ShellRoute;
use bevy::app::AppExit;
use bevy::ecs::error::{BevyError, ErrorContext, Severity};
use bevy::prelude::*;
use character_persist::SaveRoot;
use maybraid_world::AgentDrive;
use menu_playground::{end_character_editor, CharacterLeavePrompt};
use menu_screens::{cancel_pending_create, MenuScreens};
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Env override. Unset means debug / agent runs are strict and release recovers.
pub const ENV_STRICT_ERRORS: &str = "MAYBRAID_STRICT_ERRORS";

pub const RECOVERY_LOOP_MAX: usize = 3;
pub const RECOVERY_LOOP_WINDOW: Duration = Duration::from_secs(10);
pub const TOAST_SECS: f32 = 6.0;

const LAST_SESSION_FILE: &str = "last_session";

static STRICT: AtomicBool = AtomicBool::new(false);

fn inbox() -> &'static Mutex<Vec<RecoveryRequest>> {
	static INBOX: OnceLock<Mutex<Vec<RecoveryRequest>>> = OnceLock::new();
	INBOX.get_or_init(|| Mutex::new(Vec::new()))
}

fn lock_inbox() -> std::sync::MutexGuard<'static, Vec<RecoveryRequest>> {
	inbox().lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverySource {
	Main,
	Render,
}

#[derive(Debug, Clone)]
pub struct RecoveryRequest {
	pub message: String,
	pub context: String,
	pub source: RecoverySource,
	pub at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDestination {
	World,
	Home,
	Halt,
	Logged,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
	Strict,
	Recover,
}

impl RecoveryMode {
	pub fn from_runtime(agent: bool) -> Self {
		if strict_errors_from_env(
			std::env::var(ENV_STRICT_ERRORS).ok().as_deref(),
			agent,
			cfg!(debug_assertions),
		) {
			Self::Strict
		} else {
			Self::Recover
		}
	}
}

/// `--recover` child: resume Discovery when the last session and character are valid.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct RecoverBoot;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct LastRecovery {
	pub destination: RecoveryDestination,
}

#[derive(Resource, Debug, Clone)]
pub struct RecoveryToast {
	pub message: String,
	pub remaining_secs: f32,
}

impl RecoveryToast {
	pub fn routed(destination: RecoveryDestination) -> Self {
		let place = match destination {
			RecoveryDestination::World => "the game",
			RecoveryDestination::Home => "Home",
			RecoveryDestination::Halt => "a stop",
			RecoveryDestination::Logged => "the current screen",
		};
		Self {
			message: format!(
				"Something went wrong; you've been returned to {place}. Details: {}",
				error_log_path().display()
			),
			remaining_secs: TOAST_SECS,
		}
	}

	pub fn restarted_to_world() -> Self {
		Self {
			message: format!(
				"The game was restarted; returning to Discovery. Details: {}",
				crate::panic_log::panic_log_path().display()
			),
			remaining_secs: TOAST_SECS,
		}
	}

	pub fn restarted_to_home() -> Self {
		Self {
			message: format!(
				"The game was restarted after a crash. Details: {}",
				crate::panic_log::panic_log_path().display()
			),
			remaining_secs: TOAST_SECS,
		}
	}
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoverySystems {
	Apply,
}

#[derive(Resource, Debug, Default)]
struct RecoveryHistory {
	times: VecDeque<Instant>,
}

#[derive(Resource, Debug, Default)]
struct RecoveryApply {
	in_progress: bool,
}

#[derive(Component)]
struct RecoveryToastRoot;

pub struct RecoveryPlugin {
	pub mode: RecoveryMode,
}

impl RecoveryPlugin {
	pub fn recovering() -> Self {
		Self { mode: RecoveryMode::Recover }
	}

	pub fn strict() -> Self {
		Self { mode: RecoveryMode::Strict }
	}
}

impl Plugin for RecoveryPlugin {
	fn build(&self, app: &mut App) {
		STRICT.store(matches!(self.mode, RecoveryMode::Strict), Ordering::Relaxed);
		clear_recovery_inbox();
		app.insert_resource(self.mode)
			.set_error_handler(main_error_handler)
			.init_resource::<RecoveryHistory>()
			.init_resource::<RecoveryApply>()
			.configure_sets(First, RecoverySystems::Apply)
			.add_systems(First, apply_recovery.in_set(RecoverySystems::Apply))
			.add_systems(Update, tick_recovery_toast);
		if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
			render_app
				.world_mut()
				.insert_resource(bevy::ecs::error::FallbackErrorHandler(render_error_handler));
		}
	}
}

pub fn strict_errors_from_env(raw: Option<&str>, agent: bool, debug: bool) -> bool {
	if let Some(raw) = raw {
		let trimmed = raw.trim();
		if !trimmed.is_empty() {
			return env_flag_on(Some(trimmed));
		}
	}
	agent || debug
}

pub fn env_flag_on(raw: Option<&str>) -> bool {
	matches!(
		raw.map(str::trim).unwrap_or_default().to_ascii_lowercase().as_str(),
		"1" | "true" | "yes" | "on"
	)
}

pub fn recovery_destination(flow: GameFlow, escalate: bool) -> RecoveryDestination {
	match (flow, escalate) {
		(GameFlow::World, false) => RecoveryDestination::World,
		(GameFlow::World, true) => RecoveryDestination::Home,
		(_, false) => RecoveryDestination::Home,
		(_, true) => RecoveryDestination::Halt,
	}
}

pub fn should_escalate(
	times: &[Instant],
	now: Instant,
	in_progress: bool,
	incoming: usize,
) -> bool {
	if in_progress {
		return true;
	}
	let recent = times
		.iter()
		.filter(|at| now.duration_since(**at) <= RECOVERY_LOOP_WINDOW)
		.count()
		+ incoming;
	recent > RECOVERY_LOOP_MAX
}

pub fn take_recovery_requests() -> Vec<RecoveryRequest> {
	lock_inbox().drain(..).collect()
}

pub fn clear_recovery_inbox() {
	lock_inbox().clear();
}

pub fn write_last_session(root: &SaveRoot, session: PlaySession) {
	if session != PlaySession::Discovery {
		clear_last_session(root);
		return;
	}
	if let Err(error) = fs::create_dir_all(&root.path) {
		warn!("last session dir {}: {error}", root.path.display());
		return;
	}
	if let Err(error) = fs::write(root.path.join(LAST_SESSION_FILE), "discovery\n") {
		warn!("last session write {}: {error}", root.path.display());
	}
}

pub fn clear_last_session(root: &SaveRoot) {
	let path = root.path.join(LAST_SESSION_FILE);
	if path.exists() {
		if let Err(error) = fs::remove_file(&path) {
			warn!("last session clear {}: {error}", path.display());
		}
	}
}

pub fn read_last_session(root: &SaveRoot) -> Option<PlaySession> {
	read_last_session_at(&root.path)
}

pub fn read_last_session_at(path: &Path) -> Option<PlaySession> {
	let text = fs::read_to_string(path.join(LAST_SESSION_FILE)).ok()?;
	match text.trim() {
		"discovery" => Some(PlaySession::Discovery),
		_ => None,
	}
}

pub fn can_resume_world(root: &SaveRoot, has_active_character: bool) -> bool {
	has_active_character && read_last_session(root) == Some(PlaySession::Discovery)
}

fn main_error_handler(error: BevyError, ctx: ErrorContext) {
	handle_error(error, ctx, RecoverySource::Main);
}

fn render_error_handler(error: BevyError, ctx: ErrorContext) {
	handle_error(error, ctx, RecoverySource::Render);
}

fn handle_error(error: BevyError, ctx: ErrorContext, source: RecoverySource) {
	if error.severity() < Severity::Error {
		bevy::ecs::error::match_severity(error, ctx);
		return;
	}
	if STRICT.load(Ordering::Relaxed) {
		bevy::ecs::error::panic(error, ctx);
		return;
	}
	let context = format!("{} {}", ctx.kind(), ctx.name());
	let message = format!("{error}");
	error!("recoverable {context}: {message}");
	let _ = append_error_log(&format!("{context}\n{message}"));
	lock_inbox().push(RecoveryRequest { message, context, source, at: Instant::now() });
}

fn apply_recovery(
	mut commands: Commands,
	screens: MenuScreens,
	mut route: ShellRoute,
	flow: Res<State<GameFlow>>,
	mut next_pause: Option<ResMut<NextState<WorldPause>>>,
	mut prompt: Option<ResMut<CharacterLeavePrompt>>,
	mut history: ResMut<RecoveryHistory>,
	mut apply: ResMut<RecoveryApply>,
	agent: Option<Res<AgentDrive>>,
	mut exits: MessageWriter<AppExit>,
) {
	let requests = take_recovery_requests();
	if requests.is_empty() {
		apply.in_progress = false;
		return;
	}
	let now = Instant::now();
	history.times.retain(|at| now.duration_since(*at) <= RECOVERY_LOOP_WINDOW);
	let has_main = requests.iter().any(|request| request.source == RecoverySource::Main);
	let escalate = should_escalate(
		&Vec::from_iter(history.times.iter().copied()),
		now,
		apply.in_progress,
		requests.len(),
	);
	for request in &requests {
		history.times.push_back(request.at);
	}
	if !has_main && !escalate {
		commands.insert_resource(LastRecovery { destination: RecoveryDestination::Logged });
		return;
	}
	let flow = *flow.get();
	let destination = recovery_destination(flow, escalate);
	apply.in_progress = true;
	tear_down_shell(&mut commands, &screens, prompt.as_deref_mut());
	match destination {
		RecoveryDestination::World => {
			if let Some(next) = next_pause.as_mut() {
				next.set(WorldPause::Playing);
			}
		}
		RecoveryDestination::Home => {
			history.times.clear();
			route.enter(GameFlow::Home);
		}
		RecoveryDestination::Halt => {
			exits.write(AppExit::from_code(2));
		}
		RecoveryDestination::Logged => {}
	}
	commands.insert_resource(LastRecovery { destination });
	if destination != RecoveryDestination::Logged {
		commands.insert_resource(RecoveryToast::routed(destination));
	}
	if agent.is_some() {
		exits.write(AppExit::from_code(2));
	}
}

fn tear_down_shell(
	commands: &mut Commands,
	screens: &MenuScreens,
	prompt: Option<&mut CharacterLeavePrompt>,
) {
	screens.clear(commands);
	end_character_editor(commands);
	cancel_pending_create(commands);
	if let Some(prompt) = prompt {
		prompt.close();
	}
}

fn tick_recovery_toast(
	mut commands: Commands,
	toast: Option<ResMut<RecoveryToast>>,
	time: Res<Time>,
	roots: Query<Entity, With<RecoveryToastRoot>>,
) {
	let Some(mut toast) = toast else {
		for entity in &roots {
			commands.entity(entity).try_despawn();
		}
		return;
	};
	toast.remaining_secs -= time.delta_secs();
	if toast.remaining_secs <= 0.0 {
		commands.remove_resource::<RecoveryToast>();
		for entity in &roots {
			commands.entity(entity).try_despawn();
		}
		return;
	}
	if !toast.is_changed() && !roots.is_empty() {
		return;
	}
	for entity in &roots {
		commands.entity(entity).try_despawn();
	}
	commands.spawn((
		RecoveryToastRoot,
		Node {
			position_type: PositionType::Absolute,
			bottom: px(24.0),
			left: px(24.0),
			right: px(24.0),
			padding: UiRect::all(px(12.0)),
			justify_content: JustifyContent::Center,
			..default()
		},
		BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.82)),
		Text::new(toast.message.clone()),
	));
}

pub fn remember_world_session(root: Res<SaveRoot>, session: Res<PlaySession>) {
	write_last_session(&root, *session);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::flow::GameFlow;
	use bevy::state::app::StatesPlugin;
	use layer_stack::ActiveGenerationMode;
	use menu_screens::{request_show_in_game, MenuScreen, MenuShowRequest};
	use std::sync::Mutex;
	use std::time::Instant;

	static TEST_LOCK: Mutex<()> = Mutex::new(());

	#[derive(Component)]
	struct Gone;

	#[derive(Resource)]
	struct MissingTarget(Entity);

	fn lock_recovery_tests() -> std::sync::MutexGuard<'static, ()> {
		TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	fn recovery_app(flow: GameFlow) -> App {
		clear_recovery_inbox();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins(RecoveryPlugin::recovering())
			.init_state::<GameFlow>()
			.add_sub_state::<WorldPause>()
			.init_resource::<PlaySession>()
			.insert_resource(NextState::<ActiveGenerationMode>::Unchanged);
		if flow != GameFlow::Home {
			app.world_mut().resource_mut::<NextState<GameFlow>>().set(flow);
			app.update();
		}
		if flow == GameFlow::World {
			app.world_mut().resource_mut::<NextState<WorldPause>>().set(WorldPause::Menu);
			app.update();
		}
		clear_recovery_inbox();
		app.world_mut().remove_resource::<LastRecovery>();
		app
	}

	fn queue_insert_on_missing(target: Res<MissingTarget>, mut commands: Commands) {
		commands.entity(target.0).insert(Gone);
	}

	fn run_until_recovery(app: &mut App) -> LastRecovery {
		app.update();
		app.update();
		app.world()
			.get_resource::<LastRecovery>()
			.cloned()
			.unwrap_or(LastRecovery { destination: RecoveryDestination::Logged })
	}

	#[test]
	fn strict_defaults_match_the_issue() {
		assert!(strict_errors_from_env(None, false, true));
		assert!(!strict_errors_from_env(None, false, false));
		assert!(strict_errors_from_env(None, true, false));
		assert!(!strict_errors_from_env(Some("0"), true, true));
		assert!(strict_errors_from_env(Some("1"), false, false));
		assert!(strict_errors_from_env(Some("yes"), false, false));
	}

	#[test]
	fn destination_table_matches_game_flow() {
		assert_eq!(recovery_destination(GameFlow::World, false), RecoveryDestination::World);
		assert_eq!(recovery_destination(GameFlow::World, true), RecoveryDestination::Home);
		assert_eq!(recovery_destination(GameFlow::Home, false), RecoveryDestination::Home);
		assert_eq!(recovery_destination(GameFlow::Characters, false), RecoveryDestination::Home);
		assert_eq!(recovery_destination(GameFlow::LoadingWorld, false), RecoveryDestination::Home);
		assert_eq!(recovery_destination(GameFlow::Home, true), RecoveryDestination::Halt);
		assert_eq!(recovery_destination(GameFlow::LoadingWorld, true), RecoveryDestination::Halt);
	}

	#[test]
	fn loop_guard_trips_after_more_than_n_in_the_window() {
		let now = Instant::now();
		let times = [now, now, now];
		assert!(!should_escalate(&times, now, false, 0));
		assert!(should_escalate(&times, now, false, 1));
		assert!(should_escalate(&[], now, true, 1));
	}

	fn trigger_missing_insert(app: &mut App) {
		let gone = app.world_mut().spawn_empty().id();
		app.world_mut().despawn(gone);
		app.insert_resource(MissingTarget(gone));
		app.add_systems(Update, queue_insert_on_missing);
	}

	#[test]
	fn world_error_returns_to_playing_and_clears_menus() -> anyhow::Result<()> {
		let _guard = lock_recovery_tests();
		let mut app = recovery_app(GameFlow::World);
		let mut commands = app.world_mut().commands();
		request_show_in_game(&mut commands);
		app.world_mut().spawn(MenuScreen);
		app.world_mut().insert_resource(menu_playground::CharacterEditorReturn::InGame);
		app.world_mut().flush();
		trigger_missing_insert(&mut app);
		let last = run_until_recovery(&mut app);
		anyhow::ensure!(last.destination == RecoveryDestination::World);
		{
			let world = app.world_mut();
			let mut screens = world.query::<&MenuScreen>();
			anyhow::ensure!(screens.iter(world).count() == 0);
		}
		{
			let world = app.world_mut();
			let mut requests = world.query::<&MenuShowRequest>();
			anyhow::ensure!(requests.iter(world).count() == 0);
		}
		// `end_character_editor` marks the session; `finish_character_editor_exit`
		// drops Return once CharacterScreen is gone (same frame in the full app).
		let pause = app.world().resource::<State<WorldPause>>();
		anyhow::ensure!(*pause.get() == WorldPause::Playing, "pause {pause:?}");
		let toast = app
			.world()
			.get_resource::<RecoveryToast>()
			.ok_or_else(|| anyhow::anyhow!("toast"))?;
		anyhow::ensure!(toast.message.contains("the game"));
		Ok(())
	}

	#[test]
	fn home_characters_and_loading_route_home() -> anyhow::Result<()> {
		let _guard = lock_recovery_tests();
		for flow in [GameFlow::Home, GameFlow::Characters, GameFlow::LoadingWorld] {
			let mut app = recovery_app(flow);
			app.world_mut().spawn(MenuScreen);
			trigger_missing_insert(&mut app);
			let last = run_until_recovery(&mut app);
			anyhow::ensure!(last.destination == RecoveryDestination::Home, "{flow:?}");
			let flow_now = *app.world().resource::<State<GameFlow>>().get();
			anyhow::ensure!(flow_now == GameFlow::Home, "{flow:?} became {flow_now:?}");
		}
		Ok(())
	}

	#[test]
	fn loop_guard_escalates_world_to_home_then_halts() -> anyhow::Result<()> {
		let _guard = lock_recovery_tests();
		let mut app = recovery_app(GameFlow::World);
		trigger_missing_insert(&mut app);
		for _ in 0..RECOVERY_LOOP_MAX {
			lock_inbox().push(RecoveryRequest {
				message: String::from("prior"),
				context: String::from("command"),
				source: RecoverySource::Main,
				at: Instant::now(),
			});
		}
		let last = run_until_recovery(&mut app);
		anyhow::ensure!(last.destination == RecoveryDestination::Home);

		app.insert_resource(State::new(GameFlow::Home));
		for _ in 0..RECOVERY_LOOP_MAX + 1 {
			lock_inbox().push(RecoveryRequest {
				message: String::from("home"),
				context: String::from("command"),
				source: RecoverySource::Main,
				at: Instant::now(),
			});
		}
		app.world_mut().insert_resource(RecoveryApply { in_progress: false });
		let last = run_until_recovery(&mut app);
		anyhow::ensure!(last.destination == RecoveryDestination::Halt);
		let exits = app.world_mut().resource_mut::<Messages<AppExit>>().drain().count();
		anyhow::ensure!(exits >= 1, "halt should emit AppExit");
		Ok(())
	}

	#[test]
	fn render_errors_log_without_routing_until_the_guard_trips() -> anyhow::Result<()> {
		let _guard = lock_recovery_tests();
		let mut app = recovery_app(GameFlow::World);
		lock_inbox().push(RecoveryRequest {
			message: String::from("render"),
			context: String::from("system"),
			source: RecoverySource::Render,
			at: Instant::now(),
		});
		let last = run_until_recovery(&mut app);
		anyhow::ensure!(last.destination == RecoveryDestination::Logged);
		anyhow::ensure!(matches!(
			app.world().resource::<NextState<GameFlow>>(),
			NextState::Unchanged
		));
		Ok(())
	}

	#[test]
	fn agent_runs_treat_recovery_as_failure() -> anyhow::Result<()> {
		let _guard = lock_recovery_tests();
		let mut app = recovery_app(GameFlow::Home);
		app.insert_resource(AgentDrive::from_args(maybraid_agent::AgentArgs {
			script: Some(std::path::PathBuf::from("walk.jsonl")),
			..Default::default()
		}));
		trigger_missing_insert(&mut app);
		let last = run_until_recovery(&mut app);
		anyhow::ensure!(last.destination == RecoveryDestination::Home);
		let exits = app.world_mut().resource_mut::<Messages<AppExit>>().drain().count();
		anyhow::ensure!(exits >= 1, "agent recovery must exit");
		Ok(())
	}

	#[test]
	fn last_session_roundtrip() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		anyhow::ensure!(read_last_session(&root).is_none());
		write_last_session(&root, PlaySession::Discovery);
		anyhow::ensure!(read_last_session(&root) == Some(PlaySession::Discovery));
		anyhow::ensure!(can_resume_world(&root, true));
		anyhow::ensure!(!can_resume_world(&root, false));
		clear_last_session(&root);
		anyhow::ensure!(read_last_session(&root).is_none());
		Ok(())
	}

	#[test]
	#[should_panic]
	fn strict_mode_panics_on_a_command_error() {
		let _guard = lock_recovery_tests();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins(RecoveryPlugin::strict())
			.init_state::<GameFlow>()
			.add_sub_state::<WorldPause>()
			.init_resource::<PlaySession>()
			.insert_resource(NextState::<ActiveGenerationMode>::Unchanged);
		trigger_missing_insert(&mut app);
		app.update();
	}
}
